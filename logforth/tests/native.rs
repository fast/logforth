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
    impl logforth::kv::ToValue for Expensive {
        fn to_value(&self) -> Value<'_> {
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
    let logger = Arc::new(
        logforth::builder()
            .dispatch(|d| d.append(output.clone()))
            .dispatch(|d| d.append(output.clone()))
            .build(),
    );
    let name = String::from("worker");
    let calls = Cell::new(0);
    let expected_line = line!() + 1;
    logforth::info!(&logger, {
        String::from("name") => name,
        "count" => { calls.set(calls.get() + 1); 3u32 },
        "ready" => true,
        "missing" => None::<u64>,
        "wide" => u128::MAX,
        "debug" => Value::debug(&vec![1, 2]),
        "display" => Value::display(&format_args!("{name}:{}", 3)),
    }, "{name} finished {count}", count = { calls.set(calls.get() + 1); 3 });
    assert_eq!(calls.get(), 2);
    assert_eq!(name, "worker");
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    for (event, _) in events.iter() {
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
    logforth::log!(logger.named("runtime"), Level::Warn4, "keep this too");
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    events[0].0.with(|r| assert_eq!(r.level(), Level::Fatal));
    events[1].0.with(|r| {
        assert_eq!(r.level(), Level::Warn4);
        assert_eq!(r.target(), "runtime");
        assert_eq!(r.module_path(), Some(module_path!()));
    });
    assert_eq!(output.flushes.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "filter-rustlog")]
#[test]
fn targets_route_fields_only_events_and_prefilter_expensive_values() {
    use logforth::filter::RustLogFilter;

    let ordinary = Capture::default();
    let usage = Capture::default();
    let logger = logforth::builder()
        .dispatch(|d| {
            d.filter("info,usage=off".parse::<RustLogFilter>().unwrap())
                .append(ordinary.clone())
        })
        .dispatch(|d| {
            d.filter("off,usage=info".parse::<RustLogFilter>().unwrap())
                .append(usage.clone())
        })
        .build();
    logforth::debug!(logger.named("usage"), "{}", panic_if_called());
    logforth::info!(logger.named("usage"), { "units" => 7u64 });
    logforth::warn!(logger, "queue is full");
    logforth::error!(logger, "worker failed");
    logforth::trace!(logger, "{}", panic_if_called());
    assert_eq!(ordinary.events.lock().unwrap().len(), 2);
    let usage = usage.events.lock().unwrap();
    assert_eq!(usage.len(), 1);
    usage[0].0.with(|r| {
        assert_eq!(r.target(), "usage");
        assert_eq!(r.payload().to_string(), "");
        assert_eq!(r.key_values().get("units").unwrap().to_u64(), Some(7));
    });
}

#[cfg(feature = "filter-rustlog")]
fn panic_if_called() -> usize {
    panic!("disabled message expression");
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
        logger.named("bridge"),
    )))
    .unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    logforth::info!(logger.named("service"), { "ready" => true }, "started");
    log::warn!(target: "dependency", attempts = 2u64; "retrying");
    logger.flush();
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    events[0].0.with(|r| assert_eq!(r.target(), "service"));
    events[1].0.with(|r| {
        assert_eq!(r.target(), "dependency");
        assert_eq!(r.key_values().get("attempts").unwrap().to_u64(), Some(2));
    });
    assert_eq!(output.flushes.load(Ordering::SeqCst), 1);
}

#[cfg(all(
    feature = "serde",
    feature = "append-async",
    feature = "layout-json",
    feature = "diagnostic-fastrace"
))]
#[test]
fn async_dispatch_owns_nested_values_and_captures_diagnostics_before_flush() -> Result<(), Error> {
    use logforth::Layout;
    use logforth::append::asynchronous::AsyncBuilder;
    use logforth::diagnostic::FastraceDiagnostic;
    use logforth::diagnostic::StaticDiagnostic;

    #[derive(serde::Serialize)]
    struct Snapshot {
        queues: Vec<Vec<u32>>,
        healthy: bool,
    }

    let output = Capture::default();
    let mut context = StaticDiagnostic::default();
    context.insert("service", "worker");
    let logger = logforth::builder()
        .dispatch(|d| {
            d.diagnostic(context)
                .diagnostic(FastraceDiagnostic::default())
                .append(
                    AsyncBuilder::new("native-test")
                        .append(output.clone())
                        .build(),
                )
        })
        .build();
    {
        let span = fastrace::Span::root("batch", fastrace::collector::SpanContext::random());
        let _guard = span.set_local_parent();
        let snapshot = Snapshot {
            queues: vec![vec![1, 2], vec![3]],
            healthy: true,
        };
        logforth::info!(logger, {
            "snapshot" => logforth::kv::serde(&snapshot),
            "details" => Value::debug(&snapshot.queues),
        }, "resource sample");
    }
    logger.flush();
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    let (record, diags) = &events[0];
    record.with(|r| {
        let data = logforth::layout::JsonLayout::default()
            .format(&r, &[])
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&data).unwrap();
        assert_eq!(
            json["kvs"]["snapshot"],
            serde_json::json!({"queues": [[1, 2], [3]], "healthy": true})
        );
        assert_eq!(json["kvs"]["details"], "[[1, 2], [3]]");
    });
    let get = |key: &str| {
        diags
            .iter()
            .find(|(k, _)| k.view().as_str() == key)
            .unwrap()
            .1
            .view()
    };
    assert_eq!(get("service").to_str(), Some("worker"));
    assert_eq!(get("trace_id").to_str().unwrap().len(), 32);
    assert_eq!(get("span_id").to_str().unwrap().len(), 16);
    assert_eq!(output.flushes.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn named_handles_share_the_pipeline_and_independent_loggers_keep_their_policy() {
    let ordinary = Capture::default();
    let logger = logforth::builder()
        .dispatch(|d| d.filter(LevelFilter::Off).append(ordinary.clone()))
        .build();
    let usage_output = Capture::default();
    let usage = logforth::builder()
        .dispatch(|d| d.append(usage_output.clone()))
        .build()
        .named("usage");
    let cloned = usage.clone();
    let other = usage.named("resource");
    drop(usage);
    logforth::info!(logger.named("worker"), "disabled");
    logforth::info!(cloned, { "units" => 5i64 },);
    logforth::info!(other, "enabled");
    other.flush();
    assert!(ordinary.events.lock().unwrap().is_empty());
    let events = usage_output.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    events[0]
        .0
        .with(|r| assert_eq!(r.target_static(), Some("usage")));
    events[1]
        .0
        .with(|r| assert_eq!(r.target_static(), Some("resource")));
    assert_eq!(usage_output.flushes.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "serde")]
#[test]
fn failed_serialization_preserves_the_event_and_other_fields() {
    struct Broken;
    impl serde::Serialize for Broken {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("snapshot unavailable"))
        }
    }
    let output = Capture::default();
    let logger = logforth::builder()
        .dispatch(|d| d.append(output.clone()))
        .build();
    logforth::warn!(logger, {
        "snapshot" => logforth::kv::serde(&Broken),
        "retry" => true,
    }, "resource monitor failed");
    let events = output.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    events[0].0.with(|r| {
        assert_eq!(r.payload().to_string(), "resource monitor failed");
        assert_eq!(r.key_values().get("retry").unwrap().to_bool(), Some(true));
        let value = r.key_values().get("snapshot").unwrap();
        let error = value.to_str().unwrap();
        assert!(error.starts_with("<serialization failed:"));
        assert!(error.contains("snapshot unavailable"));
    });
}
