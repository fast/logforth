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

use crate::Append;
use crate::AppendImpl;

pub struct BoxDynAppend(Box<dyn Append + Send + Sync>);

impl BoxDynAppend {
    pub fn new(append: impl Append + Send + Sync + 'static) -> Self {
        Self(Box::new(append))
    }
}

impl Debug for BoxDynAppend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxDynAppend {{ ... }}")
    }
}

impl Append for BoxDynAppend {
    fn enabled(&self, metadata: &Metadata) -> bool {
        (*self.0).enabled(metadata)
    }

    fn try_append(&self, record: &Record) -> anyhow::Result<()> {
        (*self.0).try_append(record)
    }

    fn flush(&self) {
        (*self.0).flush()
    }
}

impl From<BoxDynAppend> for AppendImpl {
    fn from(append: BoxDynAppend) -> Self {
        AppendImpl::BoxDyn(append)
    }
}
