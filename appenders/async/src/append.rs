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

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Trap;
use logforth_core::kv;
use logforth_core::kv::Visitor;
use logforth_core::record::Record;
use logforth_core::trap::BestEffortTrap;

use crate::Overflow;
use crate::Task;
use crate::state::AsyncState;
use crate::worker::Worker;

/// A composable appender, logging and flushing asynchronously.
#[derive(Debug)]
pub struct Async {
    appends: Arc<[Box<dyn Append>]>,
    overflow: Overflow,
    state: AsyncState,
    trap: Arc<dyn Trap>,
}

impl Append for Async {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut diagnostics = vec![];

        let mut collector = DiagnosticCollector(&mut diagnostics);
        for d in diags {
            d.visit(&mut collector)?;
        }

        let overflow = self.overflow;
        let task = Task::Log {
            appends: self.appends.clone(),
            record: Box::new(record.to_owned()),
            diags: diagnostics,
        };
        self.state.send_task(task, overflow)
    }

    fn flush(&self) -> Result<(), Error> {
        let (completion, done) = crossbeam_channel::bounded(0);
        let task = Task::Flush {
            appends: self.appends.clone(),
            completion,
        };
        self.state.send_task(task, Overflow::Block)?;
        match done.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(Error::new(
                "async appender worker exited before completing flush",
            )),
        }
    }

    fn exit(&self) -> Result<(), Error> {
        // If the program is tearing down, this will be the final flush. `crossbeam`
        // uses thread-local internally, which is not supported in `atexit` callback.
        // This can be bypassed by flushing sinks directly on the current thread, but
        // before we do that we have to join the thread to ensure that any pending log
        // tasks are completed.
        //
        // @see https://github.com/SpriteOvO/spdlog-rs/issues/64
        self.state.destroy();
        for append in self.appends.iter() {
            if let Err(err) = append.exit() {
                self.trap.trap(&err);
            }
        }
        Ok(())
    }
}

/// A builder for configuring an async appender.
pub struct AsyncBuilder {
    thread_name: String,
    appends: Vec<Box<dyn Append>>,
    buffered_lines_limit: Option<usize>,
    trap: Arc<dyn Trap>,
    overflow: Overflow,
}

impl AsyncBuilder {
    /// Create a new async appender builder.
    pub fn new(thread_name: impl Into<String>) -> AsyncBuilder {
        AsyncBuilder {
            thread_name: thread_name.into(),
            appends: vec![],
            buffered_lines_limit: None,
            trap: Arc::new(BestEffortTrap::default()),
            overflow: Overflow::Block,
        }
    }

    /// Set the buffer size of pending messages.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: Option<usize>) -> Self {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Set the overflow policy to block when the buffer is full.
    pub fn overflow_block(mut self) -> Self {
        self.overflow = Overflow::Block;
        self
    }

    /// Set the overflow policy to drop incoming messages when the buffer is full.
    pub fn overflow_drop_incoming(mut self) -> Self {
        self.overflow = Overflow::DropIncoming;
        self
    }

    /// Set the trap for this async appender.
    pub fn trap(mut self, trap: impl Into<Box<dyn Trap>>) -> Self {
        let trap = trap.into();
        self.trap = trap.into();
        self
    }

    /// Add an appender to this async appender.
    pub fn append(mut self, append: impl Into<Box<dyn Append>>) -> Self {
        self.appends.push(append.into());
        self
    }

    /// Build the async appender.
    pub fn build(self) -> Async {
        let Self {
            thread_name,
            appends,
            buffered_lines_limit,
            trap,
            overflow,
        } = self;

        let appends = appends.into_boxed_slice().into();

        let (sender, receiver) = match buffered_lines_limit {
            Some(limit) => crossbeam_channel::bounded(limit),
            None => crossbeam_channel::unbounded(),
        };

        let worker = Worker::new(receiver, trap.clone());
        let thread_handle = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || worker.run())
            .expect("failed to spawn async appender thread");
        let state = AsyncState::new(sender, thread_handle);

        Async {
            appends,
            overflow,
            state,
            trap,
        }
    }
}

struct DiagnosticCollector<'a>(&'a mut Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl<'a> Visitor for DiagnosticCollector<'a> {
    fn visit(&mut self, key: kv::Key, value: kv::Value) -> Result<(), Error> {
        self.0.push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logforth_core::Trap;
    use logforth_core::record::Record;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};

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

        let err = async_append.flush().expect_err("flush should fail");
        let err_text = err.to_string();
        assert!(err_text.contains("failed to flush"));
        assert!(err_text.contains("flush failed"));
    }
}
