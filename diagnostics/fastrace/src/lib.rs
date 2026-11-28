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

//! Diagnostic context integration with [`fastrace`].
//!
//! [`FastraceDiagnostic`] queries [`SpanContext::current_local_parent`] on the current thread and,
//! when a local parent span is active, visits three well-known structured fields without heap
//! allocation:
//! - `"trace_id"`: 32-character lowercase hexadecimal `TraceId`;
//! - `"span_id"`: 16-character lowercase hexadecimal `SpanId`;
//! - `"sampled"`: boolean sampling flag (`SpanContext::sampled`).
//!
//! Following [`fastrace`]'s library convention, this crate does not enable the `fastrace/enable`
//! Cargo feature on its own; enable `fastrace/enable` in the final application binary for span
//! contexts to be active at runtime.
//!
//! # Examples
//!
//! ```
//! use fastrace::Span;
//! use fastrace::collector::SpanContext;
//! use logforth_core::LoggerBuilder;
//! use logforth_core::append::Testing;
//! use logforth_diagnostic_fastrace::FastraceDiagnostic;
//!
//! let logger = LoggerBuilder::new()
//!     .dispatch(|d| {
//!         d.diagnostic(FastraceDiagnostic::new())
//!             .append(Testing::new())
//!     })
//!     .build();
//!
//! let root = Span::root("root", SpanContext::random());
//! let _guard = root.set_local_parent();
//! logforth_core::info!(logger, "inside span");
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use fastrace::collector::SpanContext;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;

// Keep synchronized with "Appendix: Key-Value Naming Conventions" in DESIGN.md.
const KEY_TRACE_ID: &str = "trace_id";
const KEY_SPAN_ID: &str = "span_id";
const KEY_SAMPLED: &str = "sampled";
const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

fn encode_hex<const BYTES: usize, const HEX_LEN: usize>(bytes: [u8; BYTES]) -> [u8; HEX_LEN] {
    let mut out = [0u8; HEX_LEN];
    for (i, byte) in bytes.into_iter().enumerate() {
        out[i * 2] = HEX_LOWER[usize::from(byte >> 4)];
        out[i * 2 + 1] = HEX_LOWER[usize::from(byte & 0x0f)];
    }
    out
}

/// A [`Diagnostic`] that enriches log records with `"trace_id"`, `"span_id"`, and `"sampled"` from
/// the active local [`fastrace`] span.
///
/// When no local parent span is active on the current thread, [`Diagnostic::visit`] is a no-op.
///
/// # Examples
///
/// ```
/// use logforth_diagnostic_fastrace::FastraceDiagnostic;
///
/// const DIAG: FastraceDiagnostic = FastraceDiagnostic::new();
/// # let _ = DIAG;
/// ```
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct FastraceDiagnostic {}

impl FastraceDiagnostic {
    /// Creates a new [`FastraceDiagnostic`].
    pub const fn new() -> Self {
        Self {}
    }
}

impl Diagnostic for FastraceDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        if let Some(span) = SpanContext::current_local_parent() {
            let trace_id = encode_hex::<16, 32>(span.trace_id.0.to_be_bytes());
            let span_id = encode_hex::<8, 16>(span.span_id.0.to_be_bytes());
            let trace_id = std::str::from_utf8(&trace_id).expect("hex digits are valid ASCII");
            let span_id = std::str::from_utf8(&span_id).expect("hex digits are valid ASCII");

            visitor.visit(Key::new(KEY_TRACE_ID).view(), Value::str(trace_id).view())?;
            visitor.visit(Key::new(KEY_SPAN_ID).view(), Value::str(span_id).view())?;
            visitor.visit(
                Key::new(KEY_SAMPLED).view(),
                Value::bool(span.sampled).view(),
            )?;
        }

        Ok(())
    }
}
