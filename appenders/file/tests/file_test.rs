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

use std::error::Error as _;
use std::fs;
use std::io;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;

use logforth_append_file::FileBuilder;
use logforth_append_file::jiff::tz::TimeZone;
use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Layout;
use logforth_core::record::Record;
use tempfile::tempdir;

#[derive(Debug)]
struct PayloadLayout;

impl Layout for PayloadLayout {
    fn format(&self, record: &Record, _: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        Ok(record.payload().to_string().into_bytes())
    }
}

fn append_str(appender: &impl Append, text: &str) -> Result<(), Error> {
    appender.append(
        &Record::builder().payload(format_args!("{text}")).build(),
        &[],
    )
}

#[test]
fn validates_filename_and_suffix_before_creating_directory() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("new_logs");

    let invalid_stems = [
        "",
        ".",
        "..",
        "/logs",
        "a/b",
        "a\\b",
        "a\0b",
        #[cfg(windows)]
        "C:relative",
    ];

    for stem in invalid_stems {
        assert!(
            FileBuilder::new(&path, stem).build().is_err(),
            "stem {stem:?} should be rejected"
        );
    }
    for suffix in [".", "..", "...", "a/b", "a\\b", "a\0b"] {
        assert!(
            FileBuilder::new(&path, "app")
                .filename_suffix(suffix)
                .build()
                .is_err(),
            "suffix {suffix:?} should be rejected"
        );
    }
    assert!(!path.exists());

    for (suffix, expected_name) in [("", "app"), (".log", "app.log"), ("..log", "app..log")] {
        let appender = FileBuilder::new(&path, "app")
            .filename_suffix(suffix)
            .layout(PayloadLayout)
            .build()
            .unwrap();
        append_str(&appender, "record").unwrap();
        appender.flush().unwrap();
        assert_eq!(fs::read(path.join(expected_name)).unwrap(), b"record\n");
    }
}

#[test]
fn checks_size_before_writing_including_newline_and_keeps_oversized_records_whole() {
    let dir = tempdir().unwrap();
    let appender = FileBuilder::new(dir.path(), "app")
        .layout(PayloadLayout)
        .rollover_size(NonZeroU64::new(6).unwrap())
        .build()
        .unwrap();

    for payload in ["oversized", "ab", "cd", "1234", "xy", "another oversized"] {
        append_str(&appender, payload).unwrap();
    }

    let expected = [
        b"oversized\n".as_slice(),
        b"ab\ncd\n",
        b"1234\n",
        b"xy\n",
        b"another oversized\n",
    ];
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), expected.len());
    for (i, contents) in expected.into_iter().enumerate() {
        assert_eq!(
            fs::read(dir.path().join(format!("app.{}", i + 1))).unwrap(),
            contents
        );
    }
}

#[test]
fn size_retention_prunes_oldest_numeric_segments_and_ignores_non_matching_entries() {
    let dir = tempdir().unwrap();
    let unrelated = [
        "my_applog",
        "my_appXlog",
        "my_app2.log",
        "my_app.01.log",
        "my_app.log",
        "my_app.0.log",
        "my_app.+1.log",
        "my_app.18446744073709551616.log",
        "my_app.1log",
        "my_app.2026-11-25.log",
        "my_app.2026-11-25.1.log",
    ];
    for name in unrelated {
        fs::write(dir.path().join(name), b"keep").unwrap();
    }
    fs::create_dir(dir.path().join("my_app.3.log")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("my_app.log", dir.path().join("my_app.4.log")).unwrap();

    for seq in [1, 2, 10] {
        fs::write(dir.path().join(format!("my_app.{seq}.log")), b"full").unwrap();
    }

    let appender = FileBuilder::new(dir.path(), "my_app")
        .filename_suffix("log")
        .layout(PayloadLayout)
        .rollover_size(NonZeroU64::MIN)
        .max_log_files(NonZeroUsize::new(2).unwrap())
        .build()
        .unwrap();

    append_str(&appender, "eleven").unwrap();
    append_str(&appender, "twelve").unwrap();

    for seq in [1, 2, 10] {
        assert!(!dir.path().join(format!("my_app.{seq}.log")).exists());
    }
    assert_eq!(
        fs::read(dir.path().join("my_app.11.log")).unwrap(),
        b"eleven\n"
    );
    assert_eq!(
        fs::read(dir.path().join("my_app.12.log")).unwrap(),
        b"twelve\n"
    );
    for name in unrelated {
        assert_eq!(fs::read(dir.path().join(name)).unwrap(), b"keep", "{name}");
    }
    assert!(dir.path().join("my_app.3.log").is_dir());
    #[cfg(unix)]
    assert!(dir.path().join("my_app.4.log").is_symlink());
}

#[test]
fn startup_retention_applies_limit_across_multiple_periods() {
    let dir = tempdir().unwrap();
    for name in [
        "test.2000-01-01.10",
        "test.2000-01-02.2",
        "test.2000-01-02.10",
    ] {
        fs::write(dir.path().join(name), name).unwrap();
    }

    let _appender = FileBuilder::new(dir.path(), "test")
        .rollover_daily()
        .timezone(TimeZone::UTC)
        .rollover_size(NonZeroU64::MIN)
        .max_log_files(NonZeroUsize::new(2).unwrap())
        .build()
        .unwrap();

    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    assert!(!dir.path().join("test.2000-01-01.10").exists());
    assert!(!dir.path().join("test.2000-01-02.2").exists());
    assert!(dir.path().join("test.2000-01-02.10").is_file());
}

#[test]
fn append_mode_writes_at_eof_after_external_truncation() {
    let dir = tempdir().unwrap();
    for size in [None, NonZeroU64::new(100)] {
        let mut builder = FileBuilder::new(dir.path(), "typed.log").layout(PayloadLayout);
        if let Some(max) = size {
            builder = builder.rollover_size(max);
        }
        let appender = builder.build().unwrap();
        let path = dir.path().join(if size.is_some() {
            "typed.log.1"
        } else {
            "typed.log"
        });

        append_str(&appender, "original record").unwrap();
        fs::write(&path, b"x").unwrap();
        append_str(&appender, "y").unwrap();
        assert_eq!(fs::read(path).unwrap(), b"xy\n");
    }
}

#[test]
fn sequence_exhaustion_preserves_existing_segment_and_returns_single_layer_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("my_app.{}", u64::MAX));
    fs::write(&path, b"x").unwrap();

    let err = FileBuilder::new(dir.path(), "my_app")
        .rollover_size(NonZeroU64::MIN)
        .build()
        .unwrap_err();
    assert_eq!(
        err.source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(path).unwrap(), b"x");
}
