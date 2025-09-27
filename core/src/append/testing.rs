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

use crate::Diagnostic;
use crate::Error;
use crate::Layout;
use crate::append::Append;
use crate::layout::PlainTextLayout;
use crate::record::Record;

/// An appender that writes log records that can be captured by a test harness (like `cargo test`),
/// and thus the outputs are suppressed unless `--nocapture` or `--show-output` is specified.
///
/// # Examples
///
/// ```
/// use logforth_core::append::Testing;
///
/// let test_appender = Testing::default();
/// ```
#[derive(Debug)]
pub struct Testing {
    layout: Box<dyn Layout>,
}

impl Default for Testing {
    fn default() -> Self {
        Self {
            layout: Box::new(PlainTextLayout::default()),
        }
    }
}

impl Testing {
    /// Set the layout for the [`Testing`] appender.
    ///
    /// Default to [`PlainTextLayout`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append::Testing;
    /// use logforth_core::layout::PlainTextLayout;
    ///
    /// let test_appender = Testing::default().with_layout(PlainTextLayout::default());
    /// ```
    pub fn with_layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for Testing {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let bytes = self.layout.format(record, diags)?;
        eprintln!("{}", String::from_utf8_lossy(&bytes));
        Ok(())
    }
}
