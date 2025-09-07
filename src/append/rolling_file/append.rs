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

use log::Record;

use crate::Diagnostic;
use crate::Error;
use crate::Layout;
use crate::append::Append;
use crate::append::rolling_file::Rotation;
use crate::append::rolling_file::rolling::RollingFileWriter;
use crate::append::rolling_file::rolling::RollingFileWriterBuilder;
use crate::layout::TextLayout;

/// A builder to configure and create an [`RollingFile`] appender.
#[derive(Debug)]
pub struct RollingFileBuilder {
    builder: RollingFileWriterBuilder,
    layout: Box<dyn Layout>,
}

impl RollingFileBuilder {
    /// Create a new builder.
    ///
    /// # Error
    ///
    /// If `filename` is empty, [`RollingFileBuilder::build`] would return an error.
    pub fn new(basedir: impl Into<PathBuf>, filename: impl Into<String>) -> Self {
        Self {
            builder: RollingFileWriterBuilder::new(basedir, filename),
            layout: Box::new(TextLayout::default().no_color()),
        }
    }

    /// Build the [`RollingFile`] appender.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The log directory cannot be created.
    /// * The configured filename is empty.
    pub fn build(self) -> Result<RollingFile, Error> {
        let RollingFileBuilder { builder, layout } = self;
        let writer = builder.build()?;
        Ok(RollingFile::new(writer, layout))
    }

    /// Sets the layout for the logs.
    ///
    /// Default to [`TextLayout`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::rolling_file::RollingFileBuilder;
    /// use logforth::layout::JsonLayout;
    ///
    /// let builder = RollingFileBuilder::new("my_service", "my_app");
    /// builder.layout(JsonLayout::default());
    /// ```
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Sets the rotation policy.
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.builder = self.builder.rotation(rotation);
        self
    }

    /// Sets the filename suffix.
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.builder = self.builder.filename_suffix(suffix);
        self
    }

    /// Sets the maximum number of log files to keep.
    pub fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.builder = self.builder.max_log_files(n);
        self
    }

    /// Sets the maximum size of a log file in bytes.
    pub fn max_file_size(mut self, n: NonZeroUsize) -> Self {
        self.builder = self.builder.max_file_size(n);
        self
    }
}

/// An appender that writes log records to rolling files.
#[derive(Debug)]
pub struct RollingFile {
    writer: Mutex<RollingFileWriter>,
    layout: Box<dyn Layout>,
}

impl RollingFile {
    fn new(writer: RollingFileWriter, layout: Box<dyn Layout>) -> Self {
        let writer = Mutex::new(writer);
        Self { writer, layout }
    }

    fn writer(&self) -> MutexGuard<'_, RollingFileWriter> {
        self.writer.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Append for RollingFile {
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
