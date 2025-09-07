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

use std::borrow::Cow;

use jiff::Zoned;
use log::Record;

use crate::Diagnostic;
use crate::Error;
use crate::append::Append;
use crate::diagnostic::Visitor;

/// An appender that adds log records to fastrace as an event associated to the current span.
///
/// # Examples
///
/// ```
/// use logforth::append::FastraceEvent;
///
/// let fastrace_appender = FastraceEvent::default();
/// ```
#[derive(Default, Debug, Clone)]
pub struct FastraceEvent {
    _private: (), // suppress structure literal syntax
}

impl Append for FastraceEvent {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let message = format!("{}", record.args());

        let mut collector = KvCollector { kv: Vec::new() };
        record
            .key_values()
            .visit(&mut collector)
            .map_err(Error::from_kv_error)?;
        for d in diags {
            d.visit(&mut collector)?;
        }

        fastrace::local::LocalSpan::add_event(fastrace::Event::new(message).with_properties(
            || {
                [
                    (Cow::from("level"), Cow::from(record.level().as_str())),
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

impl<'kvs> log::kv::VisitSource<'kvs> for KvCollector {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        self.kv.push((key.to_string(), value.to_string()));
        Ok(())
    }
}

impl Visitor for KvCollector {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) -> Result<(), Error> {
        self.kv.push((key.into_owned(), value.into_owned()));
        Ok(())
    }
}
