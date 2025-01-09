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

use std::borrow::Cow;
use std::fmt::Write;

use colored::Color;
use colored::ColoredString;
use colored::Colorize;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use jiff::Zoned;
use log::Level;

use crate::layout::KvDisplay;
use crate::layout::Layout;
use crate::Diagnostic;

/// A layout that formats log record as text.
///
/// Output format:
///
/// ```text
/// 2024-08-11T22:44:57.172105+08:00 ERROR rolling_file: examples/rolling_file.rs:51 Hello error!
/// 2024-08-11T22:44:57.172219+08:00  WARN rolling_file: examples/rolling_file.rs:52 Hello warn!
/// 2024-08-11T22:44:57.172276+08:00  INFO rolling_file: examples/rolling_file.rs:53 Hello info!
/// 2024-08-11T22:44:57.172329+08:00 DEBUG rolling_file: examples/rolling_file.rs:54 Hello debug!
/// 2024-08-11T22:44:57.172382+08:00 TRACE rolling_file: examples/rolling_file.rs:55 Hello trace!
/// ```
///
/// By default, log levels are colored. You can turn on the `no-color` feature flag to disable this
/// feature. Instead, you can also set the `no_color` field to `true` to disable coloring.
///
/// You can also customize the color of each log level by setting the `colors` field with a
/// [`LevelColor`] instance.
///
/// You can customize the timezone of the timestamp by setting the `tz` field with a [`TimeZone`]
/// instance. Otherwise, the system timezone is used.
///
/// # Examples
///
/// ```
/// use logforth::layout::TextLayout;
///
/// let text_layout = TextLayout::default();
/// ```
#[derive(Debug, Clone, Default)]
pub struct TextLayout {
    colors: LevelColor,
    no_color: bool,
    tz: Option<TimeZone>,
}

impl TextLayout {
    /// Disables colored output.
    pub fn no_color(mut self) -> Self {
        self.no_color = true;
        self
    }

    /// Sets the timezone for timestamps.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::tz::TimeZone;
    /// use logforth::layout::TextLayout;
    ///
    /// let text_layout = TextLayout::default().timezone(TimeZone::UTC);
    /// ```
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.tz = Some(tz);
        self
    }

    /// Customize the color of each log level.
    ///
    /// No effect if `no_color` is set to `true` or the `no-color` feature flag is enabled.
    pub fn colors(mut self, colors: LevelColor) -> Self {
        self.colors = colors;
        self
    }

    /// Customize the color of the error log level. Default to red.
    ///
    /// No effect if `no_color` is set to `true` or the `no-color` feature flag is enabled.
    pub fn error_color(mut self, color: Color) -> Self {
        self.colors.error = color;
        self
    }

    /// Customize the color of the warn log level. Default to yellow.
    ///
    /// No effect if `no_color` is set to `true` or the `no-color` feature flag is enabled.
    pub fn warn_color(mut self, color: Color) -> Self {
        self.colors.warn = color;
        self
    }

    /// Customize the color of the info log level/ Default to green.
    ///
    /// No effect if `no_color` is set to `true` or the `no-color` feature flag is enabled.
    pub fn info_color(mut self, color: Color) -> Self {
        self.colors.info = color;
        self
    }

    /// Customize the color of the debug log level. Default to blue.
    ///
    /// No effect if `no_color` is set to `true` or the `no-color` feature flag is enabled.
    pub fn debug_color(mut self, color: Color) -> Self {
        self.colors.debug = color;
        self
    }

    /// Customize the color of the trace log level. Default to magenta.
    ///
    /// No effect if `no_color` is set to `true` or the `no-color` feature flag is enabled.
    pub fn trace_color(mut self, color: Color) -> Self {
        self.colors.trace = color;
        self
    }
}

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

impl TextLayout {
    pub(crate) fn format(
        &self,
        record: &log::Record,
        marker: Option<&Diagnostic>,
    ) -> anyhow::Result<Vec<u8>> {
        let time = match self.tz.clone() {
            Some(tz) => Timestamp::now().to_zoned(tz),
            None => Zoned::now(),
        };
        let level = if self.no_color {
            ColoredString::from(record.level().to_string())
        } else {
            let color = match record.level() {
                Level::Error => self.colors.error,
                Level::Warn => self.colors.warn,
                Level::Info => self.colors.info,
                Level::Debug => self.colors.debug,
                Level::Trace => self.colors.trace,
            };
            ColoredString::from(record.level().to_string()).color(color)
        };
        let target = record.target();
        let file = filename(record);
        let line = record.line().unwrap_or_default();
        let message = record.args();
        let kvs = KvDisplay::new(record.key_values());

        let mut text = format!("{time:.6} {level:>5} {target}: {file}:{line} {message}{kvs}");

        if let Some(marker) = marker {
            marker.mark(|key, value| {
                write!(&mut text, " {key}={value}").unwrap();
            });
        }

        Ok(text.into_bytes())
    }
}

impl From<TextLayout> for Layout {
    fn from(layout: TextLayout) -> Self {
        Layout::Text(layout)
    }
}

// obtain filename only from record's full file path
// reason: the module is already logged + full file path is noisy for text layout
fn filename<'a>(record: &'a log::Record<'a>) -> Cow<'a, str> {
    record
        .file()
        .map(std::path::Path::new)
        .and_then(std::path::Path::file_name)
        .map(std::ffi::OsStr::to_string_lossy)
        .unwrap_or_default()
}
