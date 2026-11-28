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
//!
//! [`Error`] boxes its internal state so `Result<(), Error>` occupies a single pointer word on hot
//! formatting, visiting, and appending paths. Operational failures inside a [`Logger`](crate::Logger)
//! are wrapped with stage context and delivered to the configured [`ErrorSink`].

use std::borrow::Cow;
use std::error::Error as _;
use std::fmt;

mod black_hole;
pub use self::black_hole::BlackHoleErrorSink;

mod sink;
pub use self::sink::ErrorSink;

mod stderr;
pub use self::stderr::StderrErrorSink;

/// A single-word error returned or reported by Logforth components.
///
/// Formatting with `{}` writes [`Error::message`] alone; alternate formatting `{:#}` appends the
/// causal chain separated by `: `.
pub struct Error {
    state: Box<ErrorState>,
}

struct ErrorState {
    message: Cow<'static, str>,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl Error {
    /// Creates a new [`Error`] with `message` and no underlying causal source.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            state: Box::new(ErrorState {
                message: message.into(),
                source: None,
            }),
        }
    }

    /// Returns this error's own message without causal sources.
    pub fn message(&self) -> &str {
        &self.state.message
    }

    /// Attaches `src` as the underlying causal error, replacing any previously attached source.
    pub fn with_source(
        mut self,
        src: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
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

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.state
            .source
            .as_deref()
            .map(|err| err as &(dyn std::error::Error + 'static))
    }
}
