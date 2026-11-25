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
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use jiff::SignedDuration;
use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use logforth_core::Error;

use crate::clock::Clock;
use crate::rotation::Rotation;

const RETRY_COOLDOWN: SignedDuration = SignedDuration::from_secs(1);

#[derive(Debug)]
pub(crate) struct RollingFileWriter {
    files: SegmentFiles,
    max_files: Option<NonZeroUsize>,
    timezone: TimeZone,
    clock: Clock,
    active: Segment,
    period_bounds: Option<Range<Timestamp>>,
    retry_cooldown: Option<Range<Timestamp>>,
}

impl RollingFileWriter {
    pub(crate) fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
        let now = self.clock.now();
        if let Err(err) = self.rotate_if_needed(now, buf.len()) {
            self.retry_cooldown = Some(retry_window(now));
            self.write_record(buf)?;
            return Err(err);
        }
        self.write_record(buf)
    }

    fn size_exceeded(&self, buf_len: usize) -> bool {
        self.files.max_size.is_some_and(|max| {
            self.active.size > 0 && self.active.size.saturating_add(buf_len as u64) > max.get()
        })
    }

    fn should_rotate(&self, now: Timestamp, buf_len: usize) -> bool {
        if self
            .retry_cooldown
            .as_ref()
            .is_some_and(|window| window.contains(&now))
        {
            return false;
        }
        self.period_bounds
            .as_ref()
            .is_some_and(|bounds| !bounds.contains(&now))
            || self.size_exceeded(buf_len)
    }

    fn rotate_if_needed(&mut self, now: Timestamp, buf_len: usize) -> Result<bool, Error> {
        if !self.should_rotate(now, buf_len) {
            return Ok(false);
        }
        let period = self.files.rotation.period_at(now, &self.timezone)?;
        let key = period.as_ref().map(|p| p.key);
        let bounds = period.map(|p| p.bounds);
        if key == self.active.name.period && !self.size_exceeded(buf_len) {
            self.period_bounds = bounds;
            self.retry_cooldown = None;
            return Ok(false);
        }
        let segment = if self.files.max_size.is_some() {
            let previous = if key == self.active.name.period {
                self.active.name.seq
            } else {
                None
            };
            self.files.create_segment(key, previous)?
        } else {
            let name = SegmentName {
                period: key,
                seq: None,
            };
            Segment::open(self.files.path(name), name, 0)?
        };
        self.active = segment;
        self.period_bounds = bounds;
        self.retry_cooldown = None;
        self.prune_if_needed()?;
        Ok(true)
    }

    fn write_record(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.active.file.write_all(buf).map_err(|err| {
            // A failed write_all may already have appended part of the record.
            if let Ok(metadata) = self.active.file.metadata() {
                self.active.size = metadata.len();
            }
            file_error("write log file", &self.active.path, err)
        })?;
        self.active.size = self.active.size.saturating_add(buf.len() as u64);
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> Result<(), Error> {
        self.active
            .file
            .flush()
            .map_err(|err| file_error("flush log file", &self.active.path, err))
    }

    fn prune_if_needed(&self) -> Result<(), Error> {
        let Some(max_files) = self.max_files else {
            return Ok(());
        };
        let mut matching = self.files.list_matching_files()?;
        let excess = matching.len().saturating_sub(max_files.get());
        if excess == 0 {
            return Ok(());
        }
        matching.sort_unstable_by_key(|(name, _)| *name);
        for (_, path) in matching
            .into_iter()
            .filter(|(_, path)| *path != self.active.path)
            .take(excess)
        {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(file_error("remove old log file", &path, err)),
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct RollingFileWriterBuilder {
    files: SegmentFiles,
    max_files: Option<NonZeroUsize>,
    timezone: TimeZone,
    clock: Clock,
}

impl RollingFileWriterBuilder {
    pub(crate) fn new(basedir: impl Into<PathBuf>, filename: impl Into<String>) -> Self {
        Self {
            files: SegmentFiles {
                basedir: basedir.into(),
                filename: filename.into(),
                filename_suffix: None,
                rotation: Rotation::Never,
                max_size: None,
            },
            max_files: None,
            timezone: TimeZone::system(),
            clock: Clock::DefaultClock,
        }
    }

    pub(crate) fn rotation(mut self, rotation: Rotation) -> Self {
        self.files.rotation = rotation;
        self
    }

    pub(crate) fn timezone(mut self, tz: TimeZone) -> Self {
        self.timezone = tz;
        self
    }

    pub(crate) fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        self.files.filename_suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix('.').unwrap_or(&suffix).to_owned())
        };
        self
    }

    pub(crate) fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.max_files = Some(n);
        self
    }

    pub(crate) fn max_file_size(mut self, n: NonZeroU64) -> Self {
        self.files.max_size = Some(n);
        self
    }

    pub(crate) fn build(self) -> Result<RollingFileWriter, Error> {
        let Self {
            files,
            max_files,
            timezone,
            clock,
        } = self;
        validate_component("filename", &files.filename)?;
        if let Some(suffix) = &files.filename_suffix {
            validate_component("filename suffix", suffix)?;
        }
        fs::create_dir_all(&files.basedir)
            .map_err(|err| file_error("create log directory", &files.basedir, err))?;
        let period = files.rotation.period_at(clock.now(), &timezone)?;
        let active = files.open_initial(period.as_ref().map(|p| p.key))?;
        let writer = RollingFileWriter {
            files,
            max_files,
            timezone,
            clock,
            active,
            period_bounds: period.map(|p| p.bounds),
            retry_cooldown: None,
        };
        if writer.files.rotation != Rotation::Never || writer.files.max_size.is_some() {
            // Startup cleanup is best-effort so locked historical files never block initialization.
            let _ = writer.prune_if_needed();
        }
        Ok(writer)
    }
}

