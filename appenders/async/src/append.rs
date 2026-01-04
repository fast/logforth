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
    state: AsyncState,
}

impl Append for Async {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut diagnostics = vec![];

        let mut collector = DiagnosticCollector(&mut diagnostics);
        for d in diags {
            d.visit(&mut collector)?;
        }

        let task = Task::Log {
            record: Box::new(record.to_owned()),
            diags: diagnostics,
        };
        self.state.send_task(task)
    }

    fn flush(&self) -> Result<(), Error> {
        let (done_tx, done_rx) = oneshot::channel();

        let task = Task::Flush { done: done_tx };
        self.state.send_task(task)?;

        match done_rx.recv() {
            Ok(None) => Ok(()),
            Ok(Some(err)) => Err(err),
            Err(err) => Err(Error::new("worker exited before completing flush").with_source(err)),
        }
    }
}

/// A builder for configuring an async appender.
pub struct AsyncBuilder {
    thread_name: String,
    appends: Vec<Box<dyn Append>>,
    buffered_lines_limit: Option<usize>,
    trap: Box<dyn Trap>,
    overflow: Overflow,
}

impl AsyncBuilder {
    /// Create a new async appender builder.
    pub fn new(thread_name: impl Into<String>) -> AsyncBuilder {
        AsyncBuilder {
            thread_name: thread_name.into(),
            appends: vec![],
            buffered_lines_limit: None,
            trap: Box::new(BestEffortTrap::default()),
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

        let (sender, receiver) = match buffered_lines_limit {
            Some(limit) => crossbeam_channel::bounded(limit),
            None => crossbeam_channel::unbounded(),
        };

        let worker = Worker::new(appends, receiver, trap);
        let thread_handle = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || worker.run())
            .expect("failed to spawn async appender thread");

        let state = AsyncState::new(overflow, sender, thread_handle);
        Async { state }
    }
}

struct DiagnosticCollector<'a>(&'a mut Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl<'a> Visitor for DiagnosticCollector<'a> {
    fn visit(&mut self, key: kv::Key, value: kv::Value) -> Result<(), Error> {
        self.0.push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}
