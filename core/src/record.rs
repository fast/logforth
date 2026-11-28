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

//! Structured log records, severity levels, and level predicates.
//!
//! Synchronous logging paths construct a stack-borrowed [`Record`] via [`RecordBuilder::new`] (or
//! the native logging macros) without heap allocation. Asynchronous boundaries promote a [`Record`]
//! into a `'static + Send + Sync` [`RecordOwned`] via [`Record::to_owned`] and re-project a
//! borrowed view on the worker side through [`RecordOwned::with`].

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use crate::Error;
use crate::kv;
use crate::kv::KeyValues;
use crate::str::RefStr;

/// A borrowed log event carrying timestamp, severity, source location, payload, and structured
/// key-values.
#[derive(Clone, Debug)]
pub struct Record<'a> {
    // the observed time
    now: SystemTime,

    // the metadata
    level: Level,
    module_path: Option<RefStr<'a>>,
    file: Option<RefStr<'a>>,
    line: Option<u32>,
    column: Option<u32>,

    // the payload
    payload: fmt::Arguments<'a>,

    // structural logging
    kvs: KeyValues<'a>,
}

impl<'a> Record<'a> {
    /// Returns the timestamp observed when the record was created.
    pub const fn time(&self) -> SystemTime {
        self.now
    }

    /// Returns the severity [`Level`] of the record.
    pub const fn level(&self) -> Level {
        self.level
    }

    /// Returns the Rust module path where the record was emitted, if set.
    pub const fn module_path(&self) -> Option<&'a str> {
        match self.module_path {
            Some(s) => Some(s.get()),
            None => None,
        }
    }

    /// Returns the Rust module path if it was supplied with `'static` provenance.
    pub const fn module_path_static(&self) -> Option<&'static str> {
        match self.module_path {
            Some(s) => s.get_static(),
            None => None,
        }
    }

    /// Returns the source file path where the record was emitted, if set.
    pub const fn file(&self) -> Option<&'a str> {
        match self.file {
            Some(s) => Some(s.get()),
            None => None,
        }
    }

    /// Returns the source file path if it was supplied with `'static` provenance.
    pub const fn file_static(&self) -> Option<&'static str> {
        match self.file {
            Some(s) => s.get_static(),
            None => None,
        }
    }

    /// Returns the terminal filename component of [`Record::file`], if present.
    pub fn filename(&self) -> Option<&'a str> {
        self.file()
            .map(std::path::Path::new)
            .and_then(std::path::Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
    }

    /// Returns the 1-based source line number where the record was emitted, if set.
    pub const fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns the 1-based source column number where the record was emitted, if set.
    pub const fn column(&self) -> Option<u32> {
        self.column
    }

    /// Returns the formatted message arguments of the record.
    pub const fn payload(&self) -> fmt::Arguments<'a> {
        self.payload
    }

    /// Returns the message body as a `'static` string slice when it contains no formatting
    /// placeholders.
    pub const fn payload_static(&self) -> Option<&'static str> {
        self.payload.as_str()
    }

    /// Returns the structured key-value pairs attached to this record.
    pub const fn key_values(&self) -> KeyValues<'a> {
        self.kvs
    }

    /// Creates a [`RecordBuilder`] initialized with a copy of this record's fields.
    pub const fn to_builder(&self) -> RecordBuilder<'a> {
        RecordBuilder {
            record: Record {
                now: self.now,
                level: self.level,
                module_path: self.module_path,
                file: self.file,
                line: self.line,
                column: self.column,
                payload: self.payload,
                kvs: self.kvs,
            },
        }
    }

    /// Converts this borrowed record into a `'static` [`RecordOwned`] snapshot.
    ///
    /// Fields backed by `'static` strings preserve their static borrows without heap allocation,
    /// while formatted payloads and borrowed values are materialized.
    pub fn to_owned(&self) -> RecordOwned {
        RecordOwned {
            now: self.now,
            level: self.level,
            module_path: self.module_path.map(|m| m.into_cow_static()),
            file: self.file.map(|f| f.into_cow_static()),
            line: self.line,
            column: self.column,
            payload: if let Some(s) = self.payload.as_str() {
                Cow::Borrowed(s)
            } else {
                Cow::Owned(self.payload.to_string())
            },
            kvs: self
                .kvs
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }
    }
}

