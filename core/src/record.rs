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
use crate::str::Str;

// This struct is preferred over `Str` because we need to return a &'a str
// when holding only a reference to the str ref. But `Str::get` return a &str
// that lives as long as the `Str` itself, which is not necessarily 'a.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum MaybeStaticStr<'a> {
    Str(&'a str),
    Static(&'static str),
}

impl<'a> MaybeStaticStr<'a> {
    fn get(&self) -> &'a str {
        match *self {
            MaybeStaticStr::Str(s) => s,
            MaybeStaticStr::Static(s) => s,
        }
    }

    fn get_static(&self) -> Option<&'static str> {
        match *self {
            MaybeStaticStr::Str(_) => None,
            MaybeStaticStr::Static(s) => Some(s),
        }
    }

    fn into_str(self) -> Str<'static> {
        match self {
            MaybeStaticStr::Str(s) => Str::new_shared(s),
            MaybeStaticStr::Static(s) => Str::new(s),
        }
    }
}

/// The payload of a log message.
#[derive(Clone, Debug)]
pub struct Record<'a> {
    // the observed time
    now: SystemTime,

    // the metadata
    metadata: Metadata<'a>,
    module_path: Option<MaybeStaticStr<'a>>,
    file: Option<MaybeStaticStr<'a>>,
    line: Option<u32>,

    // the payload
    payload: Str<'static>,

    // structural logging
    kvs: KeyValues<'a>,
}

impl<'a> Record<'a> {
    /// The observed time.
    pub fn time(&self) -> SystemTime {
        self.now
    }

    /// Metadata about the log directive.
    pub fn metadata(&self) -> &Metadata<'a> {
        &self.metadata
    }

    /// The verbosity level of the message.
    pub fn level(&self) -> Level {
        self.metadata.level()
    }

    /// The name of the target of the directive.
    pub fn target(&self) -> &'a str {
        self.metadata.target()
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
            metadata: MetadataOwned {
                level: self.metadata.level,
                target: Str::new_shared(self.metadata.target),
            },
            module_path: self.module_path.map(MaybeStaticStr::into_str),
            file: self.file.map(MaybeStaticStr::into_str),
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
                metadata: Metadata {
                    level: self.metadata.level,
                    target: self.metadata.target,
                },
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
                metadata: MetadataBuilder::default().build(),
                module_path: None,
                file: None,
                line: None,
                payload: Default::default(),
                kvs: Default::default(),
            },
        }
    }
}

impl<'a> RecordBuilder<'a> {
    /// Set [`payload`](Record::payload).
    pub fn payload(mut self, payload: impl Into<Cow<'static, str>>) -> Self {
        self.record.payload = match payload.into() {
            Cow::Borrowed(s) => Str::new(s),
            Cow::Owned(s) => Str::new_shared(s),
        };
        self
    }

    /// Set [`metadata`](Record::metadata).
    ///
    /// Construct a `Metadata` object with [`MetadataBuilder`].
    pub fn metadata(mut self, metadata: Metadata<'a>) -> Self {
        self.record.metadata = metadata;
        self
    }

    /// Set [`Metadata::level`].
    pub fn level(mut self, level: Level) -> Self {
        self.record.metadata.level = level;
        self
    }

    /// Set [`Metadata::target`].
    pub fn target(mut self, target: &'a str) -> Self {
        self.record.metadata.target = target;
        self
    }

    /// Set [`module_path`](Record::module_path).
    pub fn module_path(mut self, path: Option<&'a str>) -> Self {
        self.record.module_path = path.map(MaybeStaticStr::Str);
        self
    }

    /// Set [`module_path`](Record::module_path) to a `'static` string.
    pub fn module_path_static(mut self, path: &'static str) -> Self {
        self.record.module_path = Some(MaybeStaticStr::Static(path));
        self
    }

    /// Set [`file`](Record::file).
    pub fn file(mut self, file: Option<&'a str>) -> Self {
        self.record.file = file.map(MaybeStaticStr::Str);
        self
    }

