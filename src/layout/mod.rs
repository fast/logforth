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

//! Describe how to format a log record.

use crate::Encoder;
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

/// A layout describes how to format a log record.
#[derive(Debug)]
pub enum Layout {
    Custom(CustomLayout),
    Text(TextLayout),
    #[cfg(feature = "json")]
    Json(JsonLayout),
}

impl Layout {
    pub(crate) fn format(&self, record: &log::Record) -> anyhow::Result<String> {
        match self {
            Layout::Custom(layout) => layout.format(record),
            Layout::Text(layout) => layout.format(record),
            #[cfg(feature = "json")]
            Layout::Json(layout) => layout.format(record),
        }
    }
}

pub trait IntoLayout {
    fn into(self) -> Layout;
}

impl<L: Into<Layout>> IntoLayout for L {
    fn into(self) -> Layout {
        self.into()
    }
}

impl<L: Into<Encoder>> IntoLayout for L {
    fn into(self) -> Layout {
        let encoder = self.into();
        Layout::Custom(CustomLayout::new(move |record| {
            let bytes = encoder.format(record)?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }))
    }
}
