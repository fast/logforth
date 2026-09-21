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

use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use logforth::Append;
use logforth::Diagnostic;
use logforth::Error;
use logforth::Filter;
use logforth::Level;
use logforth::LevelFilter;
use logforth::filter::FilterResult;
use logforth::kv::KeyOwned;
use logforth::kv::Value;
use logforth::kv::ValueOwned;
use logforth::record::FilterCriteria;
use logforth::record::Record;
use logforth::record::RecordOwned;

type Captured = (RecordOwned, Vec<(KeyOwned, ValueOwned)>);

#[derive(Clone, Debug, Default)]
struct Capture {
    events: Arc<Mutex<Vec<Captured>>>,
    flushes: Arc<AtomicUsize>,
}

impl Append for Capture {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut fields = Vec::new();
        for diag in diags {
            diag.visit(&mut |key: logforth::kv::KeyView<'_>,
                             value: logforth::kv::ValueView<'_>| {
                fields.push((key.to_owned(), value.to_owned()));
                Ok(())
            })?;
        }
        self.events
            .lock()
            .unwrap()
            .push((record.to_owned(), fields));
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        self.flushes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn disabled_events_skip_payload_keys_and_conversions() {
    struct Expensive;
    impl From<&Expensive> for Value<'_> {
        fn from(_: &Expensive) -> Self {
            panic!("disabled field conversion");
        }
    }

    let output = Capture::default();
    let logger = logforth::builder()
        .dispatch(|d| d.filter(LevelFilter::Off).append(output.clone()))
        .build();
    let probes = Cell::new(0);
    let payloads = Cell::new(0);
    let key = || {
        payloads.set(payloads.get() + 1);
        "field"
    };
    let value = || {
        payloads.set(payloads.get() + 1);
        Expensive
    };
    logforth::log!(
        { probes.set(probes.get() + 1); &logger },
        { probes.set(probes.get() + 1); Level::Info },
        { key() => value() },
        "{}", { payloads.set(payloads.get() + 1); 42 },
    );
    assert_eq!(probes.get(), 2);
    assert_eq!(payloads.get(), 0);
    assert!(output.events.lock().unwrap().is_empty());
}

#[test]
fn fields_are_borrowed_typed_and_evaluated_once_across_dispatches() {
    let output = Capture::default();
    let mut context = logforth::diagnostic::StaticDiagnostic::default();
    context.insert("name", "service");
    let nested = ValueOwned::map([("depth".into(), 2u64.into())]);
    let logger = Arc::new(
        logforth::builder()
            .dispatch(|d| d.diagnostic(context.clone()).append(output.clone()))
            .dispatch(|d| d.diagnostic(context.clone()).append(output.clone()))
            .build(),
    );
    let name = String::from("worker");
    let name_ref = &name;
    let count = 3u32;
    let count_ref = &count;
    let nested_ref = &nested;
    let ready = Value::from(true);
    let ready_ref = &ready;
    let calls = Cell::new(0);
    let expected_line = line!() + 1;
    logforth::info!(&logger, {
        String::from("name") => name,
        "count" => { calls.set(calls.get() + 1); count_ref },
        "borrowed_name" => name_ref,
        "ready" => ready_ref,
        "missing" => Value::none(),
        "nested" => nested_ref,
        "wide" => u128::MAX,
        "debug" => Value::debug(&vec![1, 2]),
        "display" => Value::display(&format_args!("{name}:{}", 3)),
    }, "{name} finished {count}", count = { calls.set(calls.get() + 1); 3 });
    assert_eq!(calls.get(), 2);
    assert_eq!(name, "worker");
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    for (event, diagnostics) in events.iter() {
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].0.view().as_str(), "name");
        assert_eq!(diagnostics[0].1.view().to_str(), Some("service"));
        event.with(|r| {
            assert_eq!(r.target_static(), Some(module_path!()));
            assert_eq!(r.module_path_static(), Some(module_path!()));
            assert_eq!(r.file_static(), Some(file!()));
            assert_eq!(r.line(), Some(expected_line));
            assert_eq!(r.column(), Some(5));
            assert_eq!(r.payload().to_string(), "worker finished 3");
            let fields = r.key_values();
            assert_eq!(fields.get("name").unwrap().to_str(), Some("worker"));
            assert_eq!(fields.get("count").unwrap().to_u64(), Some(3));
            assert_eq!(
                fields.get("borrowed_name").unwrap().to_str(),
                Some("worker")
            );
            assert_eq!(
                fields
                    .get("nested")
                    .unwrap()
                    .to_map()
                    .unwrap()
                    .get("depth")
                    .unwrap()
                    .to_u64(),
                Some(2)
            );
            assert_eq!(fields.get("ready").unwrap().to_bool(), Some(true));
            assert!(matches!(
                fields.get("missing").unwrap(),
                logforth::kv::ValueView::None
            ));
            assert_eq!(fields.get("wide").unwrap().to_u128(), Some(u128::MAX));
            assert_eq!(fields.get("debug").unwrap().to_str(), Some("[1, 2]"));
            assert_eq!(fields.get("display").unwrap().to_str(), Some("worker:3"));
        });
    }
}

