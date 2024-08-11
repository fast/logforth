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
use std::time::SystemTime;

use colored::Color;
use colored::ColoredString;
use colored::Colorize;
use log::Level;

use crate::layout::KvDisplay;
use crate::layout::Layout;

/// A layout that formats log record as text.
///
/// Output format:
///
/// ```text
/// 2024-08-02T12:49:03.102343Z ERROR simple_stdio: examples/simple_stdio.rs:32 Hello error!
/// 2024-08-02T12:49:03.102442Z  WARN simple_stdio: examples/simple_stdio.rs:33 Hello warn!
/// 2024-08-02T12:49:03.102447Z  INFO simple_stdio: examples/simple_stdio.rs:34 Hello info!
/// 2024-08-02T12:49:03.102450Z DEBUG simple_stdio: examples/simple_stdio.rs:35 Hello debug!
/// 2024-08-02T12:49:03.102453Z TRACE simple_stdio: examples/simple_stdio.rs:36 Hello trace!
/// ```
///
/// By default, log levels are colored. You can turn on the `no-color` feature flag to disable this
/// feature.
///
/// You can also customize the color of each log level by setting the `colors` field with a
/// [`LevelColor`] instance.
#[derive(Default, Debug, Clone)]
pub struct TextLayout {
    pub colors: LevelColor,
}

/// Customize the color of each log level.
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
        let module = record.module_path().unwrap_or_default();
        let file = record.file().unwrap_or_default();
        let line = record.line().unwrap_or_default();
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
