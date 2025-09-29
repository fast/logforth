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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

struct LogCrateLogger(());

impl log::Log for LogCrateLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let logger = logforth_core::default_logger();
        log::Log::enabled(logger, metadata)
    }

    fn log(&self, record: &log::Record) {
        let logger = logforth_core::default_logger();
        log::Log::log(logger, record)
    }

    fn flush(&self) {
        let logger = logforth_core::default_logger();
        log::Log::flush(logger)
    }
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
