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

use std::sync::Arc;
use std::sync::Mutex;

use log::Log;
use logforth_bridge_log::LogBridge;
use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Logger;
use logforth_core::builder;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;

#[derive(Debug, PartialEq, Eq)]
enum CapturedVal {
    Str(String),
    I64(i64),
    Other,
}

#[derive(Debug)]
struct CapturedRecord {
    module_path: Option<String>,
    kvs: Vec<(String, CapturedVal)>,
}

#[derive(Debug, Default)]
struct CapturingAppender {
    records: Mutex<Vec<CapturedRecord>>,
}

impl Append for CapturingAppender {
    fn append(&self, record: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        let kvs = record
            .key_values()
            .iter()
            .map(|(k, v)| {
                let val = if let Some(s) = v.to_str() {
                    CapturedVal::Str(s.to_owned())
                } else if let Some(n) = v.to_i64() {
                    CapturedVal::I64(n)
                } else {
                    CapturedVal::Other
                };
                (k.as_str().to_owned(), val)
            })
            .collect();
        self.records.lock().unwrap().push(CapturedRecord {
            module_path: record.module_path().map(str::to_owned),
            kvs,
        });
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn max_level_reflects_logger_enabled_levels() {
    assert_eq!(
        LogBridge::new(Logger::noop()).max_level(),
        log::LevelFilter::Off
    );

    let info_logger = builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .append(CapturingAppender::default())
        })
        .build();
    assert_eq!(
        LogBridge::new(info_logger).max_level(),
        log::LevelFilter::Info
    );

    let multi_logger = builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Error))
                .append(CapturingAppender::default())
        })
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Debug))
                .append(CapturingAppender::default())
        })
        .build();
    assert_eq!(
        LogBridge::new(multi_logger).max_level(),
        log::LevelFilter::Debug
    );

    let all_logger = builder()
        .dispatch(|d| {
            d.filter(LevelFilter::All)
                .append(CapturingAppender::default())
        })
        .build();
    assert_eq!(
        LogBridge::new(all_logger).max_level(),
        log::LevelFilter::Trace
    );
}

#[test]
fn enabled_checks_logger_level_conservatively() {
    let logger = builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .append(CapturingAppender::default())
        })
        .build();
    let bridge = LogBridge::new(logger);

    assert!(
        bridge.enabled(
            &log::Metadata::builder()
                .level(log::Level::Info)
                .target("my_app::query")
                .build()
        )
    );
    assert!(
        !bridge.enabled(
            &log::Metadata::builder()
                .level(log::Level::Debug)
                .target("my_app::query")
                .build()
        )
    );
}

#[test]
fn log_bridge_emits_logforth_target_only_when_target_differs_from_module_path() {
    let sink = Arc::new(CapturingAppender::default());
    let logger = builder().dispatch(|d| d.append(sink.clone())).build();
    let bridge = LogBridge::new(logger);

    // Same target and module_path: no `logforth.target` KV emitted.
    bridge.log(
        &log::Record::builder()
            .level(log::Level::Info)
            .target("my_app::service")
            .module_path_static(Some("my_app::service"))
            .args(format_args!("normal"))
            .build(),
    );

    // Overridden target with no extra KVs: `logforth.target` emitted via stack fast path.
    bridge.log(
        &log::Record::builder()
            .level(log::Level::Warn)
            .target("my_app::query")
            .module_path_static(Some("my_app::db::driver"))
            .args(format_args!("slow query"))
            .build(),
    );

    // Overridden target alongside structured `log::kv` entries: `logforth.target` emitted first.
    let extra_kvs: &[(&str, i32)] = &[("rows", 42)];
    bridge.log(
        &log::Record::builder()
            .level(log::Level::Warn)
            .target("my_app::query")
            .module_path_static(Some("my_app::db::driver"))
            .key_values(&extra_kvs)
            .args(format_args!("query with kvs"))
            .build(),
    );

    // Explicit empty target overriding a non-empty module_path: `logforth.target` emitted as `""`.
    bridge.log(
        &log::Record::builder()
            .level(log::Level::Info)
            .target("")
            .module_path_static(Some("my_app::service"))
            .args(format_args!("empty override"))
            .build(),
    );

    let captured = sink.records.lock().unwrap();
    assert_eq!(captured.len(), 4);

    assert_eq!(captured[0].module_path.as_deref(), Some("my_app::service"));
    assert!(captured[0].kvs.is_empty());

    assert_eq!(
        captured[1].module_path.as_deref(),
        Some("my_app::db::driver")
    );
    assert_eq!(
        captured[1].kvs,
        vec![(
            "logforth.target".to_owned(),
            CapturedVal::Str("my_app::query".to_owned())
        )]
    );

    assert_eq!(
        captured[2].module_path.as_deref(),
        Some("my_app::db::driver")
    );
    assert_eq!(
        captured[2].kvs,
        vec![
            (
                "logforth.target".to_owned(),
                CapturedVal::Str("my_app::query".to_owned())
            ),
            ("rows".to_owned(), CapturedVal::I64(42)),
        ]
    );

    assert_eq!(captured[3].module_path.as_deref(), Some("my_app::service"));
    assert_eq!(
        captured[3].kvs,
        vec![(
            "logforth.target".to_owned(),
            CapturedVal::Str(String::new())
        )]
    );
}
