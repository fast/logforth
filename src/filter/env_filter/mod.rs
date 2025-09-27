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

//! A filter that can be configured via environment variables.
//!
//! Log levels are controlled on a per-module basis, and by default all logging is disabled except
//! for the `error` level.
//!
//! Despite having "env" in its name, you can also use [`EnvFilterBuilder::from_spec`] to configure
//! the filter by directly passing a specification string.
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
//! The path to the module is rooted in the name of the crate it was compiled  for, so if your
//! program is contained in a file `hello.rs`, for example, to turn on logging for this file you
//! would use a value of `RUST_LOG=hello`. Furthermore, this path is a prefix-search, so all modules
//! nested in the specified module will also have logging enabled.
//!
//! When providing the crate name or a module path, explicitly specifying the log level is optional.
//! If omitted, all logging for the item (and its children) will be enabled.
//!
//! The names of the log levels that may be specified correspond to the variations of the
//! [`log::Level`] enum from the `log` crate. They are:
//!
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
//! Some examples of valid values  are:
//!
//! * `hello` turns on all logging for the 'hello' module
//! * `trace` turns on all logging for the application, regardless of its name
//! * `TRACE` turns on all logging for the application, regardless of its name (same as previous)
//! * `info` turns on all info logging
//! * `INFO` turns on all info logging (same as previous)
//! * `hello=debug` turns on debug logging for 'hello'
//! * `hello=DEBUG` turns on debug logging for 'hello' (same as previous)
//! * `hello,std::option` turns on hello, and std's option logging
//! * `error,hello=warn` turn on global error logging and also warn for hello
//! * `error,hello=off`  turn on global error logging, but turn off logging for hello
//! * `off` turns off all logging for the application
//! * `OFF` turns off all logging for the application (same as previous)

use std::borrow::Cow;
use std::str::FromStr;

use crate::Diagnostic;
use crate::Error;
use crate::Filter;
use crate::LevelFilter;
use crate::Metadata;
use crate::filter::FilterResult;

#[cfg(test)]
mod tests;

/// The default environment variable for filtering logs.
pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

/// A filter consists of one or more comma-separated directives which match on [`log::Record`].
///
/// Each directive may have a corresponding maximum verbosity [`level`][log::Level] which enables
/// records that match.
///
/// Less exclusive levels (like `trace` or `info`) are considered to be more verbose than more
/// exclusive levels (like `error` or `warn`).
///
/// Read more from the [module level documentation](self) about the directive syntax and use cases.
#[derive(Debug)]
pub struct EnvFilter {
    directives: Vec<Directive>,
}

impl EnvFilter {
    fn from_directives(directives: Vec<Directive>) -> Self {
        let mut directives = directives;
        directives.sort();
        EnvFilter { directives }
    }
}

impl Filter for EnvFilter {
    fn enabled(&self, metadata: &Metadata, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        let level = metadata.level();
        let target = metadata.target();

        // search for the longest match, the vector is assumed to be pre-sorted
        for directive in self.directives.iter().rev() {
            let name = directive.name.as_deref();
            if name.is_none_or(|n| target.starts_with(n)) {
                // longest match wins; return immediately
                return if directive.level < level {
                    FilterResult::Reject
                } else {
                    FilterResult::Neutral
                };
            }
        }

        FilterResult::Reject
    }
}

impl From<LevelFilter> for EnvFilter {
    fn from(filter: LevelFilter) -> Self {
        EnvFilterBuilder::default().filter_level(filter).build()
    }
}

impl<'a> From<&'a str> for EnvFilter {
    fn from(filter: &'a str) -> Self {
        EnvFilterBuilder::from_spec(filter).build()
    }
}

impl FromStr for EnvFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EnvFilterBuilder::try_from_spec(s).map(|b| b.build())
    }
}

/// A builder for [`EnvFilter`].
///
/// It can be used to parse a set of directives from a string before building an [`EnvFilter`]
/// instance.
///
/// ## Example
///
/// ```
/// use logforth::filter::env_filter::EnvFilterBuilder;
///
/// // Parse the filter from the default environment variable `RUST_LOG`.
/// let builder = EnvFilterBuilder::from_default_env();
/// let filter = builder.build();
/// ```
#[derive(Debug, Default)]
pub struct EnvFilterBuilder {
    directives: Vec<Directive>,
}

