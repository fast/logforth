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

use log::LevelFilter;
use log::Metadata;

use crate::filter::FilterResult;
use crate::filter::{Filter, FilterImpl};

#[derive(Debug, Clone)]
pub struct LogLevelFilter {
    max_level: LevelFilter,
}

impl LogLevelFilter {
    pub fn new(level: LevelFilter) -> Self {
        Self { max_level: level }
    }
}

impl Filter for LogLevelFilter {
    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        let level = metadata.level();
        if level <= self.max_level {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl From<LogLevelFilter> for FilterImpl {
    fn from(filter: LogLevelFilter) -> Self {
        FilterImpl::LogLevel(filter)
    }
}
