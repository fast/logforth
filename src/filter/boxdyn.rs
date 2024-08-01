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
use log::RecordBuilder;

use crate::Filter;
use crate::FilterImpl;
use crate::FilterResult;
use crate::Layout;

#[derive(Debug)]
pub struct BoxDynFilter(Box<dyn Filter>);

impl BoxDynFilter {
    pub fn new(layout: impl Layout + 'static) -> Self {
        Self(Box::new(layout))
    }
}

impl Filter for BoxDynFilter {
    fn filter(&self, record: &Record) -> FilterResult {
        (**self.0).filter(record)
    }

    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        (**self.0).filter_metadata(metadata)
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

impl<T: Fn(&Record) -> FilterResult> Filter for T {
    fn filter(&self, record: &Record) -> FilterResult {
        self(record)
    }

    fn filter_metadata(&self, metadata: &Metadata) -> FilterResult {
        let record = RecordBuilder::new().metadata(metadata.clone()).build();
        self(&record)
    }
}
