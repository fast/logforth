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

//! Log record and metadata.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use crate::Error;
use crate::kv;
use crate::kv::KeyValues;
use crate::str::{OwnedStr, RefStr};

/// The payload of a log message.
#[derive(Clone, Debug)]
pub struct Record<'a> {
    // the observed time
    now: SystemTime,

    // the metadata
    level: Level,
    target: RefStr<'a>,
    module_path: Option<RefStr<'a>>,
    file: Option<RefStr<'a>>,
    line: Option<u32>,

    // the payload
    payload: OwnedStr,

    // structural logging
    kvs: KeyValues<'a>,
}

impl<'a> Record<'a> {
    /// The observed time.
    pub fn time(&self) -> SystemTime {
        self.now
    }

    /// The verbosity level of the message.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The name of the target of the directive.
    pub fn target(&self) -> &'a str {
        self.target.get()
    }

    /// The name of the target of the directive, if it is a `'static` str.
    pub fn target_static(&self) -> Option<&'a str> {
        self.target.get_static()
    }

    /// The module path of the message.
    pub fn module_path(&self) -> Option<&'a str> {
        self.module_path.map(|s| s.get())
    }

    /// The module path of the message, if it is a `'static` str.
    pub fn module_path_static(&self) -> Option<&'static str> {
        self.module_path.and_then(|s| s.get_static())
    }

    /// The source file containing the message.
    pub fn file(&self) -> Option<&'a str> {
        self.file.map(|s| s.get())
    }

    /// The source file containing the message, if it is a `'static` str.
    pub fn file_static(&self) -> Option<&'static str> {
        self.file.and_then(|s| s.get_static())
    }

    /// The filename of the source file.
    // obtain filename only from record's full file path
    // reason: the module is already logged + full file path is noisy for some layouts
    pub fn filename(&self) -> Cow<'a, str> {
        self.file()
            .map(std::path::Path::new)
            .and_then(std::path::Path::file_name)
            .map(std::ffi::OsStr::to_string_lossy)
            .unwrap_or_default()
    }

    /// The line containing the message.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// The message body.
    pub fn payload(&self) -> &str {
        self.payload.get()
    }

    /// The message body, if it is a `'static` str.
    pub fn payload_static(&self) -> Option<&'static str> {
        self.payload.get_static()
    }

    /// The key-values.
    pub fn key_values(&self) -> &KeyValues<'a> {
        &self.kvs
    }

    /// Convert to an owned record.
    pub fn to_owned(&self) -> RecordOwned {
        RecordOwned {
            now: self.now,
            level: self.level,
            target: self.target.to_owned(),
            module_path: self.module_path.as_ref().map(RefStr::to_owned),
            file: self.file.as_ref().map(RefStr::to_owned),
            line: self.line,
            payload: self.payload.clone(),
            kvs: self
                .kvs
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }
    }

    /// Create a builder initialized with the current record's values.
    pub fn to_builder(&self) -> RecordBuilder<'a> {
        RecordBuilder {
            record: Record {
                now: self.now,
                level: self.level,
                target: self.target,
                module_path: self.module_path,
                file: self.file,
                line: self.line,
                payload: self.payload.clone(),
                kvs: self.kvs.clone(),
            },
        }
    }

    /// Returns a new builder.
    pub fn builder() -> RecordBuilder<'a> {
        RecordBuilder::default()
    }
}

/// Builder for [`Record`].
#[derive(Debug)]
pub struct RecordBuilder<'a> {
    record: Record<'a>,
}

impl Default for RecordBuilder<'_> {
    fn default() -> Self {
        RecordBuilder {
            record: Record {
                now: SystemTime::now(),
                level: Level::Info,
                target: RefStr::Static(""),
                module_path: None,
                file: None,
                line: None,
                payload: OwnedStr::Static(""),
                kvs: Default::default(),
            },
        }
    }
}

