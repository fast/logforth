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

use anyhow::Context;
use anyhow::ensure;
use jiff::Zoned;
use jiff::civil::DateTime;
use std::fs;
use std::fs::File;
use std::fs::{Metadata, OpenOptions};
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use crate::append::rolling_file::Rotation;
use crate::append::rolling_file::clock::Clock;

/// A writer for rolling files.
#[derive(Debug)]
pub struct RollingFileWriter {
    state: State,
    writer: File,
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = self.state.clock.now();
        let writer = &mut self.writer;
        if self.state.should_rollover_on_date(&now) {
            self.state.advance_date(&now);
            self.state.refresh_writer(&now, 0, writer);
        }
        if self.state.should_rollover_on_size() {
            let cnt = self.state.advance_cnt();
            self.state.refresh_writer(&now, cnt, writer);
        }

        writer
            .write(buf)
            .inspect(|&n| self.state.current_filesize += n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// A builder for configuring [`RollingFileWriter`].
#[derive(Debug)]
pub struct RollingFileWriterBuilder {
    // required
    basedir: PathBuf,
    filename: String,

    // has default
    rotation: Rotation,
    filename_suffix: Option<String>,
    max_size: usize,
    max_files: Option<usize>,
    clock: Clock,
}

impl RollingFileWriterBuilder {
    /// Creates a new [`RollingFileWriterBuilder`].
    #[must_use]
    pub fn new(basedir: impl Into<PathBuf>, filename: impl Into<String>) -> Self {
        Self {
            basedir: basedir.into(),
            filename: filename.into(),
            rotation: Rotation::Never,
            filename_suffix: None,
            max_size: usize::MAX,
            max_files: None,
            clock: Clock::DefaultClock,
        }
    }

    /// Sets the rotation policy.
    #[must_use]
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets the filename suffix.
    #[must_use]
    pub fn filename_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        self.filename_suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix)
        };
        self
    }

    /// Sets the maximum number of log files to keep.
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

    #[cfg(test)]
    fn clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Builds the [`RollingFileWriter`].
    pub fn build(self) -> anyhow::Result<RollingFileWriter> {
        let Self {
            basedir,
            rotation,
            filename,
            filename_suffix,
            max_size,
            max_files,
            clock,
        } = self;

        ensure!(!filename.is_empty(), "filename must not be empty");

        let (state, writer) = State::new(
            rotation,
            basedir,
            filename,
            filename_suffix,
            max_size,
            max_files,
            clock,
        )?;

        Ok(RollingFileWriter { state, writer })
    }
}

#[derive(Debug)]
struct LogFile {
    filepath: PathBuf,
    metadata: Metadata,
    datetime: DateTime,
    count: usize,
}

#[derive(Debug)]
struct State {
    log_dir: PathBuf,
    log_filename: String,
    log_filename_suffix: Option<String>,
    date_format: &'static str,
    rotation: Rotation,
    current_count: usize,
    current_filesize: usize,
    next_date_timestamp: Option<usize>,
    max_size: usize,
    max_files: Option<usize>,
    clock: Clock,
}

impl State {
    fn new(
        rotation: Rotation,
        dir: impl AsRef<Path>,
        log_filename: String,
        log_filename_suffix: Option<String>,
        max_size: usize,
        max_files: Option<usize>,
        clock: Clock,
    ) -> anyhow::Result<(Self, File)> {
        let now = clock.now();
        let log_dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&log_dir).context("failed to create log directory")?;

        let mut state = State {
            log_dir,
            log_filename,
            log_filename_suffix,
            date_format: rotation.date_format(),
            current_count: 0,
            current_filesize: 0,
            next_date_timestamp: rotation.next_date_timestamp(&now),
            rotation,
            max_size,
            max_files,
            clock,
        };

        let mut files = state.list_sorted_logs()?;

