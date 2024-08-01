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

use log::Record;

use crate::layout::Layout;
use crate::layout::LayoutImpl;

pub struct BoxDyn(Box<dyn Layout + Send + Sync>);

impl Debug for BoxDyn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxDynLayout {{ ... }}")
    }
}

impl BoxDyn {
    pub fn new(layout: impl Layout + Send + Sync + 'static) -> Self {
        Self(Box::new(layout))
    }
}

impl Layout for BoxDyn {
    fn format_record<'a>(&'_ self, record: &'a Record<'a>) -> anyhow::Result<Record<'a>> {
        (*self.0).format_record(record)
    }
}

impl From<BoxDyn> for LayoutImpl {
    fn from(layout: BoxDyn) -> Self {
        LayoutImpl::BoxDyn(layout)
    }
}

impl<T: for<'a> Fn(&Record<'a>) -> anyhow::Result<Record<'a>>> Layout for T {
    fn format_record<'a>(&'_ self, record: &'a Record<'a>) -> anyhow::Result<Record<'a>> {
        self(record)
    }
}
