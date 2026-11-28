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

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Layout;
use logforth_core::append::Testing;
use logforth_core::builder;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::kv::Key;
use logforth_core::kv::KeyValues;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::layout::PlainTextLayout;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;
use tests_integration::RecordingErrorSink;

#[derive(Debug, PartialEq, Eq)]
struct RecordSnapshot {
    time: SystemTime,
    level: Level,
    module_path: Option<String>,
    file: Option<String>,
    filename: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    payload: String,
    kvs: Vec<(String, String)>,
}

const fn check_const_record_accessors<'a>(record: &Record<'a>) -> KeyValues<'a> {
    let _ = record.time();
    let _ = record.level();
    let _ = record.module_path();
    let _ = record.module_path_static();
    let _ = record.file();
    let _ = record.file_static();
    let _ = record.line();
    let _ = record.column();
    let _ = record.payload();
    let _ = record.payload_static();
    let _ = record.to_builder();
    record.key_values()
}

fn snapshot_record(record: &Record<'_>) -> RecordSnapshot {
    let kvs = check_const_record_accessors(record);
    RecordSnapshot {
        time: record.time(),
        level: record.level(),
        module_path: record.module_path().map(str::to_owned),
        file: record.file().map(str::to_owned),
        filename: record.filename().map(str::to_owned),
        line: record.line(),
        column: record.column(),
        payload: record.payload().to_string(),
        kvs: kvs
            .iter()
            .map(|(k, v)| (k.as_str().to_owned(), v.to_string()))
            .collect(),
    }
}

#[test]
fn all_twenty_four_levels_preserve_severity_order_discriminants_padding_and_parsing() {
    for (idx, level) in Level::ALL.into_iter().enumerate() {
        let expected_discriminant = u8::try_from(idx + 1).unwrap();
        assert_eq!(level as u8, expected_discriminant);
        if idx > 0 {
            assert!(Level::ALL[idx - 1] < level);
        }

        let name = level.name();
        assert_eq!(level.to_string(), name);
        assert_eq!(format!("{level:>6}").len(), 6);

        assert_eq!(name.parse::<Level>().unwrap(), level);
        assert_eq!(name.to_ascii_lowercase().parse::<Level>().unwrap(), level);
    }

    for malformed in ["", "off", "all", "warning", "trace0", "trace5", "info "] {
        let err = malformed.parse::<Level>().unwrap_err();
        assert!(
            err.message().contains("malformed level"),
            "unexpected error message: {}",
            err.message()
        );
    }
}

#[test]
fn level_filter_predicates_partition_severity_space_consistently() {
    for level in Level::ALL {
        assert!(!LevelFilter::Off.test(level));
        assert!(LevelFilter::All.test(level));
    }

    for pivot in Level::ALL {
        for candidate in Level::ALL {
            let lt = LevelFilter::MoreVerbose(pivot).test(candidate);
            let eq = LevelFilter::Equal(pivot).test(candidate);
            let gt = LevelFilter::MoreSevere(pivot).test(candidate);

            // Exactly one of (<, ==, >) must hold for every (pivot, candidate) pair.
            assert_eq!(u8::from(lt) + u8::from(eq) + u8::from(gt), 1);
            assert_eq!(LevelFilter::NotEqual(pivot).test(candidate), !eq);
            assert_eq!(
                LevelFilter::MoreVerboseEqual(pivot).test(candidate),
                lt || eq
            );
            assert_eq!(
                LevelFilter::MoreSevereEqual(pivot).test(candidate),
                gt || eq
            );
        }
    }
}

