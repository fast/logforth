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

//! Color utilities.

use colored::Color;
use colored::ColoredString;
use colored::Colorize;

use crate::Level;

/// Colors for different log levels.
#[derive(Debug, Clone)]
pub struct LevelColor {
    /// Color for error level logs.
    pub error: Color,
    /// Color for warning level logs.
    pub warn: Color,
    /// Color for info level logs.
    pub info: Color,
    /// Color for debug level logs.
    pub debug: Color,
    /// Color for trace level logs.
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

impl LevelColor {
    /// Colorize the log level.
    pub fn colorize_record_level(&self, no_color: bool, level: Level) -> ColoredString {
        if no_color {
            ColoredString::from(level.to_string())
        } else {
            let color = match level {
                Level::Error => self.error,
                Level::Warn => self.warn,
                Level::Info => self.info,
                Level::Debug => self.debug,
                Level::Trace => self.trace,
            };
            ColoredString::from(level.to_string()).color(color)
        }
    }
}
