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

pub use boxdyn::BoxDyn;
pub use identical::Identical;
pub use kv_display::KvDisplay;
#[cfg(feature = "json")]
pub use simple_json::SimpleJson;
pub use simple_text::SimpleText;

mod boxdyn;
mod identical;
mod kv_display;
#[cfg(feature = "json")]
mod simple_json;
mod simple_text;

pub trait Layout {
    fn format_record<'a>(&'_ self, record: &'a log::Record<'a>) -> anyhow::Result<log::Record<'a>>;
}

#[derive(Debug)]
pub enum LayoutImpl {
    BoxDyn(BoxDyn),
    Identical(Identical),
    SimpleText(SimpleText),
    #[cfg(feature = "json")]
    SimpleJson(SimpleJson),
}

impl Layout for LayoutImpl {
    fn format_record<'a>(&'_ self, record: &'a log::Record<'a>) -> anyhow::Result<log::Record<'a>> {
        match self {
            LayoutImpl::BoxDyn(layout) => layout.format_record(record),
            LayoutImpl::Identical(layout) => layout.format_record(record),
            LayoutImpl::SimpleText(layout) => layout.format_record(record),
            #[cfg(feature = "json")]
            LayoutImpl::SimpleJson(layout) => layout.format_record(record),
        }
    }
}
