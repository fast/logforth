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
#![deny(missing_docs)]

pub extern crate jiff;

use std::borrow::Cow;
use std::io::Write;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::KeyView;
use logforth_core::kv::Value;
use logforth_core::kv::ValueView;
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

// The encode logic is copied from https://github.com/go-logfmt/logfmt/blob/76262ea7/encode.go.
fn write_kv(buf: &mut Vec<u8>, key: &str, value: ValueView) -> Result<(), Error> {
    let is_special = |c: char| c.is_control() || matches!(c, ' ' | '=' | '"');

    if key.chars().any(is_special) {
        return Err(Error::new(format!("key contains special chars: {key}")));
    }

    let value = match value.to_str() {
        Some(s) => Cow::Borrowed(s),
        None => Cow::Owned(value.to_string()),
    };

    if value.chars().any(is_special) {
        write!(buf, " {key}=\"{}\"", value.escape_debug())
    } else {
        write!(buf, " {key}={value}")
    }
    .map_err(|err| Error::new("failed to format key-value pair").with_source(err))
}

impl Layout for LogfmtLayout {
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let ts = Timestamp::try_from(record.time())
            .map_err(|err| Error::new("invalid record timestamp").with_source(err))?;
        let tz = self.tz.clone().unwrap_or_else(TimeZone::system);
        let offset = tz.to_offset(ts);
        let time = ts.display_with_offset(offset);

        write!(buf, "timestamp={time:.6}")
            .map_err(|err| Error::new("failed to format record timestamp").with_source(err))?;

        let level = record.level();
        let module = record.module_path().unwrap_or_default();
        let file = record.filename();
        let line = record.line().unwrap_or_default();
        let payload = record.payload();
        let message = match record.payload_static() {
            Some(msg) => Value::static_str(msg),
            None => Value::display(&payload),
        };

        write_kv(buf, "level", Value::static_str(level.name()).view())?;
        write_kv(buf, "module", Value::str(module).view())?;
        write_kv(
            buf,
            "position",
            Value::display(&format_args!("{file}:{line}")).view(),
        )?;
        write_kv(buf, "message", message.view())?;

        let mut visit = |key: KeyView, value: ValueView| write_kv(buf, key.as_str(), value);
        record.key_values().visit(&mut visit)?;
        diags.visit(&mut visit)
    }
}
