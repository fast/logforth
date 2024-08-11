// Copyright 2024 CratesLand Developers
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

use log::Record;

use crate::append::rolling_file::non_blocking::NonBlocking;
use crate::append::Append;

/// An appender that writes log records to a file that rolls over when it reaches a certain date
/// time.
#[derive(Debug)]
pub struct RollingFile {
    writer: NonBlocking,
}

impl RollingFile {
    pub fn new(writer: NonBlocking) -> Self {
        Self { writer }
    }
}

impl Append for RollingFile {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let bytes = format!("{}\n", record.args()).into_bytes();
        self.writer.send(bytes)?;
        Ok(())
    }
}
