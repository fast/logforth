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

//! Mapped Diagnostic Context (MDC). A lighter technique consists of uniquely stamping each log
//! request.

use std::borrow::Cow;
use std::fmt;

#[cfg(feature = "fastrace")]
pub use self::fastrace::FastraceDiagnostic;
pub use self::static_global::StaticDiagnostic;
pub use self::thread_local::ThreadLocalDiagnostic;

#[cfg(feature = "fastrace")]
mod fastrace;
mod static_global;
mod thread_local;

/// A visitor to walk through diagnostic key-value pairs.
pub trait Visitor {
    /// Visits a key-value pair.
    fn visit(&mut self, key: Cow<str>, value: Cow<str>);
}

/// A trait representing a Mapped Diagnostic Context (MDC) that provides diagnostic key-values.
pub trait Diagnostic: fmt::Debug + Send + Sync + 'static {
    fn visit(&self, visitor: &mut dyn Visitor);
}

impl<T: Diagnostic> From<T> for Box<dyn Diagnostic> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