/// A builder for constructing a [`Record`].
#[derive(Debug)]
pub struct RecordBuilder<'a> {
    record: Record<'a>,
}

impl Default for RecordBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> RecordBuilder<'a> {
    /// Creates a new [`RecordBuilder`] with default metadata (`SystemTime::now()`, [`Level::Info`],
    /// empty payload, and empty key-values).
    pub fn new() -> Self {
        RecordBuilder {
            record: Record {
                now: SystemTime::now(),
                level: Level::Info,
                module_path: None,
                file: None,
                line: None,
                column: None,
                payload: format_args!(""),
                kvs: KeyValues::empty(),
            },
        }
    }
    /// Sets [`Record::time`].
    pub fn time(mut self, time: SystemTime) -> Self {
        self.record.now = time;
        self
    }

    /// Sets [`Record::payload`].
    pub fn payload(mut self, payload: fmt::Arguments<'a>) -> Self {
        self.record.payload = payload;
        self
    }

    /// Sets [`Record::level`].
    pub fn level(mut self, level: Level) -> Self {
        self.record.level = level;
        self
    }

    /// Sets [`Record::module_path`] from an optional borrowed string.
    pub fn module_path(mut self, path: Option<&'a str>) -> Self {
        self.record.module_path = path.map(RefStr::Borrowed);
        self
    }

    /// Sets [`Record::module_path`] from a `'static` string literal, preserving static provenance.
    pub fn module_path_static(mut self, path: &'static str) -> Self {
        self.record.module_path = Some(RefStr::Static(path));
        self
    }

    /// Sets [`Record::file`] from an optional borrowed string.
    pub fn file(mut self, file: Option<&'a str>) -> Self {
        self.record.file = file.map(RefStr::Borrowed);
        self
    }

    /// Sets [`Record::file`] from a `'static` string literal, preserving static provenance.
    pub fn file_static(mut self, file: &'static str) -> Self {
        self.record.file = Some(RefStr::Static(file));
        self
    }

    /// Sets [`Record::line`].
    pub fn line(mut self, line: Option<u32>) -> Self {
        self.record.line = line;
        self
    }

    /// Sets [`Record::column`].
    pub fn column(mut self, column: Option<u32>) -> Self {
        self.record.column = column;
        self
    }

    /// Sets [`Record::key_values`].
    pub fn key_values(mut self, kvs: impl Into<KeyValues<'a>>) -> Self {
        self.record.kvs = kvs.into();
        self
    }

    /// Consumes the builder and returns the constructed [`Record`].
    pub fn build(self) -> Record<'a> {
        self.record
    }
}

/// An owned `'static` snapshot of a [`Record`] suitable for sending across threads or channels.
#[derive(Clone, Debug)]
pub struct RecordOwned {
    // the observed time
    now: SystemTime,

    // the metadata
    level: Level,
    module_path: Option<Cow<'static, str>>,
    file: Option<Cow<'static, str>>,
    line: Option<u32>,
    column: Option<u32>,

    // the payload
    payload: Cow<'static, str>,

    // structural logging
    kvs: Vec<(kv::KeyOwned, kv::ValueOwned)>,
}

