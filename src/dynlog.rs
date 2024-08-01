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

use log::Record;
use log::{Log, Metadata};
use std::fmt::Debug;
use std::sync::Arc;

use crate::append::Append;
use crate::append::AppendImpl;
use crate::filter::{Filter, FilterImpl, FilterResult};

pub struct DynLog(Arc<dyn Log>);

impl Debug for DynLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynLog {{ ... }}")
    }
}

impl DynLog {
    pub fn new(log: impl Log + 'static) -> Self {
        Self(Arc::new(log))
    }

    pub fn new_arc(log: Arc<dyn Log>) -> Self {
        Self(log)
    }
}

impl Append for DynLog {
    fn try_append(&self, record: &Record) -> anyhow::Result<()> {
        (*self.0).log(record);
        Ok(())
    }

    fn flush(&self) {
        (*self.0).flush()
    }

    fn default_filters(&self) -> Option<Vec<FilterImpl>> {
        Some(vec![Self::new_arc(self.0.clone()).into()])
    }
}

impl Filter for DynLog {
    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        if self.0.enabled(metadata) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl From<DynLog> for AppendImpl {
    fn from(append: DynLog) -> Self {
        AppendImpl::DynLog(append)
    }
}

impl From<DynLog> for FilterImpl {
    fn from(filter: DynLog) -> Self {
        FilterImpl::DynLog(filter)
    }
}
