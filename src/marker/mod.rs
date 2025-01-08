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

//! Markers to enrich log records with additional information.

pub use custom::CustomMarker;
#[cfg(feature = "fastrace")]
pub use fastrace::TraceIdMarker;

mod custom;
#[cfg(feature = "fastrace")]
mod fastrace;

/// Represents a layout for formatting log records.
#[derive(Debug)]
pub enum Marker {
    Custom(CustomMarker),
    #[cfg(feature = "fastrace")]
    TraceId(TraceIdMarker),
}

impl Marker {
    pub(crate) fn mark(&self, f: impl FnMut(&str, String)) {
        match self {
            Marker::Custom(marker) => marker.mark(f),
            #[cfg(feature = "fastrace")]
            Marker::TraceId(marker) => marker.mark(f),
        }
    }
}
