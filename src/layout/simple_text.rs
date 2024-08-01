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

use crate::layout::kv_display::KvDisplay;
use crate::Layout;
use crate::LayoutImpl;

#[derive(Default, Debug, Clone)]
pub struct SimpleTextLayout;

impl Layout for SimpleTextLayout {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        let text = format!(
            "{} {:>5} {}: {}:{} {}{}",
            humantime::format_rfc3339_micros(SystemTime::now()),
            record.level(),
            record.module_path().unwrap_or(""),
            record
                .file()
                .and_then(|file| Path::new(file).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            record.line().unwrap_or(0),
            record.args(),
            KvDisplay::new(record.key_values()),
        );
        Ok(text.into_bytes())
    }
}

impl From<SimpleTextLayout> for LayoutImpl {
    fn from(layout: SimpleTextLayout) -> Self {
        LayoutImpl::SimpleText(layout)
    }
}
