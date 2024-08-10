// Copyright 2024 tison <wander4096@gmail.com>
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
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use parking_lot::RwLock;
use time::format_description;
use time::Date;
use time::Duration;
use time::OffsetDateTime;
use time::Time;

/// A file writer with the ability to rotate log files at a fixed schedule.
#[derive(Debug)]
pub struct RollingFileWriter {
    state: State,
    writer: RwLock<File>,
}

impl RollingFileWriter {
    #[must_use]
    pub fn builder() -> RollingFileWriterBuilder {
        RollingFileWriterBuilder::new()
    }

    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = self.now();
        let writer = self.writer.get_mut();
        if self.state.should_rollover_on_date(now) {
            self.state.advance_date(now);
            self.state.refresh_writer(now, 0, writer);
        }
        if self.state.should_rollover_on_size() {
            let cnt = self.state.advance_cnt();
            self.state.refresh_writer(now, cnt, writer);
        }

        writer.write(buf).map(|n| {
            self.state.current_filesize += n;
            n
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.get_mut().flush()
    }
}

/// A builder for [`RollingFileWriter`].
#[derive(Debug)]
pub struct RollingFileWriterBuilder {
    rotation: Rotation,
    prefix: Option<String>,
    suffix: Option<String>,
    max_size: usize,
    max_files: Option<usize>,
}

impl Default for RollingFileWriterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RollingFileWriterBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rotation: Rotation::Never,
            prefix: None,
            suffix: None,
            max_size: usize::MAX,
            max_files: None,
        }
    }

    #[must_use]
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    #[must_use]
    pub fn filename_prefix(mut self, prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        self.prefix = if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        };
        self
    }

    #[must_use]
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        self.suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix)
        };
        self
    }

    #[must_use]
    pub fn max_log_files(mut self, n: usize) -> Self {
        self.max_files = Some(n);
        self
    }

    /// Sets the maximum size of a log file in bytes.
    #[must_use]
    pub fn max_file_size(mut self, n: usize) -> Self {
        self.max_size = n;
        self
    }

    pub fn build(self, dir: impl AsRef<Path>) -> anyhow::Result<RollingFileWriter> {
        let Self {
            rotation,
            prefix,
            suffix,
            max_size,
            max_files,
        } = self;
        let directory = dir.as_ref().to_path_buf();
        let now = OffsetDateTime::now_utc();
        let (state, writer) = State::new(
            now, rotation, directory, prefix, suffix, max_size, max_files,
        )?;
        Ok(RollingFileWriter { state, writer })
    }
}

#[derive(Debug)]
struct State {
    log_dir: PathBuf,
    log_filename_prefix: Option<String>,
    log_filename_suffix: Option<String>,
    date_format: Vec<format_description::FormatItem<'static>>,
    rotation: Rotation,
    current_date: OffsetDateTime,
    current_count: usize,
    current_filesize: usize,
    next_date_timestamp: Option<usize>,
    max_size: usize,
    max_files: Option<usize>,
}

impl State {
    fn new(
        now: OffsetDateTime,
        rotation: Rotation,
        dir: impl AsRef<Path>,
        log_filename_prefix: Option<String>,
        log_filename_suffix: Option<String>,
        max_size: usize,
        max_files: Option<usize>,
    ) -> anyhow::Result<(Self, RwLock<File>)> {
        let log_dir = dir.as_ref().to_path_buf();
        let date_format = rotation.date_format();
        let next_date_timestamp = rotation.next_date_timestamp(&now);

        let current_date = now;
        let current_count = 0;
        let current_filesize = 0;

        let state = State {
            log_dir,
            log_filename_prefix,
            log_filename_suffix,
            date_format,
            current_date,
            current_count,
            current_filesize,
            next_date_timestamp,
            rotation,
            max_size,
            max_files,
        };

        let file = state.create_log_writer(now, 0)?;
        let writer = RwLock::new(file);
        Ok((state, writer))
    }

    fn join_date(&self, date: &OffsetDateTime, cnt: usize) -> String {
        let date = date.format(&self.date_format).expect(
            "failed to format OffsetDateTime; this is a bug in logforth rolling file appender",
        );

        match (
            &self.rotation,
            &self.log_filename_prefix,
            &self.log_filename_suffix,
        ) {
            (&Rotation::Never, Some(filename), None) => format!("{filename}.{cnt}"),
            (&Rotation::Never, Some(filename), Some(suffix)) => {
                format!("{filename}.{cnt}.{suffix}")
            }
            (&Rotation::Never, None, Some(suffix)) => format!("{cnt}.{suffix}"),
            (_, Some(filename), Some(suffix)) => format!("{filename}.{date}.{cnt}.{suffix}"),
            (_, Some(filename), None) => format!("{filename}.{date}.{cnt}"),
            (_, None, Some(suffix)) => format!("{date}.{cnt}.{suffix}"),
            (_, None, None) => format!("{date}.{cnt}"),
        }
    }

