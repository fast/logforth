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

//! Filtering for log records.

use std::borrow::Cow;
use std::env;
use std::fmt;
use std::mem;

use log::Level;
use log::LevelFilter;
use log::Metadata;
use log::Record;

use crate::Diagnostic;
use crate::Filter;
use crate::filter::FilterResult;

#[cfg(test)]
mod tests;

/// The default environment variable for filtering logs.
pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

#[derive(Debug)]
struct FilterOp {
    filter: regex::Regex,
}

impl FilterOp {
    fn new(spec: &str) -> Result<Self, String> {
        match regex::Regex::new(spec) {
            Ok(filter) => Ok(Self { filter }),
            Err(err) => Err(err.to_string()),
        }
    }

    fn is_match(&self, s: &str) -> bool {
        self.filter.is_match(s)
    }
}

impl fmt::Display for FilterOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.filter.fmt(f)
    }
}

#[derive(Debug)]
struct Directive {
    name: Option<String>,
    level: LevelFilter,
}

// Check whether a level and target are enabled by the set of directives.
fn enabled(directives: &[Directive], level: Level, target: &str) -> bool {
    // Search for the longest match, the vector is assumed to be pre-sorted.
    for directive in directives.iter().rev() {
        match directive.name {
            Some(ref name) if !target.starts_with(&**name) => {}
            Some(..) | None => return level <= directive.level,
        }
    }
    false
}

#[derive(Default, Debug)]
struct ParseResult {
    directives: Vec<Directive>,
    filter: Option<FilterOp>,
    errors: Vec<String>,
}

impl ParseResult {
    fn add_directive(&mut self, directive: Directive) {
        self.directives.push(directive);
    }

    fn set_filter(&mut self, filter: FilterOp) {
        self.filter = Some(filter);
    }

    fn add_error(&mut self, message: String) {
        self.errors.push(message);
    }

    fn ok(self) -> Result<(Vec<Directive>, Option<FilterOp>), ParseError> {
        let Self {
            directives,
            filter,
            errors,
        } = self;
        if let Some(error) = errors.into_iter().next() {
            Err(ParseError { details: error })
        } else {
            Ok((directives, filter))
        }
    }
}

/// Error during logger directive parsing process.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    details: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error parsing logger filter: {}", self.details)
    }
}

impl std::error::Error for ParseError {}

/// Parse a logging specification string (e.g: `crate1,crate2::mod3,crate3::x=error/foo`)
/// and return a vector with log directives.
fn parse_spec(spec: &str) -> ParseResult {
    let mut result = ParseResult::default();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        result.add_error(format!("invalid logging spec '{spec}' (too many '/'s)"));
        return result;
    }
    if let Some(m) = mods {
        for s in m.split(',').map(|ss| ss.trim()) {
            if s.is_empty() {
                continue;
            }
            let mut parts = s.split('=');
            let (log_level, name) =
                match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
                    (Some(part0), None, None) => {
                        // if the single argument is a log-level string or number,
                        // treat that as a global fallback
                        match part0.parse() {
                            Ok(num) => (num, None),
                            Err(_) => (LevelFilter::max(), Some(part0)),
                        }
                    }
                    (Some(part0), Some(""), None) => (LevelFilter::max(), Some(part0)),
                    (Some(part0), Some(part1), None) => {
                        if let Ok(num) = part1.parse() {
                            (num, Some(part0))
                        } else {
                            result.add_error(format!("invalid logging spec '{part1}'"));
                            continue;
                        }
                    }
                    _ => {
                        result.add_error(format!("invalid logging spec '{s}'"));
                        continue;
                    }
                };

            result.add_directive(Directive {
                name: name.map(|s| s.to_owned()),
                level: log_level,
            });
        }
    }

    if let Some(filter) = filter {
        match FilterOp::new(filter) {
            Ok(filter_op) => result.set_filter(filter_op),
            Err(err) => result.add_error(format!("invalid regex filter - {err}")),
        }
    }

    result
}

/// A builder for a log filter.
///
/// It can be used to parse a set of directives from a string before building
/// a [`EnvFilter`] instance.
///
/// ## Example
///
/// ```
/// # use std::env;
/// use env_filter::Builder;
///
/// let mut builder = Builder::new();
///
/// // Parse a logging filter from an environment variable.
/// if let Ok(rust_log) = env::var("RUST_LOG") {
///     builder.parse(&rust_log);
/// }
///
/// let filter = builder.build();
/// ```
pub struct EnvFilterBuilder {
    directives: Vec<Directive>,
    filter: Option<FilterOp>,
    built: bool,
}

impl EnvFilterBuilder {
    /// Initializes the filter builder with defaults.
    pub fn new() -> EnvFilterBuilder {
        EnvFilterBuilder {
            directives: Vec::new(),
            filter: None,
            built: false,
        }
    }

    /// Initializes the filter builder from the environment using default variable name `RUST_LOG`.
    pub fn from_default_env() -> Self {
        EnvFilterBuilder::from_env(DEFAULT_FILTER_ENV)
    }

