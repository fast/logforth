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

use crate::Diagnostic;
use crate::Error;
use crate::kv::KeyOwned;
use crate::kv::ValueOwned;
use crate::kv::Visitor;

/// A diagnostic context backed by an ordered map of key-value pairs.
///
/// Entries are visited in ascending lexicographic key order, and inserting an existing key
/// replaces its previous value.
///
/// # Examples
///
/// ```
/// use logforth_core::diagnostic::StaticDiagnostic;
///
/// let mut diagnostic = StaticDiagnostic::default();
/// diagnostic.insert("app_name", "acme");
/// diagnostic.insert("workers", 4);
/// ```
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct StaticDiagnostic {
    kvs: BTreeMap<KeyOwned, ValueOwned>,
}

impl StaticDiagnostic {
    /// Creates a new [`StaticDiagnostic`] populated from `kvs`.
    pub fn new<K, V, I>(kvs: I) -> Self
    where
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
        I: IntoIterator<Item = (K, V)>,
    {
        Self {
            kvs: kvs.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
        }
    }

    /// Inserts or replaces a key-value pair in the diagnostic map.
    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
    {
        self.kvs.insert(key.into(), value.into());
    }

    /// Removes a key-value pair from the diagnostic map.
    pub fn remove(&mut self, key: &str) {
        self.kvs.remove(key);
    }
}

impl Diagnostic for StaticDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        for (key, value) in &self.kvs {
            visitor.visit(key.view(), value.view())?;
        }
        Ok(())
    }
}
