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

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::JsonLayout;
use log::Record;

mod logfmt;
use std::borrow::Cow;

pub use logfmt::LogfmtLayout;

mod text;
pub use text::TextLayout;

use crate::diagnostic::Visitor;

/// A layout for formatting log records.
pub trait Layout: fmt::Debug + Send + Sync + 'static {
    /// Formats a log record with optional diagnostics.
    fn format(
        &self,
        record: &log::Record,
        diagnostics: &[Box<dyn Diagnostic>],
    ) -> anyhow::Result<Vec<u8>>;
}

impl<T: Layout> From<T> for Box<dyn Layout> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

// obtain filename only from record's full file path
// reason: the module is already logged + full file path is noisy for text layout
fn filename<'a>(record: &'a Record<'a>) -> Cow<'a, str> {
    record
        .file()
        .map(std::path::Path::new)
        .and_then(std::path::Path::file_name)
        .map(std::ffi::OsStr::to_string_lossy)
        .unwrap_or_default()
}

struct KvWriter {
    text: String,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvWriter {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        use std::fmt::Write;

        write!(&mut self.text, " {key}={value}")?;
        Ok(())
    }
}

impl Visitor for KvWriter {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) {
        use std::fmt::Write;

        // SAFETY: no more than an allocate-less version
        //  self.text.push_str(&format!(" {key}={value}"))
        write!(&mut self.text, " {key}={value}").unwrap();
    }
}
