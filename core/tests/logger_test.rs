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
use logforth_core::Logger;
use logforth_core::builder;
use logforth_core::error::BlackHoleErrorSink;
use logforth_core::record::FilterCriteria;
use logforth_core::record::Level;
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
    let criteria = FilterCriteria::builder().level(Level::Fatal4).build();
    let record = Record::builder().payload(format_args!("ignored")).build();

    for logger in [NOOP_LOGGER, Logger::default(), empty_built] {
        let cloned = logger.clone();
        assert!(!cloned.enabled(&criteria));
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
            d.error_sink(BlackHoleErrorSink::new())
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
