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
use crate::record::Record;

/// A layout that formats log record as plain text.
///
/// Output format:
///
/// ```text
/// 1760092868730397000 ERROR simple: simple.rs:24 Hello error!
/// 1760092868730572000  WARN simple: simple.rs:25 Hello warn!
/// 1760092868730576000  INFO simple: simple.rs:26 Hello info!
/// 1760092868730579000 DEBUG simple: simple.rs:27 Hello debug!
/// 1760092868730581000 TRACE simple: simple.rs:28 Hello trace!
/// ```
///
/// # Examples
///
/// ```
/// use logforth_core::layout::PlainTextLayout;
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

        let level = record.level().name();
        let target = record.target();
        let file = record.filename();
        let line = record.line().unwrap_or_default();
        let message = record.payload();
        write!(&mut text, " {level:>6} {target}: {file}:{line} {message}").unwrap();

        let mut visitor = KvWriter { text };
        record.key_values().visit(&mut visitor)?;
        for d in diags {
            d.visit(&mut visitor)?;
        }

        Ok(visitor.text.into_bytes())
    }
}