impl<'a> RecordBuilder<'a> {
    /// Set [`payload`](Record::payload).
    pub fn payload(mut self, payload: impl Into<Cow<'static, str>>) -> Self {
        self.record.payload = match payload.into() {
            Cow::Borrowed(s) => OwnedStr::Static(s),
            Cow::Owned(s) => OwnedStr::Owned(s.into_boxed_str()),
        };
        self
    }

    /// Set [`level`](Record::level).
    pub fn level(mut self, level: Level) -> Self {
        self.record.level = level;
        self
    }

    /// Set [`target`](Record::target).
    pub fn target(mut self, target: &'a str) -> Self {
        self.record.target = RefStr::Borrowed(target);
        self
    }

    /// Set [`target`](Record::target) to a `'static` string.
    pub fn target_static(mut self, target: &'static str) -> Self {
        self.record.target = RefStr::Static(target);
        self
    }

    /// Set [`module_path`](Record::module_path).
    pub fn module_path(mut self, path: Option<&'a str>) -> Self {
        self.record.module_path = path.map(RefStr::Borrowed);
        self
    }

    /// Set [`module_path`](Record::module_path) to a `'static` string.
    pub fn module_path_static(mut self, path: &'static str) -> Self {
        self.record.module_path = Some(RefStr::Static(path));
        self
    }

    /// Set [`file`](Record::file).
    pub fn file(mut self, file: Option<&'a str>) -> Self {
        self.record.file = file.map(RefStr::Borrowed);
        self
    }

    /// Set [`file`](Record::file) to a `'static` string.
    pub fn file_static(mut self, file: &'static str) -> Self {
        self.record.file = Some(RefStr::Static(file));
        self
    }

    /// Set [`line`](Record::line).
    pub fn line(mut self, line: Option<u32>) -> Self {
        self.record.line = line;
        self
    }

    /// Set [`key_values`](struct.Record.html#method.key_values)
    pub fn key_values(mut self, kvs: impl Into<KeyValues<'a>>) -> Self {
        self.record.kvs = kvs.into();
        self
    }

    /// Invoke the builder and return a `Record`
    pub fn build(self) -> Record<'a> {
        self.record
    }
}

/// A minimal set of criteria for pre-filtering purposes.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FilterCriteria<'a> {
    level: Level,
    target: &'a str,
}

impl<'a> FilterCriteria<'a> {
    /// Get the [`level`](Record::level).
    pub fn level(&self) -> Level {
        self.level
    }

    /// Get the [`target`](Record::target).
    pub fn target(&self) -> &'a str {
        self.target
    }

    /// Create a builder initialized with the current criteria's values.
    pub fn to_builder(&self) -> FilterCriteriaBuilder<'a> {
        FilterCriteriaBuilder {
            metadata: FilterCriteria {
                level: self.level,
                target: self.target,
            },
        }
    }

    /// Return a brand-new builder.
    pub fn builder() -> FilterCriteriaBuilder<'a> {
        FilterCriteriaBuilder::default()
    }
}

/// Builder for [`FilterCriteria`].
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FilterCriteriaBuilder<'a> {
    metadata: FilterCriteria<'a>,
}

impl Default for FilterCriteriaBuilder<'_> {
    fn default() -> Self {
        FilterCriteriaBuilder {
            metadata: FilterCriteria {
                level: Level::Info,
                target: "",
            },
        }
    }
}

impl<'a> FilterCriteriaBuilder<'a> {
    /// Setter for [`level`](FilterCriteria::level).
    pub fn level(mut self, arg: Level) -> Self {
        self.metadata.level = arg;
        self
    }

    /// Setter for [`target`](FilterCriteria::target).
    pub fn target(mut self, target: &'a str) -> Self {
        self.metadata.target = target;
        self
    }

    /// Invoke the builder and return a `Metadata`
    pub fn build(self) -> FilterCriteria<'a> {
        self.metadata
    }
}

