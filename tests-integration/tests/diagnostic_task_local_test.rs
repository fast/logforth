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
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::pin::pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread;

use logforth::Diagnostic;
use logforth::Error;
use logforth::Logger;
use logforth::LoggerBuilder;
use logforth::diagnostic::TaskLocalDiagnostic;
use logforth::diagnostic::task_local::FutureExt;
use logforth::diagnostic::task_local::TaskLocalFuture;
use logforth::kv::KeyView;
use logforth::kv::ValueOwned;
use logforth::kv::ValueView;
use logforth::layout::JsonLayout;
use tests_integration::RecordingAppender;

const CONST_TASK_DIAG: TaskLocalDiagnostic = TaskLocalDiagnostic::new();

#[derive(Default)]
struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            Poll::Pending
        }
    }
}

fn block_on_ready<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = fut.as_mut().poll(&mut cx) {
            return out;
        }
    }
}

#[test]
fn interleaved_tasks_isolate_contexts_across_yield_points_and_threads() {
    let sink = RecordingAppender::with_layout(JsonLayout::default());
    let logger = LoggerBuilder::new()
        .dispatch(|d| d.diagnostic(CONST_TASK_DIAG.clone()).append(sink.clone()))
        .build();

    let make_worker = |logger: Logger, task_id: u64, tenant: &'static str| {
        async move {
            logforth::info!(logger, "before-yield");
            YieldOnce::default().await;
            logforth::info!(logger, "after-yield");
            task_id
        }
        .with_task_local_context([
            ("task_id", ValueOwned::from(task_id)),
            ("tenant", ValueOwned::from(tenant)),
        ])
    };

    let task_a: TaskLocalFuture<_> = make_worker(logger.clone(), 1, "alpha");
    let task_b: TaskLocalFuture<_> = make_worker(logger.clone(), 2, "beta");
    let mut task_a = pin!(task_a);
    let mut task_b = pin!(task_b);
    let mut cx = Context::from_waker(Waker::noop());

    // Poll A to `Pending`, then B to `Pending`, verify outside-of-poll context is empty, then
    // resume B and A in reverse order.
    assert_eq!(task_a.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(task_b.as_mut().poll(&mut cx), Poll::Pending);
    logforth::info!(logger, "outside-poll");
    assert_eq!(task_b.as_mut().poll(&mut cx), Poll::Ready(2));
    assert_eq!(task_a.as_mut().poll(&mut cx), Poll::Ready(1));

    // Also verify `TaskLocalFuture` is `Send` and isolated across OS threads.
    let thread_task = make_worker(logger.clone(), 3, "gamma");
    let out = thread::spawn(move || block_on_ready(thread_task))
        .join()
        .unwrap();
    assert_eq!(out, 3);

    let events = sink.events();
    assert_eq!(events.len(), 7);
    assert_eq!(
        events[0].diagnostics,
        [
            ("task_id".to_owned(), "1".to_owned()),
            ("tenant".to_owned(), "alpha".to_owned()),
        ]
    );
    assert_eq!(
        events[1].diagnostics,
        [
            ("task_id".to_owned(), "2".to_owned()),
            ("tenant".to_owned(), "beta".to_owned()),
        ]
    );
    assert!(events[2].diagnostics.is_empty());
    assert_eq!(
        events[3].diagnostics,
        [
            ("task_id".to_owned(), "2".to_owned()),
            ("tenant".to_owned(), "beta".to_owned()),
        ]
    );
    assert_eq!(
        events[4].diagnostics,
        [
            ("task_id".to_owned(), "1".to_owned()),
            ("tenant".to_owned(), "alpha".to_owned()),
        ]
    );
    assert_eq!(
        events[5].diagnostics,
        [
            ("task_id".to_owned(), "3".to_owned()),
            ("tenant".to_owned(), "gamma".to_owned()),
        ]
    );

    // Verify `JsonLayout` emitted typed numeric `1` (not `"1"`), and `visit` yields `U64`.
    assert!(
        sink.lines()[0].contains("\"task_id\":1"),
        "expected numeric task_id in JSON line: {}",
        sink.lines()[0]
    );
    block_on_ready(
        async {
            let mut saw_typed_u64 = false;
            CONST_TASK_DIAG
                .visit(&mut |k: KeyView, v: ValueView| {
                    if k.as_str() == "task_id" {
                        assert_eq!(v.to_u64(), Some(7));
                        assert_eq!(v.to_str(), None);
                        saw_typed_u64 = true;
                    }
                    Ok(())
                })
                .unwrap();
            assert!(saw_typed_u64);
        }
        .with_task_local_context([("task_id", 7_u64)]),
    );
}

