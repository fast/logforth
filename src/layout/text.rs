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
use std::path::Path;
use std::time::SystemTime;

use colored::Color;
use colored::ColoredString;
use colored::Colorize;
use log::Level;

use crate::layout::KvDisplay;
use crate::layout::Layout;

#[derive(Default, Debug, Clone)]
pub struct TextLayout {
    pub colors: LevelColor,
}

#[derive(Debug, Clone)]
pub struct LevelColor {
    pub error: Color,
    pub warn: Color,
    pub info: Color,
    pub debug: Color,
    pub trace: Color,
}

impl Default for LevelColor {
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

impl TextLayout {
    pub(crate) fn format<F>(&self, record: &log::Record, f: &F) -> anyhow::Result<()>
    where
        F: Fn(Arguments) -> anyhow::Result<()>,
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

        f(format_args!(
            "{time} {level:>5} {module}: {file}:{line} {message}{kvs}"
        ))
    }
}

impl From<TextLayout> for Layout {
    fn from(layout: TextLayout) -> Self {
        Layout::Text(layout)
    }
}