/// Owned version of a log record.
#[derive(Clone, Debug)]
pub struct RecordOwned {
    // the observed time
    now: SystemTime,

    // the metadata
    level: Level,
    target: OwnedStr,
    module_path: Option<OwnedStr>,
    file: Option<OwnedStr>,
    line: Option<u32>,

    // the payload
    payload: OwnedStr,

    // structural logging
    kvs: Vec<(kv::KeyOwned, kv::ValueOwned)>,
}

impl RecordOwned {
    /// Create a `Record` referencing the data in this `RecordOwned`.
    pub fn as_record(&self) -> Record<'_> {
        Record {
            now: self.now,
            level: self.level,
            target: self.target.by_ref(),
            module_path: self.module_path.as_ref().map(OwnedStr::by_ref),
            file: self.file.as_ref().map(OwnedStr::by_ref),
            line: self.line,
            payload: self.payload.clone(),
            kvs: KeyValues::from(self.kvs.as_slice()),
        }
    }
}

/// A Level is the importance or severity of a log event.
///
/// The higher the level, the more important or severe the event.
///
/// The level design follows the [OpenTelemetry severity number specification][severity-number]
/// and [mapping guideline][mapping-guideline].
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
    /// DEBUG2. A debugging event.
    Debug3 = 7,
    /// DEBUG3. A debugging event.
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
    /// Return the string representation the short name for the `Level`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
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

/// An enum representing the available verbosity level filters of the logger.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LevelFilter {
    /// Disables all levels.
    Off,
    /// Enables if the target level is equal to the filter level.
    Equal(Level),
    /// Enables if the target level is not equal to the filter level.
    NotEqual(Level),
    /// Enables if the target level is more severe than the filter level.
    MoreSevere(Level),
    /// Enables if the target level is more severe than or equal to the filter
    /// level.
    MoreSevereEqual(Level),
    /// Enables if the target level is more verbose than the filter level.
    MoreVerbose(Level),
    /// Enables if the target level is more verbose than or equal to the filter
    /// level.
    MoreVerboseEqual(Level),
    /// Enables all levels.
    All,
}

impl LevelFilter {
    /// Checks the given level if satisfies the filter condition.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    ///
    /// let level_filter = LevelFilter::MoreSevere(Level::Info);
    ///
    /// assert_eq!(level_filter.test(Level::Trace), false);
    /// assert_eq!(level_filter.test(Level::Info), false);
    /// assert_eq!(level_filter.test(Level::Warn), true);
    /// assert_eq!(level_filter.test(Level::Error), true);
    /// ```
    pub fn test(&self, level: Level) -> bool {
        match self {
            LevelFilter::Off => false,
            LevelFilter::Equal(l) => level == *l,
            LevelFilter::NotEqual(l) => level != *l,
            LevelFilter::MoreSevere(l) => level > *l,
            LevelFilter::MoreSevereEqual(l) => level >= *l,
            LevelFilter::MoreVerbose(l) => level < *l,
            LevelFilter::MoreVerboseEqual(l) => level <= *l,
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

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip_level() {
        let levels = [
            super::Level::Trace,
            super::Level::Trace2,
            super::Level::Trace3,
            super::Level::Trace4,
            super::Level::Debug,
            super::Level::Debug2,
            super::Level::Debug3,
            super::Level::Debug4,
            super::Level::Info,
            super::Level::Info2,
            super::Level::Info3,
            super::Level::Info4,
            super::Level::Warn,
            super::Level::Warn2,
            super::Level::Warn3,
            super::Level::Warn4,
            super::Level::Error,
            super::Level::Error2,
            super::Level::Error3,
            super::Level::Error4,
            super::Level::Fatal,
            super::Level::Fatal2,
            super::Level::Fatal3,
            super::Level::Fatal4,
        ];

        for &level in &levels {
            let s = level.name();
            let parsed = s.parse::<super::Level>().unwrap();
            assert_eq!(level, parsed);
        }
    }
}
