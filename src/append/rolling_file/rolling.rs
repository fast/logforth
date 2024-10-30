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
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use jiff::Zoned;
use parking_lot::RwLock;

use crate::append::rolling_file::clock::Clock;
use crate::append::rolling_file::Rotation;

/// A writer for rolling files.
#[derive(Debug)]
pub struct RollingFileWriter {
    state: State,
    writer: RwLock<File>,
}

impl RollingFileWriter {
    /// Creates a new [`RollingFileWriterBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::rolling_file::RollingFileWriter;
    ///
    /// let builder = RollingFileWriter::builder();
    /// ```
    #[must_use]
    pub fn builder() -> RollingFileWriterBuilder {
        RollingFileWriterBuilder::new()
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = self.state.clock.now();
        let writer = self.writer.get_mut();
        if self.state.should_rollover_on_date(&now) {
            self.state.advance_date(&now);
            self.state.refresh_writer(&now, 0, writer);
        }
        if self.state.should_rollover_on_size() {
            let cnt = self.state.advance_cnt();
            self.state.refresh_writer(&now, cnt, writer);
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

/// A builder for configuring [`RollingFileWriter`].
#[derive(Debug)]
pub struct RollingFileWriterBuilder {
    rotation: Rotation,
    prefix: Option<String>,
    suffix: Option<String>,
    max_size: usize,
    max_files: Option<usize>,
    clock: Clock,
}

impl Default for RollingFileWriterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RollingFileWriterBuilder {
    /// Creates a new [`RollingFileWriterBuilder`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rotation: Rotation::Never,
            prefix: None,
            suffix: None,
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

    /// Sets the filename prefix.
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

    /// Sets the filename suffix.
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
    pub fn build(self, dir: impl AsRef<Path>) -> anyhow::Result<RollingFileWriter> {
        let Self {
            rotation,
            prefix,
            suffix,
            max_size,
            max_files,
            clock,
        } = self;
        let directory = dir.as_ref().to_path_buf();
        let (state, writer) = State::new(
            rotation, directory, prefix, suffix, max_size, max_files, clock,
        )?;
        Ok(RollingFileWriter { state, writer })
    }
}

#[derive(Debug)]
struct State {
    log_dir: PathBuf,
    log_filename_prefix: Option<String>,
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
        log_filename_prefix: Option<String>,
        log_filename_suffix: Option<String>,
        max_size: usize,
        max_files: Option<usize>,
        clock: Clock,
    ) -> anyhow::Result<(Self, RwLock<File>)> {
        let log_dir = dir.as_ref().to_path_buf();
        let date_format = rotation.date_format();
        let now = clock.now();
        let next_date_timestamp = rotation.next_date_timestamp(&now);

        let current_count = 0;
        let current_filesize = 0;

        let state = State {
            log_dir,
            log_filename_prefix,
            log_filename_suffix,
            date_format,
            current_count,
            current_filesize,
            next_date_timestamp,
            rotation,
            max_size,
            max_files,
            clock,
        };

        let file = state.create_log_writer(&now, 0)?;
        let writer = RwLock::new(file);
        Ok((state, writer))
    }

    fn join_date(&self, date: &Zoned, cnt: usize) -> String {
        let date = date.strftime(self.date_format);
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

    fn create_log_writer(&self, now: &Zoned, cnt: usize) -> anyhow::Result<File> {
        fs::create_dir_all(&self.log_dir).context("failed to create log directory")?;
        let filename = self.join_date(now, cnt);
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
                    && jiff::civil::DateTime::strptime(self.date_format, filename).is_err()
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

    fn refresh_writer(&self, now: &Zoned, cnt: usize, file: &mut File) {
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

    fn should_rollover_on_date(&self, date: &Zoned) -> bool {
        self.next_date_timestamp
            .is_some_and(|ts| date.timestamp().as_millisecond() as usize >= ts)
    }

    fn should_rollover_on_size(&self) -> bool {
        self.current_filesize >= self.max_size
    }

    fn advance_cnt(&mut self) -> usize {
        self.current_count += 1;
        self.current_filesize = 0;
        self.current_count
    }

    fn advance_date(&mut self, now: &Zoned) {
        self.current_count = 0;
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
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use tempfile::TempDir;

    use crate::append::rolling_file::clock::Clock;
    use crate::append::rolling_file::clock::ManualClock;
    use crate::append::rolling_file::RollingFileWriterBuilder;
    use crate::append::rolling_file::Rotation;

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

        let mut writer = RollingFileWriterBuilder::new()
            .rotation(Rotation::Never)
            .filename_prefix("test_prefix")
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(max_size)
            .build(&temp_dir)
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
        let mut writer = RollingFileWriterBuilder::new()
            .rotation(rotation)
            .filename_prefix("test_prefix")
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(usize::MAX)
            .clock(Clock::ManualClock(ManualClock::new(start_time.clone())))
            .build(&temp_dir)
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
        let mut writer = RollingFileWriterBuilder::new()
            .rotation(rotation)
            .filename_prefix("test_prefix")
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(file_size)
            .clock(Clock::ManualClock(ManualClock::new(start_time.clone())))
            .build(&temp_dir)
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
        let mut rng = rand::thread_rng();
        let len = rng.gen_range(50..=100);
        let random_string: String = std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(len)
            .collect();

        random_string
    }
}
