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

//! A JSON layout for formatting log records.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub extern crate jiff;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::layout::Layout;
use logforth_core::record::Record;
use serde::Serialize;
use serde_json::Map;

/// A JSON layout for formatting log records.
///
/// Output format:
///
/// ```json
/// {"timestamp":"2024-08-11T22:44:57.172051+08:00","level":"ERROR","module_path":"file","file":"examples/file.rs","line":51,"message":"Hello error!"}
/// {"timestamp":"2024-08-11T22:44:57.172187+08:00","level":"WARN","module_path":"file","file":"examples/file.rs","line":52,"message":"Hello warn!"}
/// {"timestamp":"2024-08-11T22:44:57.172246+08:00","level":"INFO","module_path":"file","file":"examples/file.rs","line":53,"message":"Hello info!"}
/// {"timestamp":"2024-08-11T22:44:57.172300+08:00","level":"DEBUG","module_path":"file","file":"examples/file.rs","line":54,"message":"Hello debug!"}
/// {"timestamp":"2024-08-11T22:44:57.172353+08:00","level":"TRACE","module_path":"file","file":"examples/file.rs","line":55,"message":"Hello trace!"}
/// ```
///
/// # Examples
///
/// ```
/// use logforth_layout_json::JsonLayout;
///
/// let json_layout = JsonLayout::default();
/// ```
#[derive(Debug, Clone)]
pub struct JsonLayout {
    timezone: TimeZone,
    timestamp_format: Option<fn(Timestamp, &TimeZone) -> String>,
}

impl Default for JsonLayout {
    fn default() -> Self {
        Self {
            timezone: TimeZone::system(),
            timestamp_format: None,
        }
    }
}

impl JsonLayout {
    /// Set the timezone for timestamps.
    ///
    /// Defaults to the system timezone.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::tz::TimeZone;
    /// use logforth_layout_json::JsonLayout;
    ///
    /// let layout = JsonLayout::default().timezone(TimeZone::UTC);
    /// ```
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.timezone = tz;
        self
    }

    /// Set a user-defined timestamp format function.
    ///
    /// Default to formatting the timestamp with offset as ISO 8601. See the example below.
    ///
    /// For other formatting options, refer to the [jiff::fmt::strtime] documentation.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::Timestamp;
    /// use jiff::tz::TimeZone;
    /// use logforth_layout_json::JsonLayout;
    ///
    /// // This is equivalent to the default timestamp format.
    /// let layout = JsonLayout::default()
    ///     .timestamp_format(|ts, tz| format!("{:.6}", ts.display_with_offset(tz.to_offset(ts))));
    /// ```
    pub fn timestamp_format(mut self, format: fn(Timestamp, &TimeZone) -> String) -> Self {
        self.timestamp_format = Some(format);
        self
    }
}

struct KvCollector<'a> {
    kvs: &'a mut Map<String, serde_json::Value>,
}

impl Visitor for KvCollector<'_> {
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        let key = key.to_string();
        match serde_json::to_value(&value) {
            Ok(value) => self.kvs.insert(key, value),
            Err(_) => self.kvs.insert(key, value.to_string().into()),
        };
        Ok(())
    }
}

fn default_timestamp_format(ts: Timestamp, tz: &TimeZone) -> String {
    let offset = tz.to_offset(ts);
    format!("{:.6}", ts.display_with_offset(offset))
}

#[derive(Debug, Clone, Serialize)]
struct RecordLine<'a> {
    timestamp: String,
    level: &'a str,
    target: &'a str,
    file: &'a str,
    line: u32,
    message: std::fmt::Arguments<'a>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    kvs: Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    diags: Map<String, serde_json::Value>,
}

impl Layout for JsonLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        let diagnostics = diags;

        // SAFETY: jiff::Timestamp::try_from only fails if the time is out of range, which is
        // very unlikely if the system clock is correct.
        let ts = Timestamp::try_from(record.time()).unwrap();
        let timestamp = if let Some(format) = self.timestamp_format {
            format(ts, &self.timezone)
        } else {
            default_timestamp_format(ts, &self.timezone)
        };

        let mut kvs = Map::new();
        let mut kvs_visitor = KvCollector { kvs: &mut kvs };
        record.key_values().visit(&mut kvs_visitor)?;

        let mut diags = Map::new();
        let mut diags_visitor = KvCollector { kvs: &mut diags };
        for d in diagnostics {
            d.visit(&mut diags_visitor)?;
        }

        let record_line = RecordLine {
            timestamp,
            level: record.level().name(),
            target: record.target(),
            file: record.file().unwrap_or_default(),
            line: record.line().unwrap_or_default(),
            message: record.payload(),
            kvs,
            diags,
        };

        // SAFETY: RecordLine is serializable.
        Ok(serde_json::to_vec(&record_line).unwrap())
    }
}
