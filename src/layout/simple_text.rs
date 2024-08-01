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

use colored::Color;
use colored::ColoredString;
use colored::Colorize;
use log::Level;

use crate::layout::kv_display::KvDisplay;
use crate::layout::Layout;
use crate::layout::LayoutImpl;

#[derive(Default, Debug, Clone)]
pub struct SimpleText {
    pub colors: ColoredLevel,
}

#[derive(Debug, Clone)]
pub struct ColoredLevel {
    pub error: Color,
    pub warn: Color,
    pub info: Color,
    pub debug: Color,
    pub trace: Color,
}

impl Default for ColoredLevel {
    fn default() -> Self {
        Self {
            error: Color::Red,
            warn: Color::Yellow,
            info: Color::Green,
            debug: Color::Blue,
            trace: Color::Magenta,
        }
    }
}

impl Layout for SimpleText {
    fn format_record<F>(&self, record: &log::Record, f: F) -> anyhow::Result<()>
    where
        F: Fn(&log::Record) -> anyhow::Result<()>,
    {
        let color = match record.level() {
            Level::Error => self.colors.error,
            Level::Warn => self.colors.warn,
            Level::Info => self.colors.info,
            Level::Debug => self.colors.debug,
            Level::Trace => self.colors.trace,
        };

        let time = humantime::format_rfc3339_micros(SystemTime::now());
        let level = ColoredString::from(record.level().to_string()).color(color);
        let module = record.module_path().unwrap_or("");
        let file = record
            .file()
            .and_then(|file| Path::new(file).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let line = record.line().unwrap_or(0);
        let message = record.args();
        let kvs = KvDisplay::new(record.key_values());

        f(&record
            .to_builder()
            .args(format_args!(
                "{time} {level:>5} {module}: {file}:{line} {message}{kvs}"
            ))
            .build())
    }
}

impl From<SimpleText> for LayoutImpl {
    fn from(layout: SimpleText) -> Self {
        LayoutImpl::SimpleText(layout)
    }
}
