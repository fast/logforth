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

use crate::filter::FilterResult;
use crate::{Diagnostic, Filter};
use log::{Level, LevelFilter, Metadata, Record};
use std::borrow::Cow;
use std::{env, fmt, mem};

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

/// The default environment variable for filtering logs.
pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

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

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use snapbox::Data;
    use snapbox::IntoData;
    use snapbox::assert_data_eq;
    use snapbox::str;

    use super::*;

    impl IntoData for ParseError {
        fn into_data(self) -> Data {
            self.to_string().into_data()
        }
    }

    #[test]
    fn parse_spec_valid() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug");

        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_owned()));
        assert_eq!(dirs[1].level, LevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_owned()));
        assert_eq!(dirs[2].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_invalid_crate() {
        // test parse_spec with multiple = in specification
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=warn=info,crate2=debug");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_data_eq!(
            &errors[0],
            str!["invalid logging spec 'crate1::mod1=warn=info'"]
        );
    }

    #[test]
    fn parse_spec_invalid_level() {
        // test parse_spec with 'noNumber' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=noNumber,crate2=debug");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_data_eq!(&errors[0], str!["invalid logging spec 'noNumber'"]);
    }

    #[test]
    fn parse_spec_string_level() {
        // test parse_spec with 'warn' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=wrong,crate2=warn");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_data_eq!(&errors[0], str!["invalid logging spec 'wrong'"]);
    }

    #[test]
    fn parse_spec_empty_level() {
        // test parse_spec with '' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=wrong,crate2=");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::max());
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_data_eq!(&errors[0], str!["invalid logging spec 'wrong'"]);
    }

    #[test]
    fn parse_spec_empty_level_isolated() {
        // test parse_spec with "" as log level (and the entire spec str)
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec(""); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated() {
        // test parse_spec with a white-space-only string specified as the log
        // level (and the entire spec str)
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("     "); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated_comma_only() {
        // The spec should contain zero or more comma-separated string slices,
        // so a comma-only string should be interpreted as two empty strings
        // (which should both be treated as invalid, so ignored).
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec(","); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated_comma_blank() {
        // The spec should contain zero or more comma-separated string slices,
        // so this bogus spec should be interpreted as containing one empty
        // string and one blank string. Both should both be treated as
        // invalid, so ignored.
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec(",     "); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated_blank_comma() {
        // The spec should contain zero or more comma-separated string slices,
        // so this bogus spec should be interpreted as containing one blank
        // string and one empty string. Both should both be treated as
        // invalid, so ignored.
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("     ,"); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global() {
        // test parse_spec with no crate
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert_eq!(dirs[1].name, Some("crate2".to_owned()));
        assert_eq!(dirs[1].level, LevelFilter::Debug);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global_bare_warn_lc() {
        // test parse_spec with no crate, in isolation, all lowercase
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global_bare_warn_uc() {
        // test parse_spec with no crate, in isolation, all uppercase
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("WARN");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global_bare_warn_mixed() {
        // test parse_spec with no crate, in isolation, mixed case
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("wArN");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_valid_filter() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug/abc");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_owned()));
        assert_eq!(dirs[1].level, LevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_owned()));
        assert_eq!(dirs[2].level, LevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "abc");
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_invalid_crate_filter() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=error=warn,crate2=debug/a.c");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "a.c");

        assert_eq!(errors.len(), 1);
        assert_data_eq!(
            &errors[0],
            str!["invalid logging spec 'crate1::mod1=error=warn'"]
        );
    }

    #[test]
    fn parse_spec_empty_with_filter() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1/a*c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate1".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::max());
        assert!(filter.is_some() && filter.unwrap().to_string() == "a*c");
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_with_multiple_filters() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("debug/abc/a.c");
        assert!(dirs.is_empty());
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_data_eq!(
            &errors[0],
            str!["invalid logging spec 'debug/abc/a.c' (too many '/'s)"]
        );
    }

    #[test]
    fn parse_spec_multiple_invalid_crates() {
        // test parse_spec with multiple = in specification
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=warn=info,crate2=debug,crate3=error=error");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 2);
        assert_data_eq!(
            &errors[0],
            str!["invalid logging spec 'crate1::mod1=warn=info'"]
        );
        assert_data_eq!(
            &errors[1],
            str!["invalid logging spec 'crate3=error=error'"]
        );
    }

    #[test]
    fn parse_spec_multiple_invalid_levels() {
        // test parse_spec with 'noNumber' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=noNumber,crate2=debug,crate3=invalid");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 2);
        assert_data_eq!(&errors[0], str!["invalid logging spec 'noNumber'"]);
        assert_data_eq!(&errors[1], str!["invalid logging spec 'invalid'"]);
    }

    #[test]
    fn parse_spec_invalid_crate_and_level() {
        // test parse_spec with 'noNumber' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=debug=info,crate2=debug,crate3=invalid");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 2);
        assert_data_eq!(
            &errors[0],
            str!["invalid logging spec 'crate1::mod1=debug=info'"]
        );
        assert_data_eq!(&errors[1], str!["invalid logging spec 'invalid'"]);
    }

    #[test]
    fn parse_error_message_single_error() {
        let error = parse_spec("crate1::mod1=debug=info,crate2=debug")
            .ok()
            .unwrap_err();
        assert_data_eq!(
            error,
            str!["error parsing logger filter: invalid logging spec 'crate1::mod1=debug=info'"]
        );
    }

    #[test]
    fn parse_error_message_multiple_errors() {
        let error = parse_spec("crate1::mod1=debug=info,crate2=debug,crate3=invalid")
            .ok()
            .unwrap_err();
        assert_data_eq!(
            error,
            str!["error parsing logger filter: invalid logging spec 'crate1::mod1=debug=info'"]
        );
    }

    fn make_logger_filter(dirs: Vec<Directive>) -> EnvFilter {
        let mut logger = EnvFilterBuilder::new().build();
        logger.directives = dirs;
        logger
    }

    #[test]
    fn filter_info() {
        let logger = EnvFilterBuilder::new()
            .filter(None, LevelFilter::Info)
            .build();
        assert!(enabled(&logger.directives, Level::Info, "crate1"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate1"));
    }

    #[test]
    fn filter_beginning_longest_match() {
        let logger = EnvFilterBuilder::new()
            .filter(Some("crate2"), LevelFilter::Info)
            .filter(Some("crate2::mod"), LevelFilter::Debug)
            .filter(Some("crate1::mod1"), LevelFilter::Warn)
            .build();
        assert!(enabled(&logger.directives, Level::Debug, "crate2::mod1"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
    }

    // Some of our tests are only correct or complete when they cover the full
    // universe of variants for log::Level. In the unlikely event that a new
    // variant is added in the future, this test will detect the scenario and
    // alert us to the need to review and update the tests. In such a
    // situation, this test will fail to compile, and the error message will
    // look something like this:
    //
    //     error[E0004]: non-exhaustive patterns: `NewVariant` not covered
    //        --> src/filter/mod.rs:413:15
    //         |
    //     413 |         match level_universe {
    //         |               ^^^^^^^^^^^^^^ pattern `NewVariant` not covered
    #[test]
    fn ensure_tests_cover_level_universe() {
        let level_universe: Level = Level::Trace; // use of trace variant is arbitrary
        match level_universe {
            Level::Error | Level::Warn | Level::Info | Level::Debug | Level::Trace => (),
        }
    }

    #[test]
    fn parse_default() {
        let logger = EnvFilterBuilder::new()
            .parse("info,crate1::mod1=warn")
            .build();
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }

    #[test]
    fn parse_default_bare_level_off_lc() {
        let logger = EnvFilterBuilder::new().parse("off").build();
        assert!(!enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_off_uc() {
        let logger = EnvFilterBuilder::new().parse("OFF").build();
        assert!(!enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_error_lc() {
        let logger = EnvFilterBuilder::new().parse("error").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_error_uc() {
        let logger = EnvFilterBuilder::new().parse("ERROR").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_warn_lc() {
        let logger = EnvFilterBuilder::new().parse("warn").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_warn_uc() {
        let logger = EnvFilterBuilder::new().parse("WARN").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_info_lc() {
        let logger = EnvFilterBuilder::new().parse("info").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_info_uc() {
        let logger = EnvFilterBuilder::new().parse("INFO").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_debug_lc() {
        let logger = EnvFilterBuilder::new().parse("debug").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_debug_uc() {
        let logger = EnvFilterBuilder::new().parse("DEBUG").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_trace_lc() {
        let logger = EnvFilterBuilder::new().parse("trace").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_trace_uc() {
        let logger = EnvFilterBuilder::new().parse("TRACE").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(enabled(&logger.directives, Level::Trace, ""));
    }

    // In practice, the desired log level is typically specified by a token
    // that is either all lowercase (e.g., 'trace') or all uppercase (.e.g,
    // 'TRACE'), but this tests serves as a reminder that
    // log::Level::from_str() ignores all case variants.
    #[test]
    fn parse_default_bare_level_debug_mixed() {
        {
            let logger = EnvFilterBuilder::new().parse("Debug").build();
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
        {
            let logger = EnvFilterBuilder::new().parse("debuG").build();
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
        {
            let logger = EnvFilterBuilder::new().parse("deBug").build();
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
        {
            let logger = EnvFilterBuilder::new().parse("DeBuG").build(); // LaTeX flavor!
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
    }

    #[test]
    fn try_parse_valid_filter() {
        let logger = EnvFilterBuilder::new()
            .try_parse("info,crate1::mod1=warn")
            .expect("valid filter returned error")
            .build();
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }

    #[test]
    fn try_parse_invalid_filter() {
        let error = EnvFilterBuilder::new()
            .try_parse("info,crate1=invalid")
            .unwrap_err();
        assert_data_eq!(
            error,
            str!["error parsing logger filter: invalid logging spec 'invalid'"]
        );
    }

    #[test]
    fn match_full_path() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(!enabled(&logger.directives, Level::Info, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
    }

    #[test]
    fn no_match() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(!enabled(&logger.directives, Level::Warn, "crate3"));
    }

    #[test]
    fn match_beginning() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod1"));
    }

    #[test]
    fn match_beginning_longest_match() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate2::mod".to_owned()),
                level: LevelFilter::Debug,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Debug, "crate2::mod1"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger_filter(vec![
            Directive {
                name: None,
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger_filter(vec![
            Directive {
                name: None,
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Off,
            },
        ]);
        assert!(!enabled(&logger.directives, Level::Error, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }
}
