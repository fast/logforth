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

use log::Metadata;
use log::Record;
use logforth_core::Logger;
use logforth_core::record::Level;

// Keep synchronized with "Appendix: Key-Value Naming Conventions" in DESIGN.md.
const KEY_LOGFORTH_TARGET: &str = "logforth.target";

/// Adapter to use a [`Logger`] instance as a `log` crate logger.
#[derive(Debug, Clone)]
pub struct LogBridge {
    logger: Logger,
}

impl LogBridge {
    /// Creates a new [`LogBridge`] wrapping `logger`.
    pub const fn new(logger: Logger) -> Self {
        Self { logger }
    }

    /// Returns the maximum [`log::LevelFilter`] enabled by the underlying [`Logger`].
    pub fn max_level(&self) -> log::LevelFilter {
        if self.logger.enabled(Level::Trace) {
            log::LevelFilter::Trace
        } else if self.logger.enabled(Level::Debug) {
            log::LevelFilter::Debug
        } else if self.logger.enabled(Level::Info) {
            log::LevelFilter::Info
        } else if self.logger.enabled(Level::Warn) {
            log::LevelFilter::Warn
        } else if self.logger.enabled(Level::Error) {
            log::LevelFilter::Error
        } else {
            log::LevelFilter::Off
        }
    }

    fn forward_enabled(&self, metadata: &Metadata) -> bool {
        self.logger.enabled(level_to_level(metadata.level()))
    }

    fn forward_log(&self, record: &Record) {
        let level = level_to_level(record.level());
        if !self.logger.enabled(level) {
            return;
        }

        let custom_target =
            (record.target() != record.module_path().unwrap_or("")).then_some(record.target());

        // basic fields
        let mut builder = logforth_core::record::Record::builder()
            .level(level)
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
        let kv_source = record.key_values();
        let kv_count = kv_source.count();
        if kv_count == 0 {
            let Some(target) = custom_target else {
                return Logger::log(&self.logger, &builder.build());
            };

            let target_kv = [(
                logforth_core::kv::Key::new(KEY_LOGFORTH_TARGET),
                logforth_core::kv::Value::str(target),
            )];
            builder = builder.key_values(logforth_core::kv::KeyValues::from(target_kv.as_slice()));
            return Logger::log(&self.logger, &builder.build());
        }

        #[cfg(not(feature = "serde"))]
        {
            let kvs = kv::key_values_stage_one(kv_source, kv_count);
            let new_kvs = kv::key_value_stage_two(&kvs, custom_target);
            builder = builder.key_values(new_kvs.to_key_values());
            Logger::log(&self.logger, &builder.build());
        }

        #[cfg(feature = "serde")]
        {
            let kvs = kv::key_values(kv_source, kv_count, custom_target);
            builder = builder.key_values(kvs.to_key_values());
            Logger::log(&self.logger, &builder.build());
        }
    }
}

impl Deref for LogBridge {
    type Target = Logger;

    fn deref(&self) -> &Self::Target {
        &self.logger
    }
}

impl log::Log for LogBridge {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.forward_enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.forward_log(record);
    }

    fn flush(&self) {
        self.logger.flush();
    }
}

fn level_to_level(level: log::Level) -> Level {
    match level {
        log::Level::Error => Level::Error,
        log::Level::Warn => Level::Warn,
        log::Level::Info => Level::Info,
        log::Level::Debug => Level::Debug,
        log::Level::Trace => Level::Trace,
    }
}

#[cfg(not(feature = "serde"))]
mod kv {
    use crate::KEY_LOGFORTH_TARGET;

