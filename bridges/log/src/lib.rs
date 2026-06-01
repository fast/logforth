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

//! A bridge to forward logs from the `log` crate to `logforth`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::ops::Deref;
use std::sync::Arc;

use log::Metadata;
use log::Record;
use logforth_core::Logger;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::record::FilterCriteria;

/// Adapter to use a `logforth` logger instance as a `log` crate logger.
#[derive(Debug)]
pub struct LogAdapter {
    logger: Arc<Logger>,
}

impl LogAdapter {
    /// Create a new `LogAdapter` instance.
    pub fn new(logger: impl Into<Arc<Logger>>) -> Self {
        Self {
            logger: logger.into(),
        }
    }
}

impl Deref for LogAdapter {
    type Target = Logger;

    fn deref(&self) -> &Self::Target {
        &self.logger
    }
}

impl log::Log for LogAdapter {
    fn enabled(&self, metadata: &Metadata) -> bool {
        forward_enabled(&self.logger, metadata)
    }

    fn log(&self, record: &Record) {
        forward_log(&self.logger, record);
    }

    fn flush(&self) {
        self.logger.flush();
    }
}

fn level_to_level(level: log::Level) -> logforth_core::record::Level {
    match level {
        log::Level::Error => logforth_core::record::Level::Error,
        log::Level::Warn => logforth_core::record::Level::Warn,
        log::Level::Info => logforth_core::record::Level::Info,
        log::Level::Debug => logforth_core::record::Level::Debug,
        log::Level::Trace => logforth_core::record::Level::Trace,
    }
}

fn forward_enabled(logger: &Logger, metadata: &Metadata) -> bool {
    let criteria = FilterCriteria::builder()
        .target(metadata.target())
        .level(level_to_level(metadata.level()))
        .build();

    Logger::enabled(logger, &criteria)
}

fn forward_log(logger: &Logger, record: &Record) {
    if !forward_enabled(logger, record.metadata()) {
        return;
    }

    // basic fields
    let mut builder = logforth_core::record::Record::builder()
        .level(level_to_level(record.level()))
        .target(record.target())
        .line(record.line());

    // optional static fields
    builder = if let Some(module_path) = record.module_path_static() {
        builder.module_path_static(module_path)
    } else {
        builder.module_path(record.module_path())
    };
    builder = if let Some(file) = record.file_static() {
        builder.file_static(file)
    } else {
        builder.file(record.file())
    };

    // payload
    builder = builder.payload(*record.args());

    // key-values
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
    record.key_values().visit(&mut visitor).unwrap();

    let mut new_kvs = Vec::with_capacity(kvs.len());
    for (k, v) in kvs.iter() {
        new_kvs.push((Key::borrowed(k.as_str()), Value::from_sval2(v)));
    }
    builder = builder.key_values(new_kvs.as_slice());

    Logger::log(logger, &builder.build());
}
