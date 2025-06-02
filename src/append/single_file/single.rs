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
use std::path::PathBuf;

use anyhow::Context;

/// A writer for files.
#[derive(Debug)]
pub struct SingleFileWriter {
    writer: File,
}

impl Write for SingleFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let writer = &mut self.writer;

        writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// A builder for configuring [`SingleFileWriter`].
#[derive(Debug)]
pub struct SingleFileWriterBuilder {
    // required
    filepath: PathBuf,
}

impl SingleFileWriterBuilder {
    /// Creates a new [`SingleFileWriterBuilder`].
    #[must_use]
    pub fn new(filepath: impl Into<PathBuf>) -> Self {
        Self {
            filepath: filepath.into(),
        }
    }

    /// Builds the [`SingleFileWriter`].
    pub fn build(self) -> anyhow::Result<SingleFileWriter> {
        let dir = &self
            .filepath
            .parent()
            .context("failed to get log directory")?;
        fs::create_dir_all(dir).context("failed to create log directory")?;
        let writer = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.filepath)
            .context("failed to create log file")?;
        Ok(SingleFileWriter { writer })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use rand::distr::Alphanumeric;
    use rand::Rng;
    use tempfile::NamedTempFile;

    use crate::append::single_file::single::SingleFileWriterBuilder;

    #[test]
    fn test_single_file() {
        // To Do: Make this a file
        let temp_file = NamedTempFile::new().expect("failed to create a temporary directory");

        let mut writer = SingleFileWriterBuilder::new(temp_file.path())
            .build()
            .unwrap();

        let rand_str = generate_random_string();
        assert_eq!(writer.write(rand_str.as_bytes()).unwrap(), rand_str.len());
        writer.flush().unwrap();
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
