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

use crate::Diagnostic;
use crate::Error;
use crate::Layout;
use crate::append::Append;
use crate::layout::PlainTextLayout;
use crate::record::Record;

/// An appender that writes formatted log records followed by `\n` to standard output.
///
/// Defaults to [`PlainTextLayout`]. Formatting runs before acquiring the standard output stream.
///
/// # Examples
///
/// ```
/// use logforth_core::append::Stdout;
///
/// let stdout_appender = Stdout::default();
/// ```
#[derive(Debug)]
pub struct Stdout {
    layout: Box<dyn Layout>,
}

impl Default for Stdout {
    fn default() -> Self {
        Self::new()
    }
}

impl Stdout {
    /// Creates a new [`Stdout`] appender formatted with [`PlainTextLayout`].
    pub fn new() -> Self {
        Self {
            layout: Box::new(PlainTextLayout::new()),
        }
    }

    /// Sets the [`Layout`] used to format records written to standard output.
    ///
    /// Defaults to [`PlainTextLayout`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append::Stdout;
    /// use logforth_core::layout::PlainTextLayout;
    ///
    /// let stdout_appender = Stdout::default().with_layout(PlainTextLayout::default());
    /// ```
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for Stdout {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut bytes = Vec::new();
        self.layout.format(record, diags, &mut bytes)?;
        bytes.push(b'\n');
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|err| Error::new("failed to write to stdout").with_source(err))?;
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        std::io::stdout()
            .flush()
            .map_err(|err| Error::new("failed to flush stdout").with_source(err))?;
        Ok(())
    }
}

/// An appender that writes formatted log records followed by `\n` to standard error.
///
/// Defaults to [`PlainTextLayout`]. Formatting runs before acquiring the standard error stream.
///
/// # Examples
///
/// ```
/// use logforth_core::append::Stderr;
///
/// let stderr_appender = Stderr::default();
/// ```
#[derive(Debug)]
pub struct Stderr {
    layout: Box<dyn Layout>,
}

impl Default for Stderr {
    fn default() -> Self {
        Self::new()
    }
}

impl Stderr {
    /// Creates a new [`Stderr`] appender formatted with [`PlainTextLayout`].
    pub fn new() -> Self {
        Self {
            layout: Box::new(PlainTextLayout::new()),
        }
    }

    /// Sets the [`Layout`] used to format records written to standard error.
    ///
    /// Defaults to [`PlainTextLayout`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append::Stderr;
    /// use logforth_core::layout::PlainTextLayout;
    ///
    /// let stderr_appender = Stderr::default().with_layout(PlainTextLayout::default());
    /// ```
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for Stderr {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut bytes = Vec::new();
        self.layout.format(record, diags, &mut bytes)?;
        bytes.push(b'\n');
        std::io::stderr()
            .write_all(&bytes)
            .map_err(|err| Error::new("failed to write to stderr").with_source(err))?;
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        std::io::stderr()
            .flush()
            .map_err(|err| Error::new("failed to flush stderr").with_source(err))?;
        Ok(())
    }
}
