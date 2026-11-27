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

use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;

use log::Log;
use logforth::Append;
use logforth::Diagnostic;
use logforth::Error;
use logforth::bridge::log::LogBridge;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::record::LevelFilter;
use logforth::record::Record;
use logforth::starter_log::LogStarterExt;

#[derive(Debug, Default)]
struct RecordingSink(Mutex<Vec<String>>);

impl Append for RecordingSink {
    fn append(&self, record: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        self.0.lock().unwrap().push(record.payload().to_string());
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn child_valid_rust_log_sets_max_level_and_conservative_log_enabled() {
    if std::env::var("LOGFORTH_STARTER_CHILD").as_deref() != Ok("valid") {
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "child_valid_rust_log_sets_max_level_and_conservative_log_enabled",
            ])
            .env("LOGFORTH_STARTER_CHILD", "valid")
            .env("RUST_LOG", "info,my_app::query=warn")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }

    logforth::starter_log::testing().install();

    assert_eq!(log::max_level(), log::LevelFilter::Info);
    assert!(log::log_enabled!(target: "my_app::query", log::Level::Info));
    assert!(log::log_enabled!(target: "my_app::http", log::Level::Info));
    assert!(!log::log_enabled!(target: "my_app::http", log::Level::Debug));

    // Second global installation via `LogStarterExt` on `LoggerBuilder` / `Logger` returns `Err`.
    assert!(logforth::builder().try_install().is_err());
    assert!(logforth::Logger::noop().try_install().is_err());
}

#[test]
fn child_invalid_rust_log_propagates_error_unless_custom_filter_set() {
    if std::env::var("LOGFORTH_STARTER_CHILD").as_deref() != Ok("invalid") {
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "child_invalid_rust_log_propagates_error_unless_custom_filter_set",
            ])
            .env("LOGFORTH_STARTER_CHILD", "invalid")
            .env("RUST_LOG", "info,crate1=invalid")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }

    let err = logforth::starter_log::testing().try_install().unwrap_err();
    assert_eq!(err.to_string(), "malformed logging spec 'invalid'");

    let err = logforth::starter_log::stdout().try_build().unwrap_err();
    assert_eq!(err.to_string(), "malformed logging spec 'invalid'");

    // Overriding `.filter(...)` skips parsing `RUST_LOG`.
    assert!(
        logforth::starter_log::testing()
            .filter(LevelFilter::Off)
            .try_build()
            .is_ok()
    );
}

#[test]
fn log_bridge_and_rustlog_filter_agree_on_logforth_target_end_to_end() {
    let sink = Arc::new(RecordingSink::default());
    let builder = RustLogFilterBuilder::from_spec("info,my_app::query=warn").unwrap();
    let filter = builder.build();
    let logger = logforth::builder()
        .dispatch(|d| d.filter(filter).append(sink.clone()))
        .build();
    let bridge = LogBridge::new(logger);

    bridge.log(
        &log::Record::builder()
            .level(log::Level::Info)
            .target("my_app::query")
            .module_path_static(Some("my_app::db::driver"))
            .args(format_args!("dropped-info"))
            .build(),
    );
    bridge.log(
        &log::Record::builder()
            .level(log::Level::Warn)
            .target("my_app::query")
            .module_path_static(Some("my_app::db::driver"))
            .args(format_args!("accepted-warn"))
            .build(),
    );

    assert_eq!(&*sink.0.lock().unwrap(), &["accepted-warn"]);
}
