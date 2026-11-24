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

//! Errors and internal error handling for Logforth.

use std::borrow::Cow;
use std::fmt;
use std::io;
use std::io::Write;
use std::sync::Arc;

type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// An error returned or reported by Logforth components.
pub struct Error {
    inner: Box<ErrorInner>,
}

struct ErrorInner {
    message: Cow<'static, str>,
    source: Option<BoxedError>,
}

impl Error {
    /// Create a new [`Error`] with the given message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                message: message.into(),
                source: None,
            }),
        }
    }

    /// Return the error message.
    pub fn message(&self) -> &str {
        &self.inner.message
    }

    /// Attach or replace the underlying causal error.
    pub fn with_source(mut self, src: impl Into<BoxedError>) -> Self {
        self.inner.source = Some(src.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.inner.message)?;

        if let Some(source) = &self.inner.source {
            write!(f, ": {source}")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return f
                .debug_struct("Error")
                .field("message", &self.inner.message)
                .field("source", &self.inner.source)
                .finish();
        }

        write!(f, "{}", self.inner.message)?;

        if let Some(source) = &self.inner.source {
            writeln!(f)?;
            writeln!(f)?;
            writeln!(f, "Caused by:")?;
            let mut curr: Option<&(dyn std::error::Error + 'static)> = Some(source.as_ref());
            let mut index = 0usize;
            while let Some(err) = curr {
                if let Some(lf) = err.downcast_ref::<Error>() {
                    writeln!(f, "   {index}: {}", lf.message())?;
                } else {
                    writeln!(f, "   {index}: {err}")?;
                }
                curr = err.source();
                index += 1;
            }
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner
            .source
            .as_deref()
            .map(|err| err as &(dyn std::error::Error + 'static))
    }
}

/// A sink that handles internal runtime errors produced by the logging pipeline.
///
/// Logging operations are designed not to crash the host application at runtime. When an appender
/// fails to emit or flush records, execution roots such as [`Logger`](crate::Logger) and
/// asynchronous workers report the resulting [`Error`] to their configured [`ErrorSink`].
#[derive(Clone)]
pub struct ErrorSink {
    inner: ErrorSinkInner,
}

#[derive(Clone)]
enum ErrorSinkInner {
    Stderr,
    BlackHole,
    Custom(Arc<dyn Fn(&Error) + Send + Sync>),
}

impl Default for ErrorSink {
    fn default() -> Self {
        Self::stderr()
    }
}

impl fmt::Debug for ErrorSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            ErrorSinkInner::Stderr => f.write_str("ErrorSink::Stderr"),
            ErrorSinkInner::BlackHole => f.write_str("ErrorSink::BlackHole"),
            ErrorSinkInner::Custom(_) => f.write_str("ErrorSink::Custom(..)"),
        }
    }
}

impl ErrorSink {
    /// Create a sink that writes internal errors to standard error on a best-effort basis.
    ///
    /// If writing to standard error fails (for example, when standard error is closed), the
    /// failure is silently ignored.
    pub const fn stderr() -> Self {
        Self {
            inner: ErrorSinkInner::Stderr,
        }
    }

    /// Create a sink that silently discards all internal errors.
    pub const fn black_hole() -> Self {
        Self {
            inner: ErrorSinkInner::BlackHole,
        }
    }

    /// Create a sink backed by a custom callback.
    pub fn new(f: impl Fn(&Error) + Send + Sync + 'static) -> Self {
        Self {
            inner: ErrorSinkInner::Custom(Arc::new(f)),
        }
    }

    /// Report an internal runtime error to this sink.
    pub fn report(&self, err: &Error) {
        match &self.inner {
            ErrorSinkInner::Stderr => {
                let _ = writeln!(io::stderr(), "logforth: {err}");
            }
            ErrorSinkInner::BlackHole => {}
            ErrorSinkInner::Custom(f) => f(err),
        }
    }
}

impl<F> From<F> for ErrorSink
where
    F: Fn(&Error) + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

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
        assert_send_sync_static::<ErrorSink>();
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
        assert_eq!(
            outer.to_string(),
            "failed to flush appender: failed to export batch: connection reset"
        );

        let debug = format!("{outer:?}");
        assert_eq!(
            debug,
            "failed to flush appender\n\nCaused by:\n   0: failed to export batch\n   1: connection reset\n   2: broken pipe\n"
        );

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

        assert_eq!(err.to_string(), "operation failed: second cause");
    }

    #[test]
    fn error_sink_variants_and_cloned_closure() {
        ErrorSink::black_hole().report(&Error::new("ignored"));

        let count = Arc::new(AtomicUsize::new(0));
        let count_ref = count.clone();
        let sink = ErrorSink::from(move |err: &Error| {
            assert_eq!(err.message(), "boom");
            count_ref.fetch_add(1, Ordering::Relaxed);
        });
        let cloned = sink.clone();

        sink.report(&Error::new("boom"));
        cloned.report(&Error::new("boom"));
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }
}
