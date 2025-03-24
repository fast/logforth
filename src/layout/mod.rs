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

mod logfmt;
pub use logfmt::LogfmtLayout;

mod text;
pub use text::TextLayout;

/// A layout for formatting log records.
pub trait Layout: fmt::Debug + Send + Sync + 'static {
    /// Formats a log record with optional diagnostics.
    fn format(&self, record: &log::Record, diagnostics: &[Diagnostic]) -> anyhow::Result<Vec<u8>>;
}

impl<T: Layout> From<T> for Box<dyn Layout> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
