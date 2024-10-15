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

use std::io::Write;

use crate::append::Append;
use crate::Encoder;

/// An appender that prints log records to stdout.
#[derive(Debug)]
pub struct Stdout {
    encoder: Encoder,
}

impl Stdout {
    /// Creates a new `Stdout` appender with the given encoder.
    pub fn new(encoder: impl Into<Encoder>) -> Self {
        Self {
            encoder: encoder.into(),
        }
    }
}

impl Append for Stdout {
    fn append(&self, record: &log::Record) -> anyhow::Result<()> {
        let mut bytes = self.encoder.format(record)?;
        bytes.push(b'\n');
        std::io::stdout().write_all(&bytes)?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

/// An appender that prints log records to stderr.
#[derive(Debug)]
pub struct Stderr {
    encoder: Encoder,
}

impl Stderr {
    /// Creates a new `Stderr` appender with the given encoder.
    pub fn new(encoder: impl Into<Encoder>) -> Self {
        Self {
            encoder: encoder.into(),
        }
    }
}

impl Append for Stderr {
    fn append(&self, record: &log::Record) -> anyhow::Result<()> {
        let mut bytes = self.encoder.format(record)?;
        bytes.push(b'\n');
        std::io::stderr().write_all(&bytes)?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}
