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
    layout: Box<dyn Layout>,
}

impl Default for Stdout {
    fn default() -> Self {
        Self {
            layout: Box::new(PlainTextLayout::default()),
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
    /// use logforth::layout::PlainTextLayout;
    ///
    /// let stdout_appender = Stdout::default().with_layout(PlainTextLayout::default());
    /// ```
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for Stdout {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut bytes = self.layout.format(record, diags)?;
        bytes.push(b'\n');
        std::io::stdout()
            .write_all(&bytes)
            .map_err(Error::from_io_error)?;
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        std::io::stdout().flush().map_err(Error::from_io_error)?;
        Ok(())
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
    layout: Box<dyn Layout>,
}

impl Default for Stderr {
    fn default() -> Self {
        Self {
            layout: Box::new(PlainTextLayout::default()),
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
    /// use logforth::layout::PlainTextLayout;
    ///
    /// let stderr_appender = Stderr::default().with_layout(PlainTextLayout::default());
    /// ```
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for Stderr {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let mut bytes = self.layout.format(record, diags)?;
        bytes.push(b'\n');
        std::io::stderr()
            .write_all(&bytes)
            .map_err(Error::from_io_error)?;
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        std::io::stderr().flush().map_err(Error::from_io_error)?;
        Ok(())
    }
}
