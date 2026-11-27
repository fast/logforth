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

use std::sync::mpsc;
use std::thread::JoinHandle;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::error::StderrErrorSink;
use logforth_core::kv;
use logforth_core::kv::Visitor;
use logforth_core::record::Record;

use crate::Overflow;
use crate::Task;
use crate::channel::Sender;
use crate::channel::channel;
use crate::worker::Worker;

/// A composable appender, logging and flushing asynchronously.
///
/// # Examples
///
/// ```
/// use logforth_append_async::AsyncBuilder;
/// use logforth_core::append::Stderr;
///
/// let async_append = AsyncBuilder::new("logforth-async-append")
///     .overflow_drop_incoming()
///     // for demonstration purposes; in practice, this can be a file appender, etc.
///     .append(Stderr::default())
///     .build();
/// ```
///
/// # Caveats
///
/// The caller or application should ensure that the `flush` method is called before the program
/// exits to write out any buffered events, especially when this appender is used in a global
/// context.
///
/// The drop glue will also flush the appender. But, in Rust, static items do not call `drop`
/// at the end of the program.
#[derive(Debug)]
pub struct Async {
    overflow: Overflow,
    sender: Option<Sender<Task>>,
    handle: Option<JoinHandle<()>>,
}

impl Async {
    fn new(overflow: Overflow, sender: Sender<Task>, handle: JoinHandle<()>) -> Self {
        Self {
            overflow,
            sender: Some(sender),
            handle: Some(handle),
        }
    }

    fn send_task(&self, task: Task) -> Result<(), Error> {
        let sender = self
            .sender
            .as_ref()
            .expect("sender is always present before drop");

        match self.overflow {
            Overflow::Block => sender.send(task).map_err(|err| {
                Error::new(match err.0 {
                    Task::Log { .. } => "failed to send log task to async appender",
                    Task::Flush { .. } => "failed to send flush task to async appender",
                })
            }),
            Overflow::DropIncoming => match sender.try_send(task) {
                Ok(()) => Ok(()),
                Err(mpsc::TrySendError::Full(_)) => Ok(()),
                Err(mpsc::TrySendError::Disconnected(task)) => Err(Error::new(match task {
                    Task::Log { .. } => "failed to send log task to async appender",
                    Task::Flush { .. } => "failed to send flush task to async appender",
                })),
            },
        }
    }
}

impl Append for Async {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut diagnostics = vec![];

        let mut collector = DiagnosticCollector(&mut diagnostics);
        diags.visit(&mut collector)?;

        let task = Task::Log {
            record: Box::new(record.to_owned()),
            diags: diagnostics,
        };
        self.send_task(task)
    }

    fn flush(&self) -> Result<(), Error> {
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let task = Task::Flush { done: done_tx };
        self.send_task(task)?;

        done_rx
            .recv()
            .map_err(|err| Error::new("worker exited before completing flush").with_source(err))
    }
}

impl Drop for Async {
    fn drop(&mut self) {
        // Drop our sender first so the worker drains pending tasks and exits its receive loop.
        drop(self.sender.take());

        if let Some(handle) = self.handle.take()
            && handle.thread().id() != std::thread::current().id()
        {
            let _ = handle.join();
        }
    }
}

/// A builder for configuring an async appender.
#[derive(Debug)]
pub struct AsyncBuilder {
    thread_name: String,
    appends: Vec<Box<dyn Append>>,
    buffered_lines_limit: Option<usize>,
    error_sink: Box<dyn ErrorSink>,
    overflow: Overflow,
}

impl AsyncBuilder {
    /// Create a new async appender builder.
    pub fn new(thread_name: impl Into<String>) -> AsyncBuilder {
        AsyncBuilder {
            thread_name: thread_name.into(),
            appends: vec![],
            buffered_lines_limit: None,
            error_sink: Box::new(StderrErrorSink::default()),
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

    /// Configure the [`ErrorSink`] used when worker appenders fail to emit or flush records.
    ///
    /// Defaults to [`StderrErrorSink`].
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = error_sink.into();
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
            error_sink,
            overflow,
        } = self;

        let (sender, receiver) = channel(buffered_lines_limit);
        let worker = Worker::new(appends, receiver, error_sink);
        let thread_handle = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || worker.run())
            .expect("failed to spawn async appender thread");

        Async::new(overflow, sender, thread_handle)
    }
}

struct DiagnosticCollector<'a>(&'a mut Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl<'a> Visitor for DiagnosticCollector<'a> {
    fn visit(&mut self, key: kv::KeyView, value: kv::ValueView) -> Result<(), Error> {
        self.0.push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}
