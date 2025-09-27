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

//! Layouts for formatting log records.

use std::fmt;

use crate::Diagnostic;
use crate::Error;
use crate::record::Record;

mod plain_text;

pub use self::plain_text::PlainTextLayout;

/// A layout for formatting log records.
pub trait Layout: fmt::Debug + Send + Sync + 'static {
    /// Formats a log record with optional diagnostics.
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error>;
}

impl<T: Layout> From<T> for Box<dyn Layout> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
