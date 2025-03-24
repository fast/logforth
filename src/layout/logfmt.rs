use jiff::tz::TimeZone;
use jiff::Timestamp;
use jiff::Zoned;

use crate::diagnostic::Visitor;
use crate::layout::text::filename;
use crate::layout::text::KvWriter;
use crate::layout::Layout;
use crate::Diagnostic;

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
    /// timestamp=2025-03-24T23:38:29.117934+08:00 level=TRACE module=rs_log position=main.rs:13 msg="Hello trace!"
    /// timestamp=2025-03-24T23:38:29.127089+08:00 level=DEBUG module=rs_log position=main.rs:14 msg="Hello debug!"
    /// timestamp=2025-03-24T23:38:29.127094+08:00 level=INFO module=rs_log position=main.rs:15 msg="Hello info!"
    /// timestamp=2025-03-24T23:38:29.127094+08:00 level=INFO module=rs_log position=main.rs:15 msg="Hello info!"
    /// timestamp=2025-03-24T23:38:29.127094+08:00 level=INFO module=rs_log position=main.rs:15 msg="Hello info!"
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

impl Layout for LogfmtLayout {
    fn format(&self, record: &log::Record, diagnostics: &[Diagnostic]) -> anyhow::Result<Vec<u8>> {
        let time = match self.tz.clone() {
            Some(tz) => Timestamp::now().to_zoned(tz),
            None => Zoned::now(),
        };
        let level = record.level().to_string();
        let target = record.target();
        let file = filename(record);
        let line = record.line().unwrap_or_default();
        let message = record.args();

        let mut visitor = KvWriter {
            text: format!("timestamp={time:.6}"),
        };

        visitor.visit("level", level);
        visitor.visit("module", target);
        visitor.visit("position", format!("{}:{}", file, line));
        // quote the message
        visitor.visit("msg", format!(r#""{message}""#));

        record.key_values().visit(&mut visitor)?;
        for d in diagnostics {
            d.visit(&mut visitor);
        }

        Ok(visitor.text.into_bytes())
    }
}
