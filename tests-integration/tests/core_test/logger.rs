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

use std::io::Write as _;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::Filter;
use logforth_core::Layout;
use logforth_core::Logger;
use logforth_core::builder;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::error::BlackHoleErrorSink;
use logforth_core::filter::FilterResult;
use logforth_core::kv::Key;
use logforth_core::kv::KeyView;
use logforth_core::kv::Value;
use logforth_core::kv::ValueView;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;
use tests_integration::FailingAppender;
use tests_integration::RecordingAppender;
use tests_integration::RecordingErrorSink;

const NOOP_LOGGER: Logger = Logger::noop();

#[test]
fn noop_default_and_empty_builders_disable_all_levels_and_skip_sinks() {
    let errors = RecordingErrorSink::default();
    let empty_logger = builder().error_sink(errors.clone()).build();
    let skipped_branch_logger = builder()
        .error_sink(errors.clone())
        .dispatch(|d| d.filter(LevelFilter::All))
        .build();

    let record = Record::builder()
        .level(Level::Fatal4)
        .payload(format_args!("ignored"))
        .build();

    for logger in [
        NOOP_LOGGER,
        Logger::default(),
        empty_logger,
        skipped_branch_logger,
    ] {
        let cloned = logger.clone();
        for level in Level::ALL {
            assert!(
                !cloned.enabled(level),
                "expected {level} to be disabled on no-op logger"
            );
        }
        cloned.log(&record);
        cloned.flush();
    }

    assert!(errors.errors().is_empty());
}

#[test]
fn cloned_handles_share_pipeline_across_threads() {
    let sink = RecordingAppender::default();
    let logger = builder().dispatch(|d| d.append(sink.clone())).build();

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for worker_id in 0..2 {
        let worker_logger = logger.clone();
        let worker_barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            worker_barrier.wait();
            worker_logger.log(
                &Record::builder()
                    .payload(format_args!("worker-{worker_id}"))
                    .build(),
            );
        }));
    }

    barrier.wait();
    for handle in handles {
        handle.join().unwrap();
    }
    logger.clone().flush();

    let mut payloads = sink.payloads();
    payloads.sort();
    assert_eq!(payloads, ["worker-0", "worker-1"]);
    assert_eq!(sink.flush_count(), 1);
}

