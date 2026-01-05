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
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use logforth_append_async::AsyncBuilder;
use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Trap;
use logforth_core::record::Record;

#[derive(Debug)]
struct BarrierAppend {
    started: Arc<AtomicBool>,
    barrier: Arc<Barrier>,
}

impl Append for BarrierAppend {
    fn append(&self, _: &Record, _: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
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
    fn append(&self, _: &Record, _: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Err(Error::new("flush failed"))
    }
}

#[derive(Debug)]
struct NoopTrap;

impl Trap for NoopTrap {
    fn trap(&self, _: &Error) {}
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

#[test]
fn flush_propagates_errors() {
    let async_append = AsyncBuilder::new("async-flush-error")
        .trap(NoopTrap)
        .append(FailingFlush)
        .build();

    let err = async_append.flush().unwrap_err();
    let err = err.to_string();
    assert!(err.contains("failed to flush"));
    assert!(err.contains("flush failed"));
}
