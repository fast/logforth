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

pub use custom::CustomLayout;
pub use identical::Identical;
pub use kv_display::KvDisplay;
#[cfg(feature = "json")]
pub use simple_json::SimpleJson;
pub use simple_text::SimpleText;

mod custom;
mod identical;
mod kv_display;
#[cfg(feature = "json")]
mod simple_json;
mod simple_text;

#[derive(Debug)]
pub enum Layout {
    Identical(Identical),
    SimpleText(SimpleText),
    #[cfg(feature = "json")]
    SimpleJson(SimpleJson),
    Custom(CustomLayout),
}

impl Layout {
    pub fn format<F>(&self, record: &log::Record, f: &F) -> anyhow::Result<()>
    where
        F: Fn(&log::Record) -> anyhow::Result<()>,
    {
        match self {
            Layout::Identical(layout) => {
                layout.format(record, &|args| f(&record.to_builder().args(args).build()))
            }
            Layout::SimpleText(layout) => {
                layout.format(record, &|args| f(&record.to_builder().args(args).build()))
            }
            #[cfg(feature = "json")]
            Layout::SimpleJson(layout) => {
                layout.format(record, &|args| f(&record.to_builder().args(args).build()))
            }
            Layout::Custom(layout) => {
                layout.format(record, &|args| f(&record.to_builder().args(args).build()))
            }
        }
    }
}
