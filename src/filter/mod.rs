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

use std::fmt;

use crate::Diagnostic;

pub mod env_filter;

pub use self::env_filter::EnvFilter;

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

/// A trait representing a filter that can be applied to log records.
pub trait Filter: fmt::Debug + Send + Sync + 'static {
    /// Returns whether the record is filtered by its given metadata.
    fn enabled(
        &self,
        metadata: &log::Metadata,
        diagnostics: &[Box<dyn Diagnostic>],
    ) -> FilterResult;

    /// Returns whether the record is filtered.
    fn matches(&self, record: &log::Record, diagnostics: &[Box<dyn Diagnostic>]) -> FilterResult {
        self.enabled(record.metadata(), diagnostics)
    }
}

impl Filter for log::LevelFilter {
    fn enabled(&self, metadata: &log::Metadata, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        if metadata.level() <= *self {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl<T: Filter> From<T> for Box<dyn Filter> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
