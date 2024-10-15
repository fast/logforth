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

use crate::Encoder;

#[derive(Default, Debug, Clone)]
pub struct JsonEncoder {
    pub tz: Option<TimeZone>,
}

impl JsonEncoder {
    pub(crate) fn format(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        let record_line = crate::format::json::do_format(record, self.tz.clone())?;
        Ok(serde_json::to_vec(&record_line)?)
    }
}

impl From<JsonEncoder> for Encoder {
    fn from(encoder: JsonEncoder) -> Self {
        Encoder::Json(encoder)
    }
}
