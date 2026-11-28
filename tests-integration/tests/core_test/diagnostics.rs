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
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::builder;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::diagnostic::ThreadLocalDiagnostic;
use logforth_core::diagnostic::ThreadLocalDiagnosticGuard;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::record::Record;
use tests_integration::RecordingAppender;
use tests_integration::collect_diagnostics;

#[derive(Debug)]
struct ScopedDiag<'a> {
    key: &'a str,
    value: &'a str,
}

impl Diagnostic for ScopedDiag<'_> {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit(
            Key::borrowed(self.key).view(),
            Value::str(self.value).view(),
        )
    }
}

#[derive(Debug)]
struct FailingDiag {
    calls: AtomicUsize,
}

impl Diagnostic for FailingDiag {
    fn visit(&self, _: &mut dyn Visitor) -> Result<(), Error> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Err(Error::new("diagnostic read failed"))
    }
}

#[derive(Debug)]
struct CountingDiag {
    key: &'static str,
    value: &'static str,
    calls: AtomicUsize,
}

impl Diagnostic for CountingDiag {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        visitor.visit(
            Key::new(self.key).view(),
            Value::static_str(self.value).view(),
        )
    }
}

#[derive(Debug)]
struct ScopedDecorator<A> {
    inner: A,
}

impl<A: Append> Append for ScopedDecorator<A> {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let dynamic_key = String::from("request_id");
        let dynamic_val = format!("req-{}", record.payload());
        let extra = ScopedDiag {
            key: &dynamic_key,
            value: &dynamic_val,
        };
        let chained: [&dyn Diagnostic; 2] = [diags, &extra];
        self.inner.append(record, &chained)
    }

    fn flush(&self) -> Result<(), Error> {
        self.inner.flush()
    }
}

#[test]
fn composite_diagnostics_visit_in_order_chain_non_static_borrows_and_short_circuit() {
    let none_diag: Option<StaticDiagnostic> = None;
    let none_boxed: Option<Box<dyn Diagnostic>> = None;
    let empty_slice: &[&dyn Diagnostic] = &[];
    let empty_boxed: Vec<Box<dyn Diagnostic>> = Vec::new();

    assert!(collect_diagnostics(&()).unwrap().is_empty());
    assert!(collect_diagnostics(&none_diag).unwrap().is_empty());
    assert!(collect_diagnostics(&none_boxed).unwrap().is_empty());
    assert!(collect_diagnostics(&empty_slice).unwrap().is_empty());
    assert!(collect_diagnostics(&empty_boxed).unwrap().is_empty());

    let first = StaticDiagnostic::new([("env", "prod")]);
    let second = StaticDiagnostic::new([("region", "us-east")]);

    let array_refs: [&dyn Diagnostic; 2] = [&first, &second];
    assert_eq!(
        collect_diagnostics(&array_refs).unwrap(),
        [
            ("env".to_owned(), "prod".to_owned()),
            ("region".to_owned(), "us-east".to_owned()),
        ]
    );
    let slice_refs: &[&dyn Diagnostic] = &array_refs;
    assert_eq!(
        collect_diagnostics(&slice_refs).unwrap(),
        [
            ("env".to_owned(), "prod".to_owned()),
            ("region".to_owned(), "us-east".to_owned()),
        ]
    );

    let some_boxed: Option<Box<dyn Diagnostic>> = Some(Box::new(first.clone()));
    assert_eq!(
        collect_diagnostics(&some_boxed).unwrap(),
        [("env".to_owned(), "prod".to_owned())]
    );

    let boxed_array: [Box<dyn Diagnostic>; 2] = [Box::new(first), Box::new(second)];
    assert_eq!(
        collect_diagnostics(&boxed_array).unwrap(),
        [
            ("env".to_owned(), "prod".to_owned()),
            ("region".to_owned(), "us-east".to_owned()),
        ]
    );
    let boxed_slice: &[Box<dyn Diagnostic>] = &boxed_array;
    assert_eq!(collect_diagnostics(&boxed_slice).unwrap().len(), 2);
    let boxed_vec = Vec::from(boxed_array);
    assert_eq!(collect_diagnostics(&boxed_vec).unwrap().len(), 2);

    // Boxed diagnostic containers also accept non-`'static` stack-borrowed trait objects.
    let dynamic_key = String::from("scoped_box");
    let dynamic_val = String::from("local");
    let borrowed_boxes: Vec<Box<dyn Diagnostic + '_>> = vec![Box::new(ScopedDiag {
        key: &dynamic_key,
        value: &dynamic_val,
    })];
    assert_eq!(
        collect_diagnostics(&borrowed_boxes).unwrap(),
        [("scoped_box".to_owned(), "local".to_owned())]
    );

    // Decorator chains stack-borrowed non-'static diagnostics onto dispatch diagnostics.
    let sink = RecordingAppender::default();
    let logger = builder()
        .dispatch(|d| {
            d.diagnostic(StaticDiagnostic::new([("service", "billing")]))
                .append(ScopedDecorator {
                    inner: sink.clone(),
                })
        })
        .build();
    logger.log(&Record::builder().payload(format_args!("42")).build());
    assert_eq!(
        sink.events()[0].diagnostics,
        [
            ("service".to_owned(), "billing".to_owned()),
            ("request_id".to_owned(), "req-42".to_owned()),
        ]
    );

    // Errors short-circuit remaining diagnostic sources.
    let failing = FailingDiag {
        calls: AtomicUsize::new(0),
    };
    let trailing = CountingDiag {
        key: "after",
        value: "unreachable",
        calls: AtomicUsize::new(0),
    };
    let err = collect_diagnostics(&[&failing as &dyn Diagnostic, &trailing]).unwrap_err();
    assert_eq!(err.message(), "diagnostic read failed");
    assert_eq!(failing.calls.load(Ordering::Relaxed), 1);
    assert_eq!(trailing.calls.load(Ordering::Relaxed), 0);
}

