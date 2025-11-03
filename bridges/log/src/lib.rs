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

use log::Metadata;
use log::Record;
use logforth_core::Logger;
use logforth_core::default_logger;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::record::FilterCriteria;
use logforth_core::record::RecordBuilder;

fn level_to_level(level: log::Level) -> logforth_core::record::Level {
    match level {
        log::Level::Error => logforth_core::record::Level::Error,
        log::Level::Warn => logforth_core::record::Level::Warn,
        log::Level::Info => logforth_core::record::Level::Info,
        log::Level::Debug => logforth_core::record::Level::Debug,
        log::Level::Trace => logforth_core::record::Level::Trace,
    }
}

struct LogCrateLogger(());

impl log::Log for LogCrateLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        forward_enabled(default_logger(), metadata)
    }

    fn log(&self, record: &Record) {
        forward_log(default_logger(), record);
    }

    fn flush(&self) {
        default_logger().flush();
    }
}

/// Adapter to use a specific `logforth` logger instance as a `log` crate logger.
pub struct LogProxy<'a>(&'a Logger);

impl<'a> LogProxy<'a> {
    /// Create a new `LogProxy` instance.
    pub fn new(logger: &'a Logger) -> Self {
        Self(logger)
    }
}

impl<'a> log::Log for LogProxy<'a> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        forward_enabled(self.0, metadata)
    }

    fn log(&self, record: &Record) {
        forward_log(self.0, record);
    }

    fn flush(&self) {
        self.0.flush();
    }
}

/// Owned version of [`LogProxy`].
pub struct OwnedLogProxy(Logger);

impl OwnedLogProxy {
    /// Create a new `OwnedLogProxy` instance.
    pub fn new(logger: Logger) -> Self {
        Self(logger)
    }
}

impl log::Log for OwnedLogProxy {
    fn enabled(&self, metadata: &Metadata) -> bool {
        forward_enabled(&self.0, metadata)
    }

    fn log(&self, record: &Record) {
        forward_log(&self.0, record);
    }

    fn flush(&self) {
        self.0.flush();
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
    let mut builder = RecordBuilder::default()
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
    builder = if let Some(payload) = record.args().as_str() {
        builder.payload(payload)
    } else {
        builder.payload(record.args().to_string())
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
        new_kvs.push((Key::new_ref(k.as_str()), Value::from_sval2(v)));
    }
    builder = builder.key_values(new_kvs.as_slice());

    Logger::log(logger, &builder.build());
}

/// Set up the log crate global logger.
///
/// This function calls [`log::set_logger`] to set up a `LogCrateProxy` and
/// all logs from log crate will be forwarded to `logforth`'s default logger.
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
/// if let Err(err) = logforth_bridge_log::try_setup() {
///     eprintln!("failed to setup log crate: {err}");
/// }
/// ```
pub fn try_setup() -> Result<(), log::SetLoggerError> {
    static LOGGER: LogCrateLogger = LogCrateLogger(());
    log::set_logger(&LOGGER)?;
    log::set_max_level(log::LevelFilter::Trace);
    Ok(())
}

/// Set up the log crate global logger.
///
/// This function calls [`log::set_logger`] to set up a `LogCrateProxy` and
/// all logs from log crate will be forwarded to `logforth`'s default logger.
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
/// logforth_bridge_log::setup();
/// logforth_core::builder().apply()
/// ```
pub fn setup() {
    try_setup().expect(
        "logforth_bridge_log::setup must be called before the log crate global logger initialized",
    )
}
