// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A filter that follows the famous `RUST_LOG` directive pattern.
//!
//! Log levels are controlled by prefix matching (typically on module paths), and by default all
//! logging is disabled except for the `error` level.
//!
//! You can use [`RustLogFilterBuilder::from_default_env`] to configure the filter from the
//! `RUST_LOG` environment variable, or [`RustLogFilterBuilder::from_spec`] to configure the
//! filter by directly passing a specification string.
//!
//! The specification string is a comma-separated list of logging directives. A logging directive is
//! of the form:
//!
//! ```text
//! prefix=level
//! ```
//!
//! `prefix` is typically a `path::to::module` prefix matched against [`Record::module_path`], or an
//! explicit `"logforth.target"` key-value entry when present (for example, when bridging `log`
//! records with an overridden target).
//!
//! The path to the module is rooted in the name of the crate it was compiled for. Thus, if your
//! program is contained in a file `hello.rs`, for example, to turn on logging for this file you
//! would use a value of `RUST_LOG=hello`. Furthermore, this path is a prefix-search, so all modules
//! nested in the specified module will also have logging enabled.
//!
//! When providing the crate name or a module path, explicitly specifying the log level is optional.
//! If omitted, all logging for the item (and its children) will be enabled.
//!
//! The names of the log levels that may be specified correspond to the variations of the [`Level`]
//! enum. The most common used levels include:
//!
//! * `fatal`
//! * `error`
//! * `warn`
//! * `info`
//! * `debug`
//! * `trace`
//!
//! There is also a pseudo logging level, `off`, which may be specified to disable all logging for a
//! given prefix or for the entire application. As with the logging levels, the letter case is not
//! significant; e.g., `debug`, `DEBUG`, and `dEbuG` all represent the same logging level.
//!
//! As the log level for a prefix is optional, the prefix to enable logging for is also optional. If
//! only a level is provided, then the fallback log level for all unmatched records is set to this
//! value.
//!
//! Some examples of valid values are:
//!
//! * `hello` turns on all logging for the 'hello' module prefix
//! * `trace` turns on all logging for the application
//! * `TRACE` turns on all logging for the application (same as previous)
//! * `info` turns on all info logging
//! * `INFO` turns on all info logging (same as previous)
//! * `hello=debug` turns on debug logging for 'hello'
//! * `hello=DEBUG` turns on debug logging for 'hello' (same as previous)
//! * `hello,std::option` turns on hello, and std's option logging
//! * `error,hello=warn` turns on fallback error logging and also warn for hello
//! * `error,hello=off` turns on fallback error logging, but turns off logging for hello
//! * `off` turns off all logging for the application
//! * `OFF` turns off all logging for the application (same as previous)

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::env::VarError;
use std::str::FromStr;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Filter;
use logforth_core::filter::FilterResult;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;

/// The default environment variable for filtering logs.
pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

// Keep synchronized with "Appendix: Key-Value Naming Conventions" in DESIGN.md.
const KEY_LOGFORTH_TARGET: &str = "logforth.target";

const fn level_bit(level: Level) -> u32 {
    1u32 << (level as u8 - 1)
}

