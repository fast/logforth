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

/// Configures the directory, segment names, rotation, and retention of a [`File`] appender.
///
/// See the [crate documentation](super) for usage examples.
#[derive(Debug)]
pub struct FileBuilder {
    builder: RollingFileWriterBuilder,
    layout: Box<dyn Layout>,
}

impl FileBuilder {
    /// Append to `filename` in `basedir`, creating the directory as needed. Rotation is disabled by
    /// default.
    ///
    /// The filename is used literally. Use `FileBuilder::new(dir, "my_app").filename_suffix("log")`
    /// to place the period and sequence before the extension, as in `my_app.2026-11-25.1.log`.
    pub fn new(basedir: impl Into<PathBuf>, filename: impl Into<String>) -> Self {
        Self {
            builder: RollingFileWriterBuilder::new(basedir, filename),
            layout: Box::new(PlainTextLayout::default()),
        }
    }

    /// Build the [`File`] appender.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid filename or suffix, an unrepresentable rotation period, or
    /// an I/O failure while preparing the directory or opening the initial file.
    pub fn build(self) -> Result<File, Error> {
        let FileBuilder { builder, layout } = self;
        let writer = builder.build()?;
        Ok(File::new(writer, layout))
    }

    /// Set the layout for the logs.
    ///
    /// Default to [`PlainTextLayout`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_append_file::FileBuilder;
    /// use logforth_layout_json::JsonLayout;
    ///
    /// let builder = FileBuilder::new("my_service", "my_app");
    /// builder.layout(JsonLayout::default());
    /// ```
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Set the rotation strategy to roll over log files minutely.
    pub fn rollover_minutely(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Minutely);
        self
    }

    /// Set the rotation strategy to roll over log files hourly.
    pub fn rollover_hourly(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Hourly);
        self
    }

    /// Roll over at the start of each day in the configured timezone.
    pub fn rollover_daily(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Daily);
        self
    }

    /// Set the timezone used for rotation boundaries and segment names. Defaults to the system
    /// timezone.
    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.builder = self.builder.timezone(tz);
        self
    }

    /// Roll over before a record would exceed the given size in bytes, including its trailing
    /// newline.
    ///
    /// A single oversized record is written whole to an empty segment. Sequence numbers increase
    /// while the period stays active. On entering another period, numbering starts at 1 and
    /// skips existing names.
    pub fn rollover_size(mut self, n: NonZeroU64) -> Self {
        self.builder = self.builder.max_file_size(n);
        self
    }

    /// Add a suffix after all rotation fields. A single leading dot is stripped and an empty input
    /// clears the suffix.
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.builder = self.builder.filename_suffix(suffix);
        self
    }

    /// Keep at most `n` matching files across all periods, including the active file.
    ///
    /// With rotation enabled, the oldest inactive segments are pruned at startup and after
    /// rollover. By default all segments are retained.
    pub fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.builder = self.builder.max_log_files(n);
        self
    }
}

/// Appends each formatted record and a newline directly to the active file.
///
/// If opening the next segment fails, the appender writes the record to the current file and
/// returns the rollover error if the write succeeds. It waits one second before retrying rotation.
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
