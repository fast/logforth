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

use log::Record;
use std::io::Write;
use std::sync::Mutex;

use crate::append::rolling_file::RollingFileWriter;
use crate::append::Append;
use crate::layout::TextLayout;
use crate::non_blocking::NonBlocking;
use crate::Diagnostic;
use crate::Layout;

#[derive(Debug)]
pub struct BlockingRollingFile {
    layout: Box<dyn Layout>,
    writer: Mutex<RollingFileWriter>,
}

impl BlockingRollingFile {
    /// Creates a new [`BlockingRollingFile`] appender.
    ///
    /// This appender by default uses [`TextLayout`] to format log records.
    pub fn new(writer: RollingFileWriter) -> Self {
        Self {
            layout: Box::new(TextLayout::default().no_color()),
            writer: Mutex::new(writer),
        }
    }

    /// Sets the layout used to format log records.
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for BlockingRollingFile {
    fn append(&self, record: &Record, diagnostics: &[Diagnostic]) -> anyhow::Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        let mut bytes = self.layout.format(record, diagnostics)?;
        bytes.push(b'\n');
        Write::write_all(&mut *writer, bytes.as_slice())?;
        Ok(())
    }

    fn flush(&self) {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(err) = Write::flush(&mut *writer) {
            eprintln!("failed to flush writer: {err}");
        }
    }
}

/// An appender that writes log records to rolling files.
#[derive(Debug)]
pub struct RollingFile {
    layout: Box<dyn Layout>,
    writer: NonBlocking<RollingFileWriter>,
}

impl RollingFile {
    /// Creates a new [`RollingFile`] appender.
    ///
    /// This appender by default uses [`TextLayout`] to format log records.
    pub fn new(writer: NonBlocking<RollingFileWriter>) -> Self {
        Self {
            layout: Box::new(TextLayout::default().no_color()),
            writer,
        }
    }

    /// Sets the layout used to format log records.
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for RollingFile {
    fn append(&self, record: &Record, diagnostics: &[Diagnostic]) -> anyhow::Result<()> {
        let mut bytes = self.layout.format(record, diagnostics)?;
        bytes.push(b'\n');
        self.writer.send(bytes)?;
        Ok(())
    }
}