        let file;
        match files.pop() {
            None => {
                // brand-new directory
                file = state.create_log_writer()?;
            }
            Some(last) => {
                let last_logfile = last.filepath;
                let current_date = rotation.current_datetime(&now);
                let current_logfile = state.current_filename();

                if last_logfile != current_logfile {
                    if current_date.is_none_or(|date| date == last.datetime) {
                        state.current_count = last.count + 1;
                    }

                    // for some reason, the `filename.suffix` file does not exist, create a new one
                    file = state.create_log_writer()?;
                } else {
                    if let Some(file) = files.pop() {
                        if current_date.is_none_or(|date| date == file.datetime) {
                            state.current_count = file.count + 1;
                        }
                    }

                    // continue to use the existing current log file
                    state.current_filesize = last.metadata.len() as usize;
                    file = OpenOptions::new()
                        .append(true)
                        .open(&current_logfile)
                        .context("failed to open existing log file")?;
                }
            }
        }

        Ok((state, file))
    }

    fn current_filename(&self) -> PathBuf {
        let filename = &self.log_filename;
        match self.log_filename_suffix.as_ref() {
            None => self.log_dir.join(filename),
            Some(suffix) => self.log_dir.join(format!("{filename}.{suffix}")),
        }
    }

    fn create_log_writer(&self) -> anyhow::Result<File> {
        let filename = self.current_filename();
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filename)
            .context("failed to create log file")
    }

    fn join_date(&self, date: &Zoned, cnt: usize) -> PathBuf {
        let date = date.strftime(self.date_format);
        let filename = match (
            &self.rotation,
            &self.log_filename,
            &self.log_filename_suffix,
        ) {
            (&Rotation::Never, filename, None) => format!("{filename}.{cnt}"),
            (&Rotation::Never, filename, Some(suffix)) => {
                format!("{filename}.{cnt}.{suffix}")
            }
            (_, filename, Some(suffix)) => format!("{filename}.{date}.{cnt}.{suffix}"),
            (_, filename, None) => format!("{filename}.{date}.{cnt}"),
        };
        self.log_dir.join(filename)
    }

    // sorted from oldest to newest
    fn list_sorted_logs(&self) -> anyhow::Result<Vec<LogFile>> {
        let read_dir = fs::read_dir(&self.log_dir)
            .with_context(|| format!("failed to read log dir: {}", self.log_dir.display()))?;

        let mut files = read_dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let filepath = entry.path();

                let metadata = entry.metadata().ok()?;
                // the appender only creates files, not directories or symlinks,
                if !metadata.is_file() {
                    return None;
                }

                let filename = entry.file_name();
                // if the filename is not a UTF-8 string, skip it.
                let mut filename = filename.to_str()?;
                if !filename.starts_with(&self.log_filename) {
                    return None;
                }
                filename = &filename[self.log_filename.len()..];

                if let Some(suffix) = &self.log_filename_suffix {
                    if !filename.ends_with(suffix) {
                        return None;
                    }
                    filename = &filename[..filename.len() - suffix.len() - 1];
                }

                if filename.is_empty() {
                    // the current log file is the largest
                    return Some(LogFile {
                        filepath,
                        metadata,
                        datetime: DateTime::MAX,
                        count: usize::MAX,
                    });
                }

                if filename.starts_with(".") {
                    filename = &filename[1..];
                } else {
                    return None;
                }

                let datetime = if self.rotation != Rotation::Never {
                    // mandatory datetime part
                    let pos = filename.find('.')?;
                    let datetime = DateTime::strptime(self.date_format, &filename[..pos]).ok()?;
                    filename = &filename[pos + 1..];
                    datetime
                } else {
                    DateTime::MAX
                };

                let count = usize::from_str(&filename[..filename.len()]).ok()?;

                Some(LogFile {
                    filepath,
                    metadata,
                    datetime,
                    count,
                })
            })
            .collect::<Vec<_>>();

        files.sort_by_key(|f| (f.datetime, f.count));
        Ok(files)
    }

    fn delete_oldest_logs(&self, max_files: usize) -> anyhow::Result<()> {
        let files = self.list_sorted_logs()?;

        if files.len() < max_files {
            return Ok(());
        }

        // delete files, so that (n-1) files remain, because we will create another log file
        for file in files.iter().take(files.len() - (max_files - 1)) {
            let filepath = &file.filepath;
            fs::remove_file(filepath).context("failed to remove old log file")?;
        }

        Ok(())
    }

    fn rotate_log_writer(&self, now: &Zoned, cnt: usize) -> anyhow::Result<File> {
        let archive_filepath = self.join_date(now, cnt);
        let current_filepath = self.current_filename();

        fs::rename(&current_filepath, &archive_filepath)?;
        if let Some(max_files) = self.max_files {
            if let Err(err) = self.delete_oldest_logs(max_files) {
                eprintln!("failed to delete oldest logs: {err}");
            }
        }

        self.create_log_writer()
    }

    fn refresh_writer(&self, now: &Zoned, cnt: usize, file: &mut File) {
        match self.rotate_log_writer(now, cnt) {
            Ok(new_file) => {
                if let Err(err) = file.flush() {
                    eprintln!("failed to flush previous writer: {err}");
                }
                *file = new_file;
            }
            Err(err) => eprintln!("failed to rotate log writer: {err}"),
        }
    }

    fn should_rollover_on_date(&self, date: &Zoned) -> bool {
        self.next_date_timestamp
            .is_some_and(|ts| date.timestamp().as_millisecond() as usize >= ts)
    }

    fn should_rollover_on_size(&self) -> bool {
        self.current_filesize >= self.max_size
    }

    fn advance_cnt(&mut self) -> usize {
        let cnt = self.current_count;
        self.current_count += 1;
        self.current_filesize = 0;
        cnt
    }

    fn advance_date(&mut self, now: &Zoned) {
        self.current_count = 1;
        self.current_filesize = 0;
        self.next_date_timestamp = self.rotation.next_date_timestamp(now);
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;
    use std::fs;
    use std::io::Write;
    use std::ops::Add;
    use std::str::FromStr;

    use jiff::Span;
    use jiff::Zoned;
    use rand::Rng;
    use rand::distr::Alphanumeric;
    use tempfile::TempDir;

    use crate::append::rolling_file::Rotation;
    use crate::append::rolling_file::clock::Clock;
    use crate::append::rolling_file::clock::ManualClock;
    use crate::append::rolling_file::rolling::RollingFileWriterBuilder;

    #[test]
    fn test_file_rolling_via_file_size() {
        test_file_rolling_for_specific_file_size(3, 1000);
        test_file_rolling_for_specific_file_size(3, 10000);
        test_file_rolling_for_specific_file_size(10, 8888);
        test_file_rolling_for_specific_file_size(10, 10000);
        test_file_rolling_for_specific_file_size(20, 6666);
        test_file_rolling_for_specific_file_size(20, 10000);
    }

    fn test_file_rolling_for_specific_file_size(max_files: usize, max_size: usize) {
        let temp_dir = TempDir::new().expect("failed to create a temporary directory");

        let mut writer = RollingFileWriterBuilder::new(temp_dir.as_ref(), "test_file")
            .rotation(Rotation::Never)
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(max_size)
            .build()
            .unwrap();

        for i in 1..=(max_files * 2) {
            let mut expected_file_size = 0;
            while expected_file_size < max_size {
                let rand_str = generate_random_string();
                expected_file_size += rand_str.len();
                assert_eq!(writer.write(rand_str.as_bytes()).unwrap(), rand_str.len());
                assert_eq!(writer.state.current_filesize, expected_file_size);
            }

            writer.flush().unwrap();
            assert_eq!(
                fs::read_dir(&writer.state.log_dir).unwrap().count(),
                min(i, max_files)
            );
        }
    }

    #[test]
    fn test_file_rolling_via_time_rotation() {
        test_file_rolling_for_specific_time_rotation(
            Rotation::Minutely,
            Span::new().minutes(1),
            Span::new().seconds(1),
        );
        test_file_rolling_for_specific_time_rotation(
            Rotation::Hourly,
            Span::new().hours(1),
            Span::new().minutes(1),
        );
        test_file_rolling_for_specific_time_rotation(
            Rotation::Daily,
            Span::new().days(1),
            Span::new().hours(1),
        );
    }

    fn test_file_rolling_for_specific_time_rotation(
        rotation: Rotation,
        rotation_duration: Span,
        write_interval: Span,
    ) {
        let temp_dir = TempDir::new().expect("failed to create a temporary directory");
        let max_files = 10;

        let start_time = Zoned::from_str("2024-08-10T00:00:00[UTC]").unwrap();
        let mut writer = RollingFileWriterBuilder::new(temp_dir.as_ref(), "test_file")
            .rotation(rotation)
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(usize::MAX)
            .clock(Clock::ManualClock(ManualClock::new(start_time.clone())))
            .build()
            .unwrap();

        let mut cur_time = start_time;

        for i in 1..=(max_files * 2) {
            let mut expected_file_size = 0;
            let end_time = cur_time.add(rotation_duration);
            while cur_time < end_time {
                writer.state.clock.set_now(cur_time.clone());

                let rand_str = generate_random_string();
                expected_file_size += rand_str.len();

                assert_eq!(writer.write(rand_str.as_bytes()).unwrap(), rand_str.len());
                assert_eq!(writer.state.current_filesize, expected_file_size);

                cur_time = cur_time.add(write_interval);
            }

            writer.flush().unwrap();
            assert_eq!(
                fs::read_dir(&writer.state.log_dir).unwrap().count(),
                min(i, max_files)
            );
        }
    }

    #[test]
    fn test_file_rolling_via_file_size_and_time_rotation() {
        test_file_size_and_time_rotation_for_specific_time_rotation(
            Rotation::Minutely,
            Span::new().minutes(1),
            Span::new().seconds(1),
        );
        test_file_size_and_time_rotation_for_specific_time_rotation(
            Rotation::Hourly,
            Span::new().hours(1),
            Span::new().minutes(1),
        );
        test_file_size_and_time_rotation_for_specific_time_rotation(
            Rotation::Daily,
            Span::new().days(1),
            Span::new().hours(1),
        );
    }

    fn test_file_size_and_time_rotation_for_specific_time_rotation(
        rotation: Rotation,
        rotation_duration: Span,
        write_interval: Span,
    ) {
        let temp_dir = TempDir::new().expect("failed to create a temporary directory");
        let max_files = 10;
        // Small file size and too many files to ensure both of file size and time rotation can be
        // triggered.
        let total_files = 100;
        let file_size = 500;

        let start_time = Zoned::from_str("2024-08-10T00:00:00[UTC]").unwrap();
        let mut writer = RollingFileWriterBuilder::new(temp_dir.as_ref(), "test_file")
            .rotation(rotation)
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(file_size)
            .clock(Clock::ManualClock(ManualClock::new(start_time.clone())))
            .build()
            .unwrap();

        let mut cur_time = start_time;
        let mut end_time = cur_time.add(rotation_duration);
        let mut time_rotation_trigger = false;
        let mut file_size_rotation_trigger = false;

        for i in 1..=total_files {
            let mut expected_file_size = 0;
            loop {
                writer.state.clock.set_now(cur_time.clone());

                let rand_str = generate_random_string();
                expected_file_size += rand_str.len();

                assert_eq!(writer.write(rand_str.as_bytes()).unwrap(), rand_str.len());
                assert_eq!(writer.state.current_filesize, expected_file_size);

                cur_time = cur_time.add(write_interval);

                if cur_time >= end_time {
                    end_time = end_time.add(rotation_duration);
                    time_rotation_trigger = true;
                    break;
                }
                if expected_file_size >= file_size {
                    file_size_rotation_trigger = true;
                    break;
                }
            }

            writer.flush().unwrap();
            assert_eq!(
                fs::read_dir(&writer.state.log_dir).unwrap().count(),
                min(i, max_files)
            );
        }
        assert!(file_size_rotation_trigger);
        assert!(time_rotation_trigger);
    }

    fn generate_random_string() -> String {
        let mut rng = rand::rng();
        let len = rng.random_range(50..=100);
        let random_string: String = std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(len)
            .collect();

        random_string
    }
}
