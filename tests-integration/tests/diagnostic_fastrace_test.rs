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

use fastrace::Span;
use fastrace::collector::SpanContext;
use fastrace::collector::SpanId;
use fastrace::collector::TraceId;
use fastrace::local::LocalSpan;
use logforth::Diagnostic;
use logforth::Error;
use logforth::LoggerBuilder;
use logforth::diagnostic::FastraceDiagnostic;
use logforth::kv::KeyView;
use logforth::kv::ValueView;
use logforth::layout::GoogleCloudLoggingLayout;
use tests_integration::RecordingAppender;

const CONST_FASTRACE_DIAG: FastraceDiagnostic = FastraceDiagnostic::new();

#[test]
fn fastrace_diagnostic_formats_zero_padded_hex_ids_tracks_nested_spans_and_handles_unsampled() {
    let sink = RecordingAppender::default();
    let logger = LoggerBuilder::new()
        .dispatch(|d| {
            d.diagnostic(CONST_FASTRACE_DIAG.clone())
                .append(sink.clone())
        })
        .build();

    // Outside any local parent span: zero diagnostic entries.
    logforth::info!(logger, "outside-before");

    let trace_id = TraceId(0x0001_0203_0405_0607_0809_0a0b_0c0d_0e0f);
    let expected_trace_hex = "000102030405060708090a0b0c0d0e0f";
    let expected_root_span_hex: String;
    let expected_child_span_hex: String;

    {
        let root = Span::root("root", SpanContext::new(trace_id, SpanId(0)));
        expected_root_span_hex = SpanContext::from_span(&root).unwrap().span_id.to_string();
        let _root_guard = root.set_local_parent();
        logforth::info!(logger, "in-root");

        {
            let _child_guard = LocalSpan::enter_with_local_parent("child");
            let child_ctx = SpanContext::current_local_parent().unwrap();
            expected_child_span_hex = child_ctx.span_id.to_string();
            assert_eq!(child_ctx.trace_id, trace_id);
            assert_ne!(expected_child_span_hex, expected_root_span_hex);
            logforth::info!(logger, "in-child");

            // Verify typed `ValueView` projections (`str`, `str`, `bool`) and visitor error
            // short-circuiting.
            let mut seen = Vec::new();
            let err = CONST_FASTRACE_DIAG
                .visit(&mut |k: KeyView, v: ValueView| {
                    seen.push((k.as_str().to_owned(), v.to_bool(), v.to_str().is_some()));
                    if k.as_str() == "span_id" {
                        return Err(Error::new("stop after span_id"));
                    }
                    Ok(())
                })
                .unwrap_err();
            assert_eq!(err.message(), "stop after span_id");
            assert_eq!(
                seen,
                [
                    ("trace_id".to_owned(), None, true),
                    ("span_id".to_owned(), None, true),
                ]
            );
        }

        // Dropping `_child_guard` restores `root`'s span ID.
        logforth::info!(logger, "restored-root");
    }

    // Unsampled span context (`sampled = false`) still propagates trace/span IDs with `false`.
    let unsampled_span_hex: String;
    {
        let unsampled = Span::root(
            "unsampled",
            SpanContext::new(TraceId(0x0f), SpanId(0)).sampled(false),
        );
        unsampled_span_hex = SpanContext::from_span(&unsampled)
            .unwrap()
            .span_id
            .to_string();
        let _guard = unsampled.set_local_parent();

        let mut sampled_bool = None;
        FastraceDiagnostic::default()
            .visit(&mut |k: KeyView, v: ValueView| {
                if k.as_str() == "sampled" {
                    sampled_bool = v.to_bool();
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(sampled_bool, Some(false));
        logforth::info!(logger, "in-unsampled");
    }

    logforth::info!(logger, "outside-after");

    let events = sink.events();
    assert_eq!(events.len(), 6);
    assert!(events[0].diagnostics.is_empty());

    assert_eq!(
        events[1].diagnostics,
        [
            ("trace_id".to_owned(), expected_trace_hex.to_owned()),
            ("span_id".to_owned(), expected_root_span_hex),
            ("sampled".to_owned(), "true".to_owned()),
        ]
    );
    assert_eq!(
        events[2].diagnostics,
        [
            ("trace_id".to_owned(), expected_trace_hex.to_owned()),
            ("span_id".to_owned(), expected_child_span_hex),
            ("sampled".to_owned(), "true".to_owned()),
        ]
    );

    assert_eq!(events[3].diagnostics, events[1].diagnostics);
    assert_eq!(
        events[4].diagnostics,
        [
            (
                "trace_id".to_owned(),
                "0000000000000000000000000000000f".to_owned(),
            ),
            ("span_id".to_owned(), unsampled_span_hex),
            ("sampled".to_owned(), "false".to_owned()),
        ]
    );
    assert!(events[5].diagnostics.is_empty());
}

#[test]
fn fastrace_diagnostic_integrates_with_google_cloud_logging_trace_fields() {
    let gcl_layout = GoogleCloudLoggingLayout::default().trace_project_id("my-gcp-project");
    let sink = RecordingAppender::with_layout(gcl_layout);
    let logger = LoggerBuilder::new()
        .dispatch(|d| d.diagnostic(FastraceDiagnostic::new()).append(sink.clone()))
        .build();

    let root = Span::root(
        "http_get",
        SpanContext::new(
            TraceId(0x0af7_6519_16cd_43dd_8448_eb21_1c80_319c),
            SpanId(0),
        ),
    );
    let root_span_hex = SpanContext::from_span(&root).unwrap().span_id.to_string();
    let _guard = root.set_local_parent();
    logforth::info!(logger, "served request");

    let line = &sink.lines()[0];
    assert!(
        line.contains(
            "\"logging.googleapis.com/trace\":\"projects/my-gcp-project/traces/0af7651916cd43dd8448eb211c80319c\""
        ),
        "unexpected GCL output: {line}"
    );
    assert!(
        line.contains(&format!(
            "\"logging.googleapis.com/spanId\":\"{root_span_hex}\""
        )),
        "unexpected GCL output: {line}"
    );
    assert!(
        line.contains("\"logging.googleapis.com/trace_sampled\":true"),
        "unexpected GCL output: {line}"
    );
}