/// A filter consisting of a fallback rule and zero or more prefix directives.
///
/// Each directive specifies a [`LevelFilter`] for a string prefix or as the unmatched fallback.
/// When evaluating a [`Record`], `RustLogFilter` matches prefixes against an explicit
/// `"logforth.target"` key-value entry when present and falls back to [`Record::module_path`].
///
/// Read more from the [crate documentation](self) about the directive syntax and use cases.
#[derive(Clone, Debug)]
pub struct RustLogFilter {
    prefixes: Vec<PrefixDirective>,
    fallback: LevelFilter,
    may_neutral: u32,
    always_neutral: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrefixDirective {
    prefix: String,
    level: LevelFilter,
}

impl RustLogFilter {
    fn new(fallback: LevelFilter, mut prefixes: Vec<PrefixDirective>) -> Self {
        // Longest prefix first so the first `starts_with` hit is the most specific match.
        prefixes.sort_by(|a, b| {
            b.prefix
                .len()
                .cmp(&a.prefix.len())
                .then_with(|| a.prefix.cmp(&b.prefix))
        });

        let mut may_neutral = 0u32;
        let mut always_neutral = 0u32;
        for level in Level::ALL {
            let fallback_ok = fallback.test(level);
            if fallback_ok || prefixes.iter().any(|d| d.level.test(level)) {
                may_neutral |= level_bit(level);
            }
            if fallback_ok && prefixes.iter().all(|d| d.level.test(level)) {
                always_neutral |= level_bit(level);
            }
        }

        Self {
            prefixes,
            fallback,
            may_neutral,
            always_neutral,
        }
    }
}

impl Filter for RustLogFilter {
    fn filter_level(&self, level: Level) -> FilterResult {
        if (self.may_neutral & level_bit(level)) != 0 {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }

    fn filter(&self, record: &Record, _: &dyn Diagnostic) -> FilterResult {
        let level = record.level();
        let bit = level_bit(level);
        if (self.may_neutral & bit) == 0 {
            return FilterResult::Reject;
        }
        if (self.always_neutral & bit) != 0 {
            return FilterResult::Neutral;
        }

        let target = record
            .key_values()
            .get(KEY_LOGFORTH_TARGET)
            .and_then(|v| v.to_str())
            .or_else(|| record.module_path())
            .unwrap_or("");

        let rule = self
            .prefixes
            .iter()
            .find(|d| target.starts_with(&d.prefix))
            .map_or(self.fallback, |d| d.level);

        if rule.test(level) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl From<LevelFilter> for RustLogFilter {
    fn from(filter: LevelFilter) -> Self {
        RustLogFilterBuilder::default().fallback(filter).build()
    }
}

impl<'a> TryFrom<&'a str> for RustLogFilter {
    type Error = Error;

    fn try_from(filter: &'a str) -> Result<Self, Self::Error> {
        RustLogFilterBuilder::from_spec(filter).map(RustLogFilterBuilder::build)
    }
}

impl FromStr for RustLogFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RustLogFilterBuilder::from_spec(s).map(RustLogFilterBuilder::build)
    }
}

/// A builder for [`RustLogFilter`].
///
/// It can be used to parse a set of directives from a string and/or configure programmatic rules
/// before building a [`RustLogFilter`] instance.
///
/// ## Example
///
/// ```
/// use logforth_filter_rustlog::RustLogFilterBuilder;
///
/// // Parse the filter from the default environment variable `RUST_LOG`.
/// let builder = RustLogFilterBuilder::from_default_env().unwrap();
/// let filter = builder.build();
/// ```
#[derive(Debug, Default)]
pub struct RustLogFilterBuilder {
    fallback: Option<LevelFilter>,
    prefixes: BTreeMap<String, LevelFilter>,
}

impl RustLogFilterBuilder {
    /// Initialize the filter builder from the `RUST_LOG` environment variable.
    ///
    /// If `RUST_LOG` is unset, returns an empty builder (which defaults to `error` on
    /// [`Self::build`]).
    ///
    /// # Errors
    ///
    /// Returns an error if `RUST_LOG` is not valid UTF-8 or contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_default_env().unwrap();
    /// let filter = builder.build();
    /// ```
    pub fn from_default_env() -> Result<Self, Error> {
        Self::from_env(DEFAULT_FILTER_ENV)
    }

    /// Initialize the filter builder from the `RUST_LOG` environment variable, falling back to
    /// `default` when `RUST_LOG` is unset.
    ///
    /// # Errors
    ///
    /// Returns an error if `RUST_LOG` is not valid UTF-8 or if the effective specification
    /// contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_default_env_or("info").unwrap();
    /// let filter = builder.build();
    /// ```
    pub fn from_default_env_or(default: impl AsRef<str>) -> Result<Self, Error> {
        Self::from_env_or(DEFAULT_FILTER_ENV, default)
    }

    /// Initialize the filter builder from the named environment variable.
    ///
    /// If the variable is unset, returns an empty builder (which defaults to `error` on
    /// [`Self::build`]).
    ///
    /// # Errors
    ///
    /// Returns an error if the variable is not valid UTF-8 or contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_env("MY_LOG").unwrap();
    /// let filter = builder.build();
    /// ```
    pub fn from_env(name: impl AsRef<str>) -> Result<Self, Error> {
        match read_env_var(name.as_ref())? {
            Some(spec) => Self::from_spec(spec),
            None => Ok(Self::default()),
        }
    }

