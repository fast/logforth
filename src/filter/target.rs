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

/// A filter that checks if the log level is higher than the specified level for a specific
/// target.
#[derive(Debug, Clone)]
pub struct TargetFilter {
    target: Cow<'static, str>,
    level: log::LevelFilter,
    not: bool,
}

impl TargetFilter {
    /// The filter will be applied only if the target **has** a prefix that matches the target of
    /// the log record.
    pub fn level_for(target: impl Into<Cow<'static, str>>, level: log::LevelFilter) -> Self {
        TargetFilter {
            target: target.into(),
            level,
            not: false,
        }
    }

    /// The filter will be applied only if the target **does not have** a prefix that matches the
    /// target of the log record,
    pub fn level_for_not(target: impl Into<Cow<'static, str>>, level: log::LevelFilter) -> Self {
        TargetFilter {
            target: target.into(),
            level,
            not: true,
        }
    }

    pub(crate) fn enabled(&self, metadata: &Metadata) -> FilterResult {
        let matched = metadata.target().starts_with(self.target.as_ref());
        if (matched && !self.not) || (!matched && self.not) {
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
