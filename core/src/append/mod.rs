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

//! Sinks that deliver log records to output targets.
//!
//! [`Append`] receives structured [`Record`] metadata, payload, and [`Diagnostic`] context directly
//! so protocol-native sinks can encode typed fields without intermediate text serialization, while
//! byte-stream sinks compose a [`Layout`](crate::Layout) internally and own record framing (such as
//! a trailing newline).
//!
//! `Arc<T>` and `&'static T` forward [`Append`] so multiple dispatch branches or [`Logger`](crate::Logger)
//! instances can share a single stateful appender.

use std::fmt;
use std::sync::Arc;

use crate::Diagnostic;
use crate::Error;
use crate::record::Record;

mod stdio;
pub use self::stdio::Stderr;
pub use self::stdio::Stdout;

mod testing;
pub use self::testing::Testing;

/// A sink that processes and delivers log records.
///
/// Implementations must remain thread-safe (`Send + Sync`) and must never route log records back
/// into their owning [`Logger`](crate::Logger).
pub trait Append: fmt::Debug + Send + Sync + 'static {
    /// Delivers `record` and its associated `diags` context to the target sink.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if formatting, encoding, or writing the record fails.
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error>;

    /// Flushes any buffered records to the underlying destination.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if flushing buffered output fails.
    fn flush(&self) -> Result<(), Error>;
}

impl<T: Append> From<T> for Box<dyn Append> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: Append + ?Sized> Append for Arc<T> {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        (**self).append(record, diags)
    }

    fn flush(&self) -> Result<(), Error> {
        (**self).flush()
    }
}

impl<T: Append + ?Sized> Append for &'static T {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        (**self).append(record, diags)
    }

    fn flush(&self) -> Result<(), Error> {
        (**self).flush()
    }
}
