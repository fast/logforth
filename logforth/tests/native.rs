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

use logforth::Append;
use logforth::Diagnostic;
use logforth::Error;
use logforth::debug;
use logforth::diagnostic::ThreadLocalDiagnostic;
use logforth::error;
use logforth::error::ErrorSink;
use logforth::fatal;
use logforth::info;
use logforth::kv::Value;
use logforth::log;
use logforth::record::Level;
use logforth::record::LevelFilter;
use logforth::record::Record;
use logforth::trace;
use logforth::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedEvent {
    level: Level,
    module_path: Option<String>,
    payload: String,
    kvs: Vec<(String, String)>,
    diags: Vec<(String, String)>,
}

#[derive(Debug, Default)]
struct RecordingAppend {
    events: Mutex<Vec<CapturedEvent>>,
}

impl RecordingAppend {
    fn take(&self) -> Vec<CapturedEvent> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

impl Append for RecordingAppend {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut context = Vec::new();
        diags.visit(
            &mut |k: logforth::kv::KeyView, v: logforth::kv::ValueView| {
                context.push((k.to_string(), v.to_string()));
                Ok(())
            },
        )?;
        let event = CapturedEvent {
            level: record.level(),
            module_path: record.module_path().map(str::to_owned),
            payload: record.payload().to_string(),
            kvs: record
                .key_values()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            diags: context,
        };
        self.events.lock().unwrap().push(event);
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct CountingErrorSink {
    errors: Mutex<Vec<String>>,
}

impl ErrorSink for CountingErrorSink {
    fn sink(&self, err: &Error) {
        self.errors.lock().unwrap().push(err.to_string());
    }
}

#[test]
fn native_macros_error_sink_structured_fields_and_thread_local_diagnostic() {
    let sink = Arc::new(RecordingAppend::default());
    let error_sink = Arc::new(CountingErrorSink::default());
    let logger = logforth::builder()
        .error_sink(error_sink.clone())
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Trace))
                .diagnostic(ThreadLocalDiagnostic::default())
                .append(sink.clone())
        })
        .build();

    trace!(logger, "start");
    let outer = ThreadLocalDiagnostic::scope([("scope", "outer")]);
    debug!(logger, "dispatch");
    let inner = ThreadLocalDiagnostic::scope([("scope", "inner")]);
    info!(
        logger,
        ["job" = "sync", "attempt" = 2, "retry" = true],
        "running"
    );
    warn!(logger, ["delay" = Value::display(&3)], "delayed");
    drop(inner);
    error!(logger, "failed");
    fatal!(logger, "stopped");
    drop(outer);
    log!(logger, Level::Info2, "done");

    let events: [_; 7] = sink.take().try_into().unwrap();
    let levels = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
        Level::Fatal,
        Level::Info2,
    ];
    let payloads = [
        "start", "dispatch", "running", "delayed", "failed", "stopped", "done",
    ];
    assert_eq!(events.each_ref().map(|e| e.level), levels);
    assert_eq!(events.each_ref().map(|e| e.payload.as_str()), payloads);
    let outer = &[("scope".into(), "outer".into())][..];
    let inner = &[("scope".into(), "inner".into())][..];
    let scopes = [&[][..], outer, inner, inner, outer, outer, &[]];
    assert_eq!(events.each_ref().map(|e| e.diags.as_slice()), scopes);
    let fields = [("job", "sync"), ("attempt", "2"), ("retry", "true")];
    assert_eq!(events[2].kvs, fields.map(|(k, v)| (k.into(), v.into())));
    assert_eq!(events[3].kvs, [("delay".into(), "3".into())]);
    assert!(error_sink.errors.lock().unwrap().is_empty());
}

#[cfg(all(feature = "bridge-log", feature = "filter-rustlog"))]
#[test]
fn native_macro_and_log_bridge_share_rust_log_filter() {
    use logforth::bridge::log::LogBridge;
    use logforth::filter::rustlog::RustLogFilterBuilder;

    let spec = concat!("off,", module_path!(), "=info,custom::target=info");
    let filter = RustLogFilterBuilder::from_spec(spec).unwrap().build();
    let sink = Arc::new(RecordingAppend::default());
    let logger = logforth::builder()
        .dispatch(|d| d.filter(filter).append(sink.clone()))
        .build();
    let bridge = LogBridge::new(logger.clone());
    log::set_max_level(bridge.max_level());
    log::set_boxed_logger(Box::new(bridge)).unwrap();

    info!(logger, "native info");
    debug!(logger, "native debug");
    log::info!(target: "custom::target", "custom info");
    log::info!(target: "ignored::target", "ignored info");

    let [native, custom]: [_; 2] = sink.take().try_into().unwrap();
    assert_eq!(native.payload, "native info");
    assert_eq!(native.module_path.as_deref(), Some(module_path!()));
    assert_eq!(custom.payload, "custom info");
    assert_eq!(
        custom.kvs,
        [("logforth.target".into(), "custom::target".into())]
    );
}

#[cfg(feature = "append-async")]
#[test]
fn native_macros_survive_async_dispatch_and_flush() {
    use logforth::append::asynchronous::AsyncBuilder;

    let sink = Arc::new(RecordingAppend::default());
    let appender = AsyncBuilder::new("native-macro-async")
        .append(sink.clone())
        .build();
    let logger = logforth::builder()
        .dispatch(|d| {
            d.diagnostic(ThreadLocalDiagnostic::default())
                .append(appender)
        })
        .build();
    {
        let job = String::from("sync");
        let _scope = ThreadLocalDiagnostic::scope([("request", job.clone())]);
        info!(logger, ["job" = job], "starting {job}");
        warn!(
            logger,
            ["job" = Value::display(&format!("{job}-2"))],
            "retrying {job}"
        );
    }
    logger.flush();

    let [info, warn]: [_; 2] = sink.take().try_into().unwrap();
    assert_eq!(info.payload, "starting sync");
    assert_eq!(info.kvs, [("job".into(), "sync".into())]);
    assert_eq!(warn.payload, "retrying sync");
    assert_eq!(warn.kvs, [("job".into(), "sync-2".into())]);
    assert_eq!(info.diags, [("request".into(), "sync".into())]);
    assert_eq!(warn.diags, info.diags);
}
