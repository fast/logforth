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
    error_sink: Box<dyn ErrorSink>,
}

impl Logger {
    pub(super) fn new(dispatches: Vec<Dispatch>, error_sink: Box<dyn ErrorSink>) -> Self {
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
        let default_error_sink = &*self.error_sink;
        for dispatch in &self.dispatches {
            dispatch.log(record, default_error_sink);
        }
    }

    /// Flush any buffered records.
    pub fn flush(&self) {
        let default_error_sink = &*self.error_sink;
        for dispatch in &self.dispatches {
            dispatch.flush(default_error_sink);
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
            "A Dispatch must have at least one filter"
        );

        Self {
            filters,
            diagnostics,
            appends,
            error_sink,
        }
    }

    fn diagnostics(&self) -> &dyn Diagnostic {
        match self.diagnostics.as_slice() {
            [] => &(),
            [single] => single.as_ref(),
            _ => &self.diagnostics,
        }
    }

    fn enabled(&self, criteria: &FilterCriteria) -> bool {
        let diagnostics = self.diagnostics();

        for filter in &self.filters {
            match filter.enabled(criteria, diagnostics) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn log(&self, record: &Record, default_error_sink: &dyn ErrorSink) {
        let diagnostics = self.diagnostics();

        for filter in &self.filters {
            match filter.matches(record, diagnostics) {
                FilterResult::Reject => return,
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        let error_sink = self.error_sink.as_deref().unwrap_or(default_error_sink);
        for append in &self.appends {
            if let Err(err) = append.append(record, diagnostics) {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    use super::*;
    use crate::builder;
    use crate::error::BlackHoleErrorSink;

    #[derive(Debug)]
    struct FailingAppender(&'static str);

    impl Append for FailingAppender {
        fn append(&self, _: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
            Err(Error::new(self.0))
        }

        fn flush(&self) -> Result<(), Error> {
            Err(Error::new(self.0))
        }
    }

    #[derive(Debug)]
    struct RecordingAppender(Arc<Mutex<Vec<String>>>);

    impl Append for RecordingAppender {
        fn append(&self, record: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
            self.0.lock().unwrap().push(record.payload().to_string());
            Ok(())
        }

        fn flush(&self) -> Result<(), Error> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingErrorSink {
        errors: Mutex<Vec<String>>,
    }

    impl ErrorSink for RecordingErrorSink {
        fn sink(&self, err: &Error) {
            self.errors.lock().unwrap().push(format!("{err:#}"));
        }
    }

    #[test]
    fn logger_routes_errors_with_dispatch_fallback_and_override() {
        let logger_errors = Arc::new(RecordingErrorSink::default());
        let override_errors = Arc::new(RecordingErrorSink::default());
        let delivered = Arc::new(Mutex::new(Vec::new()));

        let logger = builder()
            .error_sink(logger_errors.clone())
            .dispatch(|d| {
                d.append(FailingAppender("primary failed"))
                    .append(RecordingAppender(delivered.clone()))
            })
            .dispatch(|d| {
                d.error_sink(BlackHoleErrorSink::new())
                    .append(FailingAppender("silenced failed"))
            })
            .dispatch(|d| {
                d.error_sink(override_errors.clone())
                    .append(FailingAppender("override failed"))
            })
            .build();

        let record = Record::builder().payload(format_args!("hello")).build();
        logger.log(&record);
        logger.flush();

        assert_eq!(&*delivered.lock().unwrap(), &["hello"]);
        assert_eq!(
            &*logger_errors.errors.lock().unwrap(),
            &[
                "failed to append record: primary failed",
                "failed to flush appender: primary failed",
            ]
        );
        assert_eq!(
            &*override_errors.errors.lock().unwrap(),
            &[
                "failed to append record: override failed",
                "failed to flush appender: override failed",
            ]
        );
    }
}
