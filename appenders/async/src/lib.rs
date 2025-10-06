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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use arc_swap::ArcSwapOption;
use crossbeam_channel::{Receiver, Sender};
use logforth_core::Error;
use logforth_core::kv::Visitor;
use logforth_core::record::{Record, RecordOwned};
use logforth_core::{Append, kv};
use logforth_core::{Diagnostic, ErrorSink};
use std::sync::Arc;
use std::thread::JoinHandle;

/// A composable appender, logging and flushing asynchronously.
#[derive(Debug)]
pub struct Async {
    appends: Arc<[Box<dyn Append>]>,
    overflow_policy: OverflowPolicy,
    state: ArcSwapOption<AsyncState>,
}

#[derive(Debug)]
struct AsyncState {
    sender: Sender<Task>,
    thread_handle: JoinHandle<()>,
}

impl Append for Async {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut diagnostics = vec![];

        struct DiagCollector<'a>(&'a mut Vec<(kv::KeyOwned, kv::ValueOwned)>);

        impl<'a> Visitor for DiagCollector<'a> {
            fn visit(&mut self, key: kv::Key, value: kv::Value) -> Result<(), Error> {
                self.0.push((key.to_owned(), value.to_owned()));
                Ok(())
            }
        }

        let mut collector = DiagCollector(&mut diagnostics);
        for d in diags {
            d.visit(&mut collector)?;
        }

        self.send_task(Task::Log {
            appends: self.appends.clone(),
            record: record.to_owned(),
            diags: diagnostics,
        })
    }

    fn flush(&self) -> Result<(), Error> {
        self.send_task(Task::Flush {
            appends: self.appends.clone(),
        })
    }
}

impl Async {
    fn send_task(&self, task: Task) -> Result<(), Error> {
        let state = self.state.load();
        // SAFETY: state is always Some before dropped.
        let state = state.as_ref().unwrap();
        let sender = &state.sender;

        match self.overflow_policy {
            OverflowPolicy::Block => sender.send(task).map_err(|err| {
                Error::new(match err.0 {
                    Task::Log { .. } => "failed to send log task to async appender",
                    Task::Flush { .. } => "failed to send flush task to async appender",
                })
            }),
            OverflowPolicy::DropIncoming => match sender.try_send(task) {
                Ok(()) => Ok(()),
                Err(crossbeam_channel::TrySendError::Full(_)) => Ok(()),
                Err(crossbeam_channel::TrySendError::Disconnected(task)) => {
                    Err(Error::new(match task {
                        Task::Log { .. } => "failed to send log task to async appender",
                        Task::Flush { .. } => "failed to send flush task to async appender",
                    }))
                }
            },
        }
    }

    fn destroy(&self) {
        if let Some(state) = self.state.swap(None) {
            // SAFETY: state has always one strong count before swapped.
            let AsyncState {
                sender,
                thread_handle,
            } = Arc::into_inner(state).unwrap();

            drop(sender);
            thread_handle.join().unwrap();
        }
    }
}

impl Drop for Async {
    fn drop(&mut self) {
        self.destroy();
    }
}

///
pub struct AsyncBuilder {
    thread_name: String,
    appends: Vec<Box<dyn Append>>,
    buffered_lines_limit: Option<usize>,
    error_sink: Box<dyn ErrorSink>,
    overflow_policy: OverflowPolicy,
}

impl AsyncBuilder {
    /// Create a new async appender builder.
    pub fn new(thread_name: impl Into<String>) -> AsyncBuilder {
        AsyncBuilder {
            thread_name: thread_name.into(),
            appends: vec![],
            buffered_lines_limit: None,
            error_sink: Box::new(PrintErrorSink),
            overflow_policy: OverflowPolicy::Block,
        }
    }

    /// Set the buffer size of pending messages.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: Option<usize>) -> Self {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Set the overflow policy for this async appender.
    pub fn overflow_policy(mut self, overflow_policy: OverflowPolicy) -> Self {
        self.overflow_policy = overflow_policy;
        self
    }

    /// Set the error sink for this async appender.
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
            overflow_policy,
        } = self;

        let (sender, receiver) = match buffered_lines_limit {
            Some(limit) => crossbeam_channel::bounded(limit),
            None => crossbeam_channel::unbounded(),
        };

        let worker = Worker {
            receiver,
            error_sink,
        };

        let thread_handle = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || worker.run())
            .expect("failed to spawn async appender thread");

        Async {
            appends: appends.into_boxed_slice().into(),
            overflow_policy,
            state: ArcSwapOption::from(Some(Arc::new(AsyncState {
                sender,
                thread_handle,
            }))),
        }
    }
}

struct PrintErrorSink;

impl ErrorSink for PrintErrorSink {
    fn sink(&self, err: &Error) {
        eprintln!("{err}");
    }
}

struct Worker {
    receiver: Receiver<Task>,
    error_sink: Box<dyn ErrorSink>,
}

impl Worker {
    fn run(self) {
        let Self {
            receiver,
            error_sink,
        } = self;

        while let Ok(task) = receiver.recv() {
            match task {
                Task::Log {
                    appends,
                    record,
                    diags,
                } => {
                    let diags: Vec<Box<dyn Diagnostic>> = vec![Box::new(OwnedDiagnostic(diags))];
                    let diags = diags.as_slice();
                    let record = record.as_record();
                    for append in appends.iter() {
                        if let Err(err) = append.append(&record, diags) {
                            let err = Error::new("failed to append record").set_source(err);
                            error_sink.sink(&err);
                        }
                    }
                }
                Task::Flush { appends } => {
                    for append in appends.iter() {
                        if let Err(err) = append.flush() {
                            let err = Error::new("failed to flush").set_source(err);
                            error_sink.sink(&err);
                        }
                    }
                }
            }
        }
    }
}

/// Overflow policy for [`Async`].
///
/// When the channel is full, an incoming operation is handled according to the
/// specified policy.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum OverflowPolicy {
    /// Blocks until the channel is not full.
    Block,
    /// Drops the incoming operation.
    DropIncoming,
}

enum Task {
    Log {
        appends: Arc<[Box<dyn Append>]>,
        record: RecordOwned,
        diags: Vec<(kv::KeyOwned, kv::ValueOwned)>,
    },
    Flush {
        appends: Arc<[Box<dyn Append>]>,
    },
}

#[derive(Debug)]
struct OwnedDiagnostic(Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl Diagnostic for OwnedDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        for (key, value) in &self.0 {
            visitor.visit(key.by_ref(), value.by_ref())?;
        }
        Ok(())
    }
}
