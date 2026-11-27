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

use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Filter;
use logforth_core::Layout;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::filter::FilterResult;
use logforth_core::kv::Key;
use logforth_core::kv::KeyView;
use logforth_core::kv::Value;
use logforth_core::kv::ValueView;
use logforth_core::kv::Visitor;
use logforth_core::record::Level;
use logforth_core::record::Record;

#[derive(Debug)]
struct ConstDiag {
    key: &'static str,
    value: &'static str,
}

impl Diagnostic for ConstDiag {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit(
            Key::new(self.key).view(),
            Value::static_str(self.value).view(),
        )
    }
}

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
        Err(Error::new("diagnostic failure"))
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

fn collect_diags(diags: &dyn Diagnostic) -> Result<Vec<(String, String)>, Error> {
    let mut entries = Vec::new();
    diags.visit(&mut |key: KeyView, value: ValueView| {
        entries.push((key.as_str().to_owned(), value.to_string()));
        Ok(())
    })?;
    Ok(entries)
}

#[test]
fn composite_diagnostics_preserve_order_and_handle_empty_sources() {
    let none_diag: Option<ConstDiag> = None;
    let none_boxed: Option<Box<dyn Diagnostic>> = None;
    let empty_slice: &[&dyn Diagnostic] = &[];
    let empty_boxed: Vec<Box<dyn Diagnostic>> = Vec::new();

    assert!(collect_diags(&()).unwrap().is_empty());
    assert!(collect_diags(&none_diag).unwrap().is_empty());
    assert!(collect_diags(&none_boxed).unwrap().is_empty());
    assert!(collect_diags(&empty_slice).unwrap().is_empty());
    assert!(collect_diags(&empty_boxed).unwrap().is_empty());

    let first = ConstDiag {
        key: "env",
        value: "prod",
    };
    let second = ConstDiag {
        key: "region",
        value: "us-east",
    };

    assert_eq!(
        collect_diags(&Some(&first)).unwrap(),
        vec![("env".to_owned(), "prod".to_owned())]
    );

    let array_refs: [&dyn Diagnostic; 2] = [&first, &second];
    assert_eq!(
        collect_diags(&array_refs).unwrap(),
        vec![
            ("env".to_owned(), "prod".to_owned()),
            ("region".to_owned(), "us-east".to_owned()),
        ]
    );

    let slice_refs: &[&dyn Diagnostic] = &array_refs;
    assert_eq!(
        collect_diags(&slice_refs).unwrap(),
        vec![
            ("env".to_owned(), "prod".to_owned()),
            ("region".to_owned(), "us-east".to_owned()),
        ]
    );

    let some_boxed: Option<Box<dyn Diagnostic>> = Some(Box::new(ConstDiag {
        key: "host",
        value: "n1",
    }));
    assert_eq!(
        collect_diags(&some_boxed).unwrap(),
        vec![("host".to_owned(), "n1".to_owned())]
    );

    let boxed_array: [Box<dyn Diagnostic>; 2] = [
        Box::new(ConstDiag {
            key: "service",
            value: "api",
        }),
        Box::new(ConstDiag {
            key: "version",
            value: "1",
        }),
    ];
    assert_eq!(
        collect_diags(&boxed_array).unwrap(),
        vec![
            ("service".to_owned(), "api".to_owned()),
            ("version".to_owned(), "1".to_owned()),
        ]
    );

    let boxed_vec = Vec::from(boxed_array);
    assert_eq!(
        collect_diags(&boxed_vec).unwrap(),
        vec![
            ("service".to_owned(), "api".to_owned()),
            ("version".to_owned(), "1".to_owned()),
        ]
    );

    let arc_vec: Vec<Arc<dyn Diagnostic>> = vec![
        Arc::new(ConstDiag {
            key: "cluster",
            value: "c1",
        }),
        Arc::new(ConstDiag {
            key: "zone",
            value: "z2",
        }),
    ];
    assert_eq!(
        collect_diags(&arc_vec).unwrap(),
        vec![
            ("cluster".to_owned(), "c1".to_owned()),
            ("zone".to_owned(), "z2".to_owned()),
        ]
    );
}

#[test]
fn composite_diagnostics_short_circuit_on_error() {
    let failing = FailingDiag {
        calls: AtomicUsize::new(0),
    };
    let trailing = CountingDiag {
        key: "after",
        value: "unreachable",
        calls: AtomicUsize::new(0),
    };

    let err = collect_diags(&[&failing as &dyn Diagnostic, &trailing]).unwrap_err();
    assert_eq!(err.message(), "diagnostic failure");
    assert_eq!(failing.calls.load(Ordering::Relaxed), 1);
    assert_eq!(trailing.calls.load(Ordering::Relaxed), 0);
}

#[derive(Debug)]
struct ScopedDecorator<A> {
    inner: A,
}

impl<A: Append> Append for ScopedDecorator<A> {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let key = String::from("request_id");
        let value = format!("req-{}", record.payload());
        let extra = ScopedDiag {
            key: &key,
            value: &value,
        };
        let chained: [&dyn Diagnostic; 2] = [diags, &extra];
        self.inner.append(record, &chained)
    }

    fn flush(&self) -> Result<(), Error> {
        self.inner.flush()
    }
}

