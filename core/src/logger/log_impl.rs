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

use std::sync::Arc;

use crate::Append;
use crate::Diagnostic;
use crate::Error;
use crate::ErrorSink;
use crate::Filter;
use crate::filter::FilterResult;
use crate::record::Level;
use crate::record::Record;

const fn level_bit(level: Level) -> u32 {
    1u32 << (level as u8 - 1)
}

fn dispatch_accepts_level(filters: &[Box<dyn Filter>], level: Level) -> bool {
    for filter in filters {
        match filter.filter_level(level) {
            FilterResult::Reject => return false,
            FilterResult::Accept => return true,
            FilterResult::Neutral => {}
        }
    }
    true
}

/// A lightweight handle to a shared logging pipeline.
///
/// Cloning a `Logger` increments an internal reference count without duplicating dispatches or
/// appenders. Use [`Logger::noop`] (or [`Logger::default`]) for a zero-allocation logger that
/// discards all records.
#[derive(Debug, Clone, Default)]
pub struct Logger {
    state: Option<Arc<LoggerState>>,
}

#[derive(Debug)]
struct LoggerState {
    level_mask: u32,
    dispatches: Box<[Dispatch]>,
    error_sink: Box<dyn ErrorSink>,
}

impl Logger {
    /// Creates a zero-allocation no-op [`Logger`] that discards all records.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::Logger;
    /// use logforth_core::record::Record;
    ///
    /// const NOOP: Logger = Logger::noop();
    /// NOOP.log(&Record::builder().payload(format_args!("ignored")).build());
    /// ```
    pub const fn noop() -> Self {
        Self { state: None }
    }

    pub(super) fn new(dispatches: Vec<Dispatch>, error_sink: Box<dyn ErrorSink>) -> Self {
        if dispatches.is_empty() {
            Self::noop()
        } else {
            let mut level_mask = 0u32;
            for dispatch in &dispatches {
                level_mask |= dispatch.level_mask;
            }
            Self {
                state: Some(Arc::new(LoggerState {
                    level_mask,
                    dispatches: dispatches.into_boxed_slice(),
                    error_sink,
                })),
            }
        }
    }

    /// Returns `true` if at least one dispatch branch may accept `level`.
    pub fn enabled(&self, level: Level) -> bool {
        self.state
            .as_deref()
            .is_some_and(|state| (state.level_mask & level_bit(level)) != 0)
    }

    /// Dispatches `record` across all configured branches.
    pub fn log(&self, record: &Record) {
        let Some(state) = self.state.as_deref() else {
            return;
        };
        if (state.level_mask & level_bit(record.level())) == 0 {
            return;
        }
        let default_error_sink = &*state.error_sink;
        for dispatch in &state.dispatches {
            dispatch.log(record, default_error_sink);
        }
    }

    /// Flushes all appenders across all configured branches.
    pub fn flush(&self) {
        let Some(state) = self.state.as_deref() else {
            return;
        };
        let default_error_sink = &*state.error_sink;
        for dispatch in &state.dispatches {
            dispatch.flush(default_error_sink);
        }
    }
}

/// A grouped set of filters, diagnostics, and appenders.
#[derive(Debug)]
pub(super) struct Dispatch {
    level_mask: u32,
    filters: Box<[Box<dyn Filter>]>,
    diagnostics: Box<[Box<dyn Diagnostic>]>,
    appends: Box<[Box<dyn Append>]>,
    error_sink: Option<Box<dyn ErrorSink>>,
}

impl Dispatch {
    pub(super) fn new(
        filters: Vec<Box<dyn Filter>>,
        diagnostics: Vec<Box<dyn Diagnostic>>,
        appends: Vec<Box<dyn Append>>,
        error_sink: Option<Box<dyn ErrorSink>>,
    ) -> Self {
        debug_assert!(
            !appends.is_empty(),
            "A Dispatch must have at least one appender"
        );

        let mut level_mask = 0u32;
        for level in Level::ALL {
            if dispatch_accepts_level(&filters, level) {
                level_mask |= level_bit(level);
            }
        }

        Self {
            level_mask,
            filters: filters.into_boxed_slice(),
            diagnostics: diagnostics.into_boxed_slice(),
            appends: appends.into_boxed_slice(),
            error_sink,
        }
    }

    fn log(&self, record: &Record, default_error_sink: &dyn ErrorSink) {
        if (self.level_mask & level_bit(record.level())) == 0 {
            return;
        }

        let diagnostics = &*self.diagnostics;
        for filter in &self.filters {
            match filter.filter(record, &diagnostics) {
                FilterResult::Reject => return,
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        let error_sink = self.error_sink.as_deref().unwrap_or(default_error_sink);
        for append in &self.appends {
            if let Err(err) = append.append(record, &diagnostics) {
                let err = Error::new("failed to append record").with_source(err);
                error_sink.sink(&err);
            }
        }
    }

    fn flush(&self, default_error_sink: &dyn ErrorSink) {
        let error_sink = self.error_sink.as_deref().unwrap_or(default_error_sink);
        for append in &self.appends {
            if let Err(err) = append.flush() {
                let err = Error::new("failed to flush appender").with_source(err);
                error_sink.sink(&err);
            }
        }
    }
}
