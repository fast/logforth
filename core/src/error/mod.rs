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

//! Error types and sinks for internal runtime errors produced by the logging pipeline.

use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;

mod black_hole;
mod sink;
mod stderr;

pub use self::black_hole::BlackHoleErrorSink;
pub use self::sink::ErrorSink;
pub use self::stderr::StderrErrorSink;

/// An error returned or reported by Logforth components.
pub struct Error {
    state: Box<ErrorState>,
}

struct ErrorState {
    message: Cow<'static, str>,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Error {
    /// Create a new [`Error`] with the given message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            state: Box::new(ErrorState {
                message: message.into(),
                source: None,
            }),
        }
    }

    /// Return the error message.
    pub fn message(&self) -> &str {
        &self.state.message
    }

    /// Attach or replace the underlying causal error.
    pub fn with_source(
        mut self,
        src: impl Into<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        self.state.source = Some(src.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.state.message)?;

        if f.alternate() {
            let mut curr = self.source();
            while let Some(cause) = curr {
                write!(f, ": {cause}")?;
                curr = cause.source();
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return f
                .debug_struct("Error")
                .field("message", &self.state.message)
                .field("source", &self.state.source)
                .finish();
        }

        f.write_str(&self.state.message)?;

        if let Some(first) = self.source() {
            write!(f, "\n\nCaused by:")?;
            let mut curr = Some(first);
            let mut index = 0usize;
            while let Some(cause) = curr {
                write!(f, "\n    {index}: {cause}")?;
                curr = cause.source();
                index += 1;
            }
        }

        Ok(())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.state
            .source
            .as_deref()
            .map(|err| err as &(dyn StdError + 'static))
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    fn assert_send_sync_static<T: Send + Sync + 'static>() {}

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

    impl StdError for TransportError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            Some(&self.cause)
        }
    }

    #[test]
    fn error_layout_and_auto_traits() {
        assert_send_sync_static::<Error>();
        assert_eq!(size_of::<Error>(), size_of::<usize>());
        assert_eq!(size_of::<Result<(), Error>>(), size_of::<usize>());
    }

    #[test]
    fn error_display_debug_and_downcast_chain() {
        let leaf = io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe");
        let transport = TransportError {
            detail: "connection reset",
            cause: leaf,
        };
        let middle = Error::new("failed to export batch").with_source(transport);
        let outer = Error::new("failed to flush appender").with_source(middle);

        assert_eq!(outer.message(), "failed to flush appender");
        assert_eq!(outer.to_string(), "failed to flush appender");
        assert_eq!(
            format!("{outer:#}"),
            "failed to flush appender: failed to export batch: connection reset: broken pipe"
        );

        assert_eq!(
            format!("{outer:?}"),
            "failed to flush appender\n\nCaused by:\n    0: failed to export batch\n    1: connection reset\n    2: broken pipe"
        );
        assert!(format!("{outer:#?}").starts_with("Error {"));

        let hop1 = outer.source().unwrap();
        let hop2 = hop1.source().unwrap();
        let hop3 = hop2.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert_eq!(hop3.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn with_source_replaces_existing_source_without_panicking() {
        let err = Error::new("operation failed")
            .with_source("first cause")
            .with_source("second cause");

        assert_eq!(err.to_string(), "operation failed");
        assert_eq!(format!("{err:#}"), "operation failed: second cause");
    }
}
