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

use crate::Error;
use crate::ErrorSink;

/// An [`ErrorSink`] that silently discards all internal errors.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct BlackHole {}

impl BlackHole {
    /// Create a new [`BlackHole`] error sink.
    pub const fn new() -> Self {
        Self {}
    }
}

impl ErrorSink for BlackHole {
    fn sink(&self, _: &Error) {}
}
