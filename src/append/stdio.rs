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

use crate::Append;
use crate::AppendImpl;
use crate::Layout;
use crate::LayoutImpl;
use crate::SimpleTextLayout;

#[derive(Debug)]
pub struct StdoutAppend {
    layout: LayoutImpl,
}

impl Default for StdoutAppend {
    fn default() -> Self {
        Self::new()
    }
}

impl StdoutAppend {
    pub fn new() -> Self {
        Self {
            layout: LayoutImpl::SimpleText(SimpleTextLayout::default()),
        }
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutImpl>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for StdoutAppend {
    fn try_append(&self, record: &log::Record) -> anyhow::Result<()> {
        let bytes = self.layout.format_bytes(record)?;
        std::io::stdout().write_all(&bytes)?;
        std::io::stdout().write_all(b"\n")?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

impl From<StdoutAppend> for AppendImpl {
    fn from(append: StdoutAppend) -> Self {
        AppendImpl::Stdout(append)
    }
}

#[derive(Debug)]
pub struct StderrAppend {
    layout: LayoutImpl,
}

impl Default for StderrAppend {
    fn default() -> Self {
        Self::new()
    }
}

impl StderrAppend {
    pub fn new() -> Self {
        Self {
            layout: LayoutImpl::SimpleText(SimpleTextLayout::default()),
        }
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutImpl>) -> Self {
        self.layout = layout.into();
        self
    }
}

impl Append for StderrAppend {
    fn try_append(&self, record: &log::Record) -> anyhow::Result<()> {
        let bytes = self.layout.format_bytes(record)?;
        std::io::stderr().write_all(&bytes)?;
        std::io::stderr().write_all(b"\n")?;
        Ok(())
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

impl From<StderrAppend> for AppendImpl {
    fn from(append: StderrAppend) -> Self {
        AppendImpl::Stderr(append)
    }
}
