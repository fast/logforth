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

use std::collections::BTreeMap;

use crate::diagnostic::Visitor;
use crate::Diagnostic;

/// A diagnostic that stores key-value pairs in a static global map.
///
/// ## Example
///
/// ```rust
/// use logforth::diagnostic::StaticDiagnostic;
///
/// let mut diagnostic = StaticDiagnostic::default();
/// diagnostic.insert("key", "value");
/// ```
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct StaticDiagnostic {
    kvs: BTreeMap<String, String>,
}

impl StaticDiagnostic {
    pub fn new(kvs: BTreeMap<String, String>) -> Self {
        Self { kvs }
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.kvs.insert(key.into(), value.into());
    }

    pub fn remove(&mut self, key: &str) {
        self.kvs.remove(key);
    }

    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        for (key, value) in self.kvs.iter() {
            visitor.visit(key, value);
        }
    }
}

impl From<StaticDiagnostic> for Diagnostic {
    fn from(diagnostic: StaticDiagnostic) -> Self {
        Diagnostic::Static(diagnostic)
    }
}
