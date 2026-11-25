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
use logforth_core::Diagnostics;
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
use logforth_core::record::FilterCriteria;
use logforth_core::record::Level;
use logforth_core::record::Record;

const _: () = {
    const fn assert_copy_send_sync<T: Copy + Send + Sync>() {}
    assert_copy_send_sync::<Diagnostics<'_>>();
};

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

fn collect_diags(diags: Diagnostics) -> Result<Vec<(String, String)>, Error> {
    let mut entries = Vec::new();
    diags.visit(&mut |key: KeyView, value: ValueView| {
        entries.push((key.as_str().to_owned(), value.to_string()));
        Ok(())
    })?;
    Ok(entries)
}

#[test]
fn diagnostics_views_normalize_empty_and_preserve_order() {
    let empty_boxed: [Box<dyn Diagnostic>; 0] = [];
    let empty_slice: [&dyn Diagnostic; 0] = [];

    for view in [
        Diagnostics::empty(),
        Diagnostics::default(),
        Diagnostics::from_boxed(&empty_boxed),
        Diagnostics::from_slice(&empty_slice),
    ] {
        assert!(view.is_empty());
        assert!(collect_diags(view).unwrap().is_empty());
    }

    let first = ConstDiag {
        key: "env",
        value: "prod",
    };
    let second = ConstDiag {
        key: "region",
        value: "us-east",
    };

    let single = Diagnostics::from_single(&first);
    assert!(!single.is_empty());
    assert_eq!(
        collect_diags(single).unwrap(),
        vec![("env".to_owned(), "prod".to_owned())]
    );

    let slice_refs: [&dyn Diagnostic; 2] = [&first, &second];
    let slice_view = Diagnostics::from_slice(&slice_refs);
    assert!(!slice_view.is_empty());
    assert_eq!(
        collect_diags(slice_view).unwrap(),
        vec![
            ("env".to_owned(), "prod".to_owned()),
            ("region".to_owned(), "us-east".to_owned()),
        ]
    );

    let boxed: Vec<Box<dyn Diagnostic>> = vec![
        Box::new(ConstDiag {
            key: "service",
            value: "api",
        }),
        Box::new(ConstDiag {
            key: "version",
            value: "1",
        }),
    ];
    let boxed_view = Diagnostics::from_boxed(&boxed);
    assert!(!boxed_view.is_empty());
    assert_eq!(
        collect_diags(boxed_view).unwrap(),
        vec![
            ("service".to_owned(), "api".to_owned()),
            ("version".to_owned(), "1".to_owned()),
        ]
    );
}

#[test]
fn diagnostics_visit_short_circuits_on_error() {
    let failing = FailingDiag {
        calls: AtomicUsize::new(0),
    };
    let trailing = CountingDiag {
        key: "after",
        value: "unreachable",
        calls: AtomicUsize::new(0),
    };
    let slice_refs: [&dyn Diagnostic; 2] = [&failing, &trailing];

    let err = collect_diags(Diagnostics::from_slice(&slice_refs)).unwrap_err();
    assert_eq!(err.message(), "diagnostic failure");
    assert_eq!(failing.calls.load(Ordering::Relaxed), 1);
    assert_eq!(trailing.calls.load(Ordering::Relaxed), 0);
}

#[derive(Debug)]
struct MatchOverrideFilter {
    blocked_substring: &'static str,
}

impl Filter for MatchOverrideFilter {
    fn enabled(&self, _: &FilterCriteria, _: Diagnostics) -> FilterResult {
        FilterResult::Neutral
    }

    fn matches(&self, record: &Record, _: Diagnostics) -> FilterResult {
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
    fn format(&self, record: &Record, diags: Diagnostics) -> Result<Vec<u8>, Error> {
        let mut line = format!("{}:{}", self.prefix, record.payload());
        diags.visit(&mut |key: KeyView, value: ValueView| {
            line.push_str(&format!(" {key}={value}"));
            Ok(())
        })?;
        Ok(line.into_bytes())
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
    fn append(&self, record: &Record, diags: Diagnostics) -> Result<(), Error> {
        let bytes = self.layout.format(record, diags)?;
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