#[test]
fn static_diagnostic_orders_keys_and_supports_insert_replace_and_remove() {
    let mut diag = StaticDiagnostic::new([("z_key", 1_i32), ("a_key", 2_i32)]);
    diag.insert("m_key", "mid");
    diag.insert("a_key", 20_i32);
    diag.remove("z_key");
    diag.remove("nonexistent");

    assert_eq!(
        collect_diagnostics(&diag).unwrap(),
        [
            ("a_key".to_owned(), "20".to_owned()),
            ("m_key".to_owned(), "mid".to_owned()),
        ]
    );
}

#[test]
fn thread_local_scopes_deduplicate_shadow_restore_out_of_order_and_isolate_threads() {
    let diag = ThreadLocalDiagnostic::default();

    let empty_guard = ThreadLocalDiagnostic::scope(std::iter::empty::<(&str, &str)>());
    assert!(collect_diagnostics(&diag).unwrap().is_empty());
    drop(empty_guard);

    // Duplicate keys within a single scope frame keep only the last entry.
    let outer = ThreadLocalDiagnostic::scope([
        ("dup", "first"),
        ("tenant", "acme"),
        ("dup", "second"),
        ("request", "outer"),
    ]);
    assert_eq!(
        collect_diagnostics(&diag).unwrap(),
        [
            ("tenant".to_owned(), "acme".to_owned()),
            ("dup".to_owned(), "second".to_owned()),
            ("request".to_owned(), "outer".to_owned()),
        ]
    );

    // Other threads do not observe the current thread's diagnostic scopes.
    let barrier = Arc::new(Barrier::new(2));
    let worker_barrier = barrier.clone();
    let worker = thread::spawn(move || {
        let worker_diag = ThreadLocalDiagnostic::default();
        assert!(collect_diagnostics(&worker_diag).unwrap().is_empty());
        let _worker_scope = ThreadLocalDiagnostic::scope([("tenant", "worker-only")]);
        worker_barrier.wait();
        assert_eq!(
            collect_diagnostics(&worker_diag).unwrap(),
            [("tenant".to_owned(), "worker-only".to_owned())]
        );
    });
    barrier.wait();

    // Nested scope shadows `request` and restores on drop.
    let inner = ThreadLocalDiagnostic::scope([("request", "inner"), ("step", "auth")]);
    assert_eq!(
        collect_diagnostics(&diag).unwrap(),
        [
            ("tenant".to_owned(), "acme".to_owned()),
            ("dup".to_owned(), "second".to_owned()),
            ("request".to_owned(), "inner".to_owned()),
            ("step".to_owned(), "auth".to_owned()),
        ]
    );
    drop(inner);
    assert_eq!(
        collect_diagnostics(&diag).unwrap(),
        [
            ("tenant".to_owned(), "acme".to_owned()),
            ("dup".to_owned(), "second".to_owned()),
            ("request".to_owned(), "outer".to_owned()),
        ]
    );
    drop(outer);
    assert!(collect_diagnostics(&diag).unwrap().is_empty());
    worker.join().unwrap();

    // Out-of-order guard drop removes the outer frame while retaining the inner frame.
    let outer = ThreadLocalDiagnostic::scope([("tenant", "acme"), ("request", "outer")]);
    let inner = ThreadLocalDiagnostic::scope([("request", "inner")]);
    drop(outer);
    assert_eq!(
        collect_diagnostics(&diag).unwrap(),
        [("request".to_owned(), "inner".to_owned())]
    );
    drop(inner);
    assert!(collect_diagnostics(&diag).unwrap().is_empty());
}

