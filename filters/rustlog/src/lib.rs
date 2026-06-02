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
//! Log levels are controlled on a per-module basis, and by default all logging is disabled except
//! for the `error` level.
//!
//! You can use [`RustLogFilterBuilder::from_default_env`] to configure the filter from the
//! `RUST_LOG` environment variable, or [`RustLogFilterBuilder::from_spec`] to configure the
//! filter by directly passing a specification string.
//!
//! The specification string is a comma-separated list of logging directives. A logging directive is
//! of the form:
//!
//! ```text
//! target=level
//! ```
//!
//! `target` is typically `path::to::module`, but it may also be set manually via the log macros.
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
//! given module or for the entire application. As with the logging levels, the letter case is not
//! significant; e.g., `debug`, `DEBUG`, and `dEbuG` all represent the same logging level.
//!
//! As the log level for a module is optional, the module to enable logging for is also optional. If
//! only a level is provided, then the global log level for all modules is set to this value.
//!
//! Some examples of valid values are:
//!
//! * `hello` turns on all logging for the 'hello' module
//! * `trace` turns on all logging for the application, regardless of its name
//! * `TRACE` turns on all logging for the application, regardless of its name (same as previous)
//! * `info` turns on all info logging
//! * `INFO` turns on all info logging (same as previous)
//! * `hello=debug` turns on debug logging for 'hello'
//! * `hello=DEBUG` turns on debug logging for 'hello' (same as previous)
//! * `hello,std::option` turns on hello, and std's option logging
//! * `error,hello=warn` turns on global error logging and also warn for hello
//! * `error,hello=off` turns on global error logging, but turn off logging for hello
//! * `off` turns off all logging for the application
//! * `OFF` turns off all logging for the application (same as previous)

use std::borrow::Cow;
use std::str::FromStr;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Filter;
use logforth_core::filter::FilterResult;
use logforth_core::record::FilterCriteria;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;

#[cfg(test)]
mod tests;

/// The default environment variable for filtering logs.
pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

/// A filter consists of one or more comma-separated directives which match on [`Record`].
///
/// Each directive may have a corresponding maximum verbosity [`Level`] which enables
/// records that match.
///
/// Less exclusive levels (like `trace` or `info`) are considered to be more verbose than more
/// exclusive levels (like `error` or `warn`).
///
/// Read more from the [crate level documentation](self) about the directive syntax and use cases.
///
/// [`Record`]: logforth_core::record::Record
#[derive(Debug)]
pub struct RustLogFilter {
    directives: Vec<Directive>,
}

impl RustLogFilter {
    fn from_directives(directives: Vec<Directive>) -> Self {
        let mut directives = directives;
        directives.sort();
        RustLogFilter { directives }
    }
}

impl Filter for RustLogFilter {
    fn enabled(&self, criteria: &FilterCriteria, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        let level = criteria.level();
        let target = criteria.target();

        // search for the longest match, the vector is assumed to be pre-sorted
        for directive in self.directives.iter().rev() {
            let name = directive.name.as_deref();
            if name.is_none_or(|n| target.starts_with(n)) {
                // longest match wins; return immediately
                return if directive.level.test(level) {
                    FilterResult::Neutral
                } else {
                    FilterResult::Reject
                };
            }
        }

        FilterResult::Reject
    }
}

impl From<LevelFilter> for RustLogFilter {
    fn from(filter: LevelFilter) -> Self {
        RustLogFilterBuilder::default().filter_level(filter).build()
    }
}

impl<'a> From<&'a str> for RustLogFilter {
    fn from(filter: &'a str) -> Self {
        RustLogFilterBuilder::from_spec(filter).build()
    }
}

impl FromStr for RustLogFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RustLogFilterBuilder::try_from_spec(s).map(|b| b.build())
    }
}

/// A builder for [`RustLogFilter`].
///
/// It can be used to parse a set of directives from a string before building an [`RustLogFilter`]
/// instance.
///
/// ## Example
///
/// ```
/// use logforth_filter_rustlog::RustLogFilterBuilder;
///
/// // Parse the filter from the default environment variable `RUST_LOG`.
/// let builder = RustLogFilterBuilder::from_default_env();
/// let filter = builder.build();
/// ```
#[derive(Debug, Default)]
pub struct RustLogFilterBuilder {
    directives: Vec<Directive>,
}

impl RustLogFilterBuilder {
    /// Initialize the filter builder from the environment using default variable name `RUST_LOG`.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the default environment variables:
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::from_default_env().build();
    /// ```
    pub fn from_default_env() -> Self {
        RustLogFilterBuilder::from_env(DEFAULT_FILTER_ENV)
    }

