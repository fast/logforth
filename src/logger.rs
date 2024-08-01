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
    pub fn new() -> Dispatch<false, false> {
        Self {
            filters: vec![],
            appends: vec![],
            layout: None,
        }
    }

    pub fn filter(mut self, filter: impl Into<Filter>) -> Dispatch<false, false> {
        self.filters.push(filter.into());
        self
    }

    pub fn layout(self, layout: impl Into<Layout>) -> Dispatch<true, false> {
        Dispatch {
            filters: self.filters,
            appends: self.appends,
            layout: Some(layout.into()),
        }
    }
}

impl<const LAYOUT: bool, const APPEND: bool> Dispatch<LAYOUT, APPEND> {
    pub fn append(self, append: impl Append) -> Dispatch<true, true> {
        Dispatch {
            filters: self.filters,
            appends: vec![Box::new(append)],
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
    fn enabled(&self, _: &Metadata) -> bool {
        true
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
