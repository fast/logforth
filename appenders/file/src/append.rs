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

use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::MutexGuard;

use jiff::tz::TimeZone;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Layout;
use logforth_core::append::Append;
use logforth_core::layout::PlainTextLayout;
use logforth_core::record::Record;

use crate::rolling::RollingFileWriter;
use crate::rolling::RollingFileWriterBuilder;
use crate::rotation::Rotation;

/// Builder for creating [`File`] appenders.
///
/// Construct a builder with [`FileBuilder::new`], optionally configure layout, suffix, rotation,
/// and retention settings, and finalize with [`build`](Self::build).
#[derive(Debug)]
pub struct FileBuilder {
    builder: RollingFileWriterBuilder,
    layout: Box<dyn Layout>,
}

impl FileBuilder {
    /// Creates a builder for log files named `filename` in `basedir`.
    ///
    /// Rotation is disabled by default. `filename` is used as a literal stem; pass an extension
    /// via [`filename_suffix`](Self::filename_suffix) so period and sequence fields appear before
    /// the extension (for example, `FileBuilder::new(dir, "my_app").filename_suffix("log")`).
    pub fn new(basedir: impl Into<PathBuf>, filename: impl Into<String>) -> Self {
        Self {
            builder: RollingFileWriterBuilder::new(basedir, filename),
            layout: Box::new(PlainTextLayout::default()),
        }
    }

    /// Sets the layout used to format each log record before appending a trailing newline
    /// (default: [`PlainTextLayout`]).
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Sets the filename suffix appended after any period and sequence fields.
    ///
    /// At most one leading `.` is stripped, and an empty string clears a previously configured
    /// suffix.
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.builder = self.builder.filename_suffix(suffix);
        self
    }

    /// Configures time-based rotation at the start of each minute in the configured
    /// [`timezone`](Self::timezone).
    pub fn rollover_minutely(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Minutely);
        self
    }

    /// Configures time-based rotation at the start of each hour in the configured
    /// [`timezone`](Self::timezone).
    pub fn rollover_hourly(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Hourly);
        self
    }

    /// Configures time-based rotation at the start of each calendar day (`00:00`) in the
    /// configured [`timezone`](Self::timezone).
    pub fn rollover_daily(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Daily);
        self
    }

    /// Sets the timezone used for time-based rotation boundaries and period filenames
    /// (default: [`TimeZone::system()`]).
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.builder = self.builder.timezone(tz);
        self
    }

    /// Configures size-based rotation before a formatted record (including its trailing newline)
    /// would cause the active segment to exceed `n` bytes.
    ///
    /// A single record larger than `n` bytes is written whole to an empty segment without rotating
    /// again beforehand. Segments are numbered in ascending order starting from `1` within each
    /// rotation period.
    pub fn rollover_size(mut self, n: NonZeroU64) -> Self {
        self.builder = self.builder.max_file_size(n);
        self
    }

    /// Sets the maximum number of matching log files retained across all periods, including the
    /// active segment.
    ///
    /// By default, all segments are retained. With rotation enabled, excess inactive segments are
    /// removed on a best-effort basis during [`build`](Self::build) and pruned on each rollover;
    /// the active segment is never removed.
    pub fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.builder = self.builder.max_log_files(n);
        self
    }

    /// Validates the builder configuration, creates `basedir` if needed, and opens the initial log
    /// file.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if:
    /// * `filename` is empty, `.`, `..`, or contains `/`, `\`, or `\0`.
    /// * `filename_suffix` (after stripping at most one leading `.`) is `.`, `..`, or contains `/`,
    ///   `\`, or `\0`.
    /// * The initial rotation period cannot be represented in `jiff`.
    /// * Creating `basedir`, scanning existing segments, or opening the initial segment file fails.
    pub fn build(self) -> Result<File, Error> {
        let FileBuilder { builder, layout } = self;
        let writer = builder.build()?;
        Ok(File::new(writer, layout))
    }
}

/// A file appender that writes formatted log records to a single file or rotating segment files.
///
/// Each record is formatted with the configured [`Layout`], terminated with `\n`, and written to
/// an append-mode file. See the [crate documentation](crate) for segment naming, retention, and
/// rollover error semantics.
#[derive(Debug)]
pub struct File {
    writer: Mutex<RollingFileWriter>,
    layout: Box<dyn Layout>,
}

impl File {
    fn new(writer: RollingFileWriter, layout: Box<dyn Layout>) -> Self {
        let writer = Mutex::new(writer);
        Self { writer, layout }
    }

    fn writer(&self) -> MutexGuard<'_, RollingFileWriter> {
        self.writer.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Append for File {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut bytes = self.layout.format(record, diags)?;
        bytes.push(b'\n');
        let mut writer = self.writer();
        writer.write_all(&bytes)
    }

    fn flush(&self) -> Result<(), Error> {
        let mut writer = self.writer();
        writer.flush()
    }
}
