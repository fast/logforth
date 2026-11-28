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
use crate::error::StderrErrorSink;
use crate::logger::log_impl::Dispatch;

/// Creates an empty [`LoggerBuilder`] for configuring log dispatch branches.
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
    LoggerBuilder::new()
}

/// A builder that assembles a [`Logger`] from ordered dispatch branches and a fallback
/// [`ErrorSink`].
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
/// use logforth_core::append::Stdout;
///
/// let logger = Logger::builder()
///     .dispatch(|d| d.append(Stdout::new()))
///     .build();
/// ```
#[must_use = "call `build` to construct a logger instance"]
#[derive(Debug)]
pub struct LoggerBuilder {
    // stashed dispatches
    dispatches: Vec<Dispatch>,
    error_sink: Box<dyn ErrorSink>,
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggerBuilder {
    /// Creates an empty [`LoggerBuilder`] with [`StderrErrorSink`] as the fallback error sink.
    pub fn new() -> Self {
        Self {
            dispatches: vec![],
            error_sink: Box::new(StderrErrorSink::new()),
        }
    }

    /// Registers a dispatch branch configured by `f`.
    ///
    /// If `f` returns a [`DispatchBuilder`] with no appenders configured, the branch is skipped.
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
        F: FnOnce(DispatchBuilder) -> DispatchBuilder,
    {
        if let Some(dispatch) = f(DispatchBuilder::new()).build() {
            self.dispatches.push(dispatch);
        }
        self
    }

    /// Sets the fallback [`ErrorSink`] used when a dispatch fails to append or flush records.
    ///
    /// Defaults to [`StderrErrorSink`]. Individual dispatches may override this via
    /// [`DispatchBuilder::error_sink`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    /// use logforth_core::error::BlackHoleErrorSink;
    ///
    /// let logger = logforth_core::builder()
    ///     .error_sink(BlackHoleErrorSink::default())
    ///     .dispatch(|d| d.append(append::Stdout::default()))
    ///     .build();
    /// ```
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = error_sink.into();
        self
    }

    /// Builds the [`Logger`].
    ///
    /// If no dispatch branch with at least one appender was registered, returns an allocation-free
    /// no-op logger equivalent to [`Logger::noop`].
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

/// A builder for a single dispatch branch within a [`Logger`].
///
/// Configured filters, diagnostics, and any [`error_sink`](Self::error_sink) override apply to the
/// entire branch across all of its appenders regardless of method call order, while multiple
/// filters, diagnostics, or appenders preserve their respective registration order. A branch
/// configured with no appenders is skipped when registered on [`LoggerBuilder`].
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
pub struct DispatchBuilder {
    filters: Vec<Box<dyn Filter>>,
    diagnostics: Vec<Box<dyn Diagnostic>>,
    appends: Vec<Box<dyn Append>>,
    error_sink: Option<Box<dyn ErrorSink>>,
}

impl DispatchBuilder {
    fn new() -> Self {
        DispatchBuilder {
            filters: vec![],
            diagnostics: vec![],
            appends: vec![],
            error_sink: None,
        }
    }

    /// Appends a [`Filter`] to this dispatch's filter chain.
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

    /// Appends a [`Diagnostic`] context source to this dispatch.
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

    /// Overrides the [`ErrorSink`] used when appenders in this dispatch fail.
    ///
    /// Calling this method multiple times replaces any previously configured branch override. When
    /// unset, errors from this dispatch fall back to [`LoggerBuilder::error_sink`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    /// use logforth_core::error::BlackHoleErrorSink;
    ///
    /// let logger = logforth_core::builder()
    ///     .dispatch(|d| {
    ///         d.error_sink(BlackHoleErrorSink::default())
    ///             .append(append::Stdout::default())
    ///     })
    ///     .build();
    /// ```
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = Some(error_sink.into());
        self
    }

    /// Appends an [`Append`] sink to receive records accepted by this dispatch.
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
    pub fn append(mut self, append: impl Into<Box<dyn Append>>) -> Self {
        self.appends.push(append.into());
        self
    }

    fn build(self) -> Option<Dispatch> {
        if self.appends.is_empty() {
            None
        } else {
            Some(Dispatch::new(
                self.filters,
                self.diagnostics,
                self.appends,
                self.error_sink,
            ))
        }
    }
}
