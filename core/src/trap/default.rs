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

use std::io;
use std::io::Write;

use crate::Error;
use crate::trap::Trap;

/// A default trap that sends errors to standard error if possible.
///
/// If standard error is not available, it does nothing.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct DefaultTrap {}

impl Trap for DefaultTrap {
    fn trap(&self, err: &Error) {
        let _ = writeln!(io::stderr(), "{err}");
    }
}
