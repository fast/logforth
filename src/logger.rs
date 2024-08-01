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

#[derive(Debug)]
pub struct DispatchBuilder {
    appends: Vec<AppendImpl>,
}

impl DispatchBuilder {
    pub fn new(append: impl Into<AppendImpl>) -> Self {
        Self {
            appends: vec![append.into()],
        }
    }

    pub fn append(mut self, append: impl Into<AppendImpl>) -> Self {
        self.appends.push(append.into());
        self
    }

    pub fn filter(self, filter: impl Into<FilterImpl>) -> DispatchFilterBuilder {
        DispatchFilterBuilder {
            appends: self.appends,
            filters: vec![filter.into()],
        }
    }

    pub fn layout(self, layout: impl Into<Layout>) -> Dispatch {
        Dispatch {
            filters: vec![],
            appends: self.appends,
            preferred_layout: Some(layout.into()),
        }
    }

    pub fn finish(self) -> Dispatch {
        Dispatch {
            filters: vec![],
            appends: self.appends,
            preferred_layout: None,
        }
    }
}

#[derive(Debug)]
pub struct DispatchFilterBuilder {
    appends: Vec<AppendImpl>,
    filters: Vec<FilterImpl>,
}

impl DispatchFilterBuilder {
    pub fn filter(mut self, filter: impl Into<FilterImpl>) -> Self {
        self.filters.push(filter.into());
        self
    }

    pub fn layout(self, layout: impl Into<Layout>) -> Dispatch {
        Dispatch {
            filters: self.filters,
            appends: self.appends,
            preferred_layout: Some(layout.into()),
        }
    }

    pub fn finish(self) -> Dispatch {
        Dispatch {
            filters: self.filters,
            appends: self.appends,
            preferred_layout: None,
        }
    }
}

#[derive(Debug)]
pub struct Dispatch {
    filters: Vec<FilterImpl>,
    appends: Vec<AppendImpl>,
    preferred_layout: Option<Layout>,
}

impl Dispatch {
    pub fn builder(append: impl Into<AppendImpl>) -> DispatchBuilder {
        DispatchBuilder::new(append)
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
        for filter in &self.filters {
            match filter.filter_metadata(record.metadata()) {
                FilterResult::Reject => return Ok(()),
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        fn do_append(
            record: &Record,
            append: &AppendImpl,
            preferred_layout: Option<&Layout>,
        ) -> anyhow::Result<()> {
            if let Some(filters) = append.default_filters() {
                for filter in filters {
                    match filter.filter_metadata(record.metadata()) {
                        FilterResult::Reject => return Ok(()),
                        FilterResult::Accept => break,
                        FilterResult::Neutral => {}
                    }
                }
            }

            match preferred_layout {
                Some(layout) => layout.format(record, &|record| append.try_append(record)),
                None => append
                    .default_layout()
                    .format(record, &|record| append.try_append(record)),
            }
        }

        let preferred_layout = self.preferred_layout.as_ref();

        for append in &self.appends {
            do_append(record, append, preferred_layout)?
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
