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

pub use custom::CustomLayout;
#[cfg(feature = "json")]
pub use json::JsonLayout;
pub use kv::collect_kvs;
pub use kv::KvDisplay;
pub use text::LevelColor;
pub use text::TextLayout;

mod custom;
#[cfg(feature = "json")]
mod json;
mod kv;
mod text;

/// Represents a layout for formatting log records.
#[derive(Debug)]
pub enum Layout {
    Custom(CustomLayout),
    Text(TextLayout),
    #[cfg(feature = "json")]
    Json(JsonLayout),
}

impl Layout {
    pub(crate) fn format(&self, record: &log::Record) -> anyhow::Result<Vec<u8>> {
        match self {
            Layout::Custom(layout) => layout.format(record),
            Layout::Text(layout) => layout.format(record),
            #[cfg(feature = "json")]
            Layout::Json(layout) => layout.format(record),
        }
    }
}
