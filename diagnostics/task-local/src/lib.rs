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

//! A diagnostic that stores key-value pairs in a task-local context stack.
//!
//! # Examples
//!
//! ```
//! use logforth_core::Diagnostic;
//! use logforth_core::kv::Visitor;
//! use logforth_diagnostic_task_local::FutureExt;
//!
//! let fut = async { log::info!("Hello, world!") };
//! fut.with_task_local_context([("key", "value")]);
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::cell::RefCell;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::KeyOwned;
use logforth_core::kv::ValueOwned;
use logforth_core::kv::Visitor;

type TaskLocalContext = Arc<[(KeyOwned, ValueOwned)]>;

thread_local! {
    static TASK_LOCAL_MAP: RefCell<Vec<TaskLocalContext>> = const { RefCell::new(Vec::new()) };
}

/// A diagnostic that stores key-value pairs in a task-local context.
///
/// Nested [`FutureExt::with_task_local_context`] scopes shadow outer values for the same key and
/// restore them when the inner future completes.
///
/// See the [crate documentation](self) for usage examples.
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct TaskLocalDiagnostic {}

impl Diagnostic for TaskLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        TASK_LOCAL_MAP
            .try_with(|map| {
                let frames = map.borrow();
                for (frame_idx, frame) in frames.iter().enumerate() {
                    let inner_frames = &frames[frame_idx + 1..];
                    for (entry_idx, (key, value)) in frame.iter().enumerate() {
                        let rest_in_frame = &frame[entry_idx + 1..];
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
    inner_frames: &[TaskLocalContext],
) -> bool {
    rest_in_frame.iter().any(|(k, _)| k == key)
        || inner_frames
            .iter()
            .any(|frame| frame.iter().any(|(k, _)| k == key))
}

/// An extension trait for futures to run them with a task-local context.
///
/// See the [crate documentation](self) for usage examples.
pub trait FutureExt: Future {
    /// Run a future with a task-local context.
    fn with_task_local_context<K, V, I>(self, kvs: I) -> impl Future<Output = Self::Output>
    where
        Self: Sized,
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
        I: IntoIterator<Item = (K, V)>,
    {
        let future = self;
        let context = kvs.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        TaskLocalFuture { future, context }
    }
}

impl<F: Future> FutureExt for F {}

#[pin_project::pin_project]
struct TaskLocalFuture<F> {
    #[pin]
    future: F,
    context: Arc<[(KeyOwned, ValueOwned)]>,
}

impl<F: Future> Future for TaskLocalFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        struct Guard;

        impl Drop for Guard {
            fn drop(&mut self) {
                let _ = TASK_LOCAL_MAP.try_with(|map| {
                    map.borrow_mut().pop();
                });
            }
        }

        let this = self.project();
        let _guard = TASK_LOCAL_MAP
            .try_with(|map| {
                map.borrow_mut().push(this.context.clone());
                Guard
            })
            .ok();
        this.future.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use logforth_core::kv::KeyView;
    use logforth_core::kv::ValueView;

    use super::*;

    #[test]
    fn test_task_local_diagnostic_shadowing_and_restore() {
        let diag = TaskLocalDiagnostic {};
        let collect = || {
            let mut entries = Vec::new();
            diag.visit(&mut |key: KeyView<'_>, value: ValueView<'_>| {
                entries.push((key.as_str().to_owned(), value.to_string()));
                Ok(())
            })
            .unwrap();
            entries
        };

        let fut = async {
            assert_eq!(
                collect(),
                [
                    ("tenant".to_owned(), "acme".to_owned()),
                    ("key".to_owned(), "v1".to_owned()),
                ]
            );
            async {
                assert_eq!(
                    collect(),
                    [
                        ("tenant".to_owned(), "acme".to_owned()),
                        ("key".to_owned(), "v2".to_owned()),
                        ("step".to_owned(), "inner".to_owned()),
                    ]
                );
            }
            .with_task_local_context([("key", "v2"), ("step", "inner")])
            .await;
            assert_eq!(
                collect(),
                [
                    ("tenant".to_owned(), "acme".to_owned()),
                    ("key".to_owned(), "v1".to_owned()),
                ]
            );
        }
        .with_task_local_context([("tenant", "acme"), ("key", "v1")]);

        pollster::block_on(fut);
    }
}
