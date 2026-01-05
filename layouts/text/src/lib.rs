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

//! A layout that formats log record as optionally colored text.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub extern crate colored;
pub extern crate jiff;

use std::fmt::Write;

use colored::Color;
use colored::ColoredString;
use colored::Colorize;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::layout::Layout;
use logforth_core::record::Level;
use logforth_core::record::Record;

/// A layout that formats log record as optionally colored text.
///
/// Output format:
///
/// ```text
/// 2024-08-11T22:44:57.172105+08:00 ERROR file: examples/file.rs:51 Hello error!
/// 2024-08-11T22:44:57.172219+08:00  WARN file: examples/file.rs:52 Hello warn!
/// 2024-08-11T22:44:57.172276+08:00  INFO file: examples/file.rs:53 Hello info!
/// 2024-08-11T22:44:57.172329+08:00 DEBUG file: examples/file.rs:54 Hello debug!
/// 2024-08-11T22:44:57.172382+08:00 TRACE file: examples/file.rs:55 Hello trace!
/// ```
///
/// By default, log levels are colored. You can set the `no_color` field to `true` to disable
/// coloring.
///
/// You can also customize the color of each log level with [`error_color`](TextLayout::error_color)
/// and so on.
///
/// You can customize the timezone of the timestamp by setting the `tz` field with a [`TimeZone`]
/// instance. Otherwise, the system timezone is used.
///
/// # Examples
///
/// ```
/// use logforth_layout_text::TextLayout;
///
/// let layout = TextLayout::default();
/// ```
#[derive(Debug, Clone)]
pub struct TextLayout {
    colors: LevelColor,
    no_color: bool,
    timezone: TimeZone,
    timestamp_format: Option<fn(Timestamp, &TimeZone) -> String>,
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            colors: LevelColor::default(),
            no_color: false,
            timezone: TimeZone::system(),
            timestamp_format: None,
        }
    }
}

impl TextLayout {
    /// Customize the color of the error log level. Default to bright red.
    ///
    /// No effect if `no_color` is set to `true`.
    pub fn fatal_color(mut self, color: Color) -> Self {
        self.colors.fatal = color;
        self
    }

    /// Customize the color of the error log level. Default to red.
    ///
    /// No effect if `no_color` is set to `true`.
    pub fn error_color(mut self, color: Color) -> Self {
        self.colors.error = color;
        self
    }

    /// Customize the color of the warn log level. Default to yellow.
    ///
    /// No effect if `no_color` is set to `true`.
    pub fn warn_color(mut self, color: Color) -> Self {
        self.colors.warn = color;
        self
    }

    /// Customize the color of the info log level/ Default to green.
    ///
    /// No effect if `no_color` is set to `true`.
    pub fn info_color(mut self, color: Color) -> Self {
        self.colors.info = color;
        self
    }

    /// Customize the color of the debug log level. Default to blue.
    ///
    /// No effect if `no_color` is set to `true`.
    pub fn debug_color(mut self, color: Color) -> Self {
        self.colors.debug = color;
        self
    }

    /// Customize the color of the trace log level. Default to magenta.
    ///
    /// No effect if `no_color` is set to `true`.
    pub fn trace_color(mut self, color: Color) -> Self {
        self.colors.trace = color;
        self
    }

    /// Disable colored output.
    pub fn no_color(mut self) -> Self {
        self.no_color = true;
        self
    }

    /// Set the timezone for timestamps.
    ///
    /// Defaults to the system timezone if not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::tz::TimeZone;
    /// use logforth_layout_text::TextLayout;
    ///
    /// let layout = TextLayout::default().timezone(TimeZone::UTC);
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
    /// use logforth_layout_text::TextLayout;
    ///
    /// // This is equivalent to the default timestamp format.
    /// let layout = TextLayout::default()
    ///     .timestamp_format(|ts, tz| format!("{:.6}", ts.display_with_offset(tz.to_offset(ts))));
    /// ```
    pub fn timestamp_format(mut self, format: fn(Timestamp, &TimeZone) -> String) -> Self {
        self.timestamp_format = Some(format);
        self
    }

    fn format_record_level(&self, level: Level) -> ColoredString {
        self.colors.colorize_record_level(self.no_color, level)
    }
}

struct KvWriter {
    text: String,
}

impl Visitor for KvWriter {
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        use std::fmt::Write;

        // SAFETY: write to a string always succeeds
        write!(&mut self.text, " {key}={value}").unwrap();
        Ok(())
    }
}

fn default_timestamp_format(ts: Timestamp, tz: &TimeZone) -> String {
    let offset = tz.to_offset(ts);
    format!("{:.6}", ts.display_with_offset(offset))
}

impl Layout for TextLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        // SAFETY: jiff::Timestamp::try_from only fails if the time is out of range, which is
        // very unlikely if the system clock is correct.
        let ts = Timestamp::try_from(record.time()).unwrap();
        let time = if let Some(format) = self.timestamp_format {
            format(ts, &self.timezone)
        } else {
            default_timestamp_format(ts, &self.timezone)
        };

        let level = self.format_record_level(record.level());
        let target = record.target();
        let file = record.filename();
        let line = record.line().unwrap_or_default();
        let message = record.payload();

        let mut visitor = KvWriter { text: time };
        write!(
            &mut visitor.text,
            " {level:>6} {target}: {file}:{line} {message}"
        )
        .unwrap();
        record.key_values().visit(&mut visitor)?;
        for d in diags {
            d.visit(&mut visitor)?;
        }

        Ok(visitor.text.into_bytes())
    }
}

/// Colors for different log levels.
#[derive(Debug, Clone)]
struct LevelColor {
    /// Color for fatal level logs.
    fatal: Color,
    /// Color for error level logs.
    error: Color,
    /// Color for warning level logs.
    warn: Color,
    /// Color for info level logs.
    info: Color,
    /// Color for debug level logs.
    debug: Color,
    /// Color for trace level logs.
    trace: Color,
}

impl Default for LevelColor {
    fn default() -> Self {
        Self {
            fatal: Color::BrightRed,
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
    fn colorize_record_level(&self, no_color: bool, level: Level) -> ColoredString {
        if no_color {
            ColoredString::from(level.to_string())
        } else {
            let color = match level {
                Level::Fatal | Level::Fatal2 | Level::Fatal3 | Level::Fatal4 => self.fatal,
                Level::Error | Level::Error2 | Level::Error3 | Level::Error4 => self.error,
                Level::Warn | Level::Warn2 | Level::Warn3 | Level::Warn4 => self.warn,
                Level::Info | Level::Info2 | Level::Info3 | Level::Info4 => self.info,
                Level::Debug | Level::Debug2 | Level::Debug3 | Level::Debug4 => self.debug,
                Level::Trace | Level::Trace2 | Level::Trace3 | Level::Trace4 => self.trace,
            };
            ColoredString::from(level.to_string()).color(color)
        }
    }
}
