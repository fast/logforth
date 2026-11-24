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

use crate::Append;
use crate::Diagnostic;
use crate::Error;
use crate::ErrorSink;
use crate::Filter;
use crate::filter::FilterResult;
use crate::record::FilterCriteria;
use crate::record::Record;

/// A logger that dispatches log records to one or more dispatcher.
#[derive(Debug)]
pub struct Logger {
    dispatches: Vec<Dispatch>,
    error_sink: ErrorSink,
}

impl Logger {
    pub(super) fn new(dispatches: Vec<Dispatch>, error_sink: ErrorSink) -> Self {
        Self {
            dispatches,
            error_sink,
        }
    }
}

impl Logger {
    /// Determine if a log message with the specified metadata would be logged.
    pub fn enabled(&self, criteria: &FilterCriteria) -> bool {
        self.dispatches
            .iter()
            .any(|dispatch| dispatch.enabled(criteria))
    }

    /// Log the [`Record`].
    pub fn log(&self, record: &Record) {
        for dispatch in &self.dispatches {
            for err in dispatch.log(record) {
                let err = Error::new("failed to append record").with_source(err);
                self.error_sink.report(&err);
            }
        }
    }

    /// Flush any buffered records.
    pub fn flush(&self) {
        for dispatch in &self.dispatches {
            for err in dispatch.flush() {
                let err = Error::new("failed to flush appender").with_source(err);
                self.error_sink.report(&err);
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    use super::*;
    use crate::builder;

    #[derive(Debug)]
    struct FailingAppender;

    impl Append for FailingAppender {
        fn append(&self, _: &Record, _: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
            Err(Error::new("disk full"))
        }

        fn flush(&self) -> Result<(), Error> {
            Err(Error::new("sync failed"))
        }
    }

    #[derive(Debug)]
    struct RecordingAppender(Arc<Mutex<Vec<String>>>);

    impl Append for RecordingAppender {
        fn append(&self, record: &Record, _: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
            self.0.lock().unwrap().push(record.payload().to_string());
            Ok(())
        }

        fn flush(&self) -> Result<(), Error> {
            Ok(())
        }
    }

    #[test]
    fn logger_routes_errors_to_sink_and_continues_remaining_appenders() {
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_sink = errors.clone();
        let delivered = Arc::new(Mutex::new(Vec::new()));

        let logger = builder()
            .error_sink(move |err: &Error| {
                errors_sink.lock().unwrap().push(err.to_string());
            })
            .dispatch(|d| {
                d.append(FailingAppender)
                    .append(RecordingAppender(delivered.clone()))
            })
            .build();

        let record = Record::builder().payload(format_args!("hello")).build();
        logger.log(&record);
        logger.flush();

        assert_eq!(&*delivered.lock().unwrap(), &["hello"]);
        assert_eq!(
            &*errors.lock().unwrap(),
            &[
                "failed to append record: disk full",
                "failed to flush appender: sync failed",
            ]
        );
    }
}
