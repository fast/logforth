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

use fastrace::collector::SpanContext;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;

/// A diagnostic that enriches log records with trace context provided by the Fastrace library.
///
/// Output format:
///
/// ```text
/// 2025-01-10T15:22:37.868815+08:00 ERROR fastrace: fastrace.rs:39 Hello syslog error! trace_id=37f9c45f918cbb477089afb0d7162e7e
/// 2025-01-10T15:22:37.868890+08:00  WARN fastrace: fastrace.rs:40 Hello syslog warn! trace_id=37f9c45f918cbb477089afb0d7162e7e
/// 2025-01-10T15:22:37.868921+08:00  INFO fastrace: fastrace.rs:41 Hello syslog info! trace_id=37f9c45f918cbb477089afb0d7162e7e
/// 2025-01-10T15:22:37.868949+08:00 DEBUG fastrace: fastrace.rs:42 Hello syslog debug! trace_id=37f9c45f918cbb477089afb0d7162e7e
/// 2025-01-10T15:22:37.868976+08:00 TRACE fastrace: fastrace.rs:43 Hello syslog trace! trace_id=37f9c45f918cbb477089afb0d7162e7e
/// ```
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

            visitor.visit(Key::new("trace_id"), Value::from_str(&trace_id))?;
            visitor.visit(Key::new("span_id"), Value::from_str(&span_id))?;
            visitor.visit(Key::new("sampled"), Value::from_bool(span.sampled))?;
        }

        Ok(())
    }
}
