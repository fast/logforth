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

use jiff::Zoned;
use log::Record;

use crate::append::Append;
use crate::layout::collect_kvs;
use crate::layout::KvDisplay;

/// An appender that adds log records to fastrace as an event associated to the current span.
#[derive(Default, Debug, Clone)]
pub struct FastraceEvent;

impl Append for FastraceEvent {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let message = format!("{}", record.args(),);
        fastrace::Event::add_to_local_parent(message, || {
            [("level", record.level()), ("timestamp", Zoned::now())]
                .chain(collect_kvs(record.key_values()))
        });
        Ok(())
    }

    fn flush(&self) {
        fastrace::flush();
    }
}
