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

use std::time::Duration;
use std::time::SystemTime;

use logforth_core::Layout;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::record::Level;
use logforth_core::record::Record;
use logforth_layout_google_cloud_logging::GoogleCloudLoggingLayout;

#[test]
fn gcl_layout_extracts_trace_routes_labels_and_prefers_record_over_diagnostic() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    let diags = StaticDiagnostic::new([
        ("trace_id", "0123456789abcdef0123456789abcdef"),
        ("span_id", "0011223344556677"),
        ("region", "us-central1"),
        ("label_override", "from-diag"),
        ("shared", "from-diag"),
    ]);
    let sampled_diag = StaticDiagnostic::new([("sampled", false)]);
    let composed_diags: [&dyn logforth_core::Diagnostic; 2] = [&diags, &sampled_diag];

    let kvs = [
        (Key::new("trace_sampled"), Value::bool(true)),
        (Key::new("label_override"), Value::static_str("from-record")),
        (Key::new("shared"), Value::static_str("from-record")),
        (Key::new("count"), Value::u64(5)),
    ];
    let record = Record::builder()
        .time(ts)
        .level(Level::Error)
        .module_path_static("app::worker")
        .file_static("src/main.rs")
        .line(Some(88))
        .key_values(kvs.as_slice())
        .payload(format_args!("job failed"))
        .build();

    let layout = GoogleCloudLoggingLayout::default()
        .trace_project_id("demo-proj")
        .label_keys(["region", "label_override"]);
    let mut buf = Vec::new();
    layout
        .format(&record, &composed_diags.as_slice(), &mut buf)
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!({
            "severity": "ERROR",
            "timestamp": "2023-11-14T22:13:20.123456789Z",
            "message": "job failed",
            "count": 5,
            "shared": "from-record",
            "logging.googleapis.com/labels": {
                "region": "us-central1",
                "label_override": "from-record",
            },
            "logging.googleapis.com/trace": "projects/demo-proj/traces/0123456789abcdef0123456789abcdef",
            "logging.googleapis.com/spanId": "0011223344556677",
            "logging.googleapis.com/trace_sampled": true,
            "logging.googleapis.com/sourceLocation": {
                "file": "src/main.rs",
                "line": 88,
                "function": "app::worker",
            },
        })
    );
}

#[test]
fn gcl_layout_keeps_trace_keys_without_project_id_and_rejects_bad_timestamp() {
    let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let kvs = [(Key::new("trace_id"), Value::static_str("abc"))];
    let record = Record::builder()
        .time(ts)
        .level(Level::Info)
        .key_values(kvs.as_slice())
        .payload(format_args!("ok"))
        .build();

    let layout = GoogleCloudLoggingLayout::default();
    let mut buf = Vec::new();
    layout.format(&record, &(), &mut buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
    assert_eq!(parsed["trace_id"], "abc");
    assert!(parsed.get("logging.googleapis.com/trace").is_none());

    let bad_record = Record::builder()
        .time(SystemTime::UNIX_EPOCH + Duration::from_secs(300_000_000_000))
        .build();
    let mut err_buf = Vec::new();
    let err = layout.format(&bad_record, &(), &mut err_buf).unwrap_err();
    assert_eq!(err.message(), "invalid record timestamp");
    assert!(err_buf.is_empty());
}
