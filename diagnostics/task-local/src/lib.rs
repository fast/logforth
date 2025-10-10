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
//! fut.with_task_local_context([("key".into(), "value".into())]);
//! ```

use std::cell::RefCell;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;

thread_local! {
    static TASK_LOCAL_MAP: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// A diagnostic that stores key-value pairs in a task-local context.
///
/// See [module-level documentation](self) for usage examples.
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TaskLocalDiagnostic {}

impl Diagnostic for TaskLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        TASK_LOCAL_MAP.with(|map| {
            let map = map.borrow();
            for (key, value) in map.iter() {
                let key = Key::new_ref(key.as_str());
                let value = Value::from(value.as_str());
                visitor.visit(key, value)?;
            }
            Ok(())
        })
    }
}

/// An extension trait for futures to run them with a task-local context.
///
/// See [module-level documentation](self) for usage examples.
pub trait FutureExt: Future {
    /// Run a future with a task-local context.
    fn with_task_local_context(
        self,
        kvs: impl IntoIterator<Item = (String, String)>,
    ) -> impl Future<Output = Self::Output>
    where
        Self: Sized,
    {
        TaskLocalFuture {
            future: Some(self),
            context: kvs.into_iter().collect(),
        }
    }
}

impl<F: Future> FutureExt for F {}

#[pin_project::pin_project]
struct TaskLocalFuture<F> {
    #[pin]
    future: Option<F>,
    context: Vec<(String, String)>,
}

impl<F: Future> Future for TaskLocalFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let mut fut = this.future;
        if let Some(future) = fut.as_mut().as_pin_mut() {
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
                for (key, value) in this.context.iter() {
                    map.push((key.clone(), value.clone()));
                }
            });

            let n = this.context.len();
            let guard = Guard { n };

            let result = match future.poll(cx) {
                Poll::Ready(output) => {
                    fut.set(None);
                    Poll::Ready(output)
                }
                Poll::Pending => Poll::Pending,
            };

            drop(guard);
            return result;
        }

        unreachable!("TaskLocalFuture polled after completion");
    }
}
