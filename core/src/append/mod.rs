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

//! Dispatch log records to various targets.

use std::fmt;

use crate::Diagnostic;
use crate::Error;
use crate::record::Record;

mod stdio;
mod testing;

pub use self::stdio::Stderr;
pub use self::stdio::Stdout;
pub use self::testing::Testing;

/// An appender that can process log records.
pub trait Append: fmt::Debug + Send + Sync + 'static {
    /// Dispatch a log record to the append target.
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error>;

    /// Flush any buffered records.
    ///
    /// Default to a no-op.
    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl<T: Append> From<T> for Box<dyn Append> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
