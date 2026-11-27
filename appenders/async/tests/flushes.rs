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
use std::sync::Barrier;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use logforth_append_async::AsyncBuilder;
use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::record::Record;

#[derive(Debug)]
struct BarrierAppend {
    started: Arc<AtomicBool>,
    barrier: Arc<Barrier>,
}

impl Append for BarrierAppend {
    fn append(&self, _: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        self.started.store(true, Ordering::SeqCst);
        self.barrier.wait();
        Ok(())
    }
}

#[derive(Debug)]
struct FailingFlush;

impl Append for FailingFlush {
    fn append(&self, _: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Err(Error::new("flush failed"))
    }
}

#[test]
fn flush_waits_for_worker_completion() {
    let started = Arc::new(AtomicBool::new(false));
    let barrier = Arc::new(Barrier::new(2));

    let append = BarrierAppend {
        started: started.clone(),
        barrier: barrier.clone(),
    };

    let async_append = AsyncBuilder::new("async-flush-wait").append(append).build();

    let barrier_for_main = barrier.clone();
    let flush_handle = std::thread::spawn(move || async_append.flush());

    while !started.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }

    assert!(!flush_handle.is_finished());

    barrier_for_main.wait();

    flush_handle
        .join()
        .expect("flush thread panicked")
        .expect("flush should succeed");
}

#[derive(Debug)]
struct CapturedErrors {
    errors: mpsc::Sender<String>,
}

impl ErrorSink for CapturedErrors {
    fn sink(&self, err: &Error) {
        self.errors.send(format!("{err:#}")).unwrap();
    }
}

#[test]
fn flush_handles_errors_in_worker_thread() {
    let (tx, rx) = mpsc::channel();

    let async_append = AsyncBuilder::new("async-flush-error")
        .error_sink(CapturedErrors { errors: tx })
        .append(FailingFlush)
        .build();

    async_append.flush().unwrap();

    let err = rx.recv().unwrap();
    assert_eq!(err, "failed to flush appender: flush failed");
}

#[derive(Debug)]
struct BlockingFirstAppend {
    first_started: Arc<AtomicBool>,
    first_barrier: Arc<Barrier>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Append for BlockingFirstAppend {
    fn append(&self, record: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        if !self.first_started.swap(true, Ordering::SeqCst) {
            self.first_barrier.wait();
        }
        self.seen.lock().unwrap().push(record.payload().to_string());
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn flush_completes_when_drop_incoming_buffer_is_full() {
    let first_started = Arc::new(AtomicBool::new(false));
    let first_barrier = Arc::new(Barrier::new(2));
    let seen = Arc::new(Mutex::new(Vec::new()));

    let async_append = Arc::new(
        AsyncBuilder::new("async-drop-incoming-flush")
            .buffered_lines_limit(Some(1))
            .overflow_drop_incoming()
            .append(BlockingFirstAppend {
                first_started: first_started.clone(),
                first_barrier: first_barrier.clone(),
                seen: seen.clone(),
            })
            .build(),
    );

    let send = |msg: &'static str| {
        async_append
            .append(
                &Record::builder().payload(format_args!("{msg}")).build(),
                &(),
            )
            .unwrap();
    };

    // First record enters `BlockingFirstAppend::append`, freeing the 1-slot channel.
    send("first");
    while !first_started.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }

    // Second record fills the 1-slot channel; third record overflows and is dropped.
    send("second");
    send("dropped");

    let flush_append = async_append.clone();
    let flush_handle = std::thread::spawn(move || flush_append.flush());

    first_barrier.wait();

    flush_handle
        .join()
        .expect("flush thread panicked")
        .expect("flush should wait for queued records instead of being dropped");

    assert_eq!(*seen.lock().unwrap(), vec!["first", "second"]);
}
