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

use log::Log;
use log::Metadata;
use log::Record;

use crate::Append;
use crate::AppendImpl;

#[derive(Debug)]
pub struct BoxLogAppend(Box<dyn Log>);

impl BoxLogAppend {
    pub fn new(log: impl Log + 'static) -> Self {
        Self(Box::new(log))
    }
}

impl Append for BoxLogAppend {
    fn enabled(&self, metadata: &Metadata) -> bool {
        (*self.0).enabled(metadata)
    }

    fn try_append(&self, record: &Record) -> anyhow::Result<()> {
        Ok((*self.0).log(record))
    }

    fn flush(&self) {
        (*self.0).flush()
    }
}

impl From<BoxLogAppend> for AppendImpl {
    fn from(append: BoxLogAppend) -> Self {
        AppendImpl::BoxLog(append)
    }
}
