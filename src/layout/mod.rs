// Copyright 2024 tison <wander4096@gmail.com>
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

pub use kv_display::KvDisplay;
use log::Record;
#[cfg(feature = "json")]
pub use simple_json::SimpleJsonLayout;
pub use simple_text::SimpleTextLayout;

mod kv_display;
#[cfg(feature = "json")]
mod simple_json;
mod simple_text;

pub trait Layout {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>>;
}

#[derive(Debug)]
pub enum LayoutImpl {
    SimpleText(SimpleTextLayout),
    #[cfg(feature = "json")]
    SimpleJson(SimpleJsonLayout),
}

impl Layout for LayoutImpl {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        match self {
            LayoutImpl::SimpleText(layout) => layout.format_bytes(record),
            #[cfg(feature = "json")]
            LayoutImpl::SimpleJson(layout) => layout.format_bytes(record),
        }
    }
}
