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

use std::fmt::Debug;

pub use boxdyn::*;
pub use boxlog::*;
pub use dispatch::*;
#[cfg(feature = "fastrace")]
pub use fastrace::*;
#[cfg(feature = "file")]
pub use file::*;
pub use stdio::*;

mod boxdyn;
mod boxlog;
mod dispatch;
#[cfg(feature = "fastrace")]
mod fastrace;
#[cfg(feature = "file")]
mod file;
mod stdio;

pub trait Append {
    /// Whether this append is enabled; default to `true`.
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    /// Dispatches a log record to the append target.
    fn try_append(&self, record: &log::Record) -> anyhow::Result<()>;

    /// Flushes any buffered records.
    fn flush(&self);
}

#[derive(Debug)]
pub enum AppendImpl {
    BoxDyn(BoxDynAppend),
    BoxLog(BoxLogAppend),
    Dispatch(DispatchAppend),
    #[cfg(feature = "fastrace")]
    Fastrace(FastraceAppend),
    #[cfg(feature = "file")]
    RollingFile(RollingFileAppend),
    Stdout(StdoutAppend),
    Stderr(StderrAppend),
}

impl Append for AppendImpl {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        match self {
            AppendImpl::BoxDyn(append) => append.enabled(metadata),
            AppendImpl::BoxLog(append) => append.enabled(metadata),
            AppendImpl::Dispatch(append) => append.enabled(metadata),
            #[cfg(feature = "fastrace")]
            AppendImpl::Fastrace(append) => append.enabled(metadata),
            #[cfg(feature = "file")]
            AppendImpl::RollingFile(append) => append.enabled(metadata),
            AppendImpl::Stdout(append) => append.enabled(metadata),
            AppendImpl::Stderr(append) => append.enabled(metadata),
        }
    }

    fn try_append(&self, record: &log::Record) -> anyhow::Result<()> {
        match self {
            AppendImpl::BoxDyn(append) => append.try_append(record),
            AppendImpl::BoxLog(append) => append.try_append(record),
            AppendImpl::Dispatch(append) => append.try_append(record),
            #[cfg(feature = "fastrace")]
            AppendImpl::Fastrace(append) => append.try_append(record),
            #[cfg(feature = "file")]
            AppendImpl::RollingFile(append) => append.try_append(record),
            AppendImpl::Stdout(append) => append.try_append(record),
            AppendImpl::Stderr(append) => append.try_append(record),
        }
    }

    fn flush(&self) {
        match self {
            AppendImpl::BoxDyn(append) => append.flush(),
            AppendImpl::BoxLog(append) => append.flush(),
            AppendImpl::Dispatch(append) => append.flush(),
            #[cfg(feature = "fastrace")]
            AppendImpl::Fastrace(append) => append.flush(),
            #[cfg(feature = "file")]
            AppendImpl::RollingFile(append) => append.flush(),
            AppendImpl::Stdout(append) => append.flush(),
            AppendImpl::Stderr(append) => append.flush(),
        }
    }
}
