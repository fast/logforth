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

use log::kv::Error;
use log::kv::ToKey;
use log::kv::Value;
use log::kv::VisitSource;

use crate::Diagnostic;

thread_local! {
    static CONTEXT: RefCell<BTreeMap<String, String>> = RefCell::new(BTreeMap::new());
}

#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ThreadLocalDiagnostic {}

impl ThreadLocalDiagnostic {
    pub fn name(&self) -> &'static str {
        "thread-local"
    }

    pub fn insert<K, V>(&self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        CONTEXT.with(|map| {
            map.borrow_mut().insert(key.into(), value.into());
        });
    }

    pub fn remove(&self, key: &str) {
        CONTEXT.with(|map| {
            map.borrow_mut().remove(key);
        });
    }
}

impl log::kv::Source for ThreadLocalDiagnostic {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn VisitSource<'kvs>) -> Result<(), Error> {
        CONTEXT.with(|map| {
            for (key, value) in map.borrow().iter() {
                visitor.visit_pair(key.to_key(), Value::from_display(value))?;
            }
            Ok(())
        })
    }
}

impl From<ThreadLocalDiagnostic> for Diagnostic {
    fn from(diagnostic: ThreadLocalDiagnostic) -> Self {
        Diagnostic::ThreadLocal(diagnostic)
    }
}