fn retry_window(now: Timestamp) -> Range<Timestamp> {
    now..now.checked_add(RETRY_COOLDOWN).unwrap_or(Timestamp::MAX)
}

fn validate_component(label: &str, value: &str) -> Result<(), Error> {
    if value.contains(['/', '\\', '\0']) || Path::new(value).file_name() != Some(value.as_ref()) {
        return Err(Error::new(format!("invalid {label}: {value:?}")));
    }
    Ok(())
}

fn file_error(operation: &str, path: &Path, err: io::Error) -> Error {
    Error::new(format!("failed to {operation}: {}", path.display())).with_source(err)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SegmentName {
    period: Option<DateTime>,
    seq: Option<NonZeroU64>,
}

#[derive(Debug)]
struct Segment {
    name: SegmentName,
    path: PathBuf,
    file: File,
    size: u64,
}

impl Segment {
    fn open(path: PathBuf, name: SegmentName, size: u64) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|err| file_error("open log file", &path, err))?;
        Ok(Self {
            name,
            path,
            file,
            size,
        })
    }
}

#[derive(Debug)]
struct SegmentFiles {
    basedir: PathBuf,
    filename: String,
    filename_suffix: Option<String>,
    rotation: Rotation,
    max_size: Option<NonZeroU64>,
}

impl SegmentFiles {
    fn path(&self, name: SegmentName) -> PathBuf {
        let mut filename = self.filename.clone();
        if let Some(period) = name.period {
            filename.push('.');
            filename.push_str(&period.strftime(self.rotation.date_format()).to_string());
        }
        if let Some(seq) = name.seq {
            filename.push('.');
            filename.push_str(&seq.to_string());
        }
        if let Some(suffix) = &self.filename_suffix {
            filename.push('.');
            filename.push_str(suffix);
        }
        self.basedir.join(filename)
    }

    fn open_initial(&self, period: Option<DateTime>) -> Result<Segment, Error> {
        let Some(max) = self.max_size else {
            let name = SegmentName { period, seq: None };
            return Segment::open(self.path(name), name, 0);
        };
        let highest = self
            .list_matching_files()?
            .into_iter()
            .filter(|(name, _)| name.period == period)
            .max_by_key(|(name, _)| name.seq);
        let mut previous = None;
        if let Some((name, path)) = highest {
            let size = fs::metadata(&path)
                .map_err(|err| file_error("read log file metadata", &path, err))?
                .len();
            if size < max.get() {
                return Segment::open(path, name, size);
            }
            previous = name.seq;
        }
        self.create_segment(period, previous)
    }

