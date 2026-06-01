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

//! A diagnostic that enriches log records with trace context provided by the Fastrace library.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use fastrace::collector::SpanContext;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;

/// A diagnostic that enriches log records with trace context provided by the Fastrace library.
///
/// ## Example
///
/// ```
/// use logforth_diagnostic_fastrace::FastraceDiagnostic;
///
/// let diagnostic = FastraceDiagnostic::default();
/// ```
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FastraceDiagnostic {}

impl Diagnostic for FastraceDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        if let Some(span) = SpanContext::current_local_parent() {
            // NOTE: TraceId and SpanId should be represented as hex strings.
            let trace_id = span.trace_id.to_string();
            let span_id = span.span_id.to_string();

            visitor.visit(
                Key::new("trace_id").view(),
                Value::str(&trace_id).view(),
            )?;
            visitor.visit(
                Key::new("span_id").view(),
                Value::str(&span_id).view(),
            )?;
            visitor.visit(
                Key::new("sampled").view(),
                Value::bool(span.sampled).view(),
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fastrace::Span;
    use logforth_core::kv::{KeyView, ValueOwned, ValueView};

    use super::*;

    #[test]
    fn key_values() {
        struct Collector(BTreeMap<String, ValueOwned>);

        impl Visitor for Collector {
            fn visit(&mut self, key: KeyView<'_>, value: ValueView<'_>) -> Result<(), Error> {
                self.0.insert(key.to_string(), value.to_owned());
                Ok(())
            }
        }

        let diagnostic = FastraceDiagnostic::default();

        let mut map = {
            let span = Span::root("test", SpanContext::random());
            let _guard = span.set_local_parent();

            let mut collector = Collector(BTreeMap::new());
            diagnostic.visit(&mut collector).unwrap();
            collector.0
        };

        let trace_id = map.remove("trace_id").unwrap();
        assert_eq!(32, trace_id.to_string().len());
        let span_id = map.remove("span_id").unwrap();
        assert_eq!(16, span_id.to_string().len());
        let sampled = map.remove("sampled").unwrap();
        assert!(sampled.view().to_bool().unwrap());

        assert!(map.is_empty());
    }
}
