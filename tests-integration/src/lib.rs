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

//! Shared integration test utilities for Logforth.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::Layout;
use logforth_core::kv::KeyView;
use logforth_core::kv::ValueView;
use logforth_core::layout::PlainTextLayout;
use logforth_core::record::Record;
use logforth_core::record::RecordOwned;

/// A captured log event stored by [`RecordingAppender`].
#[derive(Debug, Clone)]
pub struct CapturedEvent {
    /// Owned snapshot of the emitted record.
    pub record: RecordOwned,
    /// `Record::payload_static()` observed on the synchronous dispatch path.
    pub payload_static: Option<&'static str>,
    /// Formatted bytes produced by the configured [`Layout`].
    pub formatted: Vec<u8>,
    /// Diagnostic entries `(key, value.to_string())` observed during `append`.
    pub diagnostics: Vec<(String, String)>,
}

#[derive(Debug)]
struct RecordingState {
    layout: Box<dyn Layout>,
    events: Mutex<Vec<CapturedEvent>>,
    flushes: AtomicUsize,
}

/// A thread-safe in-memory [`Append`] test double that records events and flush calls.
#[derive(Debug, Clone)]
pub struct RecordingAppender {
    state: Arc<RecordingState>,
}

impl Default for RecordingAppender {
    fn default() -> Self {
        Self::with_layout(PlainTextLayout::default())
    }
}

impl RecordingAppender {
    /// Creates a new [`RecordingAppender`] using `layout` to format incoming records.
    pub fn with_layout(layout: impl Into<Box<dyn Layout>>) -> Self {
        Self {
            state: Arc::new(RecordingState {
                layout: layout.into(),
                events: Mutex::new(Vec::new()),
                flushes: AtomicUsize::new(0),
            }),
        }
    }

    /// Returns a snapshot of all captured events in arrival order.
    pub fn events(&self) -> Vec<CapturedEvent> {
        self.state.events.lock().unwrap().clone()
    }

    /// Returns the rendered payload strings of all captured events.
    pub fn payloads(&self) -> Vec<String> {
        self.state
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|event| event.record.with(|r| r.payload().to_string()))
            .collect()
    }

    /// Returns the UTF-8 formatted lines produced by the configured [`Layout`].
    pub fn lines(&self) -> Vec<String> {
        self.state
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|event| {
                std::str::from_utf8(&event.formatted)
                    .expect("formatted output must be valid UTF-8")
                    .to_owned()
            })
            .collect()
    }

    /// Returns the number of `flush` calls observed.
    pub fn flush_count(&self) -> usize {
        self.state.flushes.load(Ordering::Acquire)
    }
}

impl Append for RecordingAppender {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut formatted = Vec::new();
        self.state.layout.format(record, diags, &mut formatted)?;
        let diagnostics = collect_diagnostics(diags)?;
        self.state.events.lock().unwrap().push(CapturedEvent {
            record: record.to_owned(),
            payload_static: record.payload_static(),
            formatted,
            diagnostics,
        });
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        self.state.flushes.fetch_add(1, Ordering::Release);
        Ok(())
    }
}

/// An [`Append`] test double that always fails with a fixed message.
#[derive(Debug, Clone, Copy)]
pub struct FailingAppender {
    message: &'static str,
}

impl FailingAppender {
    /// Creates a new [`FailingAppender`] that returns `Error::new(message)` on `append` and
    /// `flush`.
    pub const fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Append for FailingAppender {
    fn append(&self, _: &Record, _: &dyn Diagnostic) -> Result<(), Error> {
        Err(Error::new(self.message))
    }

    fn flush(&self) -> Result<(), Error> {
        Err(Error::new(self.message))
    }
}

/// A thread-safe [`ErrorSink`] test double that records alternate-formatted error strings.
#[derive(Debug, Clone, Default)]
pub struct RecordingErrorSink {
    errors: Arc<Mutex<Vec<String>>>,
}

impl RecordingErrorSink {
    /// Returns all recorded `{err:#}` error strings in arrival order.
    pub fn errors(&self) -> Vec<String> {
        self.errors.lock().unwrap().clone()
    }
}

impl ErrorSink for RecordingErrorSink {
    fn sink(&self, err: &Error) {
        self.errors.lock().unwrap().push(format!("{err:#}"));
    }
}

/// Collects all entries from `diags` as `(key, value.to_string())` pairs.
pub fn collect_diagnostics(diags: &dyn Diagnostic) -> Result<Vec<(String, String)>, Error> {
    let mut entries = Vec::new();
    diags.visit(&mut |key: KeyView, value: ValueView| {
        entries.push((key.as_str().to_owned(), value.to_string()));
        Ok(())
    })?;
    Ok(entries)
}