impl EnvFilterBuilder {
    /// Initializes the filter builder from the environment using default variable name `RUST_LOG`.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the default environment variables:
    ///
    /// ```
    /// use logforth::filter::env_filter::EnvFilterBuilder;
    /// let filter = EnvFilterBuilder::from_default_env().build();
    /// ```
    pub fn from_default_env() -> Self {
        EnvFilterBuilder::from_env(DEFAULT_FILTER_ENV)
    }

    /// Initializes the filter builder from the environment using default variable name `RUST_LOG`.
    /// If the variable is not set, the default value will be used.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the default environment variables, or fallback to the default
    /// value:
    ///
    /// ```
    /// use logforth::filter::env_filter::EnvFilterBuilder;
    /// let filter = EnvFilterBuilder::from_default_env_or("info").build();
    /// ```
    pub fn from_default_env_or<'a, V>(default: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        EnvFilterBuilder::from_env_or(DEFAULT_FILTER_ENV, default)
    }

    /// Initializes the filter builder from the environment using specific variable name.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the using specific variable name:
    ///
    /// ```
    /// use logforth::filter::env_filter::EnvFilterBuilder;
    /// let filter = EnvFilterBuilder::from_env("MY_LOG").build();
    /// ```
    pub fn from_env<'a, V>(name: V) -> EnvFilterBuilder
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

    /// Initializes the filter builder from the environment using specific variable name.
    /// If the variable is not set, the default value will be used.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the using specific variable name, or fallback to the default
    /// value:
    ///
    /// ```
    /// use logforth::filter::env_filter::EnvFilterBuilder;
    /// let filter = EnvFilterBuilder::from_env_or("MY_LOG", "info").build();
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

    /// Initializes the filter builder from the passed RUST_LOG specification. Malformed directives
    /// will be ignored with a warning printed to stderr.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the passed RUST_LOG specification:
    ///
    /// ```
    /// use logforth::filter::env_filter::EnvFilterBuilder;
    /// let filter = EnvFilterBuilder::from_spec("info,my_crate=debug").build();
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
        let mut builder = EnvFilterBuilder::default();
        for directive in directives {
            builder.upsert_directive(directive);
        }
        builder
    }

    /// Initializes the filter builder from the passed RUST_LOG specification.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the passed RUST_LOG specification:
    ///
    /// ```
    /// use logforth::filter::env_filter::EnvFilterBuilder;
    /// let filter = EnvFilterBuilder::try_from_spec("info,my_crate=debug")
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
        let mut builder = EnvFilterBuilder::default();
        for directive in directives {
            builder.upsert_directive(directive);
        }
        Ok(builder)
    }

    /// Consume the builder to produce an [`EnvFilter`].
    ///
    /// If the builder has no directives configured, a default directive of the `error` level will
    /// be added.
    pub fn build(self) -> EnvFilter {
        let Self { directives } = self;

        if directives.is_empty() {
            EnvFilter::from_directives(vec![Directive {
                name: None,
                level: LevelFilter::Error,
            }])
        } else {
            EnvFilter::from_directives(directives)
        }
    }

    /// Adds a directive to the filter for a specific module.
    ///
    /// The given module will log at most the specified level provided.
    pub fn filter_module(mut self, module: impl Into<String>, level: LevelFilter) -> Self {
        let name = Some(module.into());
        self.upsert_directive(Directive { name, level });
        self
    }

    /// Adds a directive to the filter for all modules.
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
                if let Ok(level) = part0.parse() {
                    // if the single argument is a log level string, treat that as a global fallback
                    (level, None)
                } else {
                    (LevelFilter::Trace, Some(part0.to_owned()))
                }
            }
            Some(part1) => {
                if part1.is_empty() {
                    (LevelFilter::Trace, Some(part0.to_owned()))
                } else if let Ok(level) = part1.parse() {
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
