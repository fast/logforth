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

use std::fmt;

#[cfg(feature = "fastrace")]
pub use fastrace::*;
#[cfg(feature = "opentelemetry")]
pub use opentelemetry::*;
pub use stdio::*;

use crate::filter::Filter;
use crate::layout::IdenticalLayout;
use crate::layout::Layout;

#[cfg(feature = "fastrace")]
mod fastrace;
#[cfg(feature = "opentelemetry")]
mod opentelemetry;
#[cfg(feature = "file")]
pub mod rolling_file;
mod stdio;

pub trait Append: fmt::Debug + Send + Sync + 'static {
    /// Dispatches a log record to the append target.
    fn append(&self, record: &log::Record) -> anyhow::Result<()>;

    /// Flushes any buffered records.
    fn flush(&self) {}

    /// Default layout to use when [Dispatch][crate::logger::Dispatch] does not configure a
    /// preferred layout.
    fn default_layout(&self) -> Layout {
        Layout::Identical(IdenticalLayout)
    }

    /// Default filters associated to this append. [log::Log] is mixed with
    /// [Filter] and [Append].
    fn default_filters(&self) -> Vec<Filter> {
        vec![]
    }
}
