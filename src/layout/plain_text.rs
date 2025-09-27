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

use std::fmt::Write;
use std::time::SystemTime;

use crate::Diagnostic;
use crate::Error;
use crate::Layout;
use crate::kv::Key;
use crate::kv::Value;
use crate::kv::Visitor;
use crate::layout::filename;
use crate::record::Record;

/// A layout that formats log record as plain text.
///
/// Output format:
///
/// ```text
/// 2024-08-11T22:44:57.172105+08:00 ERROR file: examples/file.rs:51 Hello error!
/// 2024-08-11T22:44:57.172219+08:00  WARN file: examples/file.rs:52 Hello warn!
/// 2024-08-11T22:44:57.172276+08:00  INFO file: examples/file.rs:53 Hello info!
/// 2024-08-11T22:44:57.172329+08:00 DEBUG file: examples/file.rs:54 Hello debug!
/// 2024-08-11T22:44:57.172382+08:00 TRACE file: examples/file.rs:55 Hello trace!
/// ```
///
/// # Examples
///
/// ```
/// use logforth::layout::PlainTextLayout;
///
/// let text_layout = PlainTextLayout::default();
/// ```

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PlainTextLayout {}

struct KvWriter {
    text: String,
}

impl Visitor for KvWriter {
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        // SAFETY: write to a string always succeeds
        write!(&mut self.text, " {key}={value}").unwrap();
        Ok(())
    }
}

impl Layout for PlainTextLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        let mut text = String::new();

        let time = record.time();
        match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => {
                let time = dur.as_nanos();
                write!(&mut text, "{time}").unwrap();
            }
            Err(err) => {
                let time = err.duration().as_nanos();
                write!(&mut text, "-{time}").unwrap();
            }
        }

        let level = record.level().as_str();
        let target = record.target();
        let file = filename(record);
        let line = record.line().unwrap_or_default();
        let message = record.args();
        write!(&mut text, " {level:>5} {target}: {file}:{line} {message}").unwrap();

        let mut visitor = KvWriter { text };
        record.key_values().visit(&mut visitor)?;
        for d in diags {
            d.visit(&mut visitor)?;
        }

        Ok(visitor.text.into_bytes())
    }
}
