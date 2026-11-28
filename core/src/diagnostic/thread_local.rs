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
use std::marker::PhantomData;

use crate::Diagnostic;
use crate::Error;
use crate::kv::KeyOwned;
use crate::kv::ValueOwned;
use crate::kv::Visitor;

struct ScopeFrame {
    id: u64,
    entries: Box<[(KeyOwned, ValueOwned)]>,
}

struct ThreadLocalState {
    next_id: u64,
    frames: Vec<ScopeFrame>,
}

impl ThreadLocalState {
    const fn new() -> Self {
        Self {
            next_id: 1,
            frames: Vec::new(),
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
    not_send: PhantomData<*const ()>,
}

impl Drop for ThreadLocalDiagnosticGuard {
    fn drop(&mut self) {
        if self.id == 0 {
            return;
        }
        let _ = THREAD_LOCAL_STATE.try_with(|state| {
            let mut state = state.borrow_mut();
            if state.frames.last().is_some_and(|frame| frame.id == self.id) {
                state.frames.pop();
            } else {
                state.frames.retain(|frame| frame.id != self.id);
            }
        });
    }
}

/// A diagnostic context that reads scoped key-value pairs from thread-local storage.
///
/// Active entries are managed through [`ThreadLocalDiagnostic::scope`]. If thread-local storage is
/// already destroyed on the current thread (such as when logging from another thread-local
/// destructor during thread teardown), visiting [`ThreadLocalDiagnostic`] degrades to an empty
/// context instead of panicking.
///
/// # Examples
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
    /// Pushes scoped key-value pairs onto the current thread's diagnostic stack and returns an
    /// RAII guard that removes this scope when dropped.
    ///
    /// Nested scopes shadow outer values for the same key and restore them on drop, even if
    /// guards are dropped out of order.
    pub fn scope<K, V, I>(kvs: I) -> ThreadLocalDiagnosticGuard
    where
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
        I: IntoIterator<Item = (K, V)>,
    {
        let entries: Box<[(KeyOwned, ValueOwned)]> =
            kvs.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        if entries.is_empty() {
            return ThreadLocalDiagnosticGuard {
                id: 0,
                not_send: PhantomData,
            };
        }

        let id = THREAD_LOCAL_STATE
            .try_with(|state| {
                let mut state = state.borrow_mut();
                let id = state.next_id;
                state.next_id += 1;
                state.frames.push(ScopeFrame { id, entries });
                id
            })
            .unwrap_or(0);

        ThreadLocalDiagnosticGuard {
            id,
            not_send: PhantomData,
        }
    }
}

impl Diagnostic for ThreadLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        THREAD_LOCAL_STATE
            .try_with(|state| {
                let state = state.borrow();
                for (frame_idx, frame) in state.frames.iter().enumerate() {
                    let inner_frames = &state.frames[frame_idx + 1..];
                    for (entry_idx, (key, value)) in frame.entries.iter().enumerate() {
                        let rest_in_frame = &frame.entries[entry_idx + 1..];
                        if !is_shadowed(key, rest_in_frame, inner_frames) {
                            visitor.visit(key.view(), value.view())?;
                        }
                    }
                }
                Ok(())
            })
            .unwrap_or(Ok(()))
    }
}

fn is_shadowed(
    key: &KeyOwned,
    rest_in_frame: &[(KeyOwned, ValueOwned)],
    inner_frames: &[ScopeFrame],
) -> bool {
    rest_in_frame.iter().any(|(k, _)| k == key)
        || inner_frames
            .iter()
            .any(|frame| frame.entries.iter().any(|(k, _)| k == key))
}
