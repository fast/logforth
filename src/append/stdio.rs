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

use crate::append::Append;
use crate::layout::TextLayout;
use crate::Diagnostic;
use crate::Layout;

/// An appender that writes log records to standard output.
///
/// # Examples
///
/// ```
/// use logforth::append::Stdout;
///
/// let stdout_appender = Stdout::default();
/// ```
#[derive(Debug)]
pub struct Stdout {
    layout: Layout,
    makrer: Option<Diagnostic>,
}

impl Default for Stdout {
    fn default() -> Self {
        Self {
            layout: TextLayout::default().into(),
            makrer: None,
        }
    }
}

impl Stdout {
    /// Sets the layout for the [`Stdout`] appender.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::Stdout;
    /// use logforth::layout::TextLayout;
    ///
    /// let stdout_appender = Stdout::default().with_layout(TextLayout::default());
    /// ```
    pub fn with_layout(mut self, layout: impl Into<Layout>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Sets the marker for the [`Stdout`] appender.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::Stdout;
    /// use logforth::diagnostic::FastraceDiagnostic;
    ///
    /// let stdout_appender = Stdout::default().with_marker(FastraceDiagnostic::default());
    /// ```
    pub fn with_marker(mut self, marker: impl Into<Diagnostic>) -> Self {
        self.makrer = Some(marker.into());
        self
    }
}

impl Append for Stdout {
    fn append(&self, record: &log::Record) -> anyhow::Result<()> {
        let mut bytes = self.layout.format(record, self.makrer.as_ref())?;
        bytes.push(b'\n');
        std::io::stdout().write_all(&bytes)?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

/// An appender that writes log records to standard error.
///
/// # Examples
///
/// ```
/// use logforth::append::Stderr;
///
/// let stderr_appender = Stderr::default();
/// ```
#[derive(Debug)]
pub struct Stderr {
    layout: Layout,
    marker: Option<Diagnostic>,
}

impl Default for Stderr {
    fn default() -> Self {
        Self {
            layout: TextLayout::default().into(),
            marker: None,
        }
    }
}

impl Stderr {
    /// Sets the layout for the [`Stderr`] appender.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::Stderr;
    /// use logforth::layout::JsonLayout;
    ///
    /// let stderr_appender = Stderr::default().with_layout(JsonLayout::default());
    /// ```
    pub fn with_layout(mut self, encoder: impl Into<Layout>) -> Self {
        self.layout = encoder.into();
        self
    }

    /// Sets the marker for the [`Stderr`] appender.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::Stderr;
    /// use logforth::diagnostic::FastraceDiagnostic;
    ///
    /// let stderr_appender = Stderr::default().with_marker(FastraceDiagnostic::default());
    /// ```
    pub fn with_marker(mut self, marker: impl Into<Diagnostic>) -> Self {
        self.marker = Some(marker.into());
        self
    }
}

impl Append for Stderr {
    fn append(&self, record: &log::Record) -> anyhow::Result<()> {
        let mut bytes = self.layout.format(record, self.marker.as_ref())?;
        bytes.push(b'\n');
        std::io::stderr().write_all(&bytes)?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}