#[test]
fn record_owned_round_trip_preserves_metadata_provenance_and_structured_fields() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 987_654_321);
    let dynamic_host = String::from("127.0.0.1");
    let kvs = [
        (Key::new("host"), Value::str(&dynamic_host)),
        (Key::new("env"), Value::static_str("prod")),
        (Key::new("port"), Value::u64(8080)),
    ];

    let static_record = Record::builder()
        .time(ts)
        .level(Level::Warn2)
        .module_path_static("app::net")
        .file_static("src/net/listener.rs")
        .line(Some(77))
        .column(Some(12))
        .key_values(kvs.as_slice())
        .payload(format_args!("static payload"))
        .build();

    assert_eq!(static_record.module_path_static(), Some("app::net"));
    assert_eq!(static_record.file_static(), Some("src/net/listener.rs"));
    assert_eq!(static_record.filename(), Some("listener.rs"));
    assert_eq!(static_record.payload_static(), Some("static payload"));

    let expected_snapshot = snapshot_record(&static_record);
    let reprojected_snapshot = static_record.to_owned().with(|reprojected| {
        assert_eq!(reprojected.module_path_static(), Some("app::net"));
        assert_eq!(reprojected.file_static(), Some("src/net/listener.rs"));
        assert_eq!(
            reprojected.key_values().get("env").unwrap().to_static_str(),
            Some("prod")
        );
        assert_eq!(
            reprojected
                .key_values()
                .get("host")
                .unwrap()
                .to_static_str(),
            None
        );
        snapshot_record(&reprojected)
    });
    assert_eq!(reprojected_snapshot, expected_snapshot);

    // Override via `to_builder` with non-static borrows and verify provenance and `'a` lifetimes.
    let dynamic_mod = String::from("dynamic::worker");
    let dynamic_file = String::from("dir/worker.rs");
    let check_rebuilt = |rebuilt: &Record<'_>| {
        assert_eq!(rebuilt.module_path_static(), None);
        assert_eq!(rebuilt.file_static(), None);
        assert_eq!(rebuilt.payload_static(), None);

        let rebuilt_snapshot = snapshot_record(rebuilt);
        let owned_snapshot = rebuilt.to_owned().with(|reprojected| {
            assert_eq!(reprojected.module_path_static(), None);
            assert_eq!(reprojected.file_static(), None);
            assert_eq!(reprojected.payload_static(), None);
            snapshot_record(&reprojected)
        });
        assert_eq!(owned_snapshot, rebuilt_snapshot);
    };
    check_rebuilt(
        &static_record
            .to_builder()
            .module_path(Some(&dynamic_mod))
            .file(Some(&dynamic_file))
            .line(None)
            .column(None)
            .payload(format_args!("bound to {dynamic_host}"))
            .build(),
    );

    // `filename` and `key_values` borrow from `'a` rather than from `&Record`.
    let (borrowed_filename, borrowed_kvs) = {
        let temp = static_record
            .to_builder()
            .file(Some(&dynamic_file))
            .payload(format_args!(""))
            .build();
        (temp.filename(), temp.key_values())
    };
    assert_eq!(borrowed_filename, Some("worker.rs"));
    assert_eq!(borrowed_kvs.get("port").unwrap().to_u64(), Some(8080));

    // Unset or root-only file paths produce `filename() == None`.
    let default_record = Record::builder().build();
    assert_eq!(default_record.filename(), None);
    assert_eq!(default_record.payload_static(), Some(""));
    assert!(default_record.to_owned().with(|r| r.filename().is_none()));
    assert_eq!(Record::builder().file_static("/").build().filename(), None);
}

#[derive(Debug)]
struct FailingDiag;

impl Diagnostic for FailingDiag {
    fn visit(&self, _: &mut dyn Visitor) -> Result<(), Error> {
        Err(Error::new("diagnostic unavailable"))
    }
}

#[test]
fn plain_text_layout_appends_to_existing_buffer_and_handles_pre_epoch_and_errors() {
    let ts = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_700);
    let kvs = [
        (Key::new("port"), Value::u64(8080)),
        (Key::new("raw"), Value::bytes(b"\x00\nA")),
    ];
    let diags = StaticDiagnostic::new([("env", "prod")]);
    let host = "0.0.0.0";

    let layout = PlainTextLayout::default();
    let mut direct_buf = b"prefix|".to_vec();
    let mut owned_buf = b"prefix|".to_vec();

    let mut format_both = |record: &Record<'_>| {
        layout.format(record, &diags, &mut direct_buf).unwrap();
        record.to_owned().with(|owned_record| {
            layout
                .format(&owned_record, &diags, &mut owned_buf)
                .unwrap();
        });
    };
    format_both(
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

    let expected = "prefix|1700000000123456700   INFO app::http: http.rs:42 listening on 0.0.0.0 port=8080 raw=b\"\\0\\nA\" env=prod";
    assert_eq!(std::str::from_utf8(&direct_buf).unwrap(), expected);
    assert_eq!(direct_buf, owned_buf);

    // Pre-UNIX_EPOCH timestamps render as negative nanoseconds without panicking.
    let pre_epoch = Record::builder()
        .time(SystemTime::UNIX_EPOCH - Duration::from_micros(42))
        .level(Level::Error)
        .payload(format_args!("clock skew"))
        .build();
    let mut pre_epoch_buf = Vec::new();
    layout
        .format(&pre_epoch, &(), &mut pre_epoch_buf)
        .expect("pre-epoch timestamp should format cleanly");
    assert_eq!(
        std::str::from_utf8(&pre_epoch_buf).unwrap(),
        "-42000  ERROR : :0 clock skew"
    );

    // Diagnostic visit errors propagate out of `PlainTextLayout::format`.
    let err = layout
        .format(&pre_epoch, &FailingDiag, &mut Vec::new())
        .unwrap_err();
    assert_eq!(err.message(), "diagnostic unavailable");
}

#[derive(Debug)]
struct NonUtf8AndFallibleLayout;

impl Layout for NonUtf8AndFallibleLayout {
    fn format(&self, record: &Record, _: &dyn Diagnostic, buf: &mut Vec<u8>) -> Result<(), Error> {
        if record.level() == Level::Error {
            return Err(Error::new("layout rejected record"));
        }
        buf.extend_from_slice(b"non-utf8:\xFF\xFE:");
        buf.extend_from_slice(record.payload().to_string().as_bytes());
        Ok(())
    }
}

#[test]
fn testing_appender_handles_lossy_utf8_and_propagates_layout_errors() {
    let errors = RecordingErrorSink::default();
    let logger = builder()
        .error_sink(errors.clone())
        .dispatch(|d| d.append(Testing::default().with_layout(NonUtf8AndFallibleLayout)))
        .build();

    logger.log(
        &Record::builder()
            .level(Level::Info)
            .payload(format_args!("captured-by-test-harness"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Error)
            .payload(format_args!("triggers-layout-error"))
            .build(),
    );
    logger.flush();

    assert_eq!(
        errors.errors(),
        ["failed to append record: layout rejected record"]
    );
}
