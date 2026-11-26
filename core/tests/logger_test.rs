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
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::Filter;
use logforth_core::Logger;
use logforth_core::builder;
use logforth_core::error::BlackHoleErrorSink;
use logforth_core::filter::FilterResult;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;

const NOOP_LOGGER: Logger = Logger::noop();

const _: () = {
    assert!(size_of::<Logger>() == size_of::<usize>());
    const fn assert_handle_bounds<T: Clone + Default + Send + Sync + 'static>() {}
    assert_handle_bounds::<Logger>();
};

#[derive(Debug)]
struct FailingAppender(&'static str);

impl Append for FailingAppender {
    fn append(&self, _: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        Err(Error::new(self.0))
    }

    fn flush(&self) -> Result<(), Error> {
        Err(Error::new(self.0))
    }
}

#[derive(Debug, Default)]
struct RecordingAppender {
    records: Mutex<Vec<String>>,
    flushes: AtomicUsize,
}

impl Append for RecordingAppender {
    fn append(&self, record: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        self.records
            .lock()
            .unwrap()
            .push(record.payload().to_string());
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RecordingErrorSink {
    errors: Mutex<Vec<String>>,
}

impl ErrorSink for RecordingErrorSink {
    fn sink(&self, err: &Error) {
        self.errors.lock().unwrap().push(format!("{err:#}"));
    }
}

#[test]
fn noop_and_empty_builder_discard_records_and_report_disabled() {
    let errors = Arc::new(RecordingErrorSink::default());
    let empty_built = builder().error_sink(errors.clone()).build();
    let record = Record::builder().payload(format_args!("ignored")).build();

    for logger in [NOOP_LOGGER, Logger::default(), empty_built] {
        let cloned = logger.clone();
        assert!(!cloned.enabled(Level::Fatal4));
        cloned.log(&record);
        cloned.flush();
    }

    assert!(errors.errors.lock().unwrap().is_empty());
}

#[test]
fn cloned_logger_handles_share_pipeline_across_threads() {
    let sink = Arc::new(RecordingAppender::default());
    let logger = builder().dispatch(|d| d.append(sink.clone())).build();

    let worker_logger = logger.clone();
    std::thread::spawn(move || {
        worker_logger.log(
            &Record::builder()
                .payload(format_args!("from-thread"))
                .build(),
        );
    })
    .join()
    .unwrap();

    logger.log(&Record::builder().payload(format_args!("from-main")).build());
    logger.clone().flush();

    assert_eq!(
        &*sink.records.lock().unwrap(),
        &["from-thread", "from-main"]
    );
    assert_eq!(sink.flushes.load(Ordering::Relaxed), 1);
}

#[test]
fn logger_routes_errors_with_dispatch_fallback_and_override() {
    let logger_errors = Arc::new(RecordingErrorSink::default());
    let override_errors = Arc::new(RecordingErrorSink::default());
    let delivered = Arc::new(RecordingAppender::default());

    let logger = builder()
        .error_sink(logger_errors.clone())
        .dispatch(|d| {
            d.append(FailingAppender("primary failed"))
                .append(delivered.clone())
        })
        .dispatch(|d| {
            d.error_sink(BlackHoleErrorSink::default())
                .append(FailingAppender("silenced failed"))
        })
        .dispatch(|d| {
            d.error_sink(override_errors.clone())
                .append(FailingAppender("override failed"))
        })
        .build();

    let record = Record::builder().payload(format_args!("hello")).build();
    logger.log(&record);
    logger.flush();

    assert_eq!(&*delivered.records.lock().unwrap(), &["hello"]);
    assert_eq!(
        &*logger_errors.errors.lock().unwrap(),
        &[
            "failed to append record: primary failed",
            "failed to flush appender: primary failed",
        ]
    );
    assert_eq!(
        &*override_errors.errors.lock().unwrap(),
        &[
            "failed to append record: override failed",
            "failed to flush appender: override failed",
        ]
    );
}

#[test]
fn dispatch_builder_supports_dynamic_appenders_and_skips_empty_branches() {
    let first = Arc::new(RecordingAppender::default());
    let second = Arc::new(RecordingAppender::default());

    let logger = builder()
        .dispatch(|d| d.filter(LevelFilter::All))
        .dispatch(|mut d| {
            d = d.filter(LevelFilter::MoreSevereEqual(Level::Error));
            for sink in [&first, &second] {
                d = d.append(sink.clone());
            }
            d
        })
        .build();

    assert!(!logger.enabled(Level::Info));
    assert!(logger.enabled(Level::Error));

    logger.log(
        &Record::builder()
            .level(Level::Error)
            .payload(format_args!("boom"))
            .build(),
    );

    assert_eq!(&*first.records.lock().unwrap(), &["boom"]);
    assert_eq!(&*second.records.lock().unwrap(), &["boom"]);

    let all_empty = builder().dispatch(|d| d.filter(LevelFilter::All)).build();
    assert!(!all_empty.enabled(Level::Fatal4));
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

    fn filter(&self, record: &Record, _: &dyn Diagnostic) -> FilterResult {
        if record
            .key_values()
            .get("sampled")
            .and_then(|v| v.to_bool())
            .unwrap_or(false)
        {
            FilterResult::Accept
        } else {
            FilterResult::Neutral
        }
    }
}

#[derive(Debug)]
struct NonCopyNotWarn;

impl Filter for NonCopyNotWarn {
    fn filter_level(&self, level: Level) -> FilterResult {
        LevelFilter::NotEqual(Level::Warn).filter_level(level)
    }

    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult {
        LevelFilter::NotEqual(Level::Warn).filter(record, diags)
    }
}

#[test]
fn build_time_level_pruning_respects_promotion_and_retains_downstream_thresholds() {
    static STATIC_NOT_WARN: NonCopyNotWarn = NonCopyNotWarn;

    let narrowed_sink = Arc::new(RecordingAppender::default());
    let promoted_sink = Arc::new(RecordingAppender::default());
    let arc_threshold = Arc::new(LevelFilter::MoreSevereEqual(Level::Info));

    let logger = builder()
        .dispatch(|d| {
            d.filter(arc_threshold.clone())
                .filter(&STATIC_NOT_WARN)
                .append(narrowed_sink.clone())
        })
        .dispatch(|d| {
            d.filter(SampledDebugPromoter)
                .filter(LevelFilter::MoreSevereEqual(Level::Error))
                .append(promoted_sink.clone())
        })
        .build();

    // Branch 1 enables Info..=Fatal4 except Warn; Branch 2 enables Debug | Error..=Fatal4.
    assert!(!logger.enabled(Level::Trace));
    assert!(!logger.enabled(Level::Debug2));
    assert!(logger.enabled(Level::Debug));
    assert!(logger.enabled(Level::Info));
    assert!(!logger.enabled(Level::Warn));
    assert!(logger.enabled(Level::Error));

    // Unsampled Debug passes Branch 2's static mask (due to SampledDebugPromoter) and returns
    // Neutral, then gets rejected by the retained downstream `LevelFilter(Error)`.
    logger.log(
        &Record::builder()
            .level(Level::Debug)
            .payload(format_args!("unsampled-debug"))
            .build(),
    );

    // Sampled Debug returns Accept from SampledDebugPromoter, short-circuiting `Error`.
    let sampled_kv = [(Key::new("sampled"), Value::bool(true))];
    logger.log(
        &Record::builder()
            .level(Level::Debug)
            .key_values(sampled_kv.as_slice())
            .payload(format_args!("sampled-debug"))
            .build(),
    );

    // Warn is statically pruned by both dispatches.
    logger.log(
        &Record::builder()
            .level(Level::Warn)
            .payload(format_args!("dropped-warn"))
            .build(),
    );

    // Info lands only in Branch 1; Error lands in both.
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("only-narrowed"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Error)
            .payload(format_args!("both-error"))
            .build(),
    );

    assert_eq!(
        &*narrowed_sink.records.lock().unwrap(),
        &["only-narrowed", "both-error"]
    );
    assert_eq!(
        &*promoted_sink.records.lock().unwrap(),
        &["sampled-debug", "both-error"]
    );
}
