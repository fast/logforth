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
use crate::append::single_file::single::SingleFileWriter;
use crate::append::single_file::single::SingleFileWriterBuilder;
use crate::layout::TextLayout;
use crate::non_blocking::NonBlocking;
use crate::non_blocking::NonBlockingBuilder;

/// A builder to configure and create an [`SingleFile`] appender.
#[derive(Debug)]
pub struct SingleFileBuilder {
    builder: SingleFileWriterBuilder,
    layout: Box<dyn Layout>,

    // non-blocking options
    thread_name: String,
    buffered_lines_limit: Option<usize>,
    shutdown_timeout: Option<Duration>,
}

impl SingleFileBuilder {
    /// Create a new builder.
    pub fn new(log_path: impl Into<PathBuf>) -> Self {
        Self {
            builder: SingleFileWriterBuilder::new(log_path),
            layout: Box::new(TextLayout::default().no_color()),

            thread_name: "logforth-single-file".to_string(),
            buffered_lines_limit: None,
            shutdown_timeout: None,
        }
    }

    /// Build the [`SingleFile`] appender.
    ///
    /// # Errors
    ///
    /// Returns an error if the log file cannot be created.
    pub fn build(self) -> anyhow::Result<(SingleFile, DropGuard)> {
        let SingleFileBuilder {
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
        Ok((SingleFile::new(non_blocking, layout), Box::new(guard)))
    }

    /// Sets the layout for the logs.
    ///
    /// Default to [`TextLayout`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::single_file::SingleFileBuilder;
    /// use logforth::layout::JsonLayout;
    ///
    /// let builder = SingleFileBuilder::new("my_service.log");
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
}

/// An appender that writes log records to a file.
#[derive(Debug)]
pub struct SingleFile {
    layout: Box<dyn Layout>,
    writer: NonBlocking<SingleFileWriter>,
}

impl SingleFile {
    fn new(writer: NonBlocking<SingleFileWriter>, layout: Box<dyn Layout>) -> Self {
        Self { layout, writer }
    }
}

impl Append for SingleFile {
    fn append(&self, record: &Record, diagnostics: &[Box<dyn Diagnostic>]) -> anyhow::Result<()> {
        let mut bytes = self.layout.format(record, diagnostics)?;
        bytes.push(b'\n');
        self.writer.send(bytes)?;
        Ok(())
    }
}
