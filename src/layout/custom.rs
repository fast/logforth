// Copyright 2024 CratesLand Developers
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
use std::fmt::Debug;
use std::fmt::Formatter;

use log::Record;

use crate::layout::Layout;

// TODO(tisonkun): use trait alias when it's stable - https://github.com/rust-lang/rust/issues/41517
//  then we can use the alias for both `dyn` and `impl`.
type FormatFunction = dyn Fn(&Record, &dyn Fn(Arguments) -> anyhow::Result<()>) -> anyhow::Result<()>
    + Send
    + Sync
    + 'static;

/// A layout that you can pass the custom layout function.
///
/// The custom layout function accepts [`&log::Record`][Record], formats it into [Arguments], and
/// then passes to the closure. For example:
///
/// ```rust
/// use std::fmt::Arguments;
///
/// use log::Record;
/// use logforth::layout::CustomLayout;
///
/// let layout = CustomLayout::new(
///     |record: &Record, f: &dyn Fn(Arguments) -> anyhow::Result<()>| {
///         f(format_args!("{} - {}", record.level(), record.args()))
///     },
/// );
/// ```
pub struct CustomLayout {
    f: Box<FormatFunction>,
}

impl Debug for CustomLayout {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "CustomLayout {{ ... }}")
    }
}

impl CustomLayout {
    pub fn new(
        layout: impl Fn(&Record, &dyn Fn(Arguments) -> anyhow::Result<()>) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        CustomLayout {
            f: Box::new(layout),
        }
    }

    pub(crate) fn format<F>(&self, record: &Record, f: &F) -> anyhow::Result<()>
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
