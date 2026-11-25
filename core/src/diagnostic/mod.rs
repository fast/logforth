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

/// A Mapped Diagnostic Context (MDC) that provides diagnostic key-values.
pub trait Diagnostic: fmt::Debug + Send + Sync + 'static {
    /// Visit the MDC key-values with the provided visitor.
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error>;
}

impl<T: Diagnostic> From<T> for Box<dyn Diagnostic> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for Arc<T> {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<T: Diagnostic + ?Sized> Diagnostic for &'static T {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

/// Borrowed view over zero or more [`Diagnostic`] sources passed to filters, layouts, and
/// appenders.
#[derive(Debug, Clone, Copy)]
pub struct Diagnostics<'a>(DiagnosticsState<'a>);

#[derive(Debug, Clone, Copy)]
enum DiagnosticsState<'a> {
    Boxed(&'a [Box<dyn Diagnostic>]),
    Single(&'a dyn Diagnostic),
    Slice(&'a [&'a dyn Diagnostic]),
}

impl Default for Diagnostics<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a> Diagnostics<'a> {
    /// Creates an empty [`Diagnostics`] view with no attached sources.
    pub const fn empty() -> Self {
        Self(DiagnosticsState::Boxed(&[]))
    }

    /// Creates a [`Diagnostics`] view borrowing a slice of boxed [`Diagnostic`] trait objects.
    pub const fn from_boxed(diags: &'a [Box<dyn Diagnostic>]) -> Self {
        Self(DiagnosticsState::Boxed(diags))
    }

    /// Creates a [`Diagnostics`] view borrowing a single [`Diagnostic`] source.
    pub const fn from_single(diag: &'a dyn Diagnostic) -> Self {
        Self(DiagnosticsState::Single(diag))
    }

    /// Creates a [`Diagnostics`] view borrowing a slice of [`Diagnostic`] trait-object references.
    pub const fn from_slice(diags: &'a [&'a dyn Diagnostic]) -> Self {
        Self(DiagnosticsState::Slice(diags))
    }

    /// Visits key-value pairs from each attached [`Diagnostic`] in order.
    ///
    /// # Errors
    ///
    /// Returns the first [`Error`] produced by a [`Diagnostic`] or `visitor`, stopping traversal.
    pub fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error> {
        match self.0 {
            DiagnosticsState::Boxed(diags) => {
                for diag in diags {
                    diag.visit(visitor)?;
                }
                Ok(())
            }
            DiagnosticsState::Single(diag) => (*diag).visit(visitor),
            DiagnosticsState::Slice(diags) => {
                for &diag in diags {
                    (*diag).visit(visitor)?;
                }
                Ok(())
            }
        }
    }
}
