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

use std::error::Error as _;
use std::fmt;
use std::io;

use logforth_core::Error;

#[derive(Debug)]
struct TransportError {
    detail: &'static str,
    cause: io::Error,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.detail)
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

#[test]
fn error_formats_message_causal_chain_and_preserves_downcastable_sources() {
    let leaf = io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe");
    let transport = TransportError {
        detail: "connection reset",
        cause: leaf,
    };
    let middle = Error::new(String::from("failed to export batch")).with_source(transport);
    let outer = Error::new("failed to flush appender").with_source(middle);

    assert_eq!(outer.message(), "failed to flush appender");
    assert_eq!(outer.to_string(), "failed to flush appender");
    assert_eq!(
        format!("{outer:#}"),
        "failed to flush appender: failed to export batch: connection reset: broken pipe"
    );

    let hop1 = outer.source().unwrap();
    let hop2 = hop1.source().unwrap();
    let hop3 = hop2.source().unwrap().downcast_ref::<io::Error>().unwrap();
    assert_eq!(hop3.kind(), io::ErrorKind::BrokenPipe);

    // Standalone error without a causal source omits `: ...` in `{:#}`.
    let standalone = Error::new("malformed directive");
    assert!(standalone.source().is_none());
    assert_eq!(format!("{standalone:#}"), "malformed directive");
}

#[test]
fn with_source_replaces_existing_source_without_panicking() {
    let err = Error::new("operation failed")
        .with_source("first cause")
        .with_source("second cause");

    assert_eq!(err.message(), "operation failed");
    assert_eq!(format!("{err:#}"), "operation failed: second cause");
}
