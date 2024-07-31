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

use log::Metadata;
use log::Record;

mod stderr;
mod stdout;
mod utils;

/// enum_dispatch facade for [log::Log].
#[enum_dispatch::enum_dispatch]
pub trait Append: Sync + Send {
    /// Dispatch to [log::Log::enabled].
    fn enabled(&self, metadata: &Metadata) -> bool;

    /// Dispatch to [log::Log::log].
    fn log(&self, record: &Record);

    /// Dispatch to [log::Log::flush].
    fn flush(&self);
}

#[enum_dispatch::enum_dispatch(Append)]
pub enum AppendImpl {
    Stdout(stdout::Stdout),
    Stderr(stderr::Stderr),
}

impl log::Log for AppendImpl {
    fn enabled(&self, metadata: &Metadata) -> bool {
        Append::enabled(self, metadata)
    }

    fn log(&self, record: &Record) {
        Append::log(self, record)
    }

    fn flush(&self) {
        Append::flush(self)
    }
}
