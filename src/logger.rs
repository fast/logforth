// Copyright 2024 tison <wander4096@gmail.com>
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
use crate::layout::Layout;

/// A grouped set of appenders, filters, and optional layout.
///
/// The [Logger] facade dispatches log records to one or more [Dispatch] instances.
/// Each [Dispatch] instance contains a set of filters, appenders, and an optional layout.
///
/// `filters` are used to determine whether a log record should be passed to the appenders.
/// `appends` are used to write log records to a destination. Each appender has its own
/// default layout. If the [Dispatch] has a layout, it will be used instead of the default layout.
#[derive(Debug)]
pub struct Dispatch<const LAYOUT: bool = true, const APPEND: bool = true> {
    filters: Vec<Filter>,
    appends: Vec<Box<dyn Append>>,
    layout: Option<Layout>,
}

impl Default for Dispatch<false, false> {
    fn default() -> Dispatch<false, false> {
        Self::new()
    }
}

impl Dispatch<false, false> {
    /// Create a new incomplete [Dispatch] instance.
    ///
    /// At least one append must be added to the [Dispatch] before it can be used.
    pub fn new() -> Dispatch<false, false> {
        Self {
            filters: vec![],
            appends: vec![],
            layout: None,
        }
    }

    /// Add a [Filter] to the [Dispatch].
    pub fn filter(mut self, filter: impl Into<Filter>) -> Dispatch<false, false> {
        self.filters.push(filter.into());
        self
    }

    /// Add the preferred [Layout] to the [Dispatch]. At most one layout can be added to a
    /// [Dispatch].
    pub fn layout(self, layout: impl Into<Layout>) -> Dispatch<true, false> {
        Dispatch {
            filters: self.filters,
            appends: self.appends,
            layout: Some(layout.into()),
        }
    }
}

impl<const LAYOUT: bool, const APPEND: bool> Dispatch<LAYOUT, APPEND> {
    /// Add an [Append] to the [Dispatch].
    pub fn append(mut self, append: impl Append) -> Dispatch<true, true> {
        self.appends.push(Box::new(append));

        Dispatch {
            filters: self.filters,
            appends: self.appends,
            layout: self.layout,
        }
    }
}

impl Dispatch {
    fn enabled(&self, metadata: &Metadata) -> bool {
        for filter in &self.filters {
            match filter.filter(metadata) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn log(&self, record: &Record) -> anyhow::Result<()> {
        let layout = self.layout.as_ref();
        for append in &self.appends {
            match layout {
                Some(layout) => layout.format(record, &|record| append.append(record))?,
                None => append
                    .default_layout()
                    .format(record, &|record| append.append(record))?,
            }
        }
        Ok(())
    }

    fn flush(&self) {
        for append in &self.appends {
            append.flush();
        }
    }
}

/// A logger facade that dispatches log records to one or more [Dispatch] instances.
///
/// This struct implements [log::Log] to bridge Logforth's logging implementations
/// with the [log] crate.
#[derive(Debug)]
pub struct Logger {
    dispatches: Vec<Dispatch>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    /// Create a new [Logger] instance.
    pub fn new() -> Logger {
        Self { dispatches: vec![] }
    }
}

impl Logger {
    /// Add a [Dispatch] to the [Logger].
    pub fn dispatch(mut self, dispatch: Dispatch) -> Logger {
        self.dispatches.push(dispatch);
        self
    }

    /// Set up the global logger with the [Logger] instance.
    ///
    /// # Errors
    ///
    /// An error is returned if the global logger has already been set.
    pub fn apply(self) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(LevelFilter::Trace);
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
            if dispatch.enabled(record.metadata()) {
                if let Err(err) = dispatch.log(record) {
                    handle_error(record, err);
                }
            }
        }
    }

    fn flush(&self) {
        for dispatch in &self.dispatches {
            dispatch.flush();
        }
    }
}

// TODO(tisonkun): logback and log4j2 support custom error handling (status listener).
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