    fn create_segment(
        &self,
        period: Option<DateTime>,
        previous: Option<NonZeroU64>,
    ) -> Result<Segment, Error> {
        let mut seq = previous.map_or(NonZeroU64::MIN, |seq| seq.saturating_add(1));
        loop {
            let name = SegmentName {
                period,
                seq: Some(seq),
            };
            let path = self.path(name);
            match OpenOptions::new().create_new(true).append(true).open(&path) {
                Ok(file) => {
                    return Ok(Segment {
                        name,
                        path,
                        file,
                        size: 0,
                    });
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    seq = seq
                        .checked_add(1)
                        .ok_or_else(|| file_error("allocate log file sequence", &path, err))?;
                }
                Err(err) => return Err(file_error("create log file", &path, err)),
            }
        }
    }

    fn list_matching_files(&self) -> Result<Vec<(SegmentName, PathBuf)>, Error> {
        let read_error = |err| file_error("read log directory", &self.basedir, err);
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.basedir).map_err(read_error)? {
            let entry = entry.map_err(read_error)?;
            if let Some(name) = self.parse_segment_filename(&entry) {
                files.push((name, entry.path()));
            }
        }
        Ok(files)
    }

    fn parse_segment_filename(&self, entry: &fs::DirEntry) -> Option<SegmentName> {
        if !entry.file_type().ok()?.is_file() {
            return None;
        }
        let file_name = entry.file_name();
        let rest = file_name
            .to_str()?
            .strip_prefix(&self.filename)?
            .strip_prefix('.')?;
        let middle = match &self.filename_suffix {
            Some(suffix) => rest.strip_suffix(suffix.as_str())?.strip_suffix('.')?,
            None => rest,
        };
        let (period, seq) = match (self.rotation, self.max_size.is_some()) {
            (Rotation::Never, false) => return None,
            (Rotation::Never, true) => (None, Some(parse_seq(middle)?)),
            (rotation, false) => (Some(rotation.parse_period(middle)?), None),
            (rotation, true) => {
                let (period, seq) = middle.split_once('.')?;
                (Some(rotation.parse_period(period)?), Some(parse_seq(seq)?))
            }
        };
        Some(SegmentName { period, seq })
    }
}

