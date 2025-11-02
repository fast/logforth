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

//! A logfmt layout for formatting log records.

#![cfg_attr(docsrs, feature(doc_cfg))]

use jiff::Timestamp;
use jiff::tz::TimeZone;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::layout::Layout;
use logforth_core::record::Record;

/// A logfmt layout for formatting log records.
///
/// Output format:
///
/// ```text
/// timestamp=2025-03-31T21:04:28.986032+08:00 level=TRACE module=rs_log position=main.rs:22 message="Hello trace!"
/// timestamp=2025-03-31T21:04:28.991233+08:00 level=DEBUG module=rs_log position=main.rs:23 message="Hello debug!"
/// timestamp=2025-03-31T21:04:28.991239+08:00 level=INFO module=rs_log position=main.rs:24 message="Hello info!"
/// timestamp=2025-03-31T21:04:28.991273+08:00 level=WARN module=rs_log position=main.rs:25 message="Hello warn!"
/// timestamp=2025-03-31T21:04:28.991277+08:00 level=ERROR module=rs_log position=main.rs:26 message="Hello err!"
/// ```
///
/// # Examples
///
/// ```
/// use logforth_layout_logfmt::LogfmtLayout;
///
/// let layout = LogfmtLayout::default();
/// ```
#[derive(Default, Debug, Clone)]
pub struct LogfmtLayout {
    tz: Option<TimeZone>,
}

impl LogfmtLayout {
    /// Set the timezone for timestamps.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::tz::TimeZone;
    /// use logforth_layout_logfmt::LogfmtLayout;
    ///
    /// let layout = LogfmtLayout::default().timezone(TimeZone::UTC);
    /// ```
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.tz = Some(tz);
        self
    }
}

struct KvFormatter {
    text: String,
}

impl Visitor for KvFormatter {
    // The encode logic is copied from https://github.com/go-logfmt/logfmt/blob/76262ea7/encode.go.
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        use std::fmt::Write;

        let key = key.as_str();
        let value = value.to_string();
        let value = value.as_str();

        if key.contains([' ', '=', '"']) {
            // omit keys contain special chars
            return Err(Error::new(format!("key contains special chars: {key}")));
        }

        // SAFETY: write to a string always succeeds
        if value.contains([' ', '=', '"']) {
            write!(&mut self.text, " {key}=\"{}\"", value.escape_debug()).unwrap();
        } else {
            write!(&mut self.text, " {key}={value}").unwrap();
        }

        Ok(())
    }
}

impl Layout for LogfmtLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        // SAFETY: jiff::Timestamp::try_from only fails if the time is out of range, which is
        // very unlikely if the system clock is correct.
        let ts = Timestamp::try_from(record.time()).unwrap();
        let tz = self.tz.clone().unwrap_or_else(TimeZone::system);
        let offset = tz.to_offset(ts);
        let time = ts.display_with_offset(offset);

        let level = record.level();
        let target = record.target();
        let file = record.filename();
        let line = record.line().unwrap_or_default();
        let message = record.payload();

        let mut visitor = KvFormatter {
            text: format!("timestamp={time:.6}"),
        };

        visitor.visit(Key::new("level"), level.name().into())?;
        visitor.visit(Key::new("module"), target.into())?;
        visitor.visit(
            Key::new("position"),
            Value::from_display(&format_args!("{file}:{line}")),
        )?;
        visitor.visit(Key::new("message"), Value::from_str(message))?;

        record.key_values().visit(&mut visitor)?;
        for d in diags {
            d.visit(&mut visitor)?;
        }

        Ok(visitor.text.into_bytes())
    }
}
