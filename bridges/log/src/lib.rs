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
    let kvs = kv::key_values_stage_one(record.key_values());
    let new_kvs = kv::key_value_stage_two(&kvs);
    builder = builder.key_values(new_kvs.to_key_values());

    Logger::log(logger, &builder.build());
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

#[cfg(not(feature = "serde"))]
mod kv {
    pub(super) struct KeyValuesStageOne<'a> {
        kvs: Vec<(KeyStageOne<'a>, ValueStageOne<'a>)>,
    }

    struct KeyStageOne<'a>(log::kv::Key<'a>);

    struct ValueStageOne<'a>(MaybeOwnedValue<'a>);

    enum MaybeOwnedValue<'a> {
        Borrowed(logforth_core::kv::Value<'a>),
        Owned(String),
    }

    pub(super) fn key_values_stage_one<'a>(
        source: &'a dyn log::kv::Source,
    ) -> KeyValuesStageOne<'a> {
        let mut kvs = Vec::new();

        struct KeyValueVisitor<'a, 'b> {
            kvs: &'b mut Vec<(KeyStageOne<'a>, ValueStageOne<'a>)>,
        }

        impl<'a, 'b> log::kv::VisitSource<'a> for KeyValueVisitor<'a, 'b> {
            fn visit_pair(
                &mut self,
                key: log::kv::Key<'a>,
                value: log::kv::Value<'a>,
            ) -> Result<(), log::kv::Error> {
                let key = KeyStageOne(key);
                let value = ValueStageOne(value_to_value(value));
                self.kvs.push((key, value));
                Ok(())
            }
        }

        let mut visitor = KeyValueVisitor { kvs: &mut kvs };
        log::kv::Source::visit(source, &mut visitor).unwrap();
        KeyValuesStageOne { kvs }
    }

    fn value_to_value(value: log::kv::Value) -> MaybeOwnedValue {
        struct ValueVisitor<'a>(MaybeOwnedValue<'a>);

        impl<'a> log::kv::VisitValue<'a> for ValueVisitor<'a> {
            fn visit_any(&mut self, value: log::kv::Value) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Owned(value.to_string());
                Ok(())
            }

            fn visit_null(&mut self) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::none());
                Ok(())
            }

            fn visit_u64(&mut self, value: u64) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::u64(value));
                Ok(())
            }

            fn visit_i64(&mut self, value: i64) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::i64(value));
                Ok(())
            }

            fn visit_u128(&mut self, value: u128) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::u128(value));
                Ok(())
            }

            fn visit_i128(&mut self, value: i128) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::i128(value));
                Ok(())
            }

            fn visit_f64(&mut self, value: f64) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::f64(value));
                Ok(())
            }

            fn visit_bool(&mut self, value: bool) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::bool(value));
                Ok(())
            }

            fn visit_str(&mut self, value: &str) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Owned(value.to_string());
                Ok(())
            }

            fn visit_borrowed_str(&mut self, value: &'a str) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::str(value));
                Ok(())
            }

            fn visit_char(&mut self, value: char) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::char(value));
                Ok(())
            }
        }

        let mut visitor = ValueVisitor(MaybeOwnedValue::Borrowed(logforth_core::kv::Value::none()));
        value.visit(&mut visitor).unwrap();
        visitor.0
    }

    pub(super) struct KeyValuesStageTwo<'a> {
        kvs: Vec<(KeyStageTwo<'a>, ValueStageTwo<'a>)>,
    }

    impl<'a> KeyValuesStageTwo<'a> {
        pub(super) fn to_key_values(&self) -> logforth_core::kv::KeyValues<'_> {
            logforth_core::kv::KeyValues::from(self.kvs.as_slice())
        }
    }

    type KeyStageTwo<'a> = logforth_core::kv::Key<'a>;

    type ValueStageTwo<'a> = logforth_core::kv::Value<'a>;

    pub(super) fn key_value_stage_two<'a>(kvs: &'a KeyValuesStageOne<'a>) -> KeyValuesStageTwo<'a> {
        let mut new_kvs = Vec::with_capacity(kvs.kvs.len());
        for (k, v) in &kvs.kvs {
            let k = logforth_core::kv::Key::borrowed(k.0.as_str());
            let v = match &v.0 {
                MaybeOwnedValue::Borrowed(v) => v.clone(),
                MaybeOwnedValue::Owned(s) => logforth_core::kv::Value::str(s.as_str()),
            };
            new_kvs.push((k, v));
        }
        KeyValuesStageTwo { kvs: new_kvs }
    }
}

#[cfg(feature = "serde")]
mod kv {}
