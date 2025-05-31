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

#[cfg(feature = "fastrace")]
mod fastrace;
#[cfg(all(unix, feature = "journald"))]
mod journald;
#[cfg(feature = "opentelemetry")]
pub mod opentelemetry;
#[cfg(feature = "rolling-file")]
pub mod rolling_file;
#[cfg(feature = "single-file")]
pub mod single_file;
mod stdio;
#[cfg(feature = "syslog")]
pub mod syslog;

#[cfg(feature = "fastrace")]
pub use self::fastrace::FastraceEvent;
#[cfg(all(unix, feature = "journald"))]
pub use self::journald::Journald;
#[cfg(feature = "opentelemetry")]
pub use self::opentelemetry::OpentelemetryLog;
#[cfg(feature = "rolling-file")]
pub use self::rolling_file::RollingFile;
#[cfg(feature = "single-file")]
pub use self::single_file::SingleFile;
pub use self::stdio::Stderr;
pub use self::stdio::Stdout;
#[cfg(feature = "syslog")]
pub use self::syslog::Syslog;

/// A trait representing an appender that can process log records.
pub trait Append: fmt::Debug + Send + Sync + 'static {
    /// Dispatches a log record to the append target.
    fn append(
        &self,
        record: &log::Record,
        diagnostics: &[Box<dyn Diagnostic>],
    ) -> anyhow::Result<()>;

    /// Flushes any buffered records.
    ///
    /// Default to a no-op.
    fn flush(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<T: Append> From<T> for Box<dyn Append> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
