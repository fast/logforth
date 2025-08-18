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

use jiff::Timestamp;
use jiff::Zoned;
use jiff::tz::TimeZone;
use log::Level;
use log::Record;

use crate::Diagnostic;
use crate::diagnostic::Visitor;
use crate::layout::Layout;
use crate::layout::filename;

#[cfg(feature = "colored")]
mod colored {
    use super::*;
    use crate::color::LevelColor;
    use crate::colored::Color;

    impl TextLayout {
        /// Customize the color of each log level.
        ///
        /// No effect if `no_color` is set to `true`.
        pub fn colors(mut self, colors: LevelColor) -> Self {
            self.colors = colors;
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
    }
}

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
/// By default, log levels are colored. You can set the `no_color` field to `true` to disable
/// coloring.
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
///
/// [`LevelColor`]: crate::color::LevelColor
#[derive(Debug, Clone, Default)]
pub struct TextLayout {
    #[cfg(feature = "colored")]
    colors: crate::color::LevelColor,
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

    #[cfg(not(feature = "colored"))]
    fn format_record_level(&self, level: Level) -> String {
        level.to_string()
    }

    #[cfg(feature = "colored")]
    fn format_record_level(&self, level: Level) -> crate::colored::ColoredString {
        self.colors.colorize_record_level(self.no_color, level)
    }
}

struct KvWriter {
    text: String,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvWriter {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        use std::fmt::Write;

        write!(&mut self.text, " {key}={value}")?;
        Ok(())
    }
}

impl Visitor for KvWriter {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) -> anyhow::Result<()> {
        use std::fmt::Write;

        write!(&mut self.text, " {key}={value}")?;
        Ok(())
    }
}

impl Layout for TextLayout {
    fn format(
        &self,
        record: &Record,
        diagnostics: &[Box<dyn Diagnostic>],
    ) -> anyhow::Result<Vec<u8>> {
        let time = match self.tz.clone() {
            Some(tz) => Timestamp::now().to_zoned(tz),
            None => Zoned::now(),
        };
        let level = self.format_record_level(record.level());
        let target = record.target();
        let file = filename(record);
        let line = record.line().unwrap_or_default();
        let message = record.args();

        let mut visitor = KvWriter {
            text: format!("{time:.6} {level:>5} {target}: {file}:{line} {message}"),
        };
        record.key_values().visit(&mut visitor)?;
        for d in diagnostics {
            d.visit(&mut visitor)?;
        }

        Ok(visitor.text.into_bytes())
    }
}
