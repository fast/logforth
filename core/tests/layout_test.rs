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
use logforth_core::layout::PlainTextLayout;
use logforth_core::record::Level;
use logforth_core::record::Record;

#[test]
fn plain_text_layout_formats_record_and_preserves_existing_buffer() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    let kvs = [
        (Key::new("port"), Value::u64(8080)),
        (Key::new("raw"), Value::bytes(b"\x00\nA")),
    ];
    let diags = StaticDiagnostic::new([("env", "prod")]);
    let host = "0.0.0.0";

    let layout = PlainTextLayout::default();
    let mut buf = b"prefix|".to_vec();
    let mut owned_buf = b"prefix|".to_vec();
    let mut check = |record: &Record| {
        layout.format(record, &diags, &mut buf).unwrap();
        record.to_owned().with(|owned_record| {
            assert_eq!(owned_record.time(), ts);
            layout
                .format(&owned_record, &diags, &mut owned_buf)
                .unwrap();
        });
    };
    check(
        &Record::builder()
            .time(ts)
            .level(Level::Info)
            .module_path_static("app::http")
            .file_static("src/server/http.rs")
            .line(Some(42))
            .key_values(kvs.as_slice())
            .payload(format_args!("listening on {host}"))
            .build(),
    );

    let expected = "prefix|1700000000123456789   INFO app::http: http.rs:42 listening on 0.0.0.0 port=8080 raw=b\"\\0\\nA\" env=prod";
    assert_eq!(std::str::from_utf8(&buf).unwrap(), expected);
    assert_eq!(buf, owned_buf);
}

#[test]
fn plain_text_layout_formats_pre_epoch_timestamp() {
    let ts = SystemTime::UNIX_EPOCH - Duration::from_nanos(42);
    let record = Record::builder()
        .time(ts)
        .level(Level::Error)
        .payload(format_args!("clock skew"))
        .build();

    let mut buf = Vec::new();
    PlainTextLayout::default()
        .format(&record, &(), &mut buf)
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&buf).unwrap(),
        "-42  ERROR : :0 clock skew"
    );
}
