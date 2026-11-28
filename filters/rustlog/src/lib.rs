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

//! Prefix and fallback log filtering compatible with `RUST_LOG` directive syntax.
//!
//! [`RustLogFilter`] implements [`Filter`] using an unmatched fallback [`LevelFilter`] and zero or
//! more string-prefix rules constructed via [`RustLogFilterBuilder`]:
//! - Use [`RustLogFilterBuilder::from_default_env`] or
//!   [`RustLogFilterBuilder::from_default_env_or`] to read directives from [`DEFAULT_FILTER_ENV`]
//!   (`RUST_LOG`).
//! - Use [`RustLogFilterBuilder::from_env`] or [`RustLogFilterBuilder::from_env_or`] to read from a
//!   custom environment variable.
//! - Use [`RustLogFilterBuilder::from_spec`], [`FromStr`], or [`TryFrom<&str>`] to parse a
//!   comma-separated specification string directly, or configure rules programmatically with
//!   [`RustLogFilterBuilder::fallback`] and [`RustLogFilterBuilder::prefix`].
//!
//! # Directive syntax
//!
//! A specification is a comma-separated list of directives (`directive1,directive2,...`). Leading
//! and trailing ASCII/Unicode whitespace around commas and `=` is trimmed, empty segments are
//! ignored, and later directives override earlier ones with the same prefix (or fallback):
//!
//! | Form           | Effect                                                                        |
//! | :------------- | :---------------------------------------------------------------------------- |
//! | `level`        | Sets the fallback threshold for records matching no prefix rule.              |
//! | `=level`       | Equivalent to `level` (empty prefix sets the fallback threshold).             |
//! | `=`            | Enables all levels ([`LevelFilter::All`]) as the fallback threshold.          |
//! | `prefix=level` | Sets the threshold for targets starting with `prefix`.                        |
//! | `prefix`       | Enables all levels ([`LevelFilter::All`]) for targets starting with `prefix`. |
//! | `prefix=`      | Equivalent to `prefix` ([`LevelFilter::All`]).                                |
//!
//! Level names are ASCII case-insensitive and accept all 24 [`Level`] variants (`trace`,
//! `trace2`..`trace4`, `debug`..`debug4`, `info`..`info4`, `warn`..`warn4`, `error`..`error4`,
//! `fatal`..`fatal4`) as well as `off` ([`LevelFilter::Off`]) and `all` ([`LevelFilter::All`]).
//! Malformed directives fail fast by returning [`Error`].
//!
//! # Target matching and dispatch semantics
//!
//! - **Target precedence**: Prefix rules match an explicit `"logforth.target"` string entry in
//!   [`Record::key_values`] when present (such as custom targets bridged from `log`), falling back
//!   to [`Record::module_path`]. Longest matching prefix wins.
//! - **Default fallback**: Building an empty [`RustLogFilterBuilder`] enables `error` and more
//!   severe levels. Building a builder that configures only prefix rules sets the unmatched
//!   fallback to [`LevelFilter::Off`] so unmentioned crates remain silent.
//! - **Narrowing filter**: Accepted records return [`FilterResult::Neutral`] (never
//!   [`FilterResult::Accept`]) so subsequent filters in the same dispatch continue to run; rejected
//!   records return [`FilterResult::Reject`].
//!
//! # Examples
//!
//! ```
//! use logforth_core::LoggerBuilder;
//! use logforth_core::append::Stderr;
//! use logforth_filter_rustlog::RustLogFilterBuilder;
//!
//! let builder = RustLogFilterBuilder::from_spec("info,my_app::db=debug,hyper=warn")?;
//! let filter = builder.build();
//! let logger = LoggerBuilder::new()
//!     .dispatch(|d| d.filter(filter).append(Stderr::new()))
//!     .build();
//! # drop(logger);
//! # Ok::<(), logforth_core::Error>(())
//! ```

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

