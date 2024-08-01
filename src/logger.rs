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
use crate::append::AppendImpl;
use crate::filter::Filter;
use crate::filter::FilterImpl;
use crate::filter::FilterResult;
use crate::layout::Layout;
use crate::layout::LayoutImpl;

#[derive(Debug)]
pub struct Dispatch {
    filters: Vec<FilterImpl>,
    appends: Vec<AppendImpl>,
    preferred_layout: Option<LayoutImpl>,
}

impl Dispatch {
    pub fn new() -> Self {
        Self {
            filters: vec![],
            appends: vec![],
            preferred_layout: None,
        }
    }

    pub fn filter(mut self, filter: impl Into<FilterImpl>) -> Self {
        self.filters.push(filter.into());
        self
    }

    pub fn append(mut self, append: impl Into<AppendImpl>) -> Self {
        self.appends.push(append.into());
        self
    }

    pub fn layout(mut self, layout: impl Into<LayoutImpl>) -> Self {
        self.preferred_layout = Some(layout.into());
        self
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        for filter in &self.filters {
            match filter.filter_metadata(metadata) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn try_append(&self, record: &Record) -> anyhow::Result<()> {
        // TODO(tisonkun): perhaps too heavy to check filters twice.
        for filter in &self.filters {
            match filter.filter_metadata(record.metadata()) {
                FilterResult::Reject => return Ok(()),
                FilterResult::Accept => break,
                FilterResult::Neutral => match filter.filter(record) {
                    FilterResult::Reject => return Ok(()),
                    FilterResult::Accept => break,
                    FilterResult::Neutral => {}
                },
            }
        }

        let record = record.clone();
        for append in &self.appends {
            let record = match self.preferred_layout.as_ref() {
                Some(layout) => layout.format_record(record)?,
                None => append.default_layout().format_record(record)?,
            };
            append.try_append(&record)?;
        }
        Ok(())
    }

    fn flush(&self) {
        for append in &self.appends {
            append.flush();
        }
    }
}

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
    pub fn new() -> Self {
        Self { dispatches: vec![] }
    }

    pub fn dispatch(mut self, dispatch: Dispatch) -> Self {
        self.dispatches.push(dispatch);
        self
    }

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
            if let Err(err) = dispatch.try_append(record) {
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
