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
use std::sync::OnceLock;

use crate::Append;
use crate::Diagnostic;
use crate::Error;
use crate::Filter;
use crate::filter::FilterResult;
use crate::record::Metadata;
use crate::record::Record;

static DEFAULT_LOGGER: OnceLock<Logger> = OnceLock::new();

/// Return the default global logger instance.
///
/// If no default logger has been set, `None` is returned.
pub fn default_logger() -> Option<&'static Logger> {
    DEFAULT_LOGGER.get()
}

/// Set the default global logger instance.
///
/// If a default logger has already been set, the function returns the provided logger
/// as an error.
pub fn set_default_logger(logger: Logger) -> Result<(), Logger> {
    DEFAULT_LOGGER.set(logger)
}

/// A logger facade that dispatches log records to one or more dispatcher.
///
/// This struct implements [`log::Log`] to bridge Logforth's logging implementations
/// with the [`log`] crate.
#[derive(Debug)]
pub struct Logger {
    dispatches: Vec<Dispatch>,
}

impl Logger {
    pub(super) fn new(dispatches: Vec<Dispatch>) -> Self {
        Self { dispatches }
    }
}

impl Logger {
    /// Determines if a log message with the specified metadata would be logged.
    pub fn enabled(&self, metadata: &Metadata) -> bool {
        self.dispatches
            .iter()
            .any(|dispatch| dispatch.enabled(metadata))
    }

    /// Logs the Record.
    pub fn log(&self, record: &Record) {
        for dispatch in &self.dispatches {
            if let Err(err) = dispatch.log(record) {
                handle_log_error(record, err);
            }
        }
    }

    /// Flushes any buffered records.
    pub fn flush(&self) {
        for dispatch in &self.dispatches {
            if let Err(err) = dispatch.flush() {
                handle_flush_error(err);
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

    fn enabled(&self, metadata: &Metadata) -> bool {
        let diagnostics = &self.diagnostics;

        for filter in &self.filters {
            match filter.enabled(metadata, diagnostics) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn log(&self, record: &Record) -> Result<(), Error> {
        let diagnostics = &self.diagnostics;

        for filter in &self.filters {
            match filter.matches(record, diagnostics) {
                FilterResult::Reject => return Ok(()),
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        for append in &self.appends {
            append.append(record, diagnostics)?;
        }
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        for append in &self.appends {
            append.flush()?;
        }
        Ok(())
    }
}

fn handle_log_error(record: &Record, error: Error) {
    let Err(fallback_error) = write!(
        std::io::stderr(),
        r###"
Error perform logging.
    Attempted to log: {args}
    Record: {record:?}
    Error: {error:?}
"###,
        args = record.args(),
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
        args = record.args(),
        record = record,
        error = error,
        fallback_error = fallback_error,
    );
}

fn handle_flush_error(error: Error) {
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
