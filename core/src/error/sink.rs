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

use std::fmt;
use std::sync::Arc;

use crate::Error;

/// A terminal sink that processes internal runtime errors produced by the logging pipeline.
///
/// Logging operations are designed not to crash the host application at runtime. When an appender
/// fails to emit or flush records, execution roots such as [`Logger`](crate::Logger), an
/// overriding [`DispatchBuilder`](crate::DispatchBuilder), or asynchronous workers forward the
/// resulting [`Error`] to their configured [`ErrorSink`].
///
/// Because [`ErrorSink`] is the final stage of error handling, implementations operate on a
/// best-effort basis: they must silently absorb their own delivery failures, never panic, and
/// never route log records back into the owning [`Logger`](crate::Logger).
pub trait ErrorSink: fmt::Debug + Send + Sync + 'static {
    /// Process an internal runtime error on a best-effort basis.
    fn sink(&self, err: &Error);
}

impl<T: ErrorSink> From<T> for Box<dyn ErrorSink> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: ErrorSink + ?Sized> ErrorSink for Arc<T> {
    fn sink(&self, err: &Error) {
        (**self).sink(err);
    }
}

impl<T: ErrorSink + ?Sized> ErrorSink for &'static T {
    fn sink(&self, err: &Error) {
        (**self).sink(err);
    }
}
