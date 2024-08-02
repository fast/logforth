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

use std::fmt::Arguments;
use std::time::SystemTime;

use humantime::Rfc3339Timestamp;
use log::Record;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::layout::Layout;

#[derive(Default, Debug, Clone)]
pub struct JsonLayout;

struct KvCollector<'a> {
    kvs: &'a mut Map<String, Value>,
}

impl<'a, 'kvs> log::kv::Visitor<'kvs> for KvCollector<'a> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let k = key.to_string();
        let v = value.to_string();
        self.kvs.insert(k, v.into());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
struct RecordLine<'a> {
    #[serde(serialize_with = "serialize_timestamp")]
    timestamp: Rfc3339Timestamp,
    level: &'a str,
    module_path: &'a str,
    file: &'a str,
    line: u32,
    #[serde(serialize_with = "serialize_args")]
    message: &'a Arguments<'a>,
    kvs: Map<String, Value>,
}

fn serialize_args<S>(args: &Arguments, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_str(args)
}

fn serialize_timestamp<S>(timestamp: &Rfc3339Timestamp, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_str(&format_args!("{timestamp}"))
}

impl JsonLayout {
    pub(crate) fn format<F>(&self, record: &Record, f: &F) -> anyhow::Result<()>
    where
        F: Fn(Arguments) -> anyhow::Result<()>,
    {
        let mut kvs = Map::new();
        let mut visitor = KvCollector { kvs: &mut kvs };
        record.key_values().visit(&mut visitor)?;

        let record_line = RecordLine {
            timestamp: humantime::format_rfc3339_micros(SystemTime::now()),
            level: record.level().as_str(),
            module_path: record.module_path().unwrap_or_default(),
            file: record.file().unwrap_or_default(),
            line: record.line().unwrap_or(0),
            message: record.args(),
            kvs,
        };

        let text = serde_json::to_string(&record_line)?;
        f(format_args!("{text}"))
    }
}

impl From<JsonLayout> for Layout {
    fn from(layout: JsonLayout) -> Self {
        Layout::Json(layout)
    }
}
