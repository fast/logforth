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

mod custom;
pub use custom::CustomEncoder;

#[cfg(feature = "json")]
mod json;
use crate::Layout;
#[cfg(feature = "json")]
pub use json::JsonEncoder;

#[derive(Debug)]
pub enum Encoder {
    Custom(CustomEncoder),
    #[cfg(feature = "json")]
    Json(JsonEncoder),
}

impl Encoder {
    pub(crate) fn format(&self, record: &log::Record) -> anyhow::Result<Vec<u8>> {
        match self {
            Encoder::Custom(encoder) => encoder.format(record),
            #[cfg(feature = "json")]
            Encoder::Json(encoder) => encoder.format(record),
        }
    }
}

impl<L: Into<Layout>> From<L> for Encoder {
    fn from(layout: L) -> Self {
        let layout = layout.into();
        Encoder::Custom(CustomEncoder::new(move |record| {
            Ok(layout.format(record)?.into_bytes())
        }))
    }
}
