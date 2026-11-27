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

//! Mapped Diagnostic Context (MDC).

use std::fmt;
use std::sync::Arc;

use crate::Error;
use crate::kv;

mod static_global;
pub use self::static_global::StaticDiagnostic;

mod thread_local;
pub use self::thread_local::ThreadLocalDiagnostic;
pub use self::thread_local::ThreadLocalDiagnosticGuard;

mod impls;

/// A Mapped Diagnostic Context (MDC) that provides diagnostic key-values.
pub trait Diagnostic: fmt::Debug + Send + Sync {
    /// Visits diagnostic key-value pairs with `visitor`.
    ///
    /// This method may be called multiple times for a single log record and should remain
    /// side-effect-free.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if reading diagnostic context fails or `visitor` returns an error,
    /// stopping traversal.
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error>;
}

impl<T: Diagnostic + 'static> From<T> for Box<dyn Diagnostic> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for Arc<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for &T {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}