#[derive(Debug)]
struct MessageFilter;

impl Filter for MessageFilter {
    fn enabled(&self, _: &FilterCriteria, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        FilterResult::Neutral
    }

    fn matches(&self, record: &Record, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        if record.payload().to_string().contains("keep") {
            FilterResult::Accept
        } else {
            FilterResult::Reject
        }
    }
}

#[test]
fn full_record_filtering_and_explicit_flush_keep_the_logger_usable() {
    let output = Capture::default();
    let logger = logforth::builder()
        .dispatch(|d| d.filter(MessageFilter).append(output.clone()))
        .build();
    logforth::info!(logger, "discard");
    logforth::fatal!(logger, "keep going");
    logger.flush();
    logforth::log!(logger, Level::Warn4, "keep this too");
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    events[0].0.with(|r| assert_eq!(r.level(), Level::Fatal));
    events[1].0.with(|r| {
        assert_eq!(r.level(), Level::Warn4);
        assert_eq!(r.target(), module_path!());
        assert_eq!(r.module_path(), Some(module_path!()));
    });
    assert_eq!(output.flushes.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "filter-rustlog")]
#[test]
fn module_filters_apply_before_fields_only_events_are_built() {
    use logforth::filter::RustLogFilter;

    let output = Capture::default();
    let filter = format!("off,{}=info", module_path!())
        .parse::<RustLogFilter>()
        .unwrap();
    let logger = logforth::builder()
        .dispatch(|d| d.filter(filter).append(output.clone()))
        .build();
    let calls = Cell::new(0);
    logforth::debug!(logger, { "units" => { calls.set(calls.get() + 1); 7u64 } });
    logforth::info!(logger, { "units" => 7u64 },);
    assert_eq!(calls.get(), 0);
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    events[0].0.with(|r| {
        assert_eq!(r.target(), module_path!());
        assert_eq!(r.payload().to_string(), "");
        assert_eq!(r.key_values().get("units").unwrap().to_u64(), Some(7));
    });
}

#[cfg(feature = "bridge-log")]
#[test]
fn dependency_logs_and_native_events_share_dispatch_and_flush() {
    let output = Capture::default();
    let logger = Arc::new(
        logforth::builder()
            .dispatch(|d| d.append(output.clone()))
            .build(),
    );
    log::set_boxed_logger(Box::new(logforth::bridge::log::LogBridge::new(
        logger.clone(),
    )))
    .unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    logforth::info!(logger, { "ready" => true }, "started");
    log::warn!(target: "dependency", attempts = 2u64; "retrying");
    logger.flush();
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    events[0].0.with(|r| assert_eq!(r.target(), module_path!()));
    events[1].0.with(|r| {
        assert_eq!(r.target(), "dependency");
        assert_eq!(r.key_values().get("attempts").unwrap().to_u64(), Some(2));
    });
    assert_eq!(output.flushes.load(Ordering::SeqCst), 1);
}

#[test]
fn independent_loggers_keep_their_filtering_and_flush_policy() {
    let output = Capture::default();
    let ordinary = logforth::builder()
        .dispatch(|d| d.filter(LevelFilter::Off).append(output.clone()))
        .build();
    let usage = logforth::builder()
        .dispatch(|d| d.append(output.clone()))
        .build();
    logforth::info!(ordinary, "disabled");
    logforth::info!(usage, { "resource" => "worker", "units" => 5i64 });
    usage.flush();
    assert_eq!(output.events.lock().unwrap().len(), 1);
    assert_eq!(output.flushes.load(Ordering::SeqCst), 1);
}
