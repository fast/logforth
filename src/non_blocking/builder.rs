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

use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Context;
use crossbeam_channel::bounded;
use crossbeam_channel::unbounded;
use crossbeam_channel::SendTimeoutError;
use crossbeam_channel::Sender;

use super::worker::Worker;
use super::Message;
use super::Writer;

/// A guard that flushes log records associated with a [`NonBlocking`] writer on drop.
///
/// Writing to a [`NonBlocking`] writer will **not** immediately write the log record to the
/// underlying output. Instead, the log record will be written by a dedicated logging thread at
/// some later point. To increase throughput, the non-blocking writer will flush to the underlying
/// output on a periodic basis rather than every time a log record is written. This means that if
/// the program terminates abruptly (such as through an uncaught `panic` or a `std::process::exit`),
/// some log records may not be written.
///
/// Since logs near a crash are often necessary for diagnosing the failure, `WorkerGuard` provides a
/// mechanism to ensure that _all_ buffered logs are flushed to their output. `WorkerGuard` should
/// be assigned in the `main` function or whatever the entrypoint of the program is. This will
/// ensure that the guard will be dropped during an unwinding or when `main` exits successfully.
#[derive(Debug)]
pub struct WorkerGuard {
    _guard: Option<JoinHandle<()>>,
    sender: Sender<Message>,
    shutdown: Sender<()>,
    shutdown_timeout: Duration,
}

impl WorkerGuard {
    fn new(
        handle: JoinHandle<()>,
        sender: Sender<Message>,
        shutdown: Sender<()>,
        shutdown_timeout: Option<Duration>,
    ) -> Self {
        const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(100);

        WorkerGuard {
            _guard: Some(handle),
            sender,
            shutdown,
            shutdown_timeout: shutdown_timeout.unwrap_or(DEFAULT_SHUTDOWN_TIMEOUT),
        }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        let shutdown_timeout = self.shutdown_timeout;
        match self
            .sender
            .send_timeout(Message::Shutdown, shutdown_timeout)
        {
            Ok(()) => {
                // Attempt to wait for `Worker` to flush all messages before dropping. This happens
                // when the `Worker` calls `recv()` on a zero-capacity channel. Use `send_timeout`
                // so that drop is not blocked indefinitely.
                let _ = self.shutdown.send_timeout((), shutdown_timeout);
            }
            Err(SendTimeoutError::Disconnected(_)) => (),
            Err(SendTimeoutError::Timeout(err)) => {
                eprintln!("failed to send shutdown signal to logging worker: {err:?}",)
            }
        }
    }
}

/// A non-blocking writer for files.
#[derive(Clone, Debug)]
pub struct NonBlocking<T: Writer + Send + 'static> {
    sender: Sender<Message>,
    marker: std::marker::PhantomData<T>,
}

impl<T: Writer + Send + 'static> NonBlocking<T> {
    fn create(
        writer: T,
        thread_name: String,
        buffered_lines_limit: Option<usize>,
        shutdown_timeout: Option<Duration>,
    ) -> (Self, WorkerGuard) {
        let (sender, receiver) = match buffered_lines_limit {
            Some(cap) => bounded(cap),
            None => unbounded(),
        };

        let (shutdown_sender, shutdown_receiver) = bounded(0);

        let worker = Worker::new(writer, receiver, shutdown_receiver);
        let worker_guard = WorkerGuard::new(
            worker.make_thread(thread_name),
            sender.clone(),
            shutdown_sender,
            shutdown_timeout,
        );

        let marker = std::marker::PhantomData;
        (Self { sender, marker }, worker_guard)
    }

    pub fn send(&self, record: Vec<u8>) -> anyhow::Result<()> {
        self.sender
            .send(Message::Record(record))
            .context("failed to send log message")
    }
}

/// A builder for configuring [`NonBlocking`].
#[derive(Debug)]
pub struct NonBlockingBuilder<T: Writer + Send + 'static> {
    thread_name: String,
    buffered_lines_limit: Option<usize>,
    shutdown_timeout: Option<Duration>,
    writer: T,
}

impl<T: Writer + Send + 'static> NonBlockingBuilder<T> {
    /// Creates a new [`NonBlockingBuilder`] with the specified writer.
    pub fn new(thread_name: impl Into<String>, writer: T) -> Self {
        Self {
            thread_name: thread_name.into(),
            buffered_lines_limit: None,
            shutdown_timeout: None,
            writer,
        }
    }

    /// Sets the buffer size of pending messages.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: Option<usize>) -> Self {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Sets the shutdown timeout before the worker guard dropped.
    pub fn shutdown_timeout(mut self, shutdown_timeout: Option<Duration>) -> Self {
        self.shutdown_timeout = shutdown_timeout;
        self
    }

    /// Completes the builder, returning the configured `NonBlocking`.
    pub fn build(self) -> (NonBlocking<T>, WorkerGuard) {
        NonBlocking::create(
            self.writer,
            self.thread_name,
            self.buffered_lines_limit,
            self.shutdown_timeout,
        )
    }
}