#[test]
fn nested_scopes_deduplicate_shadow_restore_and_recover_from_panics_and_visitor_errors() {
    let sink = RecordingAppender::default();
    let logger = LoggerBuilder::new()
        .dispatch(|d| {
            d.diagnostic(TaskLocalDiagnostic::default())
                .append(sink.clone())
        })
        .build();

    // Cloneable `ready` future exercises `Clone` on `TaskLocalFuture`.
    let cloned_ready = std::future::ready(42_u32)
        .with_task_local_context([("cloned", "ok")])
        .clone();
    assert_eq!(block_on_ready(cloned_ready), 42);

    block_on_ready(
        async {
            logforth::info!(logger, "outer-before");

            async {
                YieldOnce::default().await;
                logforth::info!(logger, "inner");
            }
            .with_task_local_context([("req", "inner"), ("step", "db")])
            .await;

            // Empty inner scope does not clobber outer entries.
            async {
                logforth::info!(logger, "empty-inner");
            }
            .with_task_local_context(std::iter::empty::<(&str, &str)>())
            .await;

            // Panicking child future still pops its frame via `Guard::drop`.
            let panic_res = catch_unwind(AssertUnwindSafe(|| {
                block_on_ready(
                    async {
                        panic!("boom inside task scope");
                    }
                    .with_task_local_context([("req", "panicked"), ("leaked", "no")]),
                );
            }));
            assert!(panic_res.is_err());

            logforth::info!(logger, "outer-after");

            // Visitor errors propagate cleanly without corrupting the frame stack.
            let mut visited = 0usize;
            let err = CONST_TASK_DIAG
                .visit(&mut |_: KeyView, _: ValueView| {
                    visited += 1;
                    Err(Error::new("stop visit"))
                })
                .unwrap_err();
            assert_eq!(visited, 1);
            assert_eq!(err.message(), "stop visit");
        }
        .with_task_local_context([
            ("dup", "first"),
            ("tenant", "acme"),
            ("dup", "second"),
            ("req", "outer"),
        ]),
    );

    let events = sink.events();
    assert_eq!(events.len(), 4);

    let expected_outer = [
        ("tenant".to_owned(), "acme".to_owned()),
        ("dup".to_owned(), "second".to_owned()),
        ("req".to_owned(), "outer".to_owned()),
    ];
    assert_eq!(events[0].diagnostics, expected_outer);
    assert_eq!(
        events[1].diagnostics,
        [
            ("tenant".to_owned(), "acme".to_owned()),
            ("dup".to_owned(), "second".to_owned()),
            ("req".to_owned(), "inner".to_owned()),
            ("step".to_owned(), "db".to_owned()),
        ]
    );
    assert_eq!(events[2].diagnostics, expected_outer);
    assert_eq!(events[3].diagnostics, expected_outer);
}

#[test]
fn task_local_diagnostic_degrades_to_empty_during_tls_destructor_teardown() {
    struct TeardownPoller {
        logger: Logger,
        tag: &'static str,
    }

    impl Drop for TeardownPoller {
        fn drop(&mut self) {
            let logger = self.logger.clone();
            let tag = self.tag;
            block_on_ready(
                async move {
                    logforth::info!(logger, "dtor-{tag}");
                }
                .with_task_local_context([("teardown", tag)]),
            );
        }
    }

    thread_local! {
        static BEFORE_TASK: RefCell<Option<TeardownPoller>> = const { RefCell::new(None) };
        static AFTER_TASK: RefCell<Option<TeardownPoller>> = const { RefCell::new(None) };
    }

    let sink = RecordingAppender::default();
    let logger = LoggerBuilder::new()
        .dispatch(|d| {
            d.diagnostic(TaskLocalDiagnostic::new())
                .append(sink.clone())
        })
        .build();

    let thread_logger = logger.clone();
    thread::spawn(move || {
        BEFORE_TASK.with(|slot| {
            *slot.borrow_mut() = Some(TeardownPoller {
                logger: thread_logger.clone(),
                tag: "before",
            });
        });

        block_on_ready(async {}.with_task_local_context([("init", "1")]));

        AFTER_TASK.with(|slot| {
            *slot.borrow_mut() = Some(TeardownPoller {
                logger: thread_logger,
                tag: "after",
            });
        });
    })
    .join()
    .expect("TLS destructor task polling and logging must not panic");

    let mut payloads = sink.payloads();
    payloads.sort();
    assert_eq!(payloads, ["dtor-after", "dtor-before"]);
}
