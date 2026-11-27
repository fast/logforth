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
use logforth_layout_logfmt::LogfmtLayout;

#[test]
fn logfmt_layout_quotes_special_values_and_escapes_control_chars() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    let kvs = [
        (Key::new("plain"), Value::static_str("ok")),
        (Key::new("eq"), Value::static_str("a=b")),
        (Key::new("ctrl"), Value::static_str("line1\n\"line2\"")),
    ];
    let diags = StaticDiagnostic::new([("env", "prod west")]);
    let who = "world";

    let mut buf = Vec::new();
    LogfmtLayout::default()
        .timezone(TimeZone::UTC)
        .format(
            &Record::builder()
                .time(ts)
                .level(Level::Info)
                .module_path_static("app::worker")
                .file_static("src/worker.rs")
                .line(Some(25))
                .key_values(kvs.as_slice())
                .payload(format_args!("hello {who}"))
                .build(),
            &diags,
            &mut buf,
        )
        .unwrap();

    assert_eq!(
        std::str::from_utf8(&buf).unwrap(),
        r#"timestamp=2023-11-14T22:13:20.123456+00:00 level=INFO module=app::worker position=worker.rs:25 message="hello world" plain=ok eq="a=b" ctrl="line1\n\"line2\"" env="prod west""#,
    );
}

#[test]
fn logfmt_layout_rejects_special_keys_and_out_of_range_timestamps() {
    let layout = LogfmtLayout::default().timezone(TimeZone::UTC);
    let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);

    for bad_key in ["bad key", "bad=key", "bad\"key", "bad\nkey"] {
        let kvs = [(Key::borrowed(bad_key), Value::bool(true))];
        let record = Record::builder()
            .time(ts)
            .key_values(kvs.as_slice())
            .payload(format_args!("msg"))
            .build();
        let mut buf = Vec::new();
        let err = layout.format(&record, &(), &mut buf).unwrap_err();
        assert!(err.message().starts_with("key contains special chars:"));
    }

    let bad_ts_record = Record::builder()
        .time(SystemTime::UNIX_EPOCH + Duration::from_secs(300_000_000_000))
        .build();
    let mut err_buf = Vec::new();
    let err = layout
        .format(&bad_ts_record, &(), &mut err_buf)
        .unwrap_err();
    assert_eq!(err.message(), "invalid record timestamp");
    assert!(err_buf.is_empty());
}
