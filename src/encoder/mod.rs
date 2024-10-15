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

mod layout_wrapping;
pub use layout_wrapping::LayoutWrappingEncoder;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::JsonEncoder;

#[derive(Debug)]
pub enum Encoder {
    LayoutWrapping(LayoutWrappingEncoder),
    #[cfg(feature = "json")]
    Json(JsonEncoder),
}

impl Encoder {
    pub(crate) fn format(&self, record: &log::Record) -> anyhow::Result<Vec<u8>> {
        match self {
            Encoder::LayoutWrapping(encoder) => encoder.format(record),
            #[cfg(feature = "json")]
            Encoder::Json(encoder) => encoder.format(record),
        }
    }
}
