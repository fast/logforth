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
use std::fs::Metadata;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use jiff::Zoned;
use jiff::civil::DateTime;
use logforth_core::Error;
use logforth_core::Trap;
use logforth_core::trap::BestEffortTrap;

use crate::clock::Clock;
use crate::rotation::Rotation;

/// A writer for rolling files.
#[derive(Debug)]
pub struct RollingFileWriter {
    state: State,
    writer: File,
}

impl Drop for RollingFileWriter {
    fn drop(&mut self) {
        if let Err(err) = self.writer.flush() {
            let err = Error::new("failed to flush file writer on dropped").with_source(err);
            self.state.trap.trap(&err);
        }
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = self.state.clock.now();
        let writer = &mut self.writer;

        if self.state.should_rollover_on_date(&now) {
            self.state.current_filesize = 0;
            self.state.next_date_timestamp = self.state.rotation.next_date_timestamp(&now);
            let current = &self.state.this_date_timestamp;
            self.state.refresh_writer(current, writer);
        }

        if self.state.should_rollover_on_size() {
            self.state.current_filesize = 0;
            self.state.refresh_writer(&now, writer);
        }

        self.state.this_date_timestamp = now;

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
    max_size: Option<NonZeroUsize>,
    max_files: Option<NonZeroUsize>,
    clock: Clock,
    trap: Box<dyn Trap>,
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
            max_size: None,
            max_files: None,
            clock: Clock::DefaultClock,
            trap: Box::new(BestEffortTrap::default()),
        }
    }

    /// Set the trap for the rolling file writer.
    pub fn trap(mut self, trap: impl Into<Box<dyn Trap>>) -> Self {
        self.trap = trap.into();
        self
    }

    /// Set the rotation policy.
    #[must_use]
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the filename suffix.
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

    /// Set the maximum number of log files to keep.
    #[must_use]
    pub fn max_log_files(mut self, n: NonZeroUsize) -> Self {
        self.max_files = Some(n);
        self
    }

    /// Set the maximum size of a log file in bytes.
    #[must_use]
    pub fn max_file_size(mut self, n: NonZeroUsize) -> Self {
        self.max_size = Some(n);
        self
    }

    #[cfg(test)]
    fn clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Builds the [`RollingFileWriter`].
    pub fn build(self) -> Result<RollingFileWriter, Error> {
        let Self {
            basedir,
            rotation,
            filename,
            filename_suffix,
            max_size,
            max_files,
            clock,
            trap,
        } = self;

        if filename.is_empty() {
            return Err(Error::new("filename must not be empty"));
        }

        let (state, writer) = State::new(
            rotation,
            basedir,
            filename,
            filename_suffix,
            max_size,
            max_files,
            clock,
            trap,
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

// oldest is the least
fn compare_logfile(a: &LogFile, b: &LogFile) -> std::cmp::Ordering {
    match a.datetime.cmp(&b.datetime) {
        std::cmp::Ordering::Equal => {
            let a_rev = usize::MAX - a.count;
            let b_rev = usize::MAX - b.count;
            a_rev.cmp(&b_rev)
        }
        ord => ord,
    }
}

#[derive(Debug)]
struct State {
    log_dir: PathBuf,
    log_filename: String,
    log_filename_suffix: Option<String>,
    date_format: &'static str,
    rotation: Rotation,
    current_filesize: usize,
    this_date_timestamp: Zoned,
    next_date_timestamp: Option<usize>,
    max_size: Option<NonZeroUsize>,
    max_files: Option<NonZeroUsize>,
    clock: Clock,
    trap: Box<dyn Trap>,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    fn new(
        rotation: Rotation,
        dir: impl AsRef<Path>,
        log_filename: String,
        log_filename_suffix: Option<String>,
        max_size: Option<NonZeroUsize>,
        max_files: Option<NonZeroUsize>,
        clock: Clock,
        trap: Box<dyn Trap>,
    ) -> Result<(Self, File), Error> {
        let now = clock.now();
        let log_dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&log_dir)
            .map_err(|err| Error::new("failed to create log directory").with_source(err))?;

        let mut state = State {
            log_dir,
            log_filename,
            log_filename_suffix,
            date_format: rotation.date_format(),
            current_filesize: 0,
            this_date_timestamp: clock.now(),
            next_date_timestamp: rotation.next_date_timestamp(&now),
            rotation,
            max_size,
            max_files,
            clock,
            trap,
        };

        let files = {
            let mut files = state.list_logfiles()?;
            files.sort_by(compare_logfile);
            files
        };

        let file = match files.last() {
            None => {
                // brand-new directory
                state.create_log_writer()?
            }
            Some(last) => {
                let filename = state.current_filename();
                if last.filepath != filename {
                    // for some reason, the `filename.suffix` file does not exist; create a new one
                    state.create_log_writer()?
                } else {
                    state.current_filesize = last.metadata.len() as usize;

                    if let Ok(mtime) = last.metadata.modified()
                        && let Ok(mtime) = Zoned::try_from(mtime)
                    {
                        state.next_date_timestamp = state.rotation.next_date_timestamp(&mtime);
                        state.this_date_timestamp = mtime;
                    }

                    // continue to use the existing current log file
                    OpenOptions::new()
                        .append(true)
                        .open(&filename)
                        .map_err(|err| Error::new("failed to open current log").with_source(err))?
                }
            }
        };

        Ok((state, file))
    }

    fn current_filename(&self) -> PathBuf {
        let filename = &self.log_filename;
        match self.log_filename_suffix.as_ref() {
            None => self.log_dir.join(filename),
            Some(suffix) => self.log_dir.join(format!("{filename}.{suffix}")),
        }
    }

    fn create_log_writer(&self) -> Result<File, Error> {
        let filename = self.current_filename();
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filename)
            .map_err(|err| Error::new("failed to create log file").with_source(err))
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

    fn list_logfiles(&self) -> Result<Vec<LogFile>, Error> {
        let read_dir = fs::read_dir(&self.log_dir).map_err(|err| {
            Error::new(format!(
                "failed to read log dir: {}",
                self.log_dir.display()
            ))
            .with_source(err)
        })?;

        let files = read_dir
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
                        count: 0,
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

        Ok(files)
    }

    fn delete_oldest_logs(&self, max_files: usize) -> Result<(), Error> {
        let mut files = self.list_logfiles()?;
        if files.len() < max_files {
            return Ok(());
        }

        // delete files, so that (n-1) files remain, because we will create another log file
        files.sort_by(compare_logfile);
        for file in files.iter().take(files.len() - (max_files - 1)) {
            let filepath = &file.filepath;
            fs::remove_file(filepath).map_err(|err| {
                Error::new(format!("failed to remove old log: {}", filepath.display()))
                    .with_source(err)
            })?;
        }

        Ok(())
    }

    fn rotate_log_writer(&self, now: &Zoned) -> Result<File, Error> {
        let mut renames = vec![];
        for i in 1..self.max_files.map_or(usize::MAX, |n| n.get()) {
            let filepath = self.join_date(now, i);
            if fs::exists(&filepath).is_ok_and(|ok| ok) {
                let next = self.join_date(now, i + 1);
                renames.push((filepath, next));
            } else {
                break;
            }
        }

        for (old, new) in renames.iter().rev() {
            fs::rename(old, new).map_err(|err| {
                Error::new(format!("failed to rotate log: {}", old.display())).with_source(err)
            })?
        }

        let archive_filepath = self.join_date(now, 1);
        let current_filepath = self.current_filename();
        fs::rename(&current_filepath, &archive_filepath).map_err(|err| {
            Error::new(format!(
                "failed to archive log: {}",
                current_filepath.display()
            ))
            .with_source(err)
        })?;

        if let Some(max_files) = self.max_files
            && let Err(err) = self.delete_oldest_logs(max_files.get())
        {
            let err = Error::new("failed to delete oldest logs").with_source(err);
            self.trap.trap(&err);
        }

        self.create_log_writer()
    }

    fn refresh_writer(&self, now: &Zoned, file: &mut File) {
        match self.rotate_log_writer(now) {
            Ok(new_file) => {
                if let Err(err) = file.flush() {
                    let err = Error::new("failed to flush previous writer").with_source(err);
                    self.trap.trap(&err);
                }
                *file = new_file;
            }
            Err(err) => {
                let err = Error::new("failed to rotate log writer").with_source(err);
                self.trap.trap(&err);
            }
        }
    }

    fn should_rollover_on_date(&self, date: &Zoned) -> bool {
        self.next_date_timestamp
            .is_some_and(|ts| date.timestamp().as_millisecond() as usize >= ts)
    }

    fn should_rollover_on_size(&self) -> bool {
        self.max_size
            .is_some_and(|n| self.current_filesize >= n.get())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;
    use std::fs;
    use std::io::Write;
    use std::num::NonZeroUsize;
    use std::ops::Add;
    use std::str::FromStr;

    use jiff::Span;
    use jiff::Zoned;
    use rand::Rng;
    use rand::distr::Alphanumeric;
    use tempfile::TempDir;

    use crate::clock::Clock;
    use crate::clock::ManualClock;
    use crate::rolling::RollingFileWriterBuilder;
    use crate::rotation::Rotation;

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
        let max_files = NonZeroUsize::new(max_files).unwrap();
        let max_size = NonZeroUsize::new(max_size).unwrap();
        let temp_dir = TempDir::new().unwrap();

        let mut writer = RollingFileWriterBuilder::new(temp_dir.as_ref(), "test_file")
            .rotation(Rotation::Never)
            .filename_suffix("log")
            .max_log_files(max_files)
            .max_file_size(max_size)
            .build()
            .unwrap();

        for i in 1..=(max_files.get() * 2) {
            let mut expected_file_size = 0;
            while expected_file_size < max_size.get() {
                let rand_str = generate_random_string();
                expected_file_size += rand_str.len();
                assert_eq!(writer.write(rand_str.as_bytes()).unwrap(), rand_str.len());
                assert_eq!(writer.state.current_filesize, expected_file_size);
            }

            writer.flush().unwrap();
            assert_eq!(
                fs::read_dir(&writer.state.log_dir).unwrap().count(),
                min(i, max_files.get())
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
        let max_files = NonZeroUsize::new(10).unwrap();
        let temp_dir = TempDir::new().unwrap();

        let start_time = Zoned::from_str("2024-08-10T00:00:00[UTC]").unwrap();
        let mut writer = RollingFileWriterBuilder::new(temp_dir.as_ref(), "test_file")
            .rotation(rotation)
            .filename_suffix("log")
            .max_log_files(max_files)
            .clock(Clock::ManualClock(ManualClock::new(start_time.clone())))
            .build()
            .unwrap();

        let mut cur_time = start_time;

        for i in 1..=(max_files.get() * 2) {
            let mut expected_file_size = 0;
            let end_time = &cur_time + rotation_duration;
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
                min(i, max_files.get())
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
        let max_files = NonZeroUsize::new(10).unwrap();
        let file_size = NonZeroUsize::new(500).unwrap();
        // Small file size and too many files to ensure both of file size and time rotation can be
        // triggered.
        let total_files = 100;
        let temp_dir = TempDir::new().unwrap();

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
        let mut end_time = &cur_time + rotation_duration;
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

                cur_time += write_interval;

                if cur_time >= end_time {
                    end_time = end_time.add(rotation_duration);
                    time_rotation_trigger = true;
                    break;
                }
                if expected_file_size >= file_size.get() {
                    file_size_rotation_trigger = true;
                    break;
                }
            }

            writer.flush().unwrap();
            assert_eq!(
                fs::read_dir(&writer.state.log_dir).unwrap().count(),
                min(i, max_files.get())
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
