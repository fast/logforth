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

use log::Level;
use log::LevelFilter;
use log::Metadata;
use log::Record;
use regex::Regex;

use crate::Diagnostic;
use crate::Filter;
use crate::filter::FilterResult;

#[cfg(test)]
mod tests;

/// The default environment variable for filtering logs.
pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

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
    filter: Option<Regex>,
    errors: Vec<String>,
}

impl ParseResult {
    fn add_directive(&mut self, directive: Directive) {
        self.directives.push(directive);
    }

    fn set_filter(&mut self, filter: Regex) {
        self.filter = Some(filter);
    }

    fn add_error(&mut self, message: String) {
        self.errors.push(message);
    }
}

/// Parse a logging specification string (e.g: `crate1,crate2::mod3,crate3::x=error/foo`)
/// and return a vector with log directives.
fn parse_spec(spec: &str) -> ParseResult {
    let mut result = ParseResult::default();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        result.add_error(format!("malformed logging spec '{spec}' (too many '/'s)"));
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
                            result.add_error(format!("malformed logging spec '{part1}'"));
                            continue;
                        }
                    }
                    _ => {
                        result.add_error(format!("malformed logging spec '{s}'"));
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
        match Regex::new(filter) {
            Ok(filter_op) => result.set_filter(filter_op),
            Err(err) => result.add_error(format!("malformed regex filter: {err}")),
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
/// use logforth::filter::env_filter::EnvFilterBuilder;
///
/// // Parse a logging filter from the default environment variable `RUST_LOG`.
/// let builder = EnvFilterBuilder::from_default_env();
/// let filter = builder.build();
/// ```
#[derive(Debug, Default)]
pub struct EnvFilterBuilder {
    directives: Vec<Directive>,
    filter: Option<Regex>,
}

impl EnvFilterBuilder {
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

    /// Parses the directives string.
    pub fn from_spec<'a, V>(spec: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        let spec = spec.into();

        let ParseResult {
            directives,
            filter,
            errors,
        } = parse_spec(&spec);

        for error in errors {
            eprintln!("warning: {error}, ignoring it");
        }

        let mut builder = EnvFilterBuilder {
            filter,
            directives: vec![],
        };
        for directive in directives {
            builder.insert_directive(directive);
        }
        builder
    }

    /// Parses the directive string, returning an error if the given directive string is malformed.
    pub fn try_from_spec<'a, V>(spec: V) -> Result<Self, String>
    where
        V: Into<Cow<'a, str>>,
    {
        let spec = spec.into();

        let ParseResult {
            directives,
            filter,
            errors,
        } = parse_spec(&spec);

        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }

        let mut builder = EnvFilterBuilder {
            filter,
            directives: vec![],
        };
        for directive in directives {
            builder.insert_directive(directive);
        }
        Ok(builder)
    }

    /// Consume the builder and produce an [`EnvFilter`].
    ///
    /// If the builder has no directives, a default directive of `ERROR` level will be added.
    pub fn build(self) -> EnvFilter {
        let Self { directives, filter } = self;

        let directives = if directives.is_empty() {
            vec![Directive {
                name: None,
                level: LevelFilter::Error,
            }]
        } else {
            let mut directives = directives;
            directives.sort_by_key(|d| d.name.as_ref().map(String::len).unwrap_or(0));
            directives
        };

        EnvFilter { directives, filter }
    }

    /// Adds a directive to the filter for a specific module.
    pub fn filter_module(self, module: &str, level: LevelFilter) -> Self {
        self.filter(Some(module), level)
    }

    /// Adds a directive to the filter for all modules.
    pub fn filter_level(self, level: LevelFilter) -> Self {
        self.filter(None, level)
    }

    /// Adds a directive to the filter.
    ///
    /// The given module (if any) will log at most the specified level provided.
    /// If no module is provided then the filter will apply to all log messages.
    pub fn filter(mut self, module: Option<&str>, level: LevelFilter) -> Self {
        self.insert_directive(Directive {
            name: module.map(|s| s.to_owned()),
            level,
        });
        self
    }

    /// Insert the directive replacing any directive with the same name.
    fn insert_directive(&mut self, mut directive: Directive) {
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

/// A log filter.
///
/// This struct can be used to determine whether a log record should be written to the output.
///
/// Use the [`EnvFilterBuilder`] type to parse and construct a `Filter`.
#[derive(Debug)]
pub struct EnvFilter {
    directives: Vec<Directive>,
    filter: Option<Regex>,
}

impl Filter for EnvFilter {
    fn enabled(&self, metadata: &Metadata, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        let level = metadata.level();
        let target = metadata.target();

        if enabled(&self.directives, level, target) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }

    fn matches(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> FilterResult {
        match self.enabled(record.metadata(), diags) {
            FilterResult::Accept => FilterResult::Accept,
            FilterResult::Reject => FilterResult::Reject,
            FilterResult::Neutral => {
                if let Some(filter) = self.filter.as_ref() {
                    let payload = record.args().to_string();
                    if !filter.is_match(payload.as_str()) {
                        return FilterResult::Reject;
                    }
                }

                FilterResult::Neutral
            }
        }
    }
}
