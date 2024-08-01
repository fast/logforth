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

#[cfg(feature = "colored")]
pub use colored_simple_text::ColoredSimpleTextLayout;
use log::Record;
pub use simple_text::SimpleTextLayout;

#[cfg(feature = "colored")]
mod colored_simple_text;
mod kv_display;
mod simple_text;

pub trait Layout {
    fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>>;
}

#[derive(Debug)]
pub enum LayoutImpl {
    SimpleText(SimpleTextLayout),
    #[cfg(feature = "colored")]
    ColoredSimpleText(ColoredSimpleTextLayout),
}

macro_rules! enum_dispatch_layout {
    ($($name:ident),+) => {
        impl Layout for LayoutImpl {
            fn format_bytes(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
                match self { $( LayoutImpl::$name(layout) => layout.format_bytes(record), )+ }
            }
        }

        $(paste::paste! {
            impl From<[<$name Layout>]> for LayoutImpl {
                fn from(layout: [<$name Layout>]) -> Self { LayoutImpl::$name(layout) }
            }
        })+
    };
}

enum_dispatch_layout!(SimpleText, ColoredSimpleText);
