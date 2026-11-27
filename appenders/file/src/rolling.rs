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
pub struct RollingFileWriter {
    files: SegmentFiles,
    max_files: Option<NonZeroUsize>,
    timezone: TimeZone,
    clock: Clock,
    active: Segment,
    period_bounds: Option<Range<Timestamp>>,
    retry_cooldown: Option<Range<Timestamp>>,
}

impl RollingFileWriter {
    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
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

    pub fn flush(&mut self) -> Result<(), Error> {
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
pub struct RollingFileWriterBuilder {
    files: SegmentFiles,
    max_files: Option<NonZeroUsize>,
    timezone: TimeZone,
    clock: Clock,
}

impl RollingFileWriterBuilder {
    pub fn new(basedir: impl Into<PathBuf>, filename: impl Into<String>) -> Self {
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

    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.files.rotation = rotation;
        self
    }

    pub fn timezone(mut self, tz: TimeZone) -> Self {
        self.timezone = tz;
        self
    }

    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        self.files.filename_suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix('.').unwrap_or(&suffix).to_owned())
        };
        self
    }

    pub fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.max_files = Some(n);
        self
    }

    pub fn max_file_size(mut self, n: NonZeroU64) -> Self {
        self.files.max_size = Some(n);
        self
    }

    pub fn build(self) -> Result<RollingFileWriter, Error> {
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
