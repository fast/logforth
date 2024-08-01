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

use log::Log;
use log::Metadata;
use log::Record;

use crate::append::Append;
use crate::append::AppendImpl;

pub struct BoxLog(Box<dyn Log>);

impl Debug for BoxLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxLogAppend {{ ... }}")
    }
}

impl BoxLog {
    pub fn new(log: impl Log + 'static) -> Self {
        Self(Box::new(log))
    }
}

impl Append for BoxLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        (*self.0).enabled(metadata)
    }

    fn try_append(&self, record: &Record) -> anyhow::Result<()> {
        (*self.0).log(record);
        Ok(())
    }

    fn flush(&self) {
        (*self.0).flush()
    }
}

impl From<BoxLog> for AppendImpl {
    fn from(append: BoxLog) -> Self {
        AppendImpl::BoxLog(append)
    }
}
