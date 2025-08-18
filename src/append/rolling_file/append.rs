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

use std::path::PathBuf;
use std::time::Duration;

use log::Record;

use crate::Diagnostic;
use crate::DropGuard;
use crate::Layout;
use crate::append::Append;
use crate::append::rolling_file::Rotation;
use crate::append::rolling_file::rolling::RollingFileWriter;
use crate::append::rolling_file::rolling::RollingFileWriterBuilder;
use crate::layout::TextLayout;
use crate::non_blocking::NonBlocking;
use crate::non_blocking::NonBlockingBuilder;

/// A builder to configure and create an [`RollingFile`] appender.
#[derive(Debug)]
pub struct RollingFileBuilder {
    builder: RollingFileWriterBuilder,
    layout: Box<dyn Layout>,

    // non-blocking options
    thread_name: String,
    buffered_lines_limit: Option<usize>,
    shutdown_timeout: Option<Duration>,
}

impl RollingFileBuilder {
    /// Create a new builder.
    pub fn new(basedir: impl Into<PathBuf>) -> Self {
        Self {
            builder: RollingFileWriterBuilder::new(basedir),
            layout: Box::new(TextLayout::default().no_color()),

            thread_name: "logforth-rolling-file".to_string(),
            buffered_lines_limit: None,
            shutdown_timeout: None,
        }
    }

    /// Build the [`RollingFile`] appender.
    ///
    /// # Errors
    ///
    /// Returns an error if the log directory cannot be created.
    pub fn build(self) -> anyhow::Result<(RollingFile, DropGuard)> {
        let RollingFileBuilder {
            builder,
            layout,
            thread_name,
            buffered_lines_limit,
            shutdown_timeout,
        } = self;
        let writer = builder.build()?;
        let (non_blocking, guard) = NonBlockingBuilder::new(thread_name, writer)
            .buffered_lines_limit(buffered_lines_limit)
            .shutdown_timeout(shutdown_timeout)
            .build();
        Ok((RollingFile::new(non_blocking, layout), Box::new(guard)))
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
    /// let builder = RollingFileBuilder::new("my_service");
    /// builder.layout(JsonLayout::default());
    /// ```
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Sets the buffer size of pending messages.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: Option<usize>) -> Self {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Sets the shutdown timeout before the worker guard dropped.
    pub fn shutdown_timeout(mut self, shutdown_timeout: Option<Duration>) -> Self {
        self.shutdown_timeout = shutdown_timeout;
        self
    }

    /// Sets the thread name for the background sender thread.
    pub fn thread_name(mut self, thread_name: impl Into<String>) -> Self {
        self.thread_name = thread_name.into();
        self
    }

    /// Sets the rotation policy.
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.builder = self.builder.rotation(rotation);
        self
    }

    /// Sets the filename prefix.
    pub fn filename_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.builder = self.builder.filename_prefix(prefix);
        self
    }

    /// Sets the filename suffix.
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.builder = self.builder.filename_suffix(suffix);
        self
    }

    /// Sets the maximum number of log files to keep.
    pub fn max_log_files(mut self, n: usize) -> Self {
        self.builder = self.builder.max_log_files(n);
        self
    }

    /// Sets the maximum size of a log file in bytes.
    pub fn max_file_size(mut self, n: usize) -> Self {
        self.builder = self.builder.max_file_size(n);
        self
    }
}

/// An appender that writes log records to rolling files.
#[derive(Debug)]
pub struct RollingFile {
    layout: Box<dyn Layout>,
    writer: NonBlocking<RollingFileWriter>,
}

impl RollingFile {
    fn new(writer: NonBlocking<RollingFileWriter>, layout: Box<dyn Layout>) -> Self {
        Self { layout, writer }
    }
}

impl Append for RollingFile {
    fn append(&self, record: &Record, diagnostics: &[Box<dyn Diagnostic>]) -> anyhow::Result<()> {
        let mut bytes = self.layout.format(record, diagnostics)?;
        bytes.push(b'\n');
        self.writer.send(bytes)?;
        Ok(())
    }
}
