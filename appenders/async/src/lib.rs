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

//! A composable appender, logging and flushing asynchronously.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::error::StderrErrorSink;
use logforth_core::kv;
use logforth_core::record::Record;
use logforth_core::record::RecordOwned;

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
    sender: TaskSender,
    handle: Option<JoinHandle<()>>,
}

impl Append for Async {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut entries = Vec::new();
        diags.visit(&mut |key: kv::KeyView, value: kv::ValueView| {
            entries.push((key.to_owned(), value.to_owned()));
            Ok(())
        })?;

        let task = Task::Log {
            record: Box::new(record.to_owned()),
            diags: OwnedDiagnostic(entries),
        };
        self.sender.send_log(task)
    }

    fn flush(&self) -> Result<(), Error> {
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        self.sender
            .send_blocking(Task::Flush { done: done_tx })
            .map_err(|_| Error::new("failed to send flush task to async appender"))?;

        done_rx
            .recv()
            .map_err(|err| Error::new("worker exited before completing flush").with_source(err))
    }
}

impl Drop for Async {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take()
            && handle.thread().id() != thread::current().id()
        {
            let _ = self.sender.send_blocking(Task::Shutdown);
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
            appends: Vec::new(),
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

        let (sender, receiver) = match (buffered_lines_limit, overflow) {
            (None, _) => {
                let (tx, rx) = mpsc::channel();
                (TaskSender::Unbounded(tx), rx)
            }
            (Some(limit), Overflow::Block) => {
                let (tx, rx) = mpsc::sync_channel(limit);
                (TaskSender::BoundedBlock(tx), rx)
            }
            (Some(limit), Overflow::DropIncoming) => {
                let (tx, rx) = mpsc::sync_channel(limit);
                (TaskSender::BoundedDrop(tx), rx)
            }
        };

        let handle = thread::Builder::new()
            .name(thread_name)
            .spawn(move || run_worker(receiver, appends, error_sink))
            .expect("failed to spawn async appender thread");

        Async {
            sender,
            handle: Some(handle),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Overflow {
    Block,
    DropIncoming,
}

#[derive(Debug)]
enum TaskSender {
    Unbounded(mpsc::Sender<Task>),
    BoundedBlock(mpsc::SyncSender<Task>),
    BoundedDrop(mpsc::SyncSender<Task>),
}

impl TaskSender {
    fn send_log(&self, task: Task) -> Result<(), Error> {
        let disconnected = || Error::new("failed to send log task to async appender");
        match self {
            Self::Unbounded(tx) => tx.send(task).map_err(|_| disconnected()),
            Self::BoundedBlock(tx) => tx.send(task).map_err(|_| disconnected()),
            Self::BoundedDrop(tx) => match tx.try_send(task) {
                Ok(()) | Err(mpsc::TrySendError::Full(_)) => Ok(()),
                Err(mpsc::TrySendError::Disconnected(_)) => Err(disconnected()),
            },
        }
    }

    fn send_blocking(&self, task: Task) -> Result<(), mpsc::SendError<Task>> {
        match self {
            Self::Unbounded(tx) => tx.send(task),
            Self::BoundedBlock(tx) | Self::BoundedDrop(tx) => tx.send(task),
        }
    }
}

#[derive(Debug)]
enum Task {
    Log {
        record: Box<RecordOwned>,
        diags: OwnedDiagnostic,
    },
    Flush {
        done: mpsc::SyncSender<()>,
    },
    Shutdown,
}

#[derive(Debug)]
struct OwnedDiagnostic(Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl Diagnostic for OwnedDiagnostic {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        for (key, value) in &self.0 {
            visitor.visit(key.view(), value.view())?;
        }
        Ok(())
    }
}

fn run_worker(
    receiver: mpsc::Receiver<Task>,
    appends: Vec<Box<dyn Append>>,
    error_sink: Box<dyn ErrorSink>,
) {
    while let Ok(task) = receiver.recv() {
        match task {
            Task::Log { record, diags } => {
                record.with(|record| {
                    for append in &appends {
                        if let Err(err) = catch_panic(|| append.append(&record, &diags)) {
                            let err = Error::new("failed to append record").with_source(err);
                            error_sink.sink(&err);
                        }
                    }
                });
            }
            Task::Flush { done } => {
                for append in &appends {
                    if let Err(err) = catch_panic(|| append.flush()) {
                        let err = Error::new("failed to flush appender").with_source(err);
                        error_sink.sink(&err);
                    }
                }
                let _ = done.send(());
            }
            Task::Shutdown => break,
        }
    }
}

fn catch_panic(f: impl FnOnce() -> Result<(), Error>) -> Result<(), Error> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => Err(panic_error(payload)),
    }
}

fn panic_error(payload: Box<dyn Any + Send>) -> Error {
    let msg = payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload");
    Error::new(format!("appender panicked: {msg}"))
}