impl RecordOwned {
    /// Invokes `f` with a borrowed [`Record`] view over this owned snapshot and returns its
    /// result.
    pub fn with<R>(&self, f: impl FnOnce(Record<'_>) -> R) -> R {
        f(Record {
            now: self.now,
            level: self.level,
            module_path: match &self.module_path {
                Some(Cow::Borrowed(s)) => Some(RefStr::Static(s)),
                Some(Cow::Owned(s)) => Some(RefStr::Borrowed(s)),
                None => None,
            },
            file: match &self.file {
                Some(Cow::Borrowed(s)) => Some(RefStr::Static(s)),
                Some(Cow::Owned(s)) => Some(RefStr::Borrowed(s)),
                None => None,
            },
            line: self.line,
            column: self.column,
            payload: format_args!("{}", self.payload),
            kvs: KeyValues::from(self.kvs.as_slice()),
        })
    }
}

/// The importance or severity of a log event.
///
/// Higher discriminants represent more severe events (`Trace = 1` through `Fatal4 = 24`).
/// [`FromStr`] parses ASCII case-insensitive level names (`"trace"` through `"fatal4"`).
///
/// The 24 variants follow the [OpenTelemetry severity number specification][severity-number] and
/// [mapping guideline][mapping-guideline].
///
/// [severity-number]: https://opentelemetry.io/docs/specs/otel/logs/data-model/#field-severitynumber
/// [mapping-guideline]: https://opentelemetry.io/docs/specs/otel/logs/data-model-appendix/#appendix-b-severitynumber-example-mappings
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Level {
    /// TRACE. A fine-grained debugging event.
    ///
    /// Typically disabled in default configurations.
    Trace = 1,
    /// TRACE2. A fine-grained debugging event.
    ///
    /// Typically disabled in default configurations.
    Trace2 = 2,
    /// TRACE3. A fine-grained debugging event.
    ///
    /// Typically disabled in default configurations.
    Trace3 = 3,
    /// TRACE4. A fine-grained debugging event.
    ///
    /// Typically disabled in default configurations.
    Trace4 = 4,
    /// DEBUG. A debugging event.
    Debug = 5,
    /// DEBUG2. A debugging event.
    Debug2 = 6,
    /// DEBUG3. A debugging event.
    Debug3 = 7,
    /// DEBUG4. A debugging event.
    Debug4 = 8,
    /// INFO. An informational event.
    ///
    /// Indicates that an event happened.
    Info = 9,
    /// INFO2. An informational event.
    ///
    /// Indicates that an event happened.
    Info2 = 10,
    /// INFO3. An informational event.
    ///
    /// Indicates that an event happened.
    Info3 = 11,
    /// INFO4. An informational event.
    ///
    /// Indicates that an event happened.
    Info4 = 12,
    /// WARN. A warning event.
    ///
    /// Not an error but is likely more important than an informational event.
    Warn = 13,
    /// WARN2. A warning event.
    ///
    /// Not an error but is likely more important than an informational event.
    Warn2 = 14,
    /// WARN3. A warning event.
    ///
    /// Not an error but is likely more important than an informational event.
    Warn3 = 15,
    /// WARN4. A warning event.
    ///
    /// Not an error but is likely more important than an informational event.
    Warn4 = 16,
    /// ERROR. An error event.
    ///
    /// Something went wrong.
    Error = 17,
    /// ERROR2. An error event.
    ///
    /// Something went wrong.
    Error2 = 18,
    /// ERROR3. An error event.
    ///
    /// Something went wrong.
    Error3 = 19,
    /// ERROR4. An error event.
    ///
    /// Something went wrong.
    Error4 = 20,
    /// FATAL. A fatal error such as application or system crash.
    Fatal = 21,
    /// FATAL2. A fatal error such as application or system crash.
    Fatal2 = 22,
    /// FATAL3. A fatal error such as application or system crash.
    Fatal3 = 23,
    /// FATAL4. A fatal error such as application or system crash.
    Fatal4 = 24,
}

impl Level {
    /// All 24 severity levels in ascending severity order from [`Level::Trace`] to
    /// [`Level::Fatal4`].
    pub const ALL: [Level; 24] = [
        Level::Trace,
        Level::Trace2,
        Level::Trace3,
        Level::Trace4,
        Level::Debug,
        Level::Debug2,
        Level::Debug3,
        Level::Debug4,
        Level::Info,
        Level::Info2,
        Level::Info3,
        Level::Info4,
        Level::Warn,
        Level::Warn2,
        Level::Warn3,
        Level::Warn4,
        Level::Error,
        Level::Error2,
        Level::Error3,
        Level::Error4,
        Level::Fatal,
        Level::Fatal2,
        Level::Fatal3,
        Level::Fatal4,
    ];

