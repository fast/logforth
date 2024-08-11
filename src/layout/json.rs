// Copyright 2024 CratesLand Developers
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

use log::Record;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::layout::Layout;

/// A layout that formats log record as JSON lines.
///
/// Output format:
///
/// ```json
/// {"timestamp":"2024-08-01T13:57:05.099261Z","level":"ERROR","module_path":"rolling_file","file":"rolling_file.rs","line":48,"message":"Hello error!","kvs":{}}
/// {"timestamp":"2024-08-01T13:57:05.099313Z","level":"WARN","module_path":"rolling_file","file":"rolling_file.rs","line":49,"message":"Hello warn!","kvs":{}}
/// {"timestamp":"2024-08-01T13:57:05.099338Z","level":"INFO","module_path":"rolling_file","file":"rolling_file.rs","line":50,"message":"Hello info!","kvs":{}}
/// {"timestamp":"2024-08-01T13:57:05.099362Z","level":"DEBUG","module_path":"rolling_file","file":"rolling_file.rs","line":51,"message":"Hello debug!","kvs":{}}
/// {"timestamp":"2024-08-01T13:57:05.099386Z","level":"TRACE","module_path":"rolling_file","file":"rolling_file.rs","line":52,"message":"Hello trace!","kvs":{}}
/// ```
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
    timestamp: jiff::Zoned,
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

impl JsonLayout {
    pub(crate) fn format<F>(&self, record: &Record, f: &F) -> anyhow::Result<()>
    where
        F: Fn(Arguments) -> anyhow::Result<()>,
    {
        let mut kvs = Map::new();
        let mut visitor = KvCollector { kvs: &mut kvs };
        record.key_values().visit(&mut visitor)?;

        let record_line = RecordLine {
            timestamp: jiff::Zoned::now(),
            level: record.level().as_str(),
            module_path: record.module_path().unwrap_or_default(),
            file: record.file().unwrap_or_default(),
            line: record.line().unwrap_or_default(),
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
