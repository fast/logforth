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

use std::fmt;
use std::time::SystemTime;

use crate::kv;
use crate::str::Str;

///
pub struct Record<'a> {
    // the different fields
    now: SystemTime,

    // the metadata
    metadata: Metadata<'a>,
    module_path: Option<Str<'a>>,
    file: Option<Str<'a>>,
    line: Option<u32>,

    // the payload
    args: fmt::Arguments<'a>,

    // structural logging
    kvs: Vec<(log::kv::Key<'a>, kv::ValueOwned)>,
}

impl<'a> Record<'a> {
    ///
    pub fn time(&self) -> SystemTime {
        self.now
    }

    ///
    pub fn metadata(&self) -> &Metadata<'a> {
        &self.metadata
    }

    ///
    pub fn module_path(&self) -> Option<&str> {
        self.module_path.as_ref().map(|s| s.get())
    }

    ///
    pub fn file(&self) -> Option<&str> {
        self.file.as_ref().map(|s| s.get())
    }

    ///
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    pub(crate) fn new(record: &'a log::Record<'a>, now: SystemTime) -> Self {
        Self {
            now,
            metadata: Metadata::new(record.metadata()),
            module_path: record
                .module_path_static()
                .map(Str::new)
                .or_else(|| record.module_path().map(Str::new_ref)),
            file: record
                .file_static()
                .map(Str::new)
                .or_else(|| record.file().map(Str::new_ref)),
            line: record.line(),
            args: *record.args(),
            kvs: {
                let kvs = record.key_values();
                let mut cvt = kv::LogCrateConverter::new(kvs.count());
                assert!(kvs.visit(&mut cvt).is_ok());
                cvt.finalize()
            },
        }
    }
}

///
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata<'a> {
    level: Level,
    target: &'a str,
}

impl<'a> Metadata<'a> {
    pub(crate) fn new(metadata: &log::Metadata<'a>) -> Self {
        Self {
            level: metadata.level().into(),
            target: metadata.target(),
        }
    }

    /// Gets the level.
    pub fn level(&self) -> Level {
        self.level
    }

    /// Gets the target.
    pub fn target(&self) -> &str {
        self.target
    }
}

///
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 1,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn = 2,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info = 3,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug = 4,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace = 5,
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

impl From<Level> for log::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => Self::Error,
            Level::Warn => Self::Warn,
            Level::Info => Self::Info,
            Level::Debug => Self::Debug,
            Level::Trace => Self::Trace,
        }
    }
}