    /// Returns the uppercase canonical name of this `Level` (matching its [`fmt::Display`] output).
    pub const fn name(&self) -> &'static str {
        const LEVEL_NAMES: [&str; 24] = [
            "TRACE", "TRACE2", "TRACE3", "TRACE4", "DEBUG", "DEBUG2", "DEBUG3", "DEBUG4", "INFO",
            "INFO2", "INFO3", "INFO4", "WARN", "WARN2", "WARN3", "WARN4", "ERROR", "ERROR2",
            "ERROR3", "ERROR4", "FATAL", "FATAL2", "FATAL3", "FATAL4",
        ];
        LEVEL_NAMES[*self as usize - 1]
    }
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.name())
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.name())
    }
}

/// A severity predicate over [`Level`] values.
///
/// Implements [`Filter`](crate::Filter) by returning [`FilterResult::Neutral`](crate::filter::FilterResult::Neutral)
/// when [`LevelFilter::test`] succeeds and [`FilterResult::Reject`](crate::filter::FilterResult::Reject)
/// otherwise.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LevelFilter {
    /// Disables all levels.
    Off,
    /// Enables only the specified level.
    Equal(Level),
    /// Enables every level except the specified level.
    NotEqual(Level),
    /// Enables levels strictly more severe than the specified level.
    MoreSevere(Level),
    /// Enables levels more severe than or equal to the specified level.
    MoreSevereEqual(Level),
    /// Enables levels strictly more verbose than the specified level.
    MoreVerbose(Level),
    /// Enables levels more verbose than or equal to the specified level.
    MoreVerboseEqual(Level),
    /// Enables all levels.
    All,
}

impl LevelFilter {
    /// Returns `true` if `level` satisfies this filter condition.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    ///
    /// let level_filter = LevelFilter::MoreSevere(Level::Info);
    ///
    /// assert!(!level_filter.test(Level::Trace));
    /// assert!(!level_filter.test(Level::Info));
    /// assert!(level_filter.test(Level::Warn));
    /// assert!(level_filter.test(Level::Error));
    /// ```
    pub const fn test(&self, level: Level) -> bool {
        let level = level as u8;
        match self {
            LevelFilter::Off => false,
            LevelFilter::Equal(l) => level == (*l as u8),
            LevelFilter::NotEqual(l) => level != (*l as u8),
            LevelFilter::MoreSevere(l) => level > (*l as u8),
            LevelFilter::MoreSevereEqual(l) => level >= (*l as u8),
            LevelFilter::MoreVerbose(l) => level < (*l as u8),
            LevelFilter::MoreVerboseEqual(l) => level <= (*l as u8),
            LevelFilter::All => true,
        }
    }
}

impl FromStr for Level {
    type Err = Error;
    fn from_str(s: &str) -> Result<Level, Self::Err> {
        for (repr, level) in [
            // common cases
            ("fatal", Level::Fatal),
            ("error", Level::Error),
            ("warn", Level::Warn),
            ("info", Level::Info),
            ("debug", Level::Debug),
            ("trace", Level::Trace),
            // other offset levels
            ("fatal2", Level::Fatal2),
            ("fatal3", Level::Fatal3),
            ("fatal4", Level::Fatal4),
            ("error2", Level::Error2),
            ("error3", Level::Error3),
            ("error4", Level::Error4),
            ("warn2", Level::Warn2),
            ("warn3", Level::Warn3),
            ("warn4", Level::Warn4),
            ("info2", Level::Info2),
            ("info3", Level::Info3),
            ("info4", Level::Info4),
            ("debug2", Level::Debug2),
            ("debug3", Level::Debug3),
            ("debug4", Level::Debug4),
            ("trace2", Level::Trace2),
            ("trace3", Level::Trace3),
            ("trace4", Level::Trace4),
        ] {
            if s.eq_ignore_ascii_case(repr) {
                return Ok(level);
            }
        }

        Err(Error::new(format!("malformed level: {s:?}")))
    }
}
