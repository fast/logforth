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
use logforth_layout_text::TextLayout;

#[test]
fn text_layout_formats_utc_and_custom_timestamp_deterministically() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    let kvs = [(Key::new("port"), Value::u64(8080))];
    let diags = StaticDiagnostic::new([("env", "prod")]);
    let record = Record::builder()
        .time(ts)
        .level(Level::Info)
        .module_path_static("app::http")
        .file_static("src/server/http.rs")
        .line(Some(42))
        .key_values(kvs.as_slice())
        .payload(format_args!("server started"))
        .build();

    let default_utc = TextLayout::default().timezone(TimeZone::UTC).no_color();
    let mut default_buf = Vec::new();
    default_utc
        .format(&record, &diags, &mut default_buf)
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&default_buf).unwrap(),
        "2023-11-14T22:13:20.123456+00:00   INFO app::http: http.rs:42 server started port=8080 env=prod",
    );

    let equivalent_custom = TextLayout::default()
        .timezone(TimeZone::UTC)
        .no_color()
        .timestamp_format(|t, tz| format!("{:.6}", t.display_with_offset(tz.to_offset(t))));
    let mut custom_buf = Vec::new();
    equivalent_custom
        .format(&record, &diags, &mut custom_buf)
        .unwrap();
    assert_eq!(default_buf, custom_buf);

    let epoch_secs = TextLayout::default()
        .no_color()
        .timestamp_format(|t, _| t.as_second().to_string());
    let mut prefixed = b"head|".to_vec();
    epoch_secs.format(&record, &diags, &mut prefixed).unwrap();
    assert_eq!(
        std::str::from_utf8(&prefixed).unwrap(),
        "head|1700000000   INFO app::http: http.rs:42 server started port=8080 env=prod",
    );
}

#[test]
fn text_layout_returns_error_on_out_of_range_timestamp() {
    let out_of_range = SystemTime::UNIX_EPOCH + Duration::from_secs(300_000_000_000);
    let record = Record::builder()
        .time(out_of_range)
        .payload(format_args!("overflow"))
        .build();

    let mut buf = Vec::new();
    let err = TextLayout::default()
        .timezone(TimeZone::UTC)
        .no_color()
        .format(&record, &(), &mut buf)
        .unwrap_err();
    assert_eq!(err.message(), "invalid record timestamp");
    assert!(buf.is_empty());
}