#[test]
fn dispatch_method_order_is_commutative_while_preserving_chain_order() {
    let first = RecordingAppender::default();
    let second = RecordingAppender::default();

    // Register appenders BEFORE filters and diagnostics: filters and diagnostics must still apply
    // to both appenders in registration order.
    let logger = builder()
        .dispatch(|d| {
            d.append(first.clone())
                .diagnostic(StaticDiagnostic::new([("first_diag", "1")]))
                .filter(LevelFilter::MoreSevereEqual(Level::Info))
                .append(second.clone())
                .diagnostic(StaticDiagnostic::new([("second_diag", "2")]))
                .filter(LevelFilter::NotEqual(Level::Warn))
        })
        .build();

    assert!(!logger.enabled(Level::Debug));
    assert!(logger.enabled(Level::Info));
    assert!(!logger.enabled(Level::Warn));

    logger.log(
        &Record::builder()
            .level(Level::Debug)
            .payload(format_args!("dropped-debug"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Warn)
            .payload(format_args!("dropped-warn"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("accepted-info"))
            .build(),
    );

    assert_eq!(first.payloads(), ["accepted-info"]);
    assert_eq!(second.payloads(), ["accepted-info"]);
    assert_eq!(
        first.events()[0].diagnostics,
        [
            ("first_diag".to_owned(), "1".to_owned()),
            ("second_diag".to_owned(), "2".to_owned()),
        ]
    );
    assert_eq!(
        second.events()[0].diagnostics,
        first.events()[0].diagnostics
    );
}

#[derive(Debug)]
struct CountingFilter {
    inner: LevelFilter,
    filter_calls: AtomicUsize,
}

impl CountingFilter {
    const fn new(inner: LevelFilter) -> Self {
        Self {
            inner,
            filter_calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.filter_calls.load(Ordering::Acquire)
    }
}

impl Filter for CountingFilter {
    fn filter_level(&self, level: Level) -> FilterResult {
        self.inner.filter_level(level)
    }

    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult {
        self.filter_calls.fetch_add(1, Ordering::Release);
        self.inner.filter(record, diags)
    }
}

#[derive(Debug)]
struct SampledDebugPromoter;

impl Filter for SampledDebugPromoter {
    fn filter_level(&self, level: Level) -> FilterResult {
        if level == Level::Debug {
            FilterResult::Accept
        } else {
            FilterResult::Neutral
        }
    }

    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult {
        let record_sampled = record
            .key_values()
            .get("sampled")
            .and_then(|v| v.to_bool())
            .unwrap_or(false);
        let mut diag_sampled = false;
        let _ = diags.visit(&mut |k: KeyView, v: ValueView| {
            if k.as_str() == "sampled" && v.to_bool() == Some(true) {
                diag_sampled = true;
            }
            Ok(())
        });

        if record_sampled || diag_sampled {
            FilterResult::Accept
        } else {
            FilterResult::Neutral
        }
    }
}

#[test]
fn level_bitmask_prunes_disabled_levels_before_filter_evaluation_and_supports_promotion() {
    let counting = Arc::new(CountingFilter::new(LevelFilter::NotEqual(Level::Warn)));
    let narrowed_sink = RecordingAppender::default();
    let promoted_sink = RecordingAppender::default();

    let logger = builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .filter(counting.clone())
                .append(narrowed_sink.clone())
        })
        .dispatch(|d| {
            d.filter(SampledDebugPromoter)
                .filter(LevelFilter::MoreSevereEqual(Level::Error))
                .append(promoted_sink.clone())
        })
        .build();

    // Branch 1 accepts Info..=Fatal4 except Warn; Branch 2 accepts Debug | Error..=Fatal4.
    assert!(!logger.enabled(Level::Trace));
    assert!(logger.enabled(Level::Debug));
    assert!(!logger.enabled(Level::Debug2));
    assert!(logger.enabled(Level::Info));
    assert!(!logger.enabled(Level::Warn));
    assert!(logger.enabled(Level::Error));

    // Statically disabled levels (Trace, Warn) and Branch-1-disabled levels (Debug) must never
    // invoke `CountingFilter::filter` on Branch 1.
    for (level, msg) in [
        (Level::Trace, "pruned-trace"),
        (Level::Warn, "pruned-warn"),
        (Level::Debug, "unsampled-debug"),
    ] {
        logger.log(
            &Record::builder()
                .level(level)
                .payload(format_args!("{msg}"))
                .build(),
        );
    }
    assert_eq!(counting.calls(), 0);
    assert!(narrowed_sink.payloads().is_empty());
    assert!(promoted_sink.payloads().is_empty());

    // Promoted Debug short-circuits Branch 2's `Error` filter via `FilterResult::Accept`.
    let sampled_kv = [(Key::new("sampled"), Value::bool(true))];
    logger.log(
        &Record::builder()
            .level(Level::Debug)
            .key_values(sampled_kv.as_slice())
            .payload(format_args!("sampled-debug"))
            .build(),
    );
    assert_eq!(counting.calls(), 0);
    assert_eq!(promoted_sink.payloads(), ["sampled-debug"]);

    // Info runs Branch 1's filter chain; Error runs both branches.
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("info-msg"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Error)
            .payload(format_args!("error-msg"))
            .build(),
    );

    assert_eq!(counting.calls(), 2);
    assert_eq!(narrowed_sink.payloads(), ["info-msg", "error-msg"]);
    assert_eq!(promoted_sink.payloads(), ["sampled-debug", "error-msg"]);
}

#[test]
fn error_routing_isolates_failing_appenders_and_applies_branch_overrides() {
    let fallback_errors = RecordingErrorSink::default();
    let replaced_errors = RecordingErrorSink::default();
    let active_override_errors = RecordingErrorSink::default();
    let healthy_sibling = RecordingAppender::default();
    let healthy_trailing = RecordingAppender::default();

    let logger = builder()
        .error_sink(fallback_errors.clone())
        .dispatch(|d| {
            d.append(FailingAppender::new("primary disk full"))
                .append(healthy_sibling.clone())
        })
        .dispatch(|d| {
            d.error_sink(BlackHoleErrorSink::default())
                .append(FailingAppender::new("silenced failure"))
        })
        .dispatch(|d| {
            d.error_sink(replaced_errors.clone())
                .error_sink(active_override_errors.clone())
                .append(FailingAppender::new("override network error"))
                .append(healthy_trailing.clone())
        })
        .build();

    logger.log(&Record::builder().payload(format_args!("event-1")).build());
    logger.flush();

    assert_eq!(healthy_sibling.payloads(), ["event-1"]);
    assert_eq!(healthy_sibling.flush_count(), 1);
    assert_eq!(healthy_trailing.payloads(), ["event-1"]);
    assert_eq!(healthy_trailing.flush_count(), 1);

    assert!(replaced_errors.errors().is_empty());
    assert_eq!(
        fallback_errors.errors(),
        [
            "failed to append record: primary disk full",
            "failed to flush appender: primary disk full",
        ]
    );
    assert_eq!(
        active_override_errors.errors(),
        [
            "failed to append record: override network error",
            "failed to flush appender: override network error",
        ]
    );
}