    pub struct KeyValuesStageOne<'a> {
        kvs: Vec<(KeyStageOne<'a>, ValueStageOne<'a>)>,
    }

    struct KeyStageOne<'a>(log::kv::Key<'a>);

    struct ValueStageOne<'a>(MaybeOwnedValue<'a>);

    enum MaybeOwnedValue<'a> {
        Borrowed(logforth_core::kv::Value<'a>),
        Owned(String),
    }

    pub fn key_values_stage_one<'a>(
        source: &'a dyn log::kv::Source,
        count: usize,
    ) -> KeyValuesStageOne<'a> {
        let mut kvs = Vec::with_capacity(count);

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
                self.0 = MaybeOwnedValue::Owned(value.to_owned());
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

    pub struct KeyValuesStageTwo<'a> {
        kvs: Vec<(KeyStageTwo<'a>, ValueStageTwo<'a>)>,
    }

    impl<'a> KeyValuesStageTwo<'a> {
        pub fn to_key_values(&self) -> logforth_core::kv::KeyValues<'_> {
            logforth_core::kv::KeyValues::from(self.kvs.as_slice())
        }
    }

    type KeyStageTwo<'a> = logforth_core::kv::Key<'a>;

    type ValueStageTwo<'a> = logforth_core::kv::Value<'a>;

    pub fn key_value_stage_two<'a>(
        kvs: &'a KeyValuesStageOne<'a>,
        custom_target: Option<&'a str>,
    ) -> KeyValuesStageTwo<'a> {
        let mut new_kvs = Vec::with_capacity(kvs.kvs.len() + usize::from(custom_target.is_some()));
        if let Some(target) = custom_target {
            new_kvs.push((
                logforth_core::kv::Key::new(KEY_LOGFORTH_TARGET),
                logforth_core::kv::Value::str(target),
            ));
        }
        for (k, v) in &kvs.kvs {
            let k = if let Some(k) = k.0.to_static_str() {
                logforth_core::kv::Key::new(k)
            } else {
                logforth_core::kv::Key::borrowed(k.0.as_str())
            };
            let v = match &v.0 {
                MaybeOwnedValue::Borrowed(v) => *v,
                MaybeOwnedValue::Owned(s) => logforth_core::kv::Value::str(s.as_str()),
            };
            new_kvs.push((k, v));
        }
        KeyValuesStageTwo { kvs: new_kvs }
    }
}

#[cfg(feature = "serde")]
mod kv {
    use logforth_core::kv::KeyOwned;
    use logforth_core::kv::ValueOwned;

    use crate::KEY_LOGFORTH_TARGET;

    pub struct KeyValues {
        kvs: Vec<(KeyOwned, ValueOwned)>,
    }

    impl KeyValues {
        pub fn to_key_values(&self) -> logforth_core::kv::KeyValues<'_> {
            logforth_core::kv::KeyValues::from(self.kvs.as_slice())
        }
    }

    pub fn key_values(
        source: &dyn log::kv::Source,
        count: usize,
        custom_target: Option<&str>,
    ) -> KeyValues {
        struct KeyValueVisitor {
            kvs: Vec<(KeyOwned, ValueOwned)>,
        }

        impl<'kvs> log::kv::VisitSource<'kvs> for KeyValueVisitor {
            fn visit_pair(
                &mut self,
                key: log::kv::Key<'kvs>,
                value: log::kv::Value<'kvs>,
            ) -> Result<(), log::kv::Error> {
                let key = if let Some(key) = key.to_static_str() {
                    KeyOwned::new(key)
                } else {
                    KeyOwned::from(key.as_str().to_owned())
                };
                self.kvs.push((key, ValueOwned::from_serde(&value)));
                Ok(())
            }
        }

        let mut kvs = Vec::with_capacity(count + usize::from(custom_target.is_some()));
        if let Some(target) = custom_target {
            kvs.push((
                KeyOwned::new(KEY_LOGFORTH_TARGET),
                ValueOwned::from(target.to_owned()),
            ));
        }

        let mut visitor = KeyValueVisitor { kvs };
        log::kv::Source::visit(source, &mut visitor).unwrap();

        KeyValues { kvs: visitor.kvs }
    }
}