    fn create_log_writer(&self, now: OffsetDateTime, cnt: usize) -> anyhow::Result<File> {
        fs::create_dir_all(&self.log_dir).context("failed to create log directory")?;
        let filename = self.join_date(&now, cnt);
        if let Some(max_files) = self.max_files {
            if let Err(err) = self.delete_oldest_logs(max_files) {
                eprintln!("failed to delete oldest logs: {err}");
            }
        }
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.log_dir.join(filename))
            .context("failed to create log file")
    }

    fn delete_oldest_logs(&self, max_files: usize) -> anyhow::Result<()> {
        let read_dir = fs::read_dir(&self.log_dir)
            .with_context(|| format!("failed to read log dir: {}", self.log_dir.display()))?;

        let mut files = read_dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let metadata = entry.metadata().ok()?;

                // the appender only creates files, not directories or symlinks,
                // so we should never delete a dir or symlink.
                if !metadata.is_file() {
                    return None;
                }

                let filename = entry.file_name();
                // if the filename is not a UTF-8 string, skip it.
                let filename = filename.to_str()?;
                if let Some(prefix) = &self.log_filename_prefix {
                    if !filename.starts_with(prefix) {
                        return None;
                    }
                }

                if let Some(suffix) = &self.log_filename_suffix {
                    if !filename.ends_with(suffix) {
                        return None;
                    }
                }

                if self.log_filename_prefix.is_none()
                    && self.log_filename_suffix.is_none()
                    && Date::parse(filename, &self.date_format).is_err()
                {
                    return None;
                }

                let created = metadata.created().ok()?;
                Some((entry, created))
            })
            .collect::<Vec<_>>();

        if files.len() < max_files {
            return Ok(());
        }

        // sort the files by their creation timestamps.
        files.sort_by_key(|(_, created_at)| *created_at);

        // delete files, so that (n-1) files remain, because we will create another log file
        for (file, _) in files.iter().take(files.len() - (max_files - 1)) {
            fs::remove_file(file.path()).with_context(|| {
                format!("Failed to remove old log file {}", file.path().display())
            })?;
        }

        Ok(())
    }

    fn refresh_writer(&self, now: OffsetDateTime, cnt: usize, file: &mut File) {
        match self.create_log_writer(now, cnt) {
            Ok(new_file) => {
                if let Err(err) = file.flush() {
                    eprintln!("failed to flush previous writer: {err}");
                }
                *file = new_file;
            }
            Err(err) => eprintln!("failed to create writer for logs: {err}"),
        }
    }

    fn should_rollover_on_date(&self, date: OffsetDateTime) -> bool {
        self.next_date_timestamp
            .is_some_and(|ts| date.unix_timestamp() as usize >= ts)
    }

    fn should_rollover_on_size(&self) -> bool {
        self.current_filesize >= self.max_size
    }

    fn advance_cnt(&mut self) -> usize {
        self.current_count += 1;
        self.current_filesize = 0;
        self.current_count
    }

    fn advance_date(&mut self, now: OffsetDateTime) {
        self.current_date = now;
        self.current_count = 0;
        self.current_filesize = 0;
        self.next_date_timestamp = self.rotation.next_date_timestamp(&now);
    }
}

/// Defines a fixed period for rolling of a log file.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Rotation {
    /// Minutely Rotation
    Minutely,
    /// Hourly Rotation
    Hourly,
    /// Daily Rotation
    Daily,
    /// No Rotation
    Never,
}

impl Rotation {
    fn next_date_timestamp(&self, current_date: &OffsetDateTime) -> Option<usize> {
        let next_date = match *self {
            Rotation::Minutely => *current_date + Duration::minutes(1),
            Rotation::Hourly => *current_date + Duration::hours(1),
            Rotation::Daily => *current_date + Duration::days(1),
            Rotation::Never => return None,
        };

        Some(self.round_date(&next_date).unix_timestamp() as usize)
    }

    fn round_date(&self, date: &OffsetDateTime) -> OffsetDateTime {
        match *self {
            Rotation::Minutely => {
                let time = Time::from_hms(date.hour(), date.minute(), 0)
                    .expect("invalid time; this is a bug in logforth rolling file appender");
                date.replace_time(time)
            }
            Rotation::Hourly => {
                let time = Time::from_hms(date.hour(), 0, 0)
                    .expect("invalid time; this is a bug in logforth rolling file appender");
                date.replace_time(time)
            }
            Rotation::Daily => {
                let time = Time::from_hms(0, 0, 0)
                    .expect("invalid time; this is a bug in logforth rolling file appender");
                date.replace_time(time)
            }
            Rotation::Never => unreachable!("Rotation::Never is impossible to round."),
        }
    }

    fn date_format(&self) -> Vec<format_description::FormatItem<'static>> {
        match *self {
            Rotation::Minutely => format_description::parse("[year]-[month]-[day]-[hour]-[minute]"),
            Rotation::Hourly => format_description::parse("[year]-[month]-[day]-[hour]"),
            Rotation::Daily => format_description::parse("[year]-[month]-[day]"),
            Rotation::Never => format_description::parse("[year]-[month]-[day]"),
        }
        .expect("failed to create a formatter; this is a bug in logforth rolling file appender")
    }
}