/// Default environment variable name (`"RUST_LOG"`) read by
/// [`RustLogFilterBuilder::from_default_env`] and [`RustLogFilterBuilder::from_default_env_or`].
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
            .or_else(|| record.module_path());

        let rule = match target {
            Some(t) if !t.is_empty() => self
                .prefixes
                .iter()
                .find(|d| t.starts_with(&d.prefix))
                .map_or(self.fallback, |d| d.level),
            _ => self.fallback,
        };

        if rule.test(level) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl From<LevelFilter> for RustLogFilter {
    fn from(filter: LevelFilter) -> Self {
        RustLogFilterBuilder::new().fallback(filter).build()
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

/// A builder that parses `RUST_LOG` specifications and configures prefix rules for
/// [`RustLogFilter`].
///
/// # Examples
///
/// ```
/// use logforth_filter_rustlog::RustLogFilterBuilder;
///
/// let builder = RustLogFilterBuilder::from_default_env()?;
/// let filter = builder.build();
/// # drop(filter);
/// # Ok::<(), logforth_core::Error>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct RustLogFilterBuilder {
    fallback: Option<LevelFilter>,
    prefixes: BTreeMap<String, LevelFilter>,
}

impl RustLogFilterBuilder {
    /// Creates an empty [`RustLogFilterBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::new()
    ///     .fallback(LevelFilter::MoreSevereEqual(Level::Info))
    ///     .build();
    /// ```
    pub const fn new() -> Self {
        Self {
            fallback: None,
            prefixes: BTreeMap::new(),
        }
    }

    /// Creates a builder from the [`DEFAULT_FILTER_ENV`] (`RUST_LOG`) environment variable.
    ///
    /// If `RUST_LOG` is unset, returns an empty builder (which defaults to `error` on
    /// [`Self::build`]).
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if `RUST_LOG` is not valid UTF-8 or contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_default_env()?;
    /// let filter = builder.build();
    /// # drop(filter);
    /// # Ok::<(), logforth_core::Error>(())
    /// ```
    pub fn from_default_env() -> Result<Self, Error> {
        Self::from_env(DEFAULT_FILTER_ENV)
    }

    /// Creates a builder from [`DEFAULT_FILTER_ENV`] (`RUST_LOG`), parsing `default` when
    /// `RUST_LOG` is unset.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if `RUST_LOG` is not valid UTF-8 or if the selected specification
    /// contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_default_env_or("info")?;
    /// let filter = builder.build();
    /// # drop(filter);
    /// # Ok::<(), logforth_core::Error>(())
    /// ```
    pub fn from_default_env_or(default: impl AsRef<str>) -> Result<Self, Error> {
        Self::from_env_or(DEFAULT_FILTER_ENV, default)
    }

    /// Creates a builder from the environment variable `name`.
    ///
    /// If `name` is unset, returns an empty builder (which defaults to `error` on
    /// [`Self::build`]).
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the variable is not valid UTF-8 or contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_env("MY_LOG")?;
    /// let filter = builder.build();
    /// # drop(filter);
    /// # Ok::<(), logforth_core::Error>(())
    /// ```
    pub fn from_env(name: impl AsRef<str>) -> Result<Self, Error> {
        match read_env_var(name.as_ref())? {
            Some(spec) => Self::from_spec(spec),
            None => Ok(Self::new()),
        }
    }

    /// Creates a builder from the environment variable `name`, parsing `default` when `name` is
    /// unset.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the variable is not valid UTF-8 or if the selected specification
    /// contains a malformed directive.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_env_or("MY_LOG", "info")?;
    /// let filter = builder.build();
    /// # drop(filter);
    /// # Ok::<(), logforth_core::Error>(())
    /// ```
    pub fn from_env_or(name: impl AsRef<str>, default: impl AsRef<str>) -> Result<Self, Error> {
        match read_env_var(name.as_ref())? {
            Some(spec) => Self::from_spec(spec),
            None => Self::from_spec(default),
        }
    }

    /// Creates a builder by parsing a comma-separated `RUST_LOG` specification string.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] on the first malformed directive in `spec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let builder = RustLogFilterBuilder::from_spec("info,my_crate=debug")?;
    /// let filter = builder.build();
    /// # drop(filter);
    /// # Ok::<(), logforth_core::Error>(())
    /// ```
    pub fn from_spec(spec: impl AsRef<str>) -> Result<Self, Error> {
        let mut builder = Self::new();
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
                    if raw_level.contains('=') {
                        return Err(Error::new(format!("malformed logging spec '{item}'")));
                    }
                    let level = if raw_level.is_empty() {
                        LevelFilter::All
                    } else {
                        parse_level_filter(raw_level).ok_or_else(|| {
                            Error::new(format!("malformed logging spec '{raw_level}'"))
                        })?
                    };
                    if prefix.is_empty() {
                        builder.fallback = Some(level);
                    } else {
                        builder.prefixes.insert(prefix.to_owned(), level);
                    }
                }
            }
        }
        Ok(builder)
    }

    /// Sets the fallback [`LevelFilter`] applied when no prefix directive matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::new()
    ///     .fallback(LevelFilter::MoreSevereEqual(Level::Info))
    ///     .build();
    /// ```
    pub fn fallback(mut self, filter: LevelFilter) -> Self {
        self.fallback = Some(filter);
        self
    }

    /// Sets the [`LevelFilter`] for records whose `"logforth.target"` (when present) or
    /// [`Record::module_path`] starts with `prefix` (longest prefix match wins).
    ///
    /// Passing an empty `prefix` (`""`) sets the unmatched fallback [`LevelFilter`], equivalent to
    /// [`Self::fallback`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::new()
    ///     .fallback(LevelFilter::MoreSevereEqual(Level::Info))
    ///     .prefix("my_crate::db", LevelFilter::MoreSevereEqual(Level::Warn))
    ///     .build();
    /// ```
    pub fn prefix(mut self, prefix: impl Into<String>, filter: LevelFilter) -> Self {
        let prefix = prefix.into();
        if prefix.is_empty() {
            self.fallback = Some(filter);
        } else {
            self.prefixes.insert(prefix, filter);
        }
        self
    }

    /// Builds the [`RustLogFilter`].
    ///
    /// If neither a fallback nor any prefix directive was configured, defaults the fallback to
    /// `LevelFilter::MoreSevereEqual(Level::Error)`. If only prefix directives were configured,
    /// unmatched targets default to [`LevelFilter::Off`].
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
