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
use crate::ErrorSink;

/// An [`ErrorSink`] that writes internal errors to standard error as `logforth: {err:#}`.
///
/// If writing to standard error fails (for example, when standard error is closed or broken), the
/// failure is silently ignored.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct StderrErrorSink {}

impl ErrorSink for StderrErrorSink {
    fn sink(&self, err: &Error) {
        let _ = writeln!(io::stderr().lock(), "logforth: {err:#}");
    }
}
