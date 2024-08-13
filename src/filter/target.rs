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

use std::borrow::Cow;

use log::Metadata;

use crate::filter::Filter;
use crate::filter::FilterResult;

/// A filter that checks if the log level is at higher than the specified level for a specific
/// target.
///
/// If the target has a prefix that matches the target of the log record, the filter will be
/// applied.
#[derive(Debug, Clone)]
pub struct TargetFilter {
    target: Cow<'static, str>,
    level: log::LevelFilter,
}

impl TargetFilter {
    pub fn level_for(target: impl Into<Cow<'static, str>>, level: log::LevelFilter) -> Self {
        TargetFilter {
            target: target.into(),
            level,
        }
    }

    pub(crate) fn filter(&self, metadata: &Metadata) -> FilterResult {
        if metadata.target().starts_with(self.target.as_ref()) {
            let level = metadata.level();
            if level <= self.level {
                FilterResult::Neutral
            } else {
                FilterResult::Reject
            }
        } else {
            FilterResult::Neutral
        }
    }
}

impl From<TargetFilter> for Filter {
    fn from(filter: TargetFilter) -> Self {
        Filter::Target(filter)
    }
}
