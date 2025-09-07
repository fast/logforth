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
use log::Record;

use crate::Diagnostic;
use crate::Error;
use crate::ErrorKind;
use crate::diagnostic::Visitor;
use crate::layout::Layout;
use crate::layout::filename;

/// A logfmt layout for formatting log records.
///
/// Output format:
///
/// ```text
/// ```
///
/// # Examples
///
/// ```
/// use logforth::layout::LogfmtLayout;
///
/// let logfmt_layout = LogfmtLayout::default();
/// ```
#[derive(Default, Debug, Clone)]
pub struct LogfmtLayout {
    tz: Option<TimeZone>,
}

impl LogfmtLayout {
    /// Sets the timezone for timestamps.
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
    /// use jiff::tz::TimeZone;
    /// use logforth::layout::LogfmtLayout;
    ///
    /// let logfmt_layout = LogfmtLayout::default().timezone(TimeZone::UTC);
    /// ```
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.tz = Some(tz);
        self
    }
}

// The encode logic is copied from https://github.com/go-logfmt/logfmt/blob/76262ea7/encode.go.
fn encode_key_value(result: &mut String, key: &str, value: &str) -> Result<(), Error> {
    use std::fmt::Write;

    if key.contains([' ', '=', '"']) {
        // omit keys contain special chars
        return Err(Error::new(
            ErrorKind::Unexpected,
            format!("key contains special chars: {key}"),
        ));
    }

    // SAFETY: write to a string always succeeds
    if value.contains([' ', '=', '"']) {
        write!(result, " {key}=\"{}\"", value.escape_debug()).unwrap();
    } else {
        write!(result, " {key}={value}").unwrap();
    }

    Ok(())
}

struct KvFormatter {
    text: String,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvFormatter {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        match encode_key_value(&mut self.text, key.as_str(), value.to_string().as_str()) {
            Ok(()) => Ok(()),
            Err(err) => Err(log::kv::Error::boxed(err)),
        }
    }
}

impl Visitor for KvFormatter {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) -> Result<(), Error> {
        encode_key_value(&mut self.text, key.as_ref(), value.as_ref())?;
        Ok(())
    }
}

impl Layout for LogfmtLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        let time = match self.tz.clone() {
            Some(tz) => Timestamp::now().to_zoned(tz),
            None => Zoned::now(),
        };
        let level = record.level().to_string();
        let target = record.target();
        let file = filename(record);
        let line = record.line().unwrap_or_default();
        let message = record.args();

        let mut visitor = KvFormatter {
            text: format!("timestamp={time:.6}"),
        };

        visitor.visit(Cow::Borrowed("level"), level.into())?;
        visitor.visit(Cow::Borrowed("module"), target.into())?;
        visitor.visit(Cow::Borrowed("position"), format!("{file}:{line}").into())?;
        visitor.visit(Cow::Borrowed("message"), message.to_string().into())?;

        record
            .key_values()
            .visit(&mut visitor)
            .map_err(Error::from_kv_error)?;
        for d in diags {
            d.visit(&mut visitor)?;
        }

        Ok(visitor.text.into_bytes())
    }
}
