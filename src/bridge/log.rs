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

use crate::Logger;
use crate::default_logger;
use crate::kv::Key;
use crate::kv::Value;
use crate::record::MetadataBuilder;
use crate::record::RecordBuilder;

struct LogCrateLogger(());

impl log::Log for LogCrateLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let Some(logger) = default_logger() else {
            return false;
        };

        log::Log::enabled(logger, metadata)
    }

    fn log(&self, record: &log::Record) {
        if let Some(logger) = default_logger() {
            log::Log::log(logger, record);
        }
    }

    fn flush(&self) {
        if let Some(logger) = default_logger() {
            log::Log::flush(logger);
        }
    }
}

/// Set up the log crate global logger.
///
/// This function calls [`log::set_logger`] to set up a `LogCrateProxy` and
/// all logs from log crate will be forwarded to `logforth`'s logger.
///
/// This should be called early in the execution of a Rust program. Any log events that occur
/// before initialization will be ignored.
///
/// This function will set the global maximum log level to `Trace`. To override this, call
/// [`log::set_max_level`] after this function.
///
/// # Errors
///
/// Return an error if the log crate global logger has already been set.
///
/// # Examples
///
/// ```
/// logforth::bridge::setup_log_crate();
/// logforth::builder().apply()
/// ```
pub fn try_setup_log_crate() -> Result<(), log::SetLoggerError> {
    static LOGGER: LogCrateLogger = LogCrateLogger(());
    log::set_logger(&LOGGER)?;
    log::set_max_level(log::LevelFilter::Trace);
    Ok(())
}

/// Set up the log crate global logger.
///
/// This function calls [`log::set_logger`] to set up a `LogCrateProxy` and
/// all logs from log crate will be forwarded to `logforth`'s logger.
///
/// This should be called early in the execution of a Rust program. Any log events that occur
/// before initialization will be ignored.
///
/// This function will panic if it is called more than once, or if another library has already
/// initialized the log crate global logger.
///
/// This function will set the global maximum log level to `Trace`. To override this, call
/// [`log::set_max_level`] after this function.
///
/// # Panics
///
/// Panic if the log crate global logger has already been set.
///
/// # Examples
///
/// ```
/// logforth::bridge::setup_log_crate();
/// logforth::builder().apply()
/// ```
pub fn setup_log_crate() {
    try_setup_log_crate().expect(
        "logforth::bridge::setup_log_crate must be called before the log crate global logger initialized",
    )
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let metadata = MetadataBuilder::default()
            .target(metadata.target())
            .level(metadata.level().into())
            .build();

        Logger::enabled(self, &metadata)
    }

    fn log(&self, record: &log::Record) {
        // basic fields
        let mut builder = RecordBuilder::default()
            .args(*record.args())
            .level(record.level().into())
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
            new_kvs.push((Key::from(k.as_str()), Value::from_sval2(v)));
        }
        builder = builder.key_values(new_kvs.as_slice());

        Logger::log(self, &builder.build());
    }

    fn flush(&self) {
        Logger::flush(self);
    }
}