    /// Initializes the filter builder from the environment using default variable name `RUST_LOG`.
    /// If the variable is not set, the default value will be used.
    pub fn from_default_env_or<'a, V>(default: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        EnvFilterBuilder::from_env_or(DEFAULT_FILTER_ENV, default)
    }

    /// Initializes the filter builder from an environment.
    pub fn from_env(env: &str) -> EnvFilterBuilder {
        let mut builder = EnvFilterBuilder::new();

        if let Ok(s) = env::var(env) {
            builder.parse(&s);
        }

        builder
    }

    /// Initializes the filter builder from the environment using specific variable name.
    /// If the variable is not set, the default value will be used.
    pub fn from_env_or<'a, 'b, E, V>(name: E, default: V) -> Self
    where
        E: Into<Cow<'a, str>>,
        V: Into<Cow<'b, str>>,
    {
        let name = name.into();
        let default = default.into();

        let mut builder = EnvFilterBuilder::new();
        if let Ok(s) = env::var(&*name) {
            builder.parse(&s);
        } else {
            builder.parse(&default);
        }
        builder
    }

    /// Insert the directive replacing any directive with the same name.
    fn insert_directive(&mut self, mut directive: Directive) {
        if let Some(pos) = self
            .directives
            .iter()
            .position(|d| d.name == directive.name)
        {
            mem::swap(&mut self.directives[pos], &mut directive);
        } else {
            self.directives.push(directive);
        }
    }

    /// Adds a directive to the filter for a specific module.
    pub fn filter_module(&mut self, module: &str, level: LevelFilter) -> &mut Self {
        self.filter(Some(module), level)
    }

    /// Adds a directive to the filter for all modules.
    pub fn filter_level(&mut self, level: LevelFilter) -> &mut Self {
        self.filter(None, level)
    }

    /// Adds a directive to the filter.
    ///
    /// The given module (if any) will log at most the specified level provided.
    /// If no module is provided then the filter will apply to all log messages.
    pub fn filter(&mut self, module: Option<&str>, level: LevelFilter) -> &mut Self {
        self.insert_directive(Directive {
            name: module.map(|s| s.to_owned()),
            level,
        });
        self
    }

    /// Parses the directives string.
    ///
    /// See the [Enabling Logging] section for more details.
    ///
    /// [Enabling Logging]: ../index.html#enabling-logging
    pub fn parse(&mut self, filters: &str) -> &mut Self {
        #![allow(clippy::print_stderr)] // compatibility

        let ParseResult {
            directives,
            filter,
            errors,
        } = parse_spec(filters);

        for error in errors {
            eprintln!("warning: {error}, ignoring it");
        }

        self.filter = filter;

        for directive in directives {
            self.insert_directive(directive);
        }
        self
    }

    /// Parses the directive string, returning an error if the given directive string is invalid.
    ///
    /// See the [Enabling Logging] section for more details.
    ///
    /// [Enabling Logging]: ../index.html#enabling-logging
    pub fn try_parse(&mut self, filters: &str) -> Result<&mut Self, ParseError> {
        let (directives, filter) = parse_spec(filters).ok()?;

        self.filter = filter;

        for directive in directives {
            self.insert_directive(directive);
        }
        Ok(self)
    }

    /// Build a log filter.
    pub fn build(&mut self) -> EnvFilter {
        assert!(!self.built, "attempt to re-use consumed builder");
        self.built = true;

        let mut directives = Vec::new();
        if self.directives.is_empty() {
            // Adds the default filter if none exist
            directives.push(Directive {
                name: None,
                level: LevelFilter::Error,
            });
        } else {
            // Consume directives.
            directives = mem::take(&mut self.directives);
            // Sort the directives by length of their name, this allows a
            // little more efficient lookup at runtime.
            directives.sort_by(|a, b| {
                let a = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
                let b = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
                a.cmp(&b)
            });
        }

        EnvFilter {
            directives: mem::take(&mut directives),
            filter: mem::take(&mut self.filter),
        }
    }
}

impl Default for EnvFilterBuilder {
    fn default() -> Self {
        EnvFilterBuilder::new()
    }
}

impl fmt::Debug for EnvFilterBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.built {
            f.debug_struct("Filter").field("built", &true).finish()
        } else {
            f.debug_struct("Filter")
                .field("filter", &self.filter)
                .field("directives", &self.directives)
                .finish()
        }
    }
}

/// A log filter.
///
/// This struct can be used to determine whether a log record
/// should be written to the output.
/// Use the [`Builder`] type to parse and construct a `Filter`.
///
/// [`Builder`]: struct.Builder.html
pub struct EnvFilter {
    directives: Vec<Directive>,
    filter: Option<FilterOp>,
}

impl EnvFilter {
    /// Returns the maximum `LevelFilter` that this filter instance is
    /// configured to output.
    ///
    /// # Example
    ///
    /// ```rust
    /// use env_filter::Builder;
    /// use log::LevelFilter;
    ///
    /// let mut builder = Builder::new();
    /// builder.filter(Some("module1"), LevelFilter::Info);
    /// builder.filter(Some("module2"), LevelFilter::Error);
    ///
    /// let filter = builder.build();
    /// assert_eq!(filter.filter(), LevelFilter::Info);
    /// ```
    pub fn filter(&self) -> LevelFilter {
        self.directives
            .iter()
            .map(|d| d.level)
            .max()
            .unwrap_or(LevelFilter::Off)
    }

    /// Checks if this record matches the configured filter.
    pub fn matches(&self, record: &Record<'_>) -> bool {
        if !self.enabled(record.metadata()) {
            return false;
        }

        if let Some(filter) = self.filter.as_ref() {
            if !filter.is_match(&record.args().to_string()) {
                return false;
            }
        }

        true
    }

    /// Determines if a log message with the specified metadata would be logged.
    pub fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        let level = metadata.level();
        let target = metadata.target();

        enabled(&self.directives, level, target)
    }
}

impl fmt::Debug for EnvFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filter")
            .field("filter", &self.filter)
            .field("directives", &self.directives)
            .finish()
    }
}

impl Filter for EnvFilter {
    fn enabled(&self, metadata: &Metadata, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        if self.enabled(metadata) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }

    fn matches(&self, record: &Record, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        if self.matches(record) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}
