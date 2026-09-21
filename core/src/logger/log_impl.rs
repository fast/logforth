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

use std::io::Write;
use std::panic;
use std::sync::Arc;

use crate::Append;
use crate::Diagnostic;
use crate::Error;
use crate::Filter;
use crate::filter::FilterResult;
use crate::record::FilterCriteria;
use crate::record::Record;

/// A handle to shared dispatches, filters, diagnostics, and appenders.
///
/// Cloning or naming a logger shares the configured pipeline. The final handle
/// owns appender teardown; flushing any handle flushes the shared pipeline.
#[derive(Debug, Clone)]
pub struct Logger {
    dispatches: Arc<[Dispatch]>,
    name: Option<&'static str>,
}

impl Logger {
    pub(super) fn new(dispatches: Vec<Dispatch>) -> Self {
        Self {
            dispatches: dispatches.into(),
            name: None,
        }
    }
}

impl Logger {
    /// Create a named handle sharing this logger's dispatches and diagnostics.
    ///
    /// Native macros use this name as the record target, while source metadata
    /// still identifies the actual call site. Names replace rather than append
    /// to the current name. Cloning or naming a handle never rebuilds appenders.
    ///
    /// Use stable application categories such as `"worker"`. Event-specific data
    /// belongs in fields. For an independent filtering or output policy, build
    /// a separate logger instead of naming a shared handle.
    ///
    /// ```
    /// let logger = logforth_core::builder().build();
    /// let worker = logger.named("worker");
    /// logforth_core::info!(worker, "started");
    /// ```
    #[must_use = "use the returned logger handle to emit events with this name"]
    pub fn named(&self, name: &'static str) -> Self {
        Self {
            dispatches: self.dispatches.clone(),
            name: Some(name),
        }
    }

    /// The native logging category, or `None` for call-site module targets.
    ///
    /// Names do not rewrite records passed directly to [`Self::log`], including
    /// records forwarded by a bridge.
    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Test whether any dispatch might accept an event with this level and target.
    ///
    /// This is a conservative prefilter. A full-record filter can still reject
    /// the event, and dynamic filters can change before emission. Native macros
    /// call this before evaluating the message and fields.
    pub fn enabled(&self, criteria: &FilterCriteria) -> bool {
        self.dispatches
            .iter()
            .any(|dispatch| dispatch.enabled(criteria))
    }

    /// Log the [`Record`].
    pub fn log(&self, record: &Record) {
        for dispatch in self.dispatches.iter() {
            for err in dispatch.log(record) {
                handle_log_error(record, &err);
            }
        }
    }

    /// Flush buffered records through every configured appender.
    ///
    /// Stop logging producers before calling this during graceful shutdown, and
    /// keep any runtimes required by appenders alive until it returns. This does
    /// not disable the logger or prevent later events. Appender errors use the
    /// same stderr fallback as logging errors. There is no implicit exit hook.
    pub fn flush(&self) {
        for dispatch in self.dispatches.iter() {
            for err in dispatch.flush() {
                handle_flush_error(&err);
            }
        }
    }
}

/// A grouped set of appenders and filters.
///
/// The [`Logger`] facade dispatches log records to one or more [`Dispatch`] instances.
/// Each [`Dispatch`] instance contains a set of filters and appenders.
///
/// `filters` are used to determine whether a log record should be passed to the appenders.
/// `appends` are used to write log records to a destination.
#[derive(Debug)]
pub(super) struct Dispatch {
    filters: Vec<Box<dyn Filter>>,
    diagnostics: Vec<Box<dyn Diagnostic>>,
    appends: Vec<Box<dyn Append>>,
}

impl Dispatch {
    pub(super) fn new(
        filters: Vec<Box<dyn Filter>>,
        diagnostics: Vec<Box<dyn Diagnostic>>,
        appends: Vec<Box<dyn Append>>,
    ) -> Self {
        debug_assert!(
            !appends.is_empty(),
            "A Dispatch must have at least one filter"
        );

        Self {
            filters,
            diagnostics,
            appends,
        }
    }

    fn enabled(&self, criteria: &FilterCriteria) -> bool {
        let diagnostics = &self.diagnostics;

        for filter in &self.filters {
            match filter.enabled(criteria, diagnostics) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn log(&self, record: &Record) -> Vec<Error> {
        let diagnostics = &self.diagnostics;

        for filter in &self.filters {
            match filter.matches(record, diagnostics) {
                FilterResult::Reject => return vec![],
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        let mut errors = vec![];
        for append in &self.appends {
            if let Err(err) = append.append(record, diagnostics) {
                errors.push(err);
            }
        }
        errors
    }

    fn flush(&self) -> Vec<Error> {
        let mut errors = vec![];
        for append in &self.appends {
            if let Err(err) = append.flush() {
                errors.push(err);
            }
        }
        errors
    }
}

fn handle_log_error(record: &Record, error: &Error) {
    let Err(fallback_error) = write!(
        std::io::stderr(),
        r###"
Error perform logging.
    Attempted to log: {args}
    Record: {record:?}
    Error: {error:?}
"###,
        args = record.payload(),
        record = record,
        error = error,
    ) else {
        return;
    };

    panic!(
        r###"
Error performing stderr logging after error occurred during regular logging.
    Attempted to log: {args}
    Record: {record:?}
    Error: {error:?}
    Fallback error: {fallback_error}
"###,
        args = record.payload(),
        record = record,
        error = error,
        fallback_error = fallback_error,
    );
}

fn handle_flush_error(error: &Error) {
    let Err(fallback_error) = write!(
        std::io::stderr(),
        r###"
Error perform flush.
    Error: {error:?}
"###,
    ) else {
        return;
    };

    panic!(
        r###"
Error performing stderr logging after error occurred during regular flush.
    Error: {error:?}
    Fallback error: {fallback_error}
"###,
    );
}