fn parse_seq(s: &str) -> Option<NonZeroU64> {
    if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use tempfile::TempDir;
    use tempfile::tempdir;

    use super::*;

    fn builder(dir: &TempDir, rotation: Rotation, now: &str) -> RollingFileWriterBuilder {
        let mut builder = RollingFileWriterBuilder::new(dir.path(), "my_app")
            .rotation(rotation)
            .timezone(TimeZone::UTC)
            .filename_suffix(".log");
        builder.clock = Clock::ManualClock(now.parse().unwrap());
        builder
    }

    #[test]
    fn time_retention_recognizes_only_the_configured_period_format() {
        for (rotation, valid, invalid) in [
            (
                Rotation::Daily,
                "2024-02-29",
                ["2023-02-29", "+024-02-29", "2024-02-29-03"],
            ),
            (
                Rotation::Hourly,
                "2024-02-29-23",
                ["2024-02-29-24", "2024-02-29- 3", "2024-02-29"],
            ),
            (
                Rotation::Minutely,
                "2024-02-29-23-59",
                ["2024-02-29-23-60", "2024-02-29-23- 9", "2024-02-29-23"],
            ),
        ] {
            let dir = tempdir().unwrap();
            for period in [valid, invalid[0], invalid[1], invalid[2]] {
                fs::write(dir.path().join(format!("my_app.{period}.log")), b"old").unwrap();
            }
            let mut writer = builder(&dir, rotation, "2026-11-25T01:02:03Z")
                .max_log_files(NonZeroUsize::MIN)
                .build()
                .unwrap();
            writer.write_all(b"new").unwrap();
            assert!(!dir.path().join(format!("my_app.{valid}.log")).exists());
            for period in invalid {
                assert!(dir.path().join(format!("my_app.{period}.log")).exists());
            }
        }
    }

    #[test]
    fn restart_resumes_only_the_highest_segment_in_the_current_period() {
        let dir = tempdir().unwrap();
        let build = || {
            builder(&dir, Rotation::Hourly, "2026-11-25T17:01:00Z")
                .max_file_size(NonZeroU64::new(5).unwrap())
                .build()
                .unwrap()
        };
        let current = dir.path().join("my_app.2026-11-25-17.10.log");
        let future = dir.path().join("my_app.2026-11-25-18.99.log");
        fs::write(dir.path().join("my_app.2026-11-25-17.9.log"), b"old").unwrap();
        fs::write(&current, b"ab").unwrap();
        fs::write(&future, b"future").unwrap();

        build().write_all(b"cde").unwrap();
        assert_eq!(fs::read(&current).unwrap(), b"abcde");
        let mut writer = build();
        assert!(dir.path().join("my_app.2026-11-25-17.11.log").is_file());
        writer.write_all(b"oversized").unwrap();
        drop(writer);
        build().write_all(b"new").unwrap();
        assert_eq!(
            fs::read(dir.path().join("my_app.2026-11-25-17.12.log")).unwrap(),
            b"new"
        );
        assert_eq!(fs::read(future).unwrap(), b"future");
        build().write_all(b"123").unwrap();
        assert_eq!(
            fs::read(dir.path().join("my_app.2026-11-25-17.12.log")).unwrap(),
            b"new"
        );
        assert_eq!(
            fs::read(dir.path().join("my_app.2026-11-25-17.13.log")).unwrap(),
            b"123"
        );
    }

    #[test]
    fn collision_on_return_to_a_period_preserves_existing_segments() {
        let dir = tempdir().unwrap();
        let mut writer = builder(&dir, Rotation::Hourly, "1969-12-31T23:59:59Z")
            .max_file_size(NonZeroU64::MIN)
            .build()
            .unwrap();
        writer.write_all(b"a").unwrap();
        writer.write_all(b"b").unwrap();
        writer.clock = Clock::ManualClock("1970-01-01T00:00:00Z".parse().unwrap());
        writer.write_all(b"c").unwrap();
        writer.clock = Clock::ManualClock("1969-12-31T23:59:00Z".parse().unwrap());
        writer.write_all(b"d").unwrap();
        for (name, contents) in [
            ("my_app.1969-12-31-23.1.log", b"a"),
            ("my_app.1969-12-31-23.2.log", b"b"),
            ("my_app.1970-01-01-00.1.log", b"c"),
            ("my_app.1969-12-31-23.3.log", b"d"),
        ] {
            assert_eq!(fs::read(dir.path().join(name)).unwrap(), contents);
        }
    }

    #[test]
    fn pruning_keeps_the_active_file_when_the_clock_moves_back() {
        let dir = tempdir().unwrap();
        let future = dir.path().join("my_app.2026-11-27.log");
        let today = dir.path().join("my_app.2026-11-26.log");
        let past = dir.path().join("my_app.2026-11-25.log");
        fs::write(&future, b"future").unwrap();
        let mut writer = builder(&dir, Rotation::Daily, "2026-11-26T05:00:00Z")
            .max_log_files(NonZeroUsize::MIN)
            .build()
            .unwrap();
        writer.write_all(b"today").unwrap();
        assert!(!future.exists());
        assert_eq!(fs::read(&today).unwrap(), b"today");

        fs::write(&past, b"existing ").unwrap();
        writer.clock = Clock::ManualClock("2026-11-25T05:00:00Z".parse().unwrap());
        writer.write_all(b"past").unwrap();
        assert_eq!(fs::read(past).unwrap(), b"existing past");
        assert!(!today.exists());
    }

    #[test]
    fn fall_back_reuses_the_local_hour_then_rotates_at_its_second_end() {
        for max_size in [None, NonZeroU64::new(100)] {
            let dir = tempdir().unwrap();
            let mut b = builder(&dir, Rotation::Hourly, "2026-11-01T01:00:00-04:00")
                .timezone(TimeZone::get("America/New_York").unwrap());
            if let Some(max) = max_size {
                b = b.max_file_size(max);
            }
            let mut writer = b.build().unwrap();
            writer.write_all(b"EDT ").unwrap();
            writer.clock = Clock::ManualClock("2026-11-01T01:00:00-05:00".parse().unwrap());
            writer.write_all(b"EST").unwrap();
            writer.clock = Clock::ManualClock("2026-11-01T02:00:00-05:00".parse().unwrap());
            writer.write_all(b"later").unwrap();

            let (first, second) = if max_size.is_some() {
                ("my_app.2026-11-01-01.1.log", "my_app.2026-11-01-02.1.log")
            } else {
                ("my_app.2026-11-01-01.log", "my_app.2026-11-01-02.log")
            };
            assert_eq!(fs::read(dir.path().join(first)).unwrap(), b"EDT EST");
            assert_eq!(fs::read(dir.path().join(second)).unwrap(), b"later");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        }
    }

    #[cfg(unix)]
    #[test]
    fn failed_rollover_appends_the_record_and_waits_one_second_before_retrying() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let mut writer = builder(&dir, Rotation::Minutely, "2026-11-25T17:00:59Z")
            .build()
            .unwrap();
        let initial = dir.path().join("my_app.2026-11-25-17-00.log");
        let blocked = dir.path().join("my_app.2026-11-25-17-02.log");
        let rolled_back = dir.path().join("my_app.2026-11-25-17-01.log");
        writer.write_all(b"first ").unwrap();

        let permissions = fs::metadata(dir.path()).unwrap().permissions();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
        // Privileged test runners can still create files here. In that case the directory
        // at the candidate path prevents opening it for append.
        let blocker = fs::create_dir(&blocked);
        writer.clock = Clock::ManualClock("2026-11-25T17:02:00Z".parse().unwrap());
        let result = writer.write_all(b"failed ");
        fs::set_permissions(dir.path(), permissions).unwrap();
        let err = result.unwrap_err();
        let cause = err.source().unwrap().downcast_ref::<io::Error>().unwrap();
        match blocker {
            Ok(()) => fs::remove_dir(&blocked).unwrap(),
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
                assert_eq!(cause.kind(), io::ErrorKind::PermissionDenied);
            }
        }
        assert_eq!(fs::read(&initial).unwrap(), b"first failed ");

        writer.clock = Clock::ManualClock("2026-11-25T17:02:00.999Z".parse().unwrap());
        writer.write_all(b"cooldown").unwrap();
        assert!(!blocked.exists());

        // Stepping the wall clock backward before the failure timestamp exits cooldown immediately.
        writer.clock = Clock::ManualClock("2026-11-25T17:01:30Z".parse().unwrap());
        writer.write_all(b"back").unwrap();
        assert_eq!(fs::read(&rolled_back).unwrap(), b"back");

        // Re-trigger cooldown on `blocked`, then advance 1 second to verify normal expiry.
        let permissions = fs::metadata(dir.path()).unwrap().permissions();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
        let blocker = fs::create_dir(&blocked);
        writer.clock = Clock::ManualClock("2026-11-25T17:02:00Z".parse().unwrap());
        let _ = writer.write_all(b" retry_wait");
        fs::set_permissions(dir.path(), permissions).unwrap();
        if blocker.is_ok() {
            fs::remove_dir(&blocked).unwrap();
        }
        writer.clock = Clock::ManualClock("2026-11-25T17:02:01Z".parse().unwrap());
        writer.write_all(b"retried").unwrap();
        assert_eq!(fs::read(initial).unwrap(), b"first failed cooldown");
        assert_eq!(fs::read(rolled_back).unwrap(), b"back retry_wait");
        assert_eq!(fs::read(blocked).unwrap(), b"retried");
    }

    #[test]
    fn timestamp_overflow_reports_the_calendar_error_after_writing() {
        let dir = tempdir().unwrap();
        let mut writer = builder(&dir, Rotation::Daily, "2026-11-25T00:00:00Z")
            .build()
            .unwrap();
        writer.clock = Clock::ManualClock(Timestamp::MAX);
        let error = writer.write_all(b"still here").unwrap_err();
        assert!(error.source().unwrap().is::<jiff::Error>());
        assert_eq!(
            fs::read(dir.path().join("my_app.2026-11-25.log")).unwrap(),
            b"still here"
        );
    }
}
