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

use std::cmp;
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use crate::Error;
use crate::kv;
use crate::kv::KeyValues;
use crate::str::Str;

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

    fn into_str(self) -> Str<'static> {
        match self {
            MaybeStaticStr::Str(s) => Str::new_owned(s.to_owned()),
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
    args: fmt::Arguments<'a>,

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

    /// The source file containing the message.
    pub fn file(&self) -> Option<&'a str> {
        self.file.map(|s| s.get())
    }

    /// The line containing the message.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// The message body.
    pub fn args(&self) -> &fmt::Arguments<'a> {
        &self.args
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
                target: Str::new_owned(self.metadata.target),
            },
            module_path: self.module_path.map(MaybeStaticStr::into_str),
            file: self.file.map(MaybeStaticStr::into_str),
            line: self.line,
            args: match self.args.as_str() {
                Some(s) => Str::new(s),
                None => Str::new_owned(self.args.to_string()),
            },
            kvs: self
                .kvs
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }
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
                args: format_args!(""),
                kvs: Default::default(),
            },
        }
    }
}

impl<'a> RecordBuilder<'a> {
    /// Set [`args`](Record::args).
    pub fn args(mut self, args: fmt::Arguments<'a>) -> Self {
        self.record.args = args;
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
    args: Str<'static>,

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
    /// Process a `Record`.
    ///
    /// This is a workaround before `format_args` can return a value outlives the function call.
    pub fn execute(&self, f: impl FnOnce(&Record) -> Result<(), Error>) -> Result<(), Error> {
        f(&Record {
            now: self.now,
            metadata: Metadata {
                level: self.metadata.level,
                target: &self.metadata.target,
            },
            module_path: self.module_path.as_deref().map(MaybeStaticStr::Str),
            file: self.file.as_deref().map(MaybeStaticStr::Str),
            line: self.line,
            args: format_args!("{}", self.args),
            kvs: KeyValues::from(self.kvs.as_slice()),
        })
    }
}

/// An enum representing the available verbosity levels of the logger.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 100,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn = 200,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info = 300,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug = 400,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace = 500,
}

impl Level {
    /// Return the string representation of the `Level`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

impl From<log::Level> for Level {
    fn from(level: log::Level) -> Self {
        match level {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug => Self::Debug,
            log::Level::Trace => Self::Trace,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

/// An enum representing the available verbosity level filters of the logger.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum LevelFilter {
    /// A level lower than all log levels.
    Off = 0,
    /// Corresponds to the `Error` log level.
    Error = 100,
    /// Corresponds to the `Warn` log level.
    Warn = 200,
    /// Corresponds to the `Info` log level.
    Info = 300,
    /// Corresponds to the `Debug` log level.
    Debug = 400,
    /// Corresponds to the `Trace` log level.
    Trace = 500,
}

impl LevelFilter {
    /// Return the string representation of the `LevelFilter`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
    pub fn as_str(&self) -> &'static str {
        match self {
            LevelFilter::Off => "OFF",
            LevelFilter::Error => "ERROR",
            LevelFilter::Warn => "WARN",
            LevelFilter::Info => "INFO",
            LevelFilter::Debug => "DEBUG",
            LevelFilter::Trace => "TRACE",
        }
    }
}

impl From<log::LevelFilter> for LevelFilter {
    fn from(level: log::LevelFilter) -> Self {
        match level {
            log::LevelFilter::Off => Self::Off,
            log::LevelFilter::Error => Self::Error,
            log::LevelFilter::Warn => Self::Warn,
            log::LevelFilter::Info => Self::Info,
            log::LevelFilter::Debug => Self::Debug,
            log::LevelFilter::Trace => Self::Trace,
        }
    }
}

impl fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

impl PartialEq<LevelFilter> for Level {
    fn eq(&self, other: &LevelFilter) -> bool {
        PartialEq::eq(&(*self as usize), &(*other as usize))
    }
}

impl PartialOrd<LevelFilter> for Level {
    fn partial_cmp(&self, other: &LevelFilter) -> Option<cmp::Ordering> {
        Some(Ord::cmp(&(*self as usize), &(*other as usize)))
    }
}

impl PartialEq<Level> for LevelFilter {
    fn eq(&self, other: &Level) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<Level> for LevelFilter {
    fn partial_cmp(&self, other: &Level) -> Option<cmp::Ordering> {
        Some(Ord::cmp(&(*self as usize), &(*other as usize)))
    }
}

/// The type returned by `from_str` when the string doesn't match any of the log levels.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseLevelError {}

impl fmt::Display for ParseLevelError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("malformed log level")
    }
}

impl std::error::Error for ParseLevelError {}

impl FromStr for Level {
    type Err = ParseLevelError;
    fn from_str(s: &str) -> Result<Level, Self::Err> {
        for (name, level) in [
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

        Err(ParseLevelError {})
    }
}

impl FromStr for LevelFilter {
    type Err = ParseLevelError;
    fn from_str(s: &str) -> Result<LevelFilter, Self::Err> {
        for (name, level) in [
            ("off", LevelFilter::Off),
            ("error", LevelFilter::Error),
            ("warn", LevelFilter::Warn),
            ("info", LevelFilter::Info),
            ("debug", LevelFilter::Debug),
            ("trace", LevelFilter::Trace),
        ] {
            if s.eq_ignore_ascii_case(name) {
                return Ok(level);
            }
        }

        Err(ParseLevelError {})
    }
}
