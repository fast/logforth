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

use log::Record;

use crate::encoder::Encoder;
use crate::Layout;

#[derive(Debug)]
pub struct LayoutWrappingEncoder {
    layout: Layout,
}

impl LayoutWrappingEncoder {
    pub fn new(layout: Layout) -> Self {
        Self { layout }
    }
}

impl LayoutWrappingEncoder {
    pub(crate) fn format(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        self.layout.format(record).map(|s| s.into_bytes())
    }
}

impl From<LayoutWrappingEncoder> for Encoder {
    fn from(encoder: LayoutWrappingEncoder) -> Self {
        Encoder::LayoutWrapping(encoder)
    }
}

impl From<Layout> for Encoder {
    fn from(layout: Layout) -> Self {
        LayoutWrappingEncoder::new(layout).into()
    }
}