#[test]
fn thread_local_diagnostic_degrades_to_empty_during_tls_destructor_teardown() {
    struct TeardownLogger {
        logger: logforth_core::Logger,
        held_guard: Option<ThreadLocalDiagnosticGuard>,
        tag: &'static str,
    }

    impl Drop for TeardownLogger {
        fn drop(&mut self) {
            drop(self.held_guard.take());
            let _ephemeral = ThreadLocalDiagnostic::scope([("teardown", self.tag)]);
            self.logger.log(
                &Record::builder()
                    .payload(format_args!("dtor-{}", self.tag))
                    .build(),
            );
        }
    }

    thread_local! {
        static BEFORE_DIAG: RefCell<Option<TeardownLogger>> = const { RefCell::new(None) };
        static AFTER_DIAG: RefCell<Option<TeardownLogger>> = const { RefCell::new(None) };
    }

    let sink = RecordingAppender::default();
    let logger = builder()
        .dispatch(|d| {
            d.diagnostic(ThreadLocalDiagnostic::default())
                .append(sink.clone())
        })
        .build();

    let thread_logger = logger.clone();
    // Register destructors both before and after `ThreadLocalDiagnostic::scope` initializes its
    // own TLS key so at least one destructor executes after `THREAD_LOCAL_STATE` is destroyed,
    // regardless of platform TLS destruction order.
    thread::spawn(move || {
        BEFORE_DIAG.with(|slot| {
            *slot.borrow_mut() = Some(TeardownLogger {
                logger: thread_logger.clone(),
                held_guard: None,
                tag: "before",
            });
        });

        let guard = ThreadLocalDiagnostic::scope([("active", "yes")]);

        AFTER_DIAG.with(|slot| {
            *slot.borrow_mut() = Some(TeardownLogger {
                logger: thread_logger,
                held_guard: Some(guard),
                tag: "after",
            });
        });
    })
    .join()
    .expect("TLS destructor logging and guard drop must not panic");

    let mut payloads = sink.payloads();
    payloads.sort();
    assert_eq!(payloads, ["dtor-after", "dtor-before"]);
}
