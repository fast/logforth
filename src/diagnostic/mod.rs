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

//! Markers to enrich log records with additional information.

use std::borrow::Cow;

use log::kv::VisitSource;

#[cfg(feature = "fastrace")]
pub use self::fastrace::FastraceDiagnostic;
pub use self::thread_local::ThreadLocalDiagnostic;

#[cfg(feature = "fastrace")]
mod fastrace;
mod thread_local;

/// A visitor to walk through diagnostic key-value pairs.
pub trait Visitor {
    /// Visits a key-value pair.
    fn visit<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>;
}

impl<T: for<'a> VisitSource<'a>> Visitor for T {
    fn visit<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        self.visit_pair(
            log::kv::Key::from_str(key.into().as_ref()),
            log::kv::Value::from_display(&value.into()),
        )
        .unwrap();
    }
}

/// Mapped Diagnostic Context (MDC).
///
/// A lighter technique consists of uniquely stamping each log request.
#[derive(Debug)]
pub enum Diagnostic {
    #[cfg(feature = "fastrace")]
    Fastrace(FastraceDiagnostic),
    ThreadLocal(ThreadLocalDiagnostic),
}

impl Diagnostic {
    pub fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "fastrace")]
            Diagnostic::Fastrace(diagnostic) => diagnostic.name(),
            Diagnostic::ThreadLocal(diagnostic) => diagnostic.name(),
        }
    }

    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        match self {
            #[cfg(feature = "fastrace")]
            Diagnostic::Fastrace(diagnostic) => diagnostic.visit(visitor),
            Diagnostic::ThreadLocal(diagnostic) => diagnostic.visit(visitor),
        }
    }
}
