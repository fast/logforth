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

use crate::Marker;

#[derive(Debug, Clone, Default)]
pub struct TraceIdMarker {
    _private: (),
}

impl TraceIdMarker {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub(crate) fn mark(&self, mut f: impl FnMut(&str, String)) {
        if let Some(span) = fastrace::collector::SpanContext::current_local_parent() {
            f("trace_id", format!("{:016x}", span.trace_id.0));
        }
    }
}

impl From<TraceIdMarker> for Marker {
    fn from(marker: TraceIdMarker) -> Self {
        Marker::TraceId(marker)
    }
}
