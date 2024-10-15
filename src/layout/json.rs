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

use jiff::tz::TimeZone;
use log::Record;

use crate::encoder::LayoutWrappingEncoder;
use crate::layout::Layout;
use crate::Encoder;

/// A layout that formats log record as JSON lines.
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
/// You can customize the timezone of the timestamp by setting the `tz` field with a [`TimeZone`]
/// instance. Otherwise, the system timezone is used.
#[derive(Default, Debug, Clone)]
pub struct JsonLayout {
    pub tz: Option<TimeZone>,
}

impl JsonLayout {
    pub(crate) fn format(&self, record: &Record) -> anyhow::Result<String> {
        let record_line = crate::format::json::do_format(record, self.tz.clone())?;
        Ok(serde_json::to_string(&record_line)?)
    }
}

impl From<JsonLayout> for Layout {
    fn from(layout: JsonLayout) -> Self {
        Layout::Json(layout)
    }
}

impl From<JsonLayout> for Encoder {
    fn from(layout: JsonLayout) -> Self {
        LayoutWrappingEncoder::new(layout.into()).into()
    }
}
