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

use crate::append::rolling_file::RollingFileWriter;
use crate::append::Append;
use crate::layout::TextLayout;
use crate::non_blocking::NonBlocking;
use crate::Diagnostic;
use crate::Layout;

/// An appender that writes log records to rolling files.
#[derive(Debug)]
pub struct RollingFile {
    layout: Layout,
    marker: Option<Diagnostic>,
    writer: NonBlocking<RollingFileWriter>,
}

impl RollingFile {
    /// Creates a new [`RollingFile`] appender.
    ///
    /// This appender by default uses [`TextLayout`] to format log records.
    pub fn new(writer: NonBlocking<RollingFileWriter>) -> Self {
        Self {
            layout: TextLayout::default().no_color().into(),
            marker: None,
            writer,
        }
    }

    /// Sets the layout used to format log records.
    pub fn with_layout(mut self, layout: impl Into<Layout>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for RollingFile {
    fn append(&self, record: &Record, diagnostic: &[Diagnostic]) -> anyhow::Result<()> {
        let mut bytes = self.layout.format(record, diagnostic)?;
        bytes.push(b'\n');
        self.writer.send(bytes)?;
        Ok(())
    }
}
