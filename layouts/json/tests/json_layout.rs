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

use jiff::tz::TimeZone;
use logforth_core::Layout;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::record::Level;
use logforth_core::record::Record;
use logforth_layout_json::JsonLayout;

#[test]
fn json_layout_omits_empty_maps_and_matches_custom_timestamp_format() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    let ms = 250;
    let build_and_format = |layout: &JsonLayout| {
        let mut buf = Vec::new();
        layout
            .format(
                &Record::builder()
                    .time(ts)
                    .level(Level::Warn)
                    .module_path_static("app::db")
                    .file_static("src/db.rs")
                    .line(Some(17))
                    .payload(format_args!("slow query ({ms}ms)"))
                    .build(),
                &(),
                &mut buf,
            )
            .unwrap();
        buf
    };

    let layout = JsonLayout::default().timezone(TimeZone::UTC);
    let buf = build_and_format(&layout);
    assert_eq!(
        std::str::from_utf8(&buf).unwrap(),
        r#"{"timestamp":"2023-11-14T22:13:20.123456+00:00","level":"WARN","module_path":"app::db","file":"src/db.rs","line":17,"message":"slow query (250ms)"}"#,
    );

    let equivalent_custom = JsonLayout::default()
        .timezone(TimeZone::UTC)
        .timestamp_format(|t, tz| format!("{:.6}", t.display_with_offset(tz.to_offset(t))));
    let custom_buf = build_and_format(&equivalent_custom);
    assert_eq!(buf, custom_buf);
}

#[test]
fn json_layout_serializes_typed_kvs_diagnostics_and_rejects_bad_timestamp() {
    let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let items = [Value::i64(1), Value::i64(2)];
    let kvs = [
        (Key::new("attempt"), Value::u64(1)),
        (Key::new("attempt"), Value::u64(2)),
        (Key::new("items"), Value::list(&items)),
    ];
    let diags = StaticDiagnostic::new([("service", "billing")]);
    let record = Record::builder()
        .time(ts)
        .level(Level::Error)
        .module_path_static("app::charge")
        .file_static("src/charge.rs")
        .line(Some(9))
        .key_values(kvs.as_slice())
        .payload(format_args!("charge failed"))
        .build();

    let layout = JsonLayout::default().timezone(TimeZone::UTC);
    let mut buf = Vec::new();
    layout.format(&record, &diags, &mut buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!({
            "timestamp": "2023-11-14T22:13:20.000000+00:00",
            "level": "ERROR",
            "module_path": "app::charge",
            "file": "src/charge.rs",
            "line": 9,
            "message": "charge failed",
            "kvs": {
                "attempt": 2,
                "items": [1, 2],
            },
            "diags": {
                "service": "billing",
            },
        })
    );

    let bad_record = Record::builder()
        .time(SystemTime::UNIX_EPOCH + Duration::from_secs(300_000_000_000))
        .build();
    let mut err_buf = Vec::new();
    let err = layout.format(&bad_record, &(), &mut err_buf).unwrap_err();
    assert_eq!(err.message(), "invalid record timestamp");
    assert!(err_buf.is_empty());
}
