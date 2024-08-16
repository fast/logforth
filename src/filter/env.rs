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

use std::borrow::Cow;

pub use env_filter::Builder as EnvFilterBuilder;
use log::Metadata;

use crate::filter::FilterResult;
use crate::Filter;

const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

/// A filter that respects the `RUST_LOG` environment variable.
///
/// Read [the `env_logger` documentation](https://docs.rs/env_logger/#enabling-logging) for more.
#[derive(Debug)]
pub struct EnvFilter(env_filter::Filter);

impl EnvFilter {
    /// Initializes the filter builder from the environment using default variable name `RUST_LOG`.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the default environment variables:
    ///
    /// ```
    /// use logforth::filter::EnvFilter;
    /// let filter = EnvFilter::from_default_env();
    /// ```
    pub fn from_default_env() -> Self {
        EnvFilter::from_env(DEFAULT_FILTER_ENV)
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
    /// use logforth::filter::EnvFilter;
    /// let filter = EnvFilter::from_default_env_or("info");
    /// ```
    pub fn from_default_env_or<'a, V>(default: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        EnvFilter::from_env_or(DEFAULT_FILTER_ENV, default)
    }

    /// Initializes the filter builder from the environment using specific variable name.
    ///
    /// # Examples
    ///
    /// Initialize a filter using the using specific variable name:
    ///
    /// ```
    /// use logforth::filter::EnvFilter;
    /// let filter = EnvFilter::from_env("MY_LOG");
    /// ```
    pub fn from_env<'a, E>(name: E) -> Self
    where
        E: Into<Cow<'a, str>>,
    {
        let mut builder = EnvFilterBuilder::new();
        let name = name.into();
        if let Ok(s) = std::env::var(&*name) {
            builder.parse(&s);
        }
        EnvFilter::new(builder)
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
    /// use logforth::filter::EnvFilter;
    /// let filter = EnvFilter::from_env_or("MY_LOG", "info");
    /// ```
    pub fn from_env_or<'a, 'b, E, V>(name: E, default: V) -> Self
    where
        E: Into<Cow<'a, str>>,
        V: Into<Cow<'b, str>>,
    {
        let mut builder = EnvFilterBuilder::new();
        let name = name.into();
        let default = default.into();
        if let Ok(s) = std::env::var(&*name) {
            builder.parse(&s);
        } else {
            builder.parse(&default);
        }
        EnvFilter::new(builder)
    }

    /// Initializes the filter builder from the [EnvFilterBuilder].
    pub fn new(mut builder: EnvFilterBuilder) -> Self {
        EnvFilter(builder.build())
    }

    pub(crate) fn enabled(&self, metadata: &Metadata) -> FilterResult {
        if self.0.enabled(metadata) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }

    pub(crate) fn matches(&self, record: &log::Record) -> FilterResult {
        if self.0.matches(record) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl From<EnvFilter> for Filter {
    fn from(filter: EnvFilter) -> Self {
        Filter::Env(filter)
    }
}
