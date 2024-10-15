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

use std::fmt::Debug;
use std::fmt::Formatter;

use log::Record;

use crate::Encoder;

// TODO(tisonkun): use trait alias when it's stable - https://github.com/rust-lang/rust/issues/41517
//  then we can use the alias for both `dyn` and `impl`.
type FormatFunction = dyn Fn(&Record) -> anyhow::Result<Vec<u8>> + Send + Sync + 'static;

/// An encoder that you can pass the custom encoder function.
///
/// The custom encoder function accepts [`&log::Record`][Record] and formats it into [`Vec<u8>`].
/// For example:
///
/// ```rust
/// use log::Record;
/// use logforth::encoder::CustomEncoder;
///
/// let layout = CustomEncoder::new(|record: &Record| {
///     Ok(format!("{} - {}", record.level(), record.args()).into_bytes())
/// });
/// ```
pub struct CustomEncoder {
    f: Box<FormatFunction>,
}

impl Debug for CustomEncoder {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "CustomEncoder {{ ... }}")
    }
}

impl CustomEncoder {
    pub fn new(
        layout: impl Fn(&Record) -> anyhow::Result<Vec<u8>> + Send + Sync + 'static,
    ) -> Self {
        CustomEncoder {
            f: Box::new(layout),
        }
    }

    pub(crate) fn format(&self, record: &Record) -> anyhow::Result<Vec<u8>> {
        (self.f)(record)
    }
}

impl From<CustomEncoder> for Encoder {
    fn from(encoder: CustomEncoder) -> Self {
        Encoder::Custom(encoder)
    }
}