    /// Set [`file`](Record::file) to a `'static` string.
    pub fn file_static(mut self, file: &'static str) -> Self {
        self.record.file = Some(MaybeStaticStr::Static(file));
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

/// Metadata about a log message.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata<'a> {
    level: Level,
    target: &'a str,
}

impl<'a> Metadata<'a> {
    /// Get the level.
    pub fn level(&self) -> Level {
        self.level
    }

    /// Get the target.
    pub fn target(&self) -> &'a str {
        self.target
    }

    /// Create a builder initialized with the current metadata's values.
    pub fn to_builder(&self) -> MetadataBuilder<'a> {
        MetadataBuilder {
            metadata: Metadata {
                level: self.level,
                target: self.target,
            },
        }
    }

    /// Returns a new builder.
    pub fn builder() -> MetadataBuilder<'a> {
        MetadataBuilder::default()
    }
}

/// Builder for [`Metadata`].
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MetadataBuilder<'a> {
    metadata: Metadata<'a>,
}

impl Default for MetadataBuilder<'_> {
    fn default() -> Self {
        MetadataBuilder {
            metadata: Metadata {
                level: Level::Info,
                target: Default::default(),
            },
        }
    }
}

impl<'a> MetadataBuilder<'a> {
    /// Setter for [`level`](Metadata::level).
    pub fn level(mut self, arg: Level) -> Self {
        self.metadata.level = arg;
        self
    }

    /// Setter for [`target`](Metadata::target).
    pub fn target(mut self, target: &'a str) -> Self {
        self.metadata.target = target;
        self
    }

    /// Invoke the builder and return a `Metadata`
    pub fn build(self) -> Metadata<'a> {
        self.metadata
    }
}

/// Owned version of a log record.
#[derive(Clone, Debug)]
pub struct RecordOwned {
    // the observed time
    now: SystemTime,

    // the metadata
    metadata: MetadataOwned,
    module_path: Option<Str<'static>>,
    file: Option<Str<'static>>,
    line: Option<u32>,

    // the payload
    payload: Str<'static>,

    // structural logging
    kvs: Vec<(kv::KeyOwned, kv::ValueOwned)>,
}

/// Owned version of metadata about a log message.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct MetadataOwned {
    level: Level,
    target: Str<'static>,
}

impl RecordOwned {
    /// Create a `Record` referencing the data in this `RecordOwned`.
    pub fn as_record(&self) -> Record<'_> {
        Record {
            now: self.now,
            metadata: Metadata {
                level: self.metadata.level,
                target: &self.metadata.target,
            },
            module_path: self.module_path.as_deref().map(MaybeStaticStr::Str),
            file: self.file.as_deref().map(MaybeStaticStr::Str),
            line: self.line,
            payload: self.payload.clone(),
            kvs: KeyValues::from(self.kvs.as_slice()),
        }
    }
}

/// An enum representing the available verbosity levels of the logger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Designates critical errors.
    Crit,
    /// Designates very serious errors.
    Error,
    /// Designates hazardous situations.
    Warn,
    /// Designates useful information.
    Info,
    /// Designates lower priority information.
    Debug,
    /// Designates very low priority, often extremely verbose, information.
    Trace,
}

impl Level {
    /// Return the string representation of the `Level`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Crit => "CRIT",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
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
            LevelFilter::MoreSevere(l) => level < *l,
            LevelFilter::MoreSevereEqual(l) => level <= *l,
            LevelFilter::MoreVerbose(l) => level > *l,
            LevelFilter::MoreVerboseEqual(l) => level >= *l,
            LevelFilter::All => true,
        }
    }
}

impl FromStr for Level {
    type Err = Error;
    fn from_str(s: &str) -> Result<Level, Self::Err> {
        for (name, level) in [
            ("crit", Level::Crit),
            ("error", Level::Error),
            ("warn", Level::Warn),
            ("info", Level::Info),
            ("debug", Level::Debug),
            ("trace", Level::Trace),
        ] {
            if s.eq_ignore_ascii_case(name) {
                return Ok(level);
            }
        }

        Err(Error::new(format!("malformed level: {s:?}")))
    }
}
