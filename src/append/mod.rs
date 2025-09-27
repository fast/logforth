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

//! Dispatch log records to various targets.

use std::fmt;

use crate::Diagnostic;
use crate::Error;
use crate::record::Record;

#[cfg(feature = "append-fastrace")]
mod fastrace;
pub mod file;
#[cfg(all(unix, feature = "append-journald"))]
mod journald;
#[cfg(feature = "append-opentelemetry")]
pub mod opentelemetry;
mod stdio;
#[cfg(feature = "append-syslog")]
pub mod syslog;
mod testing;

#[cfg(feature = "append-fastrace")]
pub use self::fastrace::FastraceEvent;
pub use self::file::File;
#[cfg(all(unix, feature = "append-journald"))]
pub use self::journald::Journald;
#[cfg(feature = "append-opentelemetry")]
pub use self::opentelemetry::OpentelemetryLog;
pub use self::stdio::Stderr;
pub use self::stdio::Stdout;
#[cfg(feature = "append-syslog")]
pub use self::syslog::Syslog;
pub use self::testing::Testing;

/// A trait representing an appender that can process log records.
pub trait Append: fmt::Debug + Send + Sync + 'static {
    /// Dispatches a log record to the append target.
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error>;

    /// Flushes any buffered records.
    ///
    /// Default to a no-op.
    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl<T: Append> From<T> for Box<dyn Append> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
