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

//! Runtime-agnostic task-local diagnostic context for [`Future`]s.
//!
//! [`FutureExt::with_task_local_context`] wraps a [`Future`] in a [`TaskLocalFuture`] that pushes
//! structured `(KeyOwned, ValueOwned)` entries onto a thread-local stack for the duration of each
//! [`Future::poll`] call and pops them whenever the future yields or completes. Register
//! [`TaskLocalDiagnostic`] on a dispatch branch to expose the active task's scoped key-values to
//! filters, layouts, and appenders across `.await` points on any async executor.
//!
//! # Examples
//!
//! ```
//! use std::pin::pin;
//! use std::task::Context;
//! use std::task::Poll;
//! use std::task::Waker;
//!
//! use logforth_core::LoggerBuilder;
//! use logforth_core::append::Testing;
//! use logforth_diagnostic_task_local::FutureExt;
//! use logforth_diagnostic_task_local::TaskLocalDiagnostic;
//!
//! let logger = LoggerBuilder::new()
//!     .dispatch(|d| {
//!         d.diagnostic(TaskLocalDiagnostic::new())
//!             .append(Testing::new())
//!     })
//!     .build();
//!
//! let fut = async {
//!     logforth_core::info!(logger, "processing job");
//! }
//! .with_task_local_context([("job_id", "job-7")]);
//!
//! let mut cx = Context::from_waker(Waker::noop());
//! assert_eq!(pin!(fut).poll(&mut cx), Poll::Ready(()));
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

/// A [`Diagnostic`] that visits key-value pairs attached to the currently polled
/// [`TaskLocalFuture`].
///
/// Active entries are scoped through [`FutureExt::with_task_local_context`]. Nested scopes shadow
/// outer values for the same key and restore them when the inner future completes. If thread-local
/// storage is already destroyed on the current thread during teardown, visiting degrades to an
/// empty context instead of panicking.
///
/// # Examples
///
/// ```
/// use logforth_diagnostic_task_local::TaskLocalDiagnostic;
///
/// const DIAG: TaskLocalDiagnostic = TaskLocalDiagnostic::new();
/// # let _ = DIAG;
/// ```
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct TaskLocalDiagnostic {}

impl TaskLocalDiagnostic {
    /// Creates a new [`TaskLocalDiagnostic`].
    pub const fn new() -> Self {
        Self {}
    }
}

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

/// Extension trait attaching scoped task-local diagnostic key-values to a [`Future`].
pub trait FutureExt: Future {
    /// Wraps `self` so `kvs` are visible to [`TaskLocalDiagnostic`] whenever the returned
    /// [`TaskLocalFuture`] is polled.
    ///
    /// Nested scopes shadow outer values for the same key and restore them when the inner future
    /// completes; duplicate keys within `kvs` resolve with last-entry-wins precedence.
    fn with_task_local_context<K, V, I>(self, kvs: I) -> TaskLocalFuture<Self>
    where
        Self: Sized,
        K: Into<KeyOwned>,
        V: Into<ValueOwned>,
        I: IntoIterator<Item = (K, V)>,
    {
        let context = kvs.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        TaskLocalFuture {
            future: self,
            context,
        }
    }
}

impl<F: Future> FutureExt for F {}

/// A [`Future`] adapter created by [`FutureExt::with_task_local_context`] that installs task-local
/// diagnostic entries while polling the inner future.
#[pin_project::pin_project]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug, Clone)]
pub struct TaskLocalFuture<F> {
    #[pin]
    future: F,
    context: TaskLocalContext,
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
        if this.context.is_empty() {
            return this.future.poll(cx);
        }

        let _guard = TASK_LOCAL_MAP
            .try_with(|map| {
                map.borrow_mut().push(this.context.clone());
                Guard
            })
            .ok();
        this.future.poll(cx)
    }
}
