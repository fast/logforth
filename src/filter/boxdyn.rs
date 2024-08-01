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

use std::fmt::Debug;

use log::Metadata;
use log::Record;
use crate::filter::{Filter, FilterImpl, FilterResult};

pub struct BoxDynFilter(Box<dyn Filter + Send + Sync>);

impl Debug for BoxDynFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxDynFilter {{ ... }}")
    }
}

impl BoxDynFilter {
    pub fn new(filter: impl Filter + Send + Sync + 'static) -> Self {
        Self(Box::new(filter))
    }
}

impl Filter for BoxDynFilter {
    fn filter(&self, record: &Record) -> FilterResult {
        (*self.0).filter(record)
    }

    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        (*self.0).filter_metadata(metadata)
    }
}

impl From<BoxDynFilter> for FilterImpl {
    fn from(filter: BoxDynFilter) -> Self {
        FilterImpl::BoxDyn(filter)
    }
}

impl<T: Fn(&Metadata) -> FilterResult> Filter for T {
    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        self(metadata)
    }
}
