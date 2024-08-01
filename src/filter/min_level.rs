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
pub struct MinLevel(pub LevelFilter);

impl MinLevel {
    pub(crate) fn filter(&self, metadata: &Metadata) -> FilterResult {
        let level = metadata.level();
        if level <= self.0 {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}
impl From<MinLevel> for Filter {
    fn from(filter: MinLevel) -> Self {
        Filter::MinLevel(filter)
    }
}

impl From<LevelFilter> for Filter {
    fn from(filter: LevelFilter) -> Self {
        Filter::MinLevel(MinLevel(filter))
    }
}