    /// Initialize the filter builder from the environment using default variable name `RUST_LOG`.
    /// If the variable is not set, the default value will be used.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the default environment variables, or fallback to the default
    /// value:
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::from_default_env_or("info").build();
    /// ```
    pub fn from_default_env_or<'a, V>(default: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        RustLogFilterBuilder::from_env_or(DEFAULT_FILTER_ENV, default)
    }

    /// Initialize the filter builder from the environment using specific variable name.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the using specific variable name:
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::from_env("MY_LOG").build();
    /// ```
    pub fn from_env<'a, V>(name: V) -> RustLogFilterBuilder
    where
        V: Into<Cow<'a, str>>,
    {
        let name = name.into();
        if let Ok(s) = std::env::var(&*name) {
            Self::from_spec(s)
        } else {
            Self::default()
        }
    }

    /// Initialize the filter builder from the environment using specific variable name.
    /// If the variable is not set, the default value will be used.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the using specific variable name, or fallback to the default
    /// value:
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::from_env_or("MY_LOG", "info").build();
    /// ```
    pub fn from_env_or<'a, 'b, E, V>(name: E, default: V) -> Self
    where
        E: Into<Cow<'a, str>>,
        V: Into<Cow<'b, str>>,
    {
        let name = name.into();
        if let Ok(s) = std::env::var(&*name) {
            Self::from_spec(s)
        } else {
            let default = default.into();
            Self::from_spec(default)
        }
    }

    /// Initialize the filter builder from the passed RUST_LOG specification. Malformed directives
    /// will be ignored with a warning printed to stderr.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the passed RUST_LOG specification:
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::from_spec("info,my_crate=debug").build();
    /// ```
    pub fn from_spec<'a, V>(spec: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        let spec = spec.into();
        let ParseResult { directives, errors } = parse_spec(&spec);
        for error in errors {
            eprintln!("warning: {error}, ignoring it");
        }
        let mut builder = RustLogFilterBuilder::default();
        for directive in directives {
            builder.upsert_directive(directive);
        }
        builder
    }

    /// Initialize the filter builder from the passed RUST_LOG specification.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the passed RUST_LOG specification:
    ///
    /// ```
    /// use logforth_filter_rustlog::RustLogFilterBuilder;
    ///
    /// let filter = RustLogFilterBuilder::try_from_spec("info,my_crate=debug")
    ///     .unwrap()
    ///     .build();
    /// ```
    pub fn try_from_spec<'a, V>(spec: V) -> Result<Self, Error>
    where
        V: Into<Cow<'a, str>>,
    {
        let spec = spec.into();
        let ParseResult { directives, errors } = parse_spec(&spec);
        if let Some(error) = errors.into_iter().next() {
            return Err(Error::new(error));
        }
        let mut builder = RustLogFilterBuilder::default();
        for directive in directives {
            builder.upsert_directive(directive);
        }
        Ok(builder)
    }

    /// Consume the builder to produce an [`RustLogFilter`].
    ///
    /// If the builder has no directives configured, a default directive of the `error` level will
    /// be added.
    pub fn build(self) -> RustLogFilter {
        let Self { directives } = self;

        if directives.is_empty() {
            RustLogFilter::from_directives(vec![Directive {
                name: None,
                level: LevelFilter::MoreSevereEqual(Level::Error),
            }])
        } else {
            RustLogFilter::from_directives(directives)
        }
    }

    /// Add a directive to the filter for a specific module.
    ///
    /// The given module will log at most the specified level provided.
    pub fn filter_module(mut self, module: impl Into<String>, level: LevelFilter) -> Self {
        let name = Some(module.into());
        self.upsert_directive(Directive { name, level });
        self
    }

    /// Add a directive to the filter for all modules.
    ///
    /// All log messages will log at most the specified level provided.
    pub fn filter_level(mut self, level: LevelFilter) -> Self {
        self.upsert_directive(Directive { name: None, level });
        self
    }

    /// Insert the directive or update existing directive with the same name.
    fn upsert_directive(&mut self, mut directive: Directive) {
        if let Some(pos) = self
            .directives
            .iter()
            .position(|d| d.name == directive.name)
        {
            std::mem::swap(&mut self.directives[pos], &mut directive);
        } else {
            self.directives.push(directive);
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Directive {
    name: Option<String>,
    level: LevelFilter,
}

impl PartialOrd for Directive {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Directive {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let this_len = self.name.as_ref().map(|n| n.len()).unwrap_or(0);
        let other_len = other.name.as_ref().map(|n| n.len()).unwrap_or(0);
        Ord::cmp(&this_len, &other_len)
    }
}

#[derive(Debug)]
struct ParseResult {
    directives: Vec<Directive>,
    errors: Vec<String>,
}

/// Parse a logging specification string and return a vector with log directives.
///
/// The specification string is a comma-separated list of directives, e.g.:
///
/// * `info`
/// * `my_crate=debug,other_crate=info`
/// * `my_crate=debug,other_crate=info,trace`
/// * `my_target=debug,other_target=info,trace`
fn parse_spec(spec: &str) -> ParseResult {
    let mut directives = vec![];
    let mut errors = vec![];

    for s in spec.split(',').map(str::trim) {
        if s.is_empty() {
            continue;
        }

        let mut parts = s.split('=');
        let part0 = parts.next().map(str::trim);
        let part1 = parts.next().map(str::trim);

        let Some(part0) = part0 else {
            errors.push(format!("malformed logging spec '{s}'"));
            continue;
        };

        if parts.next().is_some() {
            errors.push(format!("malformed logging spec '{s}'"));
            continue;
        }

        let (level, name) = match part1 {
            None => {
                if let Some(level) = from_str_for_env(part0) {
                    // if the single argument is a log level string, treat that as a global fallback
                    (level, None)
                } else {
                    (LevelFilter::All, Some(part0.to_owned()))
                }
            }
            Some(part1) => {
                if part1.is_empty() {
                    (LevelFilter::All, Some(part0.to_owned()))
                } else if let Some(level) = from_str_for_env(part1) {
                    (level, Some(part0.to_owned()))
                } else {
                    errors.push(format!("malformed logging spec '{part1}'"));
                    continue;
                }
            }
        };

        directives.push(Directive { name, level });
    }

    ParseResult { directives, errors }
}

fn from_str_for_env(text: &str) -> Option<LevelFilter> {
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
