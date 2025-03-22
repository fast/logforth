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

use crate::append::Append;
use crate::diagnostic::Visitor;
use crate::Diagnostic;

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
    fn append(&self, record: &Record, diagnostics: &[Diagnostic]) -> anyhow::Result<()> {
        let message = format!("{}", record.args());

        let mut collector = KvCollector { kv: Vec::new() };
        record.key_values().visit(&mut collector)?;
        for d in diagnostics {
            d.visit(&mut collector);
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

    fn flush(&self) {
        fastrace::flush();
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
    fn visit<'k, 'v, K, V>(&mut self, key: K, value: V)
    where
        K: Into<Cow<'k, str>>,
        V: Into<Cow<'v, str>>,
    {
        let key = key.into().into_owned();
        let value = value.into().into_owned();
        self.kv.push((key, value));
    }
}
