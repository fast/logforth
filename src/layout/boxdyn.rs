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

use crate::Layout;
use crate::LayoutImpl;

pub struct BoxDynLayout(Box<dyn Layout + Send + Sync>);

impl Debug for BoxDynLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxDynLayout {{ ... }}")
    }
}

impl BoxDynLayout {
    pub fn new(layout: impl Layout + Send + Sync + 'static) -> Self {
        Self(Box::new(layout))
    }
}

impl Layout for BoxDynLayout {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        (*self.0).format_bytes(record)
    }
}

impl From<BoxDynLayout> for LayoutImpl {
    fn from(layout: BoxDynLayout) -> Self {
        LayoutImpl::BoxDyn(layout)
    }
}

impl<T: Fn(&Record) -> anyhow::Result<Vec<u8>>> Layout for T {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        self(record)
    }
}
