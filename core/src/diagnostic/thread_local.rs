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
use std::marker::PhantomData;

use crate::Diagnostic;
use crate::Error;
use crate::kv::KeyOwned;
use crate::kv::ValueOwned;
use crate::kv::Visitor;

struct ThreadLocalState {
    next_id: u64,
    map: BTreeMap<KeyOwned, Vec<(u64, ValueOwned)>>,
}

impl ThreadLocalState {
    const fn new() -> Self {
        Self {
            next_id: 1,
            map: BTreeMap::new(),
        }
    }
}

thread_local! {
    static THREAD_LOCAL_STATE: RefCell<ThreadLocalState> = const { RefCell::new(ThreadLocalState::new()) };
}

/// An RAII guard that removes scoped thread-local diagnostic entries when dropped, restoring any
/// previously shadowed values on the current thread.
///
/// This guard is `!Send` and `!Sync` because it operates on the creating thread's local storage.
#[must_use = "dropping the guard immediately restores the previous thread-local diagnostic state"]
#[derive(Debug)]
pub struct ThreadLocalDiagnosticGuard {
    id: u64,
    keys: Box<[KeyOwned]>,
    not_send: PhantomData<*const ()>,
}

impl Drop for ThreadLocalDiagnosticGuard {
    fn drop(&mut self) {
        let _ = THREAD_LOCAL_STATE.try_with(|state| {
            let mut state = state.borrow_mut();
            for key in &self.keys {
                if let Some(stack) = state.map.get_mut(key) {
                    stack.retain(|(id, _)| *id != self.id);
                    if stack.is_empty() {
                        state.map.remove(key);
                    }
                }
            }
        });
    }
}

/// A diagnostic that stores key-value pairs in a thread-local map.
///
/// Prefer [`ThreadLocalDiagnostic::scope`] for scoped context that automatically restores shadowed
/// values when the returned [`ThreadLocalDiagnosticGuard`] is dropped.
///
/// ## Example
///
/// ```
/// use logforth_core::diagnostic::ThreadLocalDiagnostic;
///
/// let _guard = ThreadLocalDiagnostic::scope([("request_id", "req-42")]);
/// ```
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct ThreadLocalDiagnostic {}

impl ThreadLocalDiagnostic {
    /// Pushes scoped key-value pairs onto the current thread's diagnostic context and returns an
    /// RAII guard that removes this scope's entries when dropped.
    ///
    /// Nested scopes shadow earlier values for the same key and restore them on drop, even if
    /// guards are dropped out of order.
    pub fn scope<K, V, I>(kvs: I) -> ThreadLocalDiagnosticGuard
    where
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
        I: IntoIterator<Item = (K, V)>,
    {
        let (id, keys) = THREAD_LOCAL_STATE
            .try_with(|state| {
                let mut state = state.borrow_mut();
                let id = state.next_id;
                state.next_id += 1;
                let mut keys = Vec::new();
                for (key, value) in kvs {
                    let key = key.into();
                    state
                        .map
                        .entry(key.clone())
                        .or_default()
                        .push((id, value.into()));
                    keys.push(key);
                }
                (id, keys.into_boxed_slice())
            })
            .unwrap_or_else(|_| (0, Box::new([])));

        ThreadLocalDiagnosticGuard {
            id,
            keys,
            not_send: PhantomData,
        }
    }

    /// Inserts or replaces an unscoped key-value pair in the current thread's diagnostic map.
    ///
    /// Active [`ThreadLocalDiagnostic::scope`] guards for the same key continue to shadow the
    /// unscoped entry until those guards are dropped.
    pub fn insert<K, V>(key: K, value: V)
    where
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
    {
        let key = key.into();
        let value = value.into();
        let _ = THREAD_LOCAL_STATE.try_with(|state| {
            let mut state = state.borrow_mut();
            let stack = state.map.entry(key).or_default();
            if stack.first().is_some_and(|(id, _)| *id == 0) {
                stack[0].1 = value;
            } else {
                stack.insert(0, (0, value));
            }
        });
    }

    /// Removes the unscoped entry for `key` from the current thread's diagnostic map.
    pub fn remove(key: &str) {
        let _ = THREAD_LOCAL_STATE.try_with(|state| {
            let mut state = state.borrow_mut();
            if let Some(stack) = state.map.get_mut(key) {
                if stack.first().is_some_and(|(id, _)| *id == 0) {
                    stack.remove(0);
                }
                if stack.is_empty() {
                    state.map.remove(key);
                }
            }
        });
    }
}

impl Diagnostic for ThreadLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        THREAD_LOCAL_STATE
            .try_with(|state| {
                let state = state.borrow();
                for (key, stack) in &state.map {
                    if let Some((_, value)) = stack.last() {
                        visitor.visit(key.view(), value.view())?;
                    }
                }
                Ok(())
            })
            .unwrap_or(Ok(()))
    }
}
