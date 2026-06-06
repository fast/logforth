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

//! A diagnostic that stores key-value pairs in a task-local map.
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
/// See the [crate documentation](self) for usage examples.
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TaskLocalDiagnostic {}

impl Diagnostic for TaskLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        TASK_LOCAL_MAP.with(|map| {
            let map = map.borrow();
            for context in map.iter() {
                for (key, value) in context.iter() {
                    visitor.visit(key.view(), value.view())?;
                }
            }
            Ok(())
        })
    }
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
        let this = self.project();

        let future = this.future;

        struct Guard {
            n: usize,
        }

        impl Drop for Guard {
            fn drop(&mut self) {
                TASK_LOCAL_MAP.with(|map| {
                    let mut map = map.borrow_mut();
                    for _ in 0..self.n {
                        map.pop();
                    }
                });
            }
        }

        TASK_LOCAL_MAP.with(|map| {
            let mut map = map.borrow_mut();
            map.push(this.context.clone());
        });

        let n = this.context.len();
        let guard = Guard { n };

        let result = future.poll(cx);

        drop(guard);
        result
    }
}

#[cfg(test)]
mod tests {
    use logforth_core::kv::KeyView;
    use logforth_core::kv::ValueView;

    use super::*;

    #[test]
    fn test_task_local_diagnostic() {
        let diag = TaskLocalDiagnostic {};
        let fut = async {
            diag.visit(&mut |key: KeyView<'_>, value: ValueView<'_>| {
                assert_eq!(key.as_str(), "key");
                assert_eq!(value.to_str().unwrap(), "value");
                Ok(())
            })
            .unwrap();
        };
        pollster::block_on(fut.with_task_local_context([("key", "value")]));
    }
}
