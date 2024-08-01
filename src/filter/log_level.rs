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

use crate::filter::Filter;
use crate::filter::FilterResult;

#[derive(Debug, Clone)]
pub struct LogLevelFilter {
    min_level: LevelFilter,
    max_level: LevelFilter,
    on_match: FilterResult,
    on_mismatch: FilterResult,
}

impl LogLevelFilter {
    pub fn new(level: LevelFilter) -> Self {
        Self {
            min_level: LevelFilter::Off,
            max_level: level,
            on_match: FilterResult::Neutral,
            on_mismatch: FilterResult::Reject,
        }
    }

    pub fn with_min_level(mut self, level: LevelFilter) -> Self {
        debug_assert!(level <= self.max_level);
        self.min_level = level;
        self
    }

    pub fn with_max_level(mut self, level: LevelFilter) -> Self {
        debug_assert!(level >= self.min_level);
        self.max_level = level;
        self
    }

    pub fn with_on_match(mut self, result: FilterResult) -> Self {
        self.on_match = result;
        self
    }

    pub fn with_on_mismatch(mut self, result: FilterResult) -> Self {
        self.on_mismatch = result;
        self
    }
}

impl Filter for LogLevelFilter {
    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        let level = metadata.level();
        if level >= self.min_level && level <= self.max_level {
            self.on_match
        } else {
            self.on_mismatch
        }
    }
}
