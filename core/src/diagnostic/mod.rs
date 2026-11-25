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
mod thread_local;

pub use self::static_global::StaticDiagnostic;
pub use self::thread_local::ThreadLocalDiagnostic;

/// A Mapped Diagnostic Context (MDC) source that visits diagnostic key-values.
///
/// Unlike long-lived pipeline traits (`Filter`, `Layout`, `Append`, and `ErrorSink`), `Diagnostic`
/// does not require `'static` on the trait itself so callers and decorators can borrow scoped or
/// composite diagnostic context on the stack per log call (`&dyn Diagnostic`). Registering a
/// diagnostic into [`DispatchBuilder::diagnostic`](crate::DispatchBuilder::diagnostic) enforces
/// `'static` at the `Box<dyn Diagnostic>` boundary.
pub trait Diagnostic: fmt::Debug + Send + Sync {
    /// Visits diagnostic key-value pairs with `visitor`.
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error>;
}

impl<T: Diagnostic + 'static> From<T> for Box<dyn Diagnostic> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for &T {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for Arc<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl Diagnostic for () {
    fn visit(&self, _: &mut dyn kv::Visitor) -> Result<(), Error> {
        Ok(())
    }
}

impl<T: Diagnostic> Diagnostic for Option<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        if let Some(diag) = self {
            diag.visit(visitor)?;
        }
        Ok(())
    }
}

impl<A: Diagnostic, B: Diagnostic> Diagnostic for (A, B) {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.0.visit(visitor)?;
        self.1.visit(visitor)
    }
}

impl<T: Diagnostic, const N: usize> Diagnostic for [T; N] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

impl<T: Diagnostic> Diagnostic for [T] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        for diag in self {
            diag.visit(visitor)?;
        }
        Ok(())
    }
}

impl<T: Diagnostic> Diagnostic for Box<[T]> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_ref().visit(visitor)
    }
}

impl<T: Diagnostic> Diagnostic for Vec<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

impl Diagnostic for [Box<dyn Diagnostic>] {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        for diag in self {
            diag.as_ref().visit(visitor)?;
        }
        Ok(())
    }
}

impl Diagnostic for Box<[Box<dyn Diagnostic>]> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_ref().visit(visitor)
    }
}

impl Diagnostic for Vec<Box<dyn Diagnostic>> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}
