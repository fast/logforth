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
use std::sync::Arc;

use crate::Diagnostic;
use crate::record::Level;
use crate::record::LevelFilter;
use crate::record::Record;

/// The result of evaluating a [`Filter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// Accept the record immediately and bypass remaining filters in the current dispatch.
    Accept,
    /// Drop the record immediately from the current dispatch.
    Reject,
    /// Express no final decision and pass control to the next filter in the dispatch.
    Neutral,
}

/// A filter applied to log records within a dispatch branch.
pub trait Filter: fmt::Debug + Send + Sync + 'static {
    /// Returns the most permissive result (`Accept > Neutral > Reject`) that [`Self::filter`] may
    /// return for any record at `level`.
    ///
    /// Evaluated once per severity level when building a [`Logger`](crate::Logger) to precompute
    /// fast-path level pruning. Narrowing filters that never promote records return
    /// [`FilterResult::Neutral`] (or [`FilterResult::Reject`] for disabled levels); filters that
    /// may dynamically promote records ahead of downstream thresholds return
    /// [`FilterResult::Accept`] for levels they may promote.
    fn filter_level(&self, level: Level) -> FilterResult;

    /// Evaluates whether `record` should be accepted, rejected, or passed to subsequent filters
    /// in the dispatch.
    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult;
}

impl Filter for LevelFilter {
    fn filter_level(&self, level: Level) -> FilterResult {
        if self.test(level) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }

    fn filter(&self, record: &Record, _: &dyn Diagnostic) -> FilterResult {
        self.filter_level(record.level())
    }
}

impl<T: Filter> From<T> for Box<dyn Filter> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: Filter + ?Sized> Filter for Arc<T> {
    fn filter_level(&self, level: Level) -> FilterResult {
        (**self).filter_level(level)
    }

    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult {
        (**self).filter(record, diags)
    }
}

impl<T: Filter + ?Sized> Filter for &'static T {
    fn filter_level(&self, level: Level) -> FilterResult {
        (**self).filter_level(level)
    }

    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult {
        (**self).filter(record, diags)
    }
}
