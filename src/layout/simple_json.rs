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

use std::path::Path;
use std::time::SystemTime;

use log::Record;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::Layout;
use crate::LayoutImpl;

#[derive(Default, Debug, Clone)]
pub struct SimpleJsonLayout;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordLine<'a> {
    timestamp: String,
    level: String,
    module_path: &'a str,
    file: &'a str,
    line: u32,
    message: String,
    kvs: Map<String, Value>,
}

impl Layout for SimpleJsonLayout {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        let mut kvs = Map::new();
        let mut visitor = KvCollector { kvs: &mut kvs };
        record.key_values().visit(&mut visitor)?;

        let timestamp = humantime::format_rfc3339_micros(SystemTime::now());
        let record_line = RecordLine {
            timestamp: format!("{timestamp}"),
            level: format!("{}", record.level()),
            module_path: record.module_path().unwrap_or(""),
            file: record
                .file()
                .and_then(|file| Path::new(file).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            line: record.line().unwrap_or(0),
            message: format!("{}", record.args()),
            kvs,
        };

        let text = serde_json::to_string(&record_line)?;
        Ok(text.into_bytes())
    }
}

impl From<SimpleJsonLayout> for LayoutImpl {
    fn from(layout: SimpleJsonLayout) -> Self {
        LayoutImpl::SimpleJson(layout)
    }
}