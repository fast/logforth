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

use crate::Str;
use crate::kv;

///
pub struct Record<'a> {
    // the observed time
    now: SystemTime,

    // the metadata
    metadata: Metadata<'a>,
    module_path: Option<Str<'a>>,
    file: Option<Str<'a>>,
    line: Option<u32>,

    // the payload
    args: fmt::Arguments<'a>,

    // structural logging
    kvs: Vec<(kv::Key<'a>, kv::Value<'a>)>,
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
    pub fn module_path(&self) -> Option<Str<'_>> {
        self.module_path.as_ref().map(|s| s.by_ref())
    }

    ///
    pub fn file(&self) -> Option<Str<'_>> {
        self.file.as_ref().map(|s| s.by_ref())
    }

    ///
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    ///
    pub fn args(&self) -> fmt::Arguments<'a> {
        self.args
    }
}

///
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata<'a> {
    level: Level,
    target: Str<'a>,
}

impl<'a> Metadata<'a> {
    /// Gets the level.
    pub fn level(&self) -> Level {
        self.level
    }

    /// Gets the target.
    pub fn target(&self) -> Str<'_> {
        self.target.by_ref()
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

pub(crate) fn scoped_metadata<T>(m: &log::Metadata<'_>, f: impl FnOnce(&Metadata<'_>) -> T) -> T {
    f(&Metadata {
        level: Level::from(m.level()),
        target: Str::new_ref(m.target()),
    })
}

pub(crate) fn scoped_record<T>(r: &log::Record<'_>, f: impl FnOnce(&Record<'_>) -> T) -> T {
    let now = SystemTime::now();

    let metadata = Metadata {
        level: Level::from(r.level()),
        target: Str::new_ref(r.target()),
    };

    let module_path = r
        .module_path_static()
        .map(|s| Str::new(s))
        .or_else(|| r.module_path().map(Str::new_ref));
    let file = r
        .file_static()
        .map(|s| Str::new(s))
        .or_else(|| r.file().map(Str::new_ref));
    let line = r.line();

    let args = *r.args();

    let mut kvs = Vec::new();
    struct KeyValueVisitor<'a, 'b> {
        kvs: &'b mut Vec<(log::kv::Key<'a>, log::kv::Value<'a>)>,
    }

    impl<'a, 'b> log::kv::VisitSource<'a> for KeyValueVisitor<'a, 'b> {
        fn visit_pair(
            &mut self,
            key: log::kv::Key<'a>,
            value: log::kv::Value<'a>,
        ) -> Result<(), log::kv::Error> {
            self.kvs.push((key, value));
            Ok(())
        }
    }

    let mut visitor = KeyValueVisitor { kvs: &mut kvs };
    r.key_values().visit(&mut visitor).unwrap();
    let kvs = kvs
        .iter()
        .map(|(k, v)| (kv::Key::from_str(k.as_ref()), kv::Value::from_sval2(v)))
        .collect();

    f(&Record {
        now,
        metadata,
        module_path,
        file,
        line,
        args,
        kvs,
    })
}
