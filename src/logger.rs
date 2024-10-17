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

use log::LevelFilter;
use log::Metadata;
use log::Record;

use crate::append::Append;
use crate::filter::Filter;
use crate::filter::FilterResult;

/// A grouped set of appenders and filters.
///
/// The [`Logger`] facade dispatches log records to one or more [`Dispatch`] instances.
/// Each [`Dispatch`] instance contains a set of filters and appenders.
///
/// `filters` are used to determine whether a log record should be passed to the appenders.
/// `appends` are used to write log records to a destination.
#[derive(Debug)]
pub struct Dispatch<const APPEND: bool = true> {
    filters: Vec<Filter>,
    appends: Vec<Box<dyn Append>>,
}

impl Default for Dispatch<false> {
    fn default() -> Dispatch<false> {
        Self::new()
    }
}

impl Dispatch<false> {
    /// Create a new incomplete [`Dispatch`] instance.
    ///
    /// At least one append must be added to the [`Dispatch`] before it can be used.
    pub fn new() -> Dispatch<false> {
        Self {
            filters: vec![],
            appends: vec![],
        }
    }

    /// Add a [`Filter`] to the [`Dispatch`].
    pub fn filter(mut self, filter: impl Into<Filter>) -> Dispatch<false> {
        self.filters.push(filter.into());
        self
    }
}

impl<const APPEND: bool> Dispatch<APPEND> {
    /// Add an [`Append`] to the [`Dispatch`].
    pub fn append(mut self, append: impl Append) -> Dispatch<true> {
        self.appends.push(Box::new(append));

        Dispatch {
            filters: self.filters,
            appends: self.appends,
        }
    }
}

impl Dispatch {
    fn enabled(&self, metadata: &Metadata) -> bool {
        for filter in &self.filters {
            match filter.enabled(metadata) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn log(&self, record: &Record) -> anyhow::Result<()> {
        for filter in &self.filters {
            match filter.matches(record) {
                FilterResult::Reject => return Ok(()),
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        for append in &self.appends {
            append.append(record)?;
        }
        Ok(())
    }

    fn flush(&self) {
        for append in &self.appends {
            append.flush();
        }
    }
}

/// A logger facade that dispatches log records to one or more [`Dispatch`] instances.
///
/// This struct implements [`log::Log`] to bridge Logforth's logging implementations
/// with the [`log`] crate.
#[derive(Debug)]
pub struct Logger {
    dispatches: Vec<Dispatch>,
    max_level: LevelFilter,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    /// Create a new [`Logger`] instance.
    #[must_use = "call `dispatch` to add a dispatch to the logger and `apply` to set the global logger"]
    pub fn new() -> Logger {
        Self {
            dispatches: vec![],
            max_level: LevelFilter::Trace,
        }
    }

    /// Set the global maximum log level.
    ///
    /// This will be passed to [`log::set_max_level`] on [`Logger::apply`].
    #[must_use = "call `apply` to set the global logger"]
    pub fn max_level(mut self, max_level: LevelFilter) -> Self {
        self.max_level = max_level;
        self
    }

    /// Add a [`Dispatch`] to the [`Logger`].
    #[must_use = "call `apply` to set the global logger"]
    pub fn dispatch(mut self, dispatch: Dispatch) -> Self {
        self.dispatches.push(dispatch);
        self
    }

    /// Set up the global logger with the [`Logger`] instance.
    ///
    /// # Errors
    ///
    /// An error is returned if the global logger has already been set.
    pub fn apply(self) -> Result<(), log::SetLoggerError> {
        let max_level = self.max_level;
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(max_level);
        Ok(())
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.dispatches
            .iter()
            .any(|dispatch| dispatch.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        for dispatch in &self.dispatches {
            if let Err(err) = dispatch.log(record) {
                handle_error(record, err);
            }
        }
    }

    fn flush(&self) {
        for dispatch in &self.dispatches {
            dispatch.flush();
        }
    }
}

fn handle_error(record: &Record, error: anyhow::Error) {
    let Err(fallback_error) = write!(
        std::io::stderr(),
        r###"
Error perform logging.
    Attempted to log: {args}
    Record: {record:?}
    Error: {error}
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
    Error: {error}
    Fallback error: {fallback_error}
"###,
        args = record.args(),
        record = record,
        error = error,
        fallback_error = fallback_error,
    );
}
