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
use crate::kv::Key;
use crate::kv::Value;
use crate::kv::Visitor;

thread_local! {
    static THREAD_LOCAL_MAP: RefCell<BTreeMap<String, String>> = const { RefCell::new(BTreeMap::new()) };
}

/// A diagnostic that stores key-value pairs in a thread-local map.
///
/// ## Example
///
/// ```
/// use logforth_core::diagnostic::ThreadLocalDiagnostic;
///
/// ThreadLocalDiagnostic::insert("key", "value");
/// ```
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ThreadLocalDiagnostic {}

impl ThreadLocalDiagnostic {
    /// Insert a key-value pair into the thread local diagnostic .
    pub fn insert<K, V>(key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        THREAD_LOCAL_MAP.with(|map| {
            map.borrow_mut().insert(key.into(), value.into());
        });
    }

    /// Remove a key-value pair from the thread local diagnostic.
    pub fn remove(key: &str) {
        THREAD_LOCAL_MAP.with(|map| {
            map.borrow_mut().remove(key);
        });
    }
}

impl Diagnostic for ThreadLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        THREAD_LOCAL_MAP.with(|map| {
            let map = map.borrow();
            for (key, value) in map.iter() {
                let key = Key::new_ref(key.as_str());
                let value = Value::from(value);
                visitor.visit(key, value)?;
            }
            Ok(())
        })
    }
}