#[derive(Debug)]
struct SubstringRejectFilter(&'static str);

impl Filter for SubstringRejectFilter {
    fn filter_level(&self, _: Level) -> FilterResult {
        FilterResult::Neutral
    }

    fn filter(&self, record: &Record, _: &dyn Diagnostic) -> FilterResult {
        if record.payload().to_string().contains(self.0) {
            FilterResult::Reject
        } else {
            FilterResult::Neutral
        }
    }
}

#[derive(Debug)]
struct PrefixLayout(&'static str);

impl Layout for PrefixLayout {
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        write!(buf, "{}:{}", self.0, record.payload()).unwrap();
        diags.visit(&mut |k: KeyView, v: ValueView| {
            write!(buf, " {k}={v}").unwrap();
            Ok(())
        })
    }
}

#[derive(Debug)]
struct StaticCountingErrorSink {
    calls: AtomicUsize,
}

impl ErrorSink for StaticCountingErrorSink {
    fn sink(&self, _: &Error) {
        self.calls.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn arc_and_static_forwarding_share_all_five_traits_across_dispatches() {
    static STATIC_FILTER: SubstringRejectFilter = SubstringRejectFilter("drop-static");
    static STATIC_LAYOUT: PrefixLayout = PrefixLayout("static");
    static STATIC_DIAG: LazyLock<StaticDiagnostic> =
        LazyLock::new(|| StaticDiagnostic::new([("origin", "static")]));
    static STATIC_APPEND: LazyLock<RecordingAppender> =
        LazyLock::new(|| RecordingAppender::with_layout(&STATIC_LAYOUT));
    static STATIC_ERROR_SINK: StaticCountingErrorSink = StaticCountingErrorSink {
        calls: AtomicUsize::new(0),
    };

    let arc_filter: Arc<dyn Filter> = Arc::new(SubstringRejectFilter("drop-arc"));
    let arc_layout: Arc<dyn Layout> = Arc::new(PrefixLayout("arc"));
    let arc_diag = Arc::new(StaticDiagnostic::new([("tenant", "acme")]));
    let arc_append = Arc::new(RecordingAppender::with_layout(arc_layout));
    let arc_error_sink: Arc<dyn ErrorSink> = Arc::new(RecordingErrorSink::default());

    let logger = builder()
        .error_sink(&STATIC_ERROR_SINK)
        .dispatch(|d| {
            d.filter(arc_filter.clone())
                .diagnostic(arc_diag.clone())
                .append(arc_append.clone())
        })
        .dispatch(|d| {
            let dyn_diag: Arc<dyn Diagnostic> = arc_diag.clone();
            let dyn_append: Arc<dyn Append> = arc_append.clone();
            d.filter(arc_filter.clone())
                .diagnostic(dyn_diag)
                .error_sink(arc_error_sink.clone())
                .append(dyn_append)
        })
        .dispatch(|d| {
            d.filter(&STATIC_FILTER)
                .diagnostic(&*STATIC_DIAG)
                .append(&*STATIC_APPEND)
                .append(FailingAppender::new("static sink error"))
        })
        .build();

    for msg in ["keep", "drop-arc", "drop-static"] {
        logger.log(&Record::builder().payload(format_args!("{msg}")).build());
    }
    logger.flush();

    assert_eq!(
        arc_append.lines(),
        [
            "arc:keep tenant=acme",
            "arc:keep tenant=acme",
            "arc:drop-static tenant=acme",
            "arc:drop-static tenant=acme",
        ]
    );
    assert_eq!(arc_append.flush_count(), 2);

    assert_eq!(
        STATIC_APPEND.lines(),
        ["static:keep origin=static", "static:drop-arc origin=static"]
    );
    assert_eq!(STATIC_APPEND.flush_count(), 1);
    // 2 accepted records + 1 flush routed to STATIC_ERROR_SINK on branch 3.
    assert_eq!(STATIC_ERROR_SINK.calls.load(Ordering::Relaxed), 3);
}
