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
use std::sync::Arc;

use crate::Diagnostic;
use crate::Error;
use crate::record::Record;

mod plain_text;

pub use self::plain_text::PlainTextLayout;

/// A layout for formatting log records into a byte buffer.
///
/// Implementations append formatted bytes to `buf` without modifying or removing bytes before its
/// starting length, and omit record framing such as a trailing newline (which belongs to the
/// owning [`Append`](crate::Append)).
pub trait Layout: fmt::Debug + Send + Sync + 'static {
    /// Appends the formatted representation of `record` and `diags` to `buf`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if formatting fails. On `Err`, `buf` may contain partial output past
    /// its starting length; callers reusing `buf` must discard it (for example, with
    /// `buf.truncate(start)`).
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error>;
}

impl<T: Layout> From<T> for Box<dyn Layout> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: Layout + ?Sized> Layout for Arc<T> {
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        (**self).format(record, diags, buf)
    }
}

impl<T: Layout + ?Sized> Layout for &'static T {
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        (**self).format(record, diags, buf)
    }
}
