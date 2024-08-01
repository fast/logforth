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
pub use log_level::LogLevelFilter;

mod log_level;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// The record will be processed without further filtering.
    Accept,
    /// The record should not be processed.
    Reject,
    /// No decision could be made, further filtering should occur.
    Neutral,
}

pub trait Filter {
    fn filter(&self, _record: &Record) -> FilterResult {
        FilterResult::Neutral
    }

    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult;
}

#[derive(Debug)]
pub enum FilterImpl {
    LogLevel(LogLevelFilter),
}

impl Filter for FilterImpl {
    fn filter(&self, record: &Record) -> FilterResult {
        match self {
            FilterImpl::LogLevel(filter) => filter.filter(record),
        }
    }

    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        match self {
            FilterImpl::LogLevel(filter) => filter.filter_metadata(metadata),
        }
    }
}
