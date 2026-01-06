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

//! Appender for integrating with [fastrace](https://crates.io/crates/fastrace).

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::borrow::Cow;

use jiff::Zoned;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::append::Append;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::record::Record;

/// An appender that adds log records to fastrace as an event associated to the current span.
///
/// # Examples
///
/// ```
/// use logforth_append_fastrace::FastraceEvent;
///
/// let fastrace_appender = FastraceEvent::default();
/// ```
///
/// # Caveats
///
/// The caller or application should ensure that the `flush` method or [`fastrace::flush`] is called
/// before the program exits to collect the final events, especially when this appender is used
/// in a global context.
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct FastraceEvent {}

impl Append for FastraceEvent {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let message = if let Some(msg) = record.payload_static() {
            Cow::Borrowed(msg)
        } else {
            Cow::Owned(record.payload().to_string())
        };

        let mut collector = KvCollector { kv: Vec::new() };
        record.key_values().visit(&mut collector)?;
        for d in diags {
            d.visit(&mut collector)?;
        }

        fastrace::local::LocalSpan::add_event(fastrace::Event::new(message).with_properties(
            || {
                [
                    (Cow::from("level"), Cow::from(record.level().name())),
                    (Cow::from("timestamp"), Cow::from(Zoned::now().to_string())),
                ]
                .into_iter()
                .chain(
                    collector
                        .kv
                        .into_iter()
                        .map(|(k, v)| (Cow::from(k), Cow::from(v))),
                )
            },
        ));

        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        fastrace::flush();
        Ok(())
    }
}

struct KvCollector {
    kv: Vec<(String, String)>,
}

impl Visitor for KvCollector {
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        self.kv.push((key.to_string(), value.to_string()));
        Ok(())
    }
}
