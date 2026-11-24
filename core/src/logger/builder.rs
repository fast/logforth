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

use crate::Append;
use crate::Diagnostic;
use crate::ErrorSink;
use crate::Filter;
use crate::Logger;
use crate::error_sink;
use crate::logger::log_impl::Dispatch;

/// Create a new empty [`LoggerBuilder`] instance for configuring log dispatching.
///
/// # Examples
///
/// ```
/// use logforth_core::append;
///
/// let logger = logforth_core::builder()
///     .dispatch(|d| d.append(append::Stderr::default()))
///     .build();
/// ```
pub fn builder() -> LoggerBuilder {
    LoggerBuilder {
        dispatches: vec![],
        error_sink: Box::new(error_sink::Stderr::default()),
    }
}

/// A builder for configuring log dispatching.
///
/// # Examples
///
/// ```
/// use logforth_core::append;
///
/// let logger = logforth_core::builder()
///     .dispatch(|d| d.append(append::Stdout::default()))
///     .build();
/// ```
#[must_use = "call `build` to construct a logger instance"]
#[derive(Debug)]
pub struct LoggerBuilder {
    // stashed dispatches
    dispatches: Vec<Dispatch>,
    error_sink: Box<dyn ErrorSink>,
}

impl LoggerBuilder {
    /// Register a new dispatch with the [`LoggerBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    ///
    /// let logger = logforth_core::builder()
    ///     .dispatch(|d| d.append(append::Stderr::default()))
    ///     .build();
    /// ```
    pub fn dispatch<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DispatchBuilder<false>) -> DispatchBuilder<true>,
    {
        self.dispatches.push(f(DispatchBuilder::new()).build());
        self
    }

    /// Configure the default [`ErrorSink`] used when dispatches fail to append or flush records.
    ///
    /// Defaults to [`error_sink::Stderr`]. Individual dispatches may override this via
    /// [`DispatchBuilder::error_sink`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    /// use logforth_core::error_sink;
    ///
    /// let logger = logforth_core::builder()
    ///     .error_sink(error_sink::BlackHole::default())
    ///     .dispatch(|d| d.append(append::Stdout::default()))
    ///     .build();
    /// ```
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = error_sink.into();
        self
    }

    /// Build the [`Logger`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::record::Record;
    ///
    /// let l = logforth_core::builder().build();
    /// let r = Record::builder()
    ///     .payload(format_args!("hello world!"))
    ///     .build();
    /// l.log(&r);
    /// ```
    pub fn build(self) -> Logger {
        Logger::new(self.dispatches, self.error_sink)
    }
}

/// A builder for configuring a log dispatch, including filters and appenders.
///
/// # Examples
///
/// ```
/// use logforth_core::append;
/// use logforth_core::record::Level;
/// use logforth_core::record::LevelFilter;
///
/// let logger = logforth_core::builder()
///     .dispatch(|d| {
///         d.filter(LevelFilter::MoreSevereEqual(Level::Info))
///             .append(append::Stdout::default())
///     })
///     .build();
/// ```
#[derive(Debug)]
pub struct DispatchBuilder<const APPEND: bool> {
    filters: Vec<Box<dyn Filter>>,
    diagnostics: Vec<Box<dyn Diagnostic>>,
    appends: Vec<Box<dyn Append>>,
    error_sink: Option<Box<dyn ErrorSink>>,
}

impl DispatchBuilder<false> {
    fn new() -> Self {
        DispatchBuilder {
            filters: vec![],
            diagnostics: vec![],
            appends: vec![],
            error_sink: None,
        }
    }

    /// Add a filter to this dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    ///
    /// let logger = logforth_core::builder()
    ///     .dispatch(|d| {
    ///         d.filter(LevelFilter::MoreSevereEqual(Level::Error))
    ///             .append(append::Stderr::default())
    ///     })
    ///     .build();
    /// ```
    pub fn filter(mut self, filter: impl Into<Box<dyn Filter>>) -> Self {
        self.filters.push(filter.into());
        self
    }

    /// Add a diagnostic to this dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    /// use logforth_core::diagnostic;
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    ///
    /// let logger = logforth_core::builder()
    ///     .dispatch(|d| {
    ///         d.filter(LevelFilter::MoreSevereEqual(Level::Error))
    ///             .diagnostic(diagnostic::ThreadLocalDiagnostic::default())
    ///             .append(append::Stderr::default())
    ///     })
    ///     .build();
    /// ```
    pub fn diagnostic(mut self, diagnostic: impl Into<Box<dyn Diagnostic>>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }

    /// Configure an [`ErrorSink`] specific to this dispatch.
    ///
    /// When unset, errors from this dispatch fall back to the [`Logger`]'s [`ErrorSink`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    /// use logforth_core::error_sink;
    ///
    /// let logger = logforth_core::builder()
    ///     .dispatch(|d| {
    ///         d.error_sink(error_sink::BlackHole::default())
    ///             .append(append::Stdout::default())
    ///     })
    ///     .build();
    /// ```
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = Some(error_sink.into());
        self
    }
}

impl DispatchBuilder<true> {
    fn build(self) -> Dispatch {
        Dispatch::new(
            self.filters,
            self.diagnostics,
            self.appends,
            self.error_sink,
        )
    }
}

impl<const APPEND: bool> DispatchBuilder<APPEND> {
    /// Add an appender to this dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    ///
    /// let logger = logforth_core::builder()
    ///     .dispatch(|d| d.append(append::Stdout::default()))
    ///     .build();
    /// ```
    pub fn append(mut self, append: impl Into<Box<dyn Append>>) -> DispatchBuilder<true> {
        self.appends.push(append.into());
        DispatchBuilder {
            filters: self.filters,
            diagnostics: self.diagnostics,
            appends: self.appends,
            error_sink: self.error_sink,
        }
    }
}
