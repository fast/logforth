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

use log::Metadata;
use log::Record;
use crate::append::AppendImpl;
use crate::filter::FilterImpl;
use crate::layout::{Layout, LayoutImpl};

#[derive(Debug)]
pub struct DispatchAppend {
    appends: Vec<AppendImpl>,
    filters: Vec<FilterImpl>,
    layout: LayoutImpl,
}

impl DispatchAppend {
    pub fn new(
        // at least one inner append
        append: impl Into<AppendImpl>,
    ) -> Self {
        Self {
            appends: vec![append.into()],
            filters: Vec::new(),
        }
    }

    pub fn chain(mut self, append: impl Into<AppendImpl>) -> Self {
        self.appends.push(append.into());
        self
    }

    pub fn filter(mut self, filter: impl Into<FilterImpl>) -> Self {
        self.filters.push(filter.into());
        self
    }
}

impl Append for DispatchAppend {
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
                FilterResult::Neutral => match filter.filter(record) {
                    FilterResult::Reject => return Ok(()),
                    FilterResult::Accept => break,
                    FilterResult::Neutral => {}
                },
            }
        }

        for append in &self.appends {
            let formatted = self.layout.format_bytes(record);
            append.try_append(record)?;
        }

        Ok(())
    }

    fn flush(&self) {
        for append in &self.appends {
            append.flush();
        }
    }
}

impl From<DispatchAppend> for AppendImpl {
    fn from(append: DispatchAppend) -> Self {
        AppendImpl::Dispatch(append)
    }
}
