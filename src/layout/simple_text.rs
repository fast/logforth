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
use log::Record;

use crate::layout::kv_display::KvDisplay;
use crate::layout::{make_record_with_args, Layout, LayoutImpl};

#[derive(Default, Debug, Clone)]
pub struct SimpleTextLayout {
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

impl Layout for SimpleTextLayout {
    fn format_record(&self, record: &Record) -> anyhow::Result<Record> {
        let color = match record.level() {
            Level::Error => self.colors.error,
            Level::Warn => self.colors.warn,
            Level::Info => self.colors.info,
            Level::Debug => self.colors.debug,
            Level::Trace => self.colors.trace,
        };
        let record_level = record.level().to_string();
        let record_level = ColoredString::from(record_level).color(color);

        let args = format_args!(
            "{} {:>5} {}: {}:{} {}{}",
            humantime::format_rfc3339_micros(SystemTime::now()),
            record_level,
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

        Ok(make_record_with_args(args, record))
    }
}

impl From<SimpleTextLayout> for LayoutImpl {
    fn from(layout: SimpleTextLayout) -> Self {
        LayoutImpl::SimpleText(layout)
    }
}
