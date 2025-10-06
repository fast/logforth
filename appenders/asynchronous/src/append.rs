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
use logforth_core::ErrorSink;
use logforth_core::kv;
use logforth_core::kv::Visitor;
use logforth_core::record::Record;

use crate::Task;
use crate::state::AppendState;
use crate::worker::Worker;

/// A composable appender, logging and flushing asynchronously.
#[derive(Debug)]
pub struct Asynchronous {
    appends: Arc<[Box<dyn Append>]>,
    overflow: Overflow,
    state: AppendState,
}

impl Append for Asynchronous {
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
        let overflow = self.overflow;
        let task = Task::Flush {
            appends: self.appends.clone(),
        };
        self.state.send_task(task, overflow)
    }
}

/// A builder for configuring an asynchronous appender.
pub struct AsyncBuilder {
    thread_name: String,
    appends: Vec<Box<dyn Append>>,
    buffered_lines_limit: Option<usize>,
    error_sink: Box<dyn ErrorSink>,
    overflow: Overflow,
}

impl AsyncBuilder {
    /// Create a new asynchronous appender builder.
    pub fn new(thread_name: impl Into<String>) -> AsyncBuilder {
        AsyncBuilder {
            thread_name: thread_name.into(),
            appends: vec![],
            buffered_lines_limit: None,
            error_sink: Box::new(PrintErrorSink),
            overflow: Overflow::Block,
        }
    }

    /// Set the buffer size of pending messages.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: Option<usize>) -> Self {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Set the overflow policy for this asynchronous appender.
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Set the error sink for this asynchronous appender.
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = error_sink.into();
        self
    }

    /// Add an appender to this asynchronous appender.
    pub fn append(mut self, append: impl Into<Box<dyn Append>>) -> Self {
        self.appends.push(append.into());
        self
    }

    /// Build the asynchronous appender.
    pub fn build(self) -> Asynchronous {
        let Self {
            thread_name,
            appends,
            buffered_lines_limit,
            error_sink,
            overflow,
        } = self;

        let appends = appends.into_boxed_slice().into();

        let (sender, receiver) = match buffered_lines_limit {
            Some(limit) => crossbeam_channel::bounded(limit),
            None => crossbeam_channel::unbounded(),
        };

        let worker = Worker::new(receiver, error_sink);
        let thread_handle = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || worker.run())
            .expect("failed to spawn asynchronous appender thread");
        let state = AppendState::new(sender, thread_handle);

        Asynchronous {
            appends,
            overflow,
            state,
        }
    }
}

/// Overflow policy for [`Asynchronous`].
///
/// When the channel is full, an incoming operation is handled according to the
/// specified policy.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum Overflow {
    /// Blocks until the channel is not full.
    Block,
    /// Drops the incoming operation.
    DropIncoming,
}

struct PrintErrorSink;

impl ErrorSink for PrintErrorSink {
    fn sink(&self, err: &Error) {
        eprintln!("{err}");
    }
}

struct DiagnosticCollector<'a>(&'a mut Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl<'a> Visitor for DiagnosticCollector<'a> {
    fn visit(&mut self, key: kv::Key, value: kv::Value) -> Result<(), Error> {
        self.0.push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}