#[test]
fn decorator_chains_borrowed_non_static_context_onto_dispatch_diagnostics() {
    let sink = Arc::new(RecordingAppend::new(PrefixLayout { prefix: "dec" }));
    let logger = logforth_core::builder()
        .dispatch(|d| {
            d.diagnostic(ConstDiag {
                key: "service",
                value: "billing",
            })
            .diagnostic(ConstDiag {
                key: "env",
                value: "staging",
            })
            .append(ScopedDecorator {
                inner: sink.clone(),
            })
        })
        .build();

    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("42"))
            .build(),
    );

    assert_eq!(
        sink.lines(),
        vec!["dec:42 service=billing env=staging request_id=req-42".to_owned()]
    );
}

#[derive(Debug)]
struct MatchOverrideFilter {
    blocked_substring: &'static str,
}

impl Filter for MatchOverrideFilter {
    fn filter_level(&self, _: Level) -> FilterResult {
        FilterResult::Neutral
    }

    fn filter(&self, record: &Record, _: &dyn Diagnostic) -> FilterResult {
        if record
            .payload()
            .to_string()
            .contains(self.blocked_substring)
        {
            FilterResult::Reject
        } else {
            FilterResult::Neutral
        }
    }
}

#[derive(Debug)]
struct PrefixLayout {
    prefix: &'static str,
}

impl Layout for PrefixLayout {
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        use std::io::Write;

        write!(buf, "{}:{}", self.prefix, record.payload()).unwrap();
        diags.visit(&mut |key: KeyView, value: ValueView| {
            write!(buf, " {key}={value}").unwrap();
            Ok(())
        })
    }
}

#[derive(Debug)]
struct RecordingAppend {
    layout: Box<dyn Layout>,
    lines: Mutex<Vec<String>>,
    flushes: AtomicUsize,
}

impl RecordingAppend {
    fn new(layout: impl Into<Box<dyn Layout>>) -> Self {
        Self {
            layout: layout.into(),
            lines: Mutex::new(Vec::new()),
            flushes: AtomicUsize::new(0),
        }
    }

    fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
}

impl Append for RecordingAppend {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error> {
        let mut bytes = Vec::new();
        self.layout.format(record, diags, &mut bytes)?;
        self.lines
            .lock()
            .unwrap()
            .push(String::from_utf8(bytes).unwrap());
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[test]
fn arc_and_static_forwarding_share_components_across_dispatches() {
    static STATIC_FILTER: MatchOverrideFilter = MatchOverrideFilter {
        blocked_substring: "blocked-static",
    };
    static STATIC_LAYOUT: PrefixLayout = PrefixLayout { prefix: "static" };
    static STATIC_DIAG: LazyLock<StaticDiagnostic> = LazyLock::new(|| {
        let mut diag = StaticDiagnostic::default();
        diag.insert("origin", "static");
        diag
    });
    static STATIC_APPEND: LazyLock<RecordingAppend> =
        LazyLock::new(|| RecordingAppend::new(&STATIC_LAYOUT));

    let shared_filter: Arc<dyn Filter> = Arc::new(MatchOverrideFilter {
        blocked_substring: "blocked-arc",
    });
    let shared_layout = Arc::new(PrefixLayout { prefix: "arc" });
    let shared_diag = Arc::new(ConstDiag {
        key: "tenant",
        value: "acme",
    });
    let shared_append = Arc::new(RecordingAppend::new(shared_layout));

    let logger = logforth_core::builder()
        .dispatch(|d| {
            d.filter(shared_filter.clone())
                .diagnostic(shared_diag.clone())
                .append(shared_append.clone())
        })
        .dispatch(|d| {
            let append_dyn: Arc<dyn Append> = shared_append.clone();
            let diag_dyn: Arc<dyn Diagnostic> = shared_diag.clone();
            d.filter(shared_filter.clone())
                .diagnostic(diag_dyn)
                .append(append_dyn)
        })
        .dispatch(|d| {
            d.filter(&STATIC_FILTER)
                .diagnostic(&*STATIC_DIAG)
                .append(&*STATIC_APPEND)
        })
        .build();

    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("accepted"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("blocked-arc"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("blocked-static"))
            .build(),
    );
    logger.flush();

    assert_eq!(
        shared_append.lines(),
        vec![
            "arc:accepted tenant=acme".to_owned(),
            "arc:accepted tenant=acme".to_owned(),
            "arc:blocked-static tenant=acme".to_owned(),
            "arc:blocked-static tenant=acme".to_owned(),
        ]
    );
    assert_eq!(shared_append.flushes.load(Ordering::Relaxed), 2);

    assert_eq!(
        STATIC_APPEND.lines(),
        vec![
            "static:accepted origin=static".to_owned(),
            "static:blocked-arc origin=static".to_owned(),
        ]
    );
    assert_eq!(STATIC_APPEND.flushes.load(Ordering::Relaxed), 1);
}
