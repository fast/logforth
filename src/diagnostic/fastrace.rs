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

use crate::Diagnostic;
use log::kv::{Error, Key, Value, VisitSource};

#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FastraceDiagnostic {}

impl FastraceDiagnostic {
    pub fn name(&self) -> &'static str {
        "fastrace"
    }
}

impl log::kv::Source for FastraceDiagnostic {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn VisitSource<'kvs>) -> Result<(), Error> {
        if let Some(span) = fastrace::collector::SpanContext::current_local_parent() {
            let trace_id = format!("{:016x}", span.trace_id.0);
            visitor.visit_pair(Key::from_str("trace_id"), Value::from_display(&trace_id))?;
        }
        Ok(())
    }
}

impl From<FastraceDiagnostic> for Diagnostic {
    fn from(diagnostic: FastraceDiagnostic) -> Self {
        Diagnostic::Fastrace(diagnostic)
    }
}
