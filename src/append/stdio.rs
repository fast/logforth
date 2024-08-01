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

use std::io::Write;

use crate::append::Append;
use crate::append::AppendImpl;

#[derive(Default, Debug)]
pub struct Stdout;

impl Append for Stdout {
    fn try_append(&self, record: &log::Record) -> anyhow::Result<()> {
        let bytes = format!("{}\n", record.args()).into_bytes();
        std::io::stdout().write_all(&bytes)?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

impl From<Stdout> for AppendImpl {
    fn from(append: Stdout) -> Self {
        AppendImpl::Stdout(append)
    }
}

#[derive(Default, Debug)]
pub struct Stderr;
impl Append for Stderr {
    fn try_append(&self, record: &log::Record) -> anyhow::Result<()> {
        let bytes = format!("{}\n", record.args()).into_bytes();
        std::io::stderr().write_all(&bytes)?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

impl From<Stderr> for AppendImpl {
    fn from(append: Stderr) -> Self {
        AppendImpl::Stderr(append)
    }
}
