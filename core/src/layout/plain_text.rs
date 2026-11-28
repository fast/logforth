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
use std::time::SystemTime;

use crate::Diagnostic;
use crate::Error;
use crate::Layout;
use crate::kv::KeyView;
use crate::kv::ValueView;
use crate::record::Record;

/// A zero-dependency layout that formats log records as single-line plain text.
///
/// Timestamps are rendered as signed nanoseconds relative to [`SystemTime::UNIX_EPOCH`], followed
/// by the right-aligned 6-character severity name, module path, source filename and line number,
/// message payload, [`Record::key_values`], and [`Diagnostic`] entries:
///
/// ```text
/// 1760092868730397000  ERROR simple: simple.rs:24 Hello error!
/// 1760092868730572000   WARN simple: simple.rs:25 Hello warn!
/// 1760092868730576000   INFO simple: simple.rs:26 Hello info!
/// 1760092868730579000  DEBUG simple: simple.rs:27 Hello debug!
/// 1760092868730581000  TRACE simple: simple.rs:28 Hello trace!
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

impl Layout for PlainTextLayout {
    fn format(
        &self,
        record: &Record,
        diags: &dyn Diagnostic,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        match record.time().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => write!(buf, "{}", dur.as_nanos()),
            Err(err) => write!(buf, "-{}", err.duration().as_nanos()),
        }
        .map_err(|err| Error::new("failed to format record timestamp").with_source(err))?;

        let level = record.level().name();
        let module_path = record.module_path().unwrap_or_default();
        let file = record.filename().unwrap_or_default();
        let line = record.line().unwrap_or_default();
        let message = record.payload();
        write!(buf, " {level:>6} {module_path}: {file}:{line} {message}")
            .map_err(|err| Error::new("failed to format log record").with_source(err))?;

        let mut visit = |key: KeyView, value: ValueView| {
            write!(buf, " {key}={value}")
                .map_err(|err| Error::new("failed to format key-value pair").with_source(err))
        };
        record.key_values().visit(&mut visit)?;
        diags.visit(&mut visit)
    }
}
