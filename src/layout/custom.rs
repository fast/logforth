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

use std::fmt::Arguments;

use educe::Educe;

use crate::layout::Layout;

#[derive(Educe)]
#[educe(Debug)]
pub struct CustomLayout {
    #[educe(Debug(ignore))]
    f: Box<
        dyn Fn(&log::Record, &dyn Fn(Arguments) -> anyhow::Result<()>) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static,
    >,
}

impl CustomLayout {
    pub fn new(
        layout: impl Fn(&log::Record, &dyn Fn(Arguments) -> anyhow::Result<()>) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        CustomLayout {
            f: Box::new(layout),
        }
    }

    pub fn format<F>(&self, record: &log::Record, f: &F) -> anyhow::Result<()>
    where
        F: Fn(Arguments) -> anyhow::Result<()>,
    {
        (self.f)(record, f)
    }
}

impl From<CustomLayout> for Layout {
    fn from(layout: CustomLayout) -> Self {
        Layout::Custom(layout)
    }
}
