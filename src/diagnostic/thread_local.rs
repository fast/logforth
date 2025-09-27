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

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::Diagnostic;
use crate::Error;
use crate::kv::Visitor;

thread_local! {
    static CONTEXT: RefCell<BTreeMap<String, String>> = const { RefCell::new(BTreeMap::new()) };
}

/// A diagnostic that stores key-value pairs in a thread-local map.
///
/// ## Example
///
/// ```rust
/// use logforth::diagnostic::ThreadLocalDiagnostic;
///
/// ThreadLocalDiagnostic::insert("key", "value");
/// ```
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ThreadLocalDiagnostic {}

impl ThreadLocalDiagnostic {
    /// Inserts a key-value pair into the thread local diagnostic .
    pub fn insert<K, V>(key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        CONTEXT.with(|map| {
            map.borrow_mut().insert(key.into(), value.into());
        });
    }

    /// Removes a key-value pair from the thread local diagnostic.
    pub fn remove(key: &str) {
        CONTEXT.with(|map| {
            map.borrow_mut().remove(key);
        });
    }
}

impl Diagnostic for ThreadLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        CONTEXT.with(|map| {
            let map = map.borrow();
            for (key, value) in map.iter() {
                visitor.visit(key.as_str().into(), value.as_str().into())?;
            }
            Ok(())
        })
    }
}
