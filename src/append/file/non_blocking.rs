// Copyright 2024 tison <wander4096@gmail.com>
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

use std::io::Write;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Context;
use crossbeam_channel::bounded;
use crossbeam_channel::unbounded;
use crossbeam_channel::SendTimeoutError;
use crossbeam_channel::Sender;

use crate::append::file::worker::Worker;
use crate::append::file::Message;

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

#[derive(Clone, Debug)]
pub struct NonBlocking {
    sender: Sender<Message>,
}

impl NonBlocking {
    fn create<T: Write + Send + 'static>(
        writer: T,
        thread_name: String,
        buffered_lines_limit: Option<usize>,
        shutdown_timeout: Option<Duration>,
    ) -> (NonBlocking, WorkerGuard) {
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

        (Self { sender }, worker_guard)
    }

    pub(super) fn send(&self, record: Vec<u8>) -> anyhow::Result<()> {
        // TODO(tisonkun): consider drop the message if the channel is full
        self.sender
            .send(Message::Record(record))
            .context("failed to send log message")
    }
}

#[derive(Debug)]
pub struct NonBlockingBuilder {
    thread_name: String,
    buffered_lines_limit: Option<usize>,
    shutdown_timeout: Option<Duration>,
}

impl NonBlockingBuilder {
    /// Sets the number of lines to buffer before dropping logs or exerting backpressure on senders.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> NonBlockingBuilder {
        self.buffered_lines_limit = Some(buffered_lines_limit);
        self
    }

    /// Sets the shutdown timeout before the worker guard dropped.
    pub fn shutdown_timeout(mut self, shutdown_timeout: Duration) -> NonBlockingBuilder {
        self.shutdown_timeout = Some(shutdown_timeout);
        self
    }

    /// Override the worker thread's name.
    ///
    /// The default worker thread name is "tracing-appender".
    pub fn thread_name(mut self, name: impl Into<String>) -> NonBlockingBuilder {
        self.thread_name = name.into();
        self
    }

    /// Completes the builder, returning the configured `NonBlocking`.
    pub fn finish<T: Write + Send + 'static>(self, writer: T) -> (NonBlocking, WorkerGuard) {
        NonBlocking::create(
            writer,
            self.thread_name,
            self.buffered_lines_limit,
            self.shutdown_timeout,
        )
    }
}

impl Default for NonBlockingBuilder {
    fn default() -> Self {
        NonBlockingBuilder {
            thread_name: "logforth-rolling-file".to_string(),
            buffered_lines_limit: None,
            shutdown_timeout: None,
        }
    }
}
