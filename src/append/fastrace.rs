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

use std::time::SystemTime;

use log::Record;

use crate::append::Append;
use crate::append::AppendImpl;
use crate::layout::KvDisplay;

#[derive(Default, Debug, Clone)]
pub struct Fastrace;

impl Append for Fastrace {
    fn try_append(&self, record: &Record) -> anyhow::Result<()> {
        let message = format!(
            "{} {:>5} {}{}",
            humantime::format_rfc3339_micros(SystemTime::now()),
            record.level(),
            record.args(),
            KvDisplay::new(record.key_values()),
        );
        fastrace::Event::add_to_local_parent(message, || []);
        Ok(())
    }
}

impl From<Fastrace> for AppendImpl {
    fn from(append: Fastrace) -> Self {
        AppendImpl::Fastrace(append)
    }
}
