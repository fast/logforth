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

#[cfg(feature = "layout-google-cloud-logging")]
mod google_cloud_logging;
#[cfg(feature = "layout-json")]
mod json;
mod logfmt;
mod plain_text;
#[cfg(feature = "layout-text")]
pub mod text;

#[cfg(feature = "layout-google-cloud-logging")]
pub use self::google_cloud_logging::GoogleCloudLoggingLayout;
#[cfg(feature = "layout-json")]
pub use self::json::JsonLayout;
pub use self::logfmt::LogfmtLayout;
pub use self::plain_text::PlainTextLayout;
#[cfg(feature = "layout-text")]
pub use self::text::TextLayout;

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

// obtain filename only from record's full file path
// reason: the module is already logged + full file path is noisy for text layout
fn filename<'a>(record: &'a Record<'a>) -> std::borrow::Cow<'a, str> {
    record
        .file()
        .map(std::path::Path::new)
        .and_then(std::path::Path::file_name)
        .map(std::ffi::OsStr::to_string_lossy)
        .unwrap_or_default()
}
