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

//! Filters for log records.

use std::str::FromStr;

use log::LevelFilter;

pub use self::custom::CustomFilter;
pub use self::env_filter::EnvFilter;

mod custom;
pub mod env_filter;

/// The result of a filter check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// The record will be processed without further filtering.
    Accept,
    /// The record should not be processed.
    Reject,
    /// No decision could be made, further filtering should occur.
    Neutral,
}

/// Represents a filter that can be applied to log records.
#[derive(Debug)]
pub enum Filter {
    /// An env_logger filter.
    Env(EnvFilter),
    /// A custom filter.
    Custom(CustomFilter),
}

impl Filter {
    pub(crate) fn enabled(&self, metadata: &log::Metadata) -> FilterResult {
        match self {
            Filter::Env(filter) => filter.enabled(metadata),
            Filter::Custom(filter) => filter.enabled(metadata),
        }
    }

    pub(crate) fn matches(&self, record: &log::Record) -> FilterResult {
        match self {
            Filter::Env(filter) => filter.matches(record),
            Filter::Custom(filter) => filter.enabled(record.metadata()),
        }
    }
}

impl From<LevelFilter> for Filter {
    fn from(filter: LevelFilter) -> Self {
        EnvFilter::from(filter).into()
    }
}

impl<'a> From<&'a str> for Filter {
    fn from(filter: &'a str) -> Self {
        EnvFilter::from(filter).into()
    }
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EnvFilter::from_str(s).map(Into::into)
    }
}
