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

use log::Metadata;

use crate::filter::Filter;
use crate::filter::FilterResult;

/// A filter that you can pass the custom filter function.
///
/// The custom filter function accepts [`&log::Metadata`][Metadata] and returns the
/// [`FilterResult`]. For example:
///
/// ```rust
/// use log::Metadata;
/// use logforth::filter::CustomFilter;
/// use logforth::filter::FilterResult;
///
/// let filter = CustomFilter::new(|metadata: &Metadata| {
///     if metadata.target() == "my_crate" {
///         FilterResult::Accept
///     } else {
///         FilterResult::Neutral
///     }
/// });
/// ```
pub struct CustomFilter {
    f: Box<dyn Fn(&Metadata) -> FilterResult + Send + Sync + 'static>,
}

impl Debug for CustomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomFilter {{ ... }}")
    }
}

impl CustomFilter {
    pub fn new(filter: impl Fn(&Metadata) -> FilterResult + Send + Sync + 'static) -> Self {
        CustomFilter {
            f: Box::new(filter),
        }
    }

    pub(crate) fn enabled(&self, metadata: &Metadata) -> FilterResult {
        (self.f)(metadata)
    }
}

impl From<CustomFilter> for Filter {
    fn from(filter: CustomFilter) -> Self {
        Filter::Custom(filter)
    }
}
