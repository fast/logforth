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

use std::fmt::Arguments;

use jiff::tz::TimeZone;
use jiff::Timestamp;
use jiff::Zoned;
use log::Record;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::layout::Layout;

/// A JSON layout for formatting log records.
///
/// Output format:
///
/// ```json
/// {"timestamp":"2024-08-11T22:44:57.172051+08:00","level":"ERROR","module_path":"rolling_file","file":"examples/rolling_file.rs","line":51,"message":"Hello error!","kvs":{}}
/// {"timestamp":"2024-08-11T22:44:57.172187+08:00","level":"WARN","module_path":"rolling_file","file":"examples/rolling_file.rs","line":52,"message":"Hello warn!","kvs":{}}
/// {"timestamp":"2024-08-11T22:44:57.172246+08:00","level":"INFO","module_path":"rolling_file","file":"examples/rolling_file.rs","line":53,"message":"Hello info!","kvs":{}}
/// {"timestamp":"2024-08-11T22:44:57.172300+08:00","level":"DEBUG","module_path":"rolling_file","file":"examples/rolling_file.rs","line":54,"message":"Hello debug!","kvs":{}}
/// {"timestamp":"2024-08-11T22:44:57.172353+08:00","level":"TRACE","module_path":"rolling_file","file":"examples/rolling_file.rs","line":55,"message":"Hello trace!","kvs":{}}
/// ```
///
/// # Examples
///
/// ```
/// use logforth::layout::JsonLayout;
///
/// let json_layout = JsonLayout::default();
/// ```
#[derive(Default, Debug, Clone)]
pub struct JsonLayout {
    tz: Option<TimeZone>,
}

impl JsonLayout {
    /// Sets the timezone for timestamps.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::tz::TimeZone;
    /// use logforth::layout::JsonLayout;
    ///
    /// let json_layout = JsonLayout::default().timezone(TimeZone::UTC);
    /// ```
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.tz = Some(tz);
        self
    }
}

struct KvCollector<'a> {
    kvs: &'a mut Map<String, Value>,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvCollector<'_> {
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
pub(crate) struct RecordLine<'a> {
    #[serde(serialize_with = "serialize_time_zone")]
    timestamp: Zoned,
    level: &'a str,
    target: &'a str,
    file: &'a str,
    line: u32,
    #[serde(serialize_with = "serialize_args")]
    message: &'a Arguments<'a>,
    kvs: Map<String, Value>,
}

fn serialize_time_zone<S>(timestamp: &Zoned, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_str(&format_args!("{timestamp:.6}"))
}

fn serialize_args<S>(args: &Arguments, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_str(args)
}

impl JsonLayout {
    pub(crate) fn format(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        let mut kvs = Map::new();
        let mut visitor = KvCollector { kvs: &mut kvs };
        record.key_values().visit(&mut visitor)?;

        let record_line = RecordLine {
            timestamp: match self.tz.clone() {
                Some(tz) => Timestamp::now().to_zoned(tz),
                None => Zoned::now(),
            },
            level: record.level().as_str(),
            target: record.target(),
            file: record.file().unwrap_or_default(),
            line: record.line().unwrap_or_default(),
            message: record.args(),
            kvs,
        };

        Ok(serde_json::to_vec(&record_line)?)
    }
}

impl From<JsonLayout> for Layout {
    fn from(layout: JsonLayout) -> Self {
        Layout::Json(layout)
    }
}