    /// Initialize the filter builder from the named environment variable, falling back to
    /// `default` when the variable is unset.
    ///
    /// # Errors
    ///
    /// Returns an error if the variable is not valid UTF-8 or if the effective specification
    /// contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_env_or("MY_LOG", "info").unwrap();
    /// let filter = builder.build();
    /// ```
    pub fn from_env_or(name: impl AsRef<str>, default: impl AsRef<str>) -> Result<Self, Error> {
        match read_env_var(name.as_ref())? {
            Some(spec) => Self::from_spec(spec),
            None => Self::from_spec(default),
        }
    }

    /// Initialize the filter builder from a `RUST_LOG` specification string.
    ///
    /// # Errors
    ///
    /// Returns an error on the first malformed directive in `spec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_spec("info,my_crate=debug").unwrap();
    /// let filter = builder.build();
    /// ```
    pub fn from_spec(spec: impl AsRef<str>) -> Result<Self, Error> {
        let mut builder = Self::default();
        for item in spec.as_ref().split(',').map(str::trim) {
            if item.is_empty() {
                continue;
            }

            match item.split_once('=') {
                None => match parse_level_filter(item) {
                    Some(level) => builder.fallback = Some(level),
                    None => {
                        builder.prefixes.insert(item.to_owned(), LevelFilter::All);
                    }
                },
                Some((raw_prefix, raw_level)) => {
                    let prefix = raw_prefix.trim();
                    let raw_level = raw_level.trim();
                    if prefix.is_empty() || raw_level.contains('=') {
                        return Err(Error::new(format!("malformed logging spec '{item}'")));
                    }
                    let level = if raw_level.is_empty() {
                        LevelFilter::All
                    } else {
                        parse_level_filter(raw_level).ok_or_else(|| {
                            Error::new(format!("malformed logging spec '{raw_level}'"))
                        })?
                    };
                    builder.prefixes.insert(prefix.to_owned(), level);
                }
            }
        }
        Ok(builder)
    }

    /// Set the fallback [`LevelFilter`] applied when no prefix directive matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::default()
    ///     .fallback(LevelFilter::MoreSevereEqual(Level::Info))
    ///     .build();
    /// ```
    pub fn fallback(mut self, filter: LevelFilter) -> Self {
        self.fallback = Some(filter);
        self
    }

    /// Set the [`LevelFilter`] for records whose `"logforth.target"` (when present) or
    /// [`Record::module_path`] starts with `prefix` (longest prefix match wins).
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::default()
    ///     .fallback(LevelFilter::MoreSevereEqual(Level::Info))
    ///     .prefix("my_crate::db", LevelFilter::MoreSevereEqual(Level::Warn))
    ///     .build();
    /// ```
    pub fn prefix(mut self, prefix: impl Into<String>, filter: LevelFilter) -> Self {
        self.prefixes.insert(prefix.into(), filter);
        self
    }

    /// Consume the builder to produce a [`RustLogFilter`].
    ///
    /// If the builder has no directives configured, a default fallback directive of the `error`
    /// level will be added.
    pub fn build(self) -> RustLogFilter {
        let fallback = match self.fallback {
            Some(fallback) => fallback,
            None if self.prefixes.is_empty() => LevelFilter::MoreSevereEqual(Level::Error),
            None => LevelFilter::Off,
        };
        let prefixes = self
            .prefixes
            .into_iter()
            .map(|(prefix, level)| PrefixDirective { prefix, level })
            .collect();
        RustLogFilter::new(fallback, prefixes)
    }
}

fn read_env_var(name: &str) -> Result<Option<String>, Error> {
    match std::env::var(name) {
        Ok(spec) => Ok(Some(spec)),
        Err(VarError::NotPresent) => Ok(None),
        Err(err @ VarError::NotUnicode(_)) => Err(Error::new(format!(
            "environment variable '{name}' is not valid UTF-8"
        ))
        .with_source(err)),
    }
}

fn parse_level_filter(text: &str) -> Option<LevelFilter> {
    if let Ok(level) = Level::from_str(text) {
        Some(LevelFilter::MoreSevereEqual(level))
    } else if text.eq_ignore_ascii_case("off") {
        Some(LevelFilter::Off)
    } else if text.eq_ignore_ascii_case("all") {
        Some(LevelFilter::All)
    } else {
        None
    }
}
