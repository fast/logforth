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

use crate::Marker;

// TODO(tisonkun): use trait alias when it's stable - https://github.com/rust-lang/rust/issues/41517
//  then we can use the alias for both `dyn` and `impl`.
type MarkerFunction = dyn Fn(&dyn FnMut(&str, String)) + Send + Sync + 'static;

pub struct CustomMarker {
    f: Box<MarkerFunction>,
}

impl Debug for CustomMarker {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "CustomMarker {{ ... }}")
    }
}

impl CustomMarker {
    pub fn new(marker: impl Fn(&dyn FnMut(&str, String)) + Send + Sync + 'static) -> Self {
        CustomMarker {
            f: Box::new(marker),
        }
    }

    pub(crate) fn mark(&self, f: impl FnMut(&str, String)) {
        (self.f)(&f);
    }
}

impl From<CustomMarker> for Marker {
    fn from(marker: CustomMarker) -> Self {
        Marker::Custom(marker)
    }
}
