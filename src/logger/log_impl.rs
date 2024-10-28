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

use log::Metadata;
use log::Record;

use crate::filter::FilterResult;
use crate::Append;
use crate::Filter;

/// A logger facade that dispatches log records to one or more [`Dispatch`] instances.
///
/// This struct implements [`log::Log`] to bridge Logforth's logging implementations
/// with the [`log`] crate.
#[derive(Debug)]
pub(super) struct Logger {
    dispatches: Vec<Dispatch>,
}

impl Logger {
    pub(super) fn new(dispatches: Vec<Dispatch>) -> Self {
        Self { dispatches }
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

/// A grouped set of appenders and filters.
///
/// The [`Logger`] facade dispatches log records to one or more [`Dispatch`] instances.
/// Each [`Dispatch`] instance contains a set of filters and appenders.
///
/// `filters` are used to determine whether a log record should be passed to the appenders.
/// `appends` are used to write log records to a destination.
#[derive(Debug)]
pub(super) struct Dispatch {
    filters: Vec<Filter>,
    appends: Vec<Box<dyn Append>>,
}

impl Dispatch {
    pub(super) fn new(filters: Vec<Filter>, appends: Vec<Box<dyn Append>>) -> Self {
        debug_assert!(
            !appends.is_empty(),
            "A Dispatch must have at least one filter"
        );

        Self { filters, appends }
    }

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
