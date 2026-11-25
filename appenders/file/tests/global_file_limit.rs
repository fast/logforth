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

use std::fs;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;

use jiff::tz::TimeZone;
use logforth_append_file::FileBuilder;
use logforth_core::Append;
use logforth_core::record::Record;
use tempfile::tempdir;

#[test]
fn size_limit_retains_the_newest_segments_in_numeric_order() {
    let dir = tempdir().unwrap();
    let appender = FileBuilder::new(dir.path(), "test")
        .filename_suffix("log")
        .rollover_size(NonZeroU64::MIN)
        .max_log_files(NonZeroUsize::new(3).unwrap())
        .build()
        .unwrap();

    for i in 1..=12 {
        appender
            .append(
                &Record::builder().payload(format_args!("entry {i}")).build(),
                &[],
            )
            .unwrap();
    }
    appender.flush().unwrap();

    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    for i in 10..=12 {
        let record = fs::read_to_string(dir.path().join(format!("test.{i}.log"))).unwrap();
        assert!(record.ends_with(&format!("entry {i}\n")));
    }
}

#[test]
fn startup_applies_one_limit_across_periods() {
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
    assert!(dir.path().join("test.2000-01-02.10").is_file());
}
