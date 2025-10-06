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

use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::MutexGuard;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Layout;
use logforth_core::Trap;
use logforth_core::append::Append;
use logforth_core::layout::PlainTextLayout;
use logforth_core::record::Record;

use crate::rolling::RollingFileWriter;
use crate::rolling::RollingFileWriterBuilder;
use crate::rotation::Rotation;

/// A builder to configure and create an [`File`] appender.
#[derive(Debug)]
pub struct FileBuilder {
    builder: RollingFileWriterBuilder,
    layout: Box<dyn Layout>,
}

impl FileBuilder {
    /// Create a new file appender builder.
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
    /// Return an error if either:
    ///
    /// * The log directory cannot be created.
    /// * The configured filename is empty.
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

    /// Set the trap for handling errors during logging.
    ///
    /// Default to [`DefaultTrap`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_append_file::FileBuilder;
    /// use logforth_core::trap::DefaultTrap;
    ///
    /// let builder = FileBuilder::new("my_service", "my_app");
    /// builder.trap(DefaultTrap::default());
    /// ```
    pub fn trap(mut self, trap: impl Into<Box<dyn Trap>>) -> Self {
        self.builder = self.builder.trap(trap);
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

    /// Set the rotation strategy to roll over log files daily at 00:00 in the local time zone.
    pub fn rollover_daily(mut self) -> Self {
        self.builder = self.builder.rotation(Rotation::Daily);
        self
    }

    /// Set the rotation strategy to roll over log files if the current log file exceeds the given
    /// size.
    ///
    /// If any time-based rotation strategy is set, the size-based rotation will be checked on the
    /// current log file after the time-based rotation check.
    pub fn rollover_size(mut self, n: NonZeroUsize) -> Self {
        self.builder = self.builder.max_file_size(n);
        self
    }

    /// Set the filename suffix.
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.builder = self.builder.filename_suffix(suffix);
        self
    }

    /// Set the maximum number of log files to keep.
    pub fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.builder = self.builder.max_log_files(n);
        self
    }
}

/// An appender that writes log records to rolling files.
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
        writer.write_all(&bytes).map_err(Error::from_io_error)?;
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        let mut writer = self.writer();
        writer.flush().map_err(Error::from_io_error)?;
        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let writer = self.writer.get_mut().unwrap_or_else(|e| e.into_inner());
        let _ = writer.flush();
    }
}
