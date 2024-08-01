use anyhow::Context;
use crossbeam_channel::{bounded, unbounded, SendTimeoutError, Sender};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

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
    error_counter: ErrorCounter,
    channel: Sender<Message>,
    is_lossy: bool,
}

impl NonBlocking {
    pub fn new<T: Write + Send + 'static>(writer: T) -> (NonBlocking, WorkerGuard) {
        NonBlockingBuilder::default().finish(writer)
    }

    fn create<T: Write + Send + 'static>(
        writer: T,
        buffered_lines_limit: Option<usize>,
        is_lossy: bool,
        thread_name: String,
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
            None,
        );

        (
            Self {
                channel: sender,
                error_counter: ErrorCounter(Arc::new(AtomicUsize::new(0))),
                is_lossy,
            },
            worker_guard,
        )
    }

    /// Returns the number of times logs where dropped. This will always return zero if
    /// `NonBlocking` is not lossy.
    pub fn drop_lines(&self) -> usize {
        self.error_counter.dropped_lines()
    }

    fn send(&self, buf: Vec<u8>) -> anyhow::Result<()> {
        if !self.is_lossy {
            return self
                .channel
                .send(Message::Record(buf))
                .context("failed to send log message");
        }

        if self.channel.try_send(Message::Record(buf)).is_err() {
            self.error_counter.drop_line();
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct NonBlockingBuilder {
    is_lossy: bool,
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

    /// Sets whether `NonBlocking` should be lossy or not.
    ///
    /// If set to `true`, logs will be dropped when the buffered limit is reached. If `false`, backpressure
    /// will be exerted on senders, blocking them until the buffer has capacity again.
    ///
    /// By default, the built `NonBlocking` will be lossy.
    pub fn lossy(mut self, is_lossy: bool) -> NonBlockingBuilder {
        self.is_lossy = is_lossy;
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
            self.buffered_lines_limit,
            self.is_lossy,
            self.thread_name,
        )
    }
}

impl Default for NonBlockingBuilder {
    fn default() -> Self {
        NonBlockingBuilder {
            is_lossy: true,
            thread_name: "logforth-rolling-file".to_string(),
            buffered_lines_limit: None,
            shutdown_timeout: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ErrorCounter(Arc<AtomicUsize>);

impl ErrorCounter {
    pub fn dropped_lines(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }

    fn drop_line(&self) {
        let mut cnt = self.0.load(Ordering::Acquire);

        if cnt == usize::MAX {
            return;
        }

        loop {
            let next = cnt.saturating_add(1);
            match self
                .0
                .compare_exchange(cnt, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return,
                Err(now) => cnt = now,
            }
        }
    }
}
