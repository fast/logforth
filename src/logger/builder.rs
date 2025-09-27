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
use crate::Filter;
use crate::Logger;
use crate::append;
use crate::filter::env_filter::EnvFilterBuilder;
use crate::logger::log_impl::Dispatch;

/// Create a new empty [`LoggerBuilder`] instance for configuring log dispatching.
///
/// # Examples
///
/// ```
/// use logforth::append;
///
/// let builder = logforth::builder()
///     .dispatch(|d| d.append(append::Stderr::default()))
///     .apply();
/// ```
pub fn builder() -> LoggerBuilder {
    LoggerBuilder::new()
}

/// Create a [`LoggerBuilder`] with a default [`append::Stdout`] appender and an [`EnvFilter`]
/// respecting `RUST_LOG`.
///
/// [`EnvFilter`]: crate::filter::EnvFilter
///
/// # Examples
///
/// ```
/// logforth::stdout().apply();
/// log::error!("This error will be logged to stdout.");
/// ```
pub fn stdout() -> LoggerBuilder {
    builder().dispatch(|d| {
        d.filter(EnvFilterBuilder::from_default_env().build())
            .append(append::Stdout::default())
    })
}

/// Create a [`LoggerBuilder`] with a default [`append::Stderr`] appender and an [`EnvFilter`]
/// respecting `RUST_LOG`.
///
/// [`EnvFilter`]: crate::filter::EnvFilter
///
/// # Examples
///
/// ```
/// logforth::stderr().apply();
/// log::info!("This info will be logged to stderr.");
/// ```
pub fn stderr() -> LoggerBuilder {
    builder().dispatch(|d| {
        d.filter(EnvFilterBuilder::from_default_env().build())
            .append(append::Stderr::default())
    })
}

/// A builder for configuring log dispatching and setting up the global logger.
///
/// # Examples
///
/// ```
/// use logforth::append;
///
/// logforth::builder()
///     .dispatch(|d| d.append(append::Stdout::default()))
///     .apply();
/// ```
#[must_use = "call `apply` to set the global logger or `build` to construct a logger instance"]
#[derive(Debug)]
pub struct LoggerBuilder {
    // stashed dispatches
    dispatches: Vec<Dispatch>,
}

impl LoggerBuilder {
    fn new() -> Self {
        LoggerBuilder { dispatches: vec![] }
    }

    /// Register a new dispatch with the [`LoggerBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append;
    ///
    /// logforth::builder()
    ///     .dispatch(|d| d.append(append::Stderr::default()))
    ///     .apply();
    /// ```
    pub fn dispatch<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DispatchBuilder<false>) -> DispatchBuilder<true>,
    {
        self.dispatches.push(f(DispatchBuilder::new()).build());
        self
    }

    /// Build the [`Logger`].
    ///
    /// # Examples
    ///
    /// ```
    /// let l = logforth::builder().build();
    /// log::error!(logger: l, "Hello error!");
    /// ```
    pub fn build(self) -> Logger {
        Logger::new(self.dispatches)
    }

    /// Set up `log`'s global logger with all the configured dispatches.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// This function will set the global maximum log level to `Trace`. To override this, call
    /// [`log::set_max_level`] after this function.
    ///
    /// Alternatively, you can obtain a [`Logger`] instance by calling [`LoggerBuilder::build`], and
    /// then call [`log::set_boxed_logger`] manually.
    ///
    /// # Errors
    ///
    /// Return an error if a global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = logforth::builder().try_apply();
    /// if let Err(e) = result {
    ///     eprintln!("Failed to set logger: {}", e);
    /// }
    /// ```
    pub fn try_apply(self) -> Result<(), log::SetLoggerError> {
        let logger = self.build();
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(log::LevelFilter::Trace);
        Ok(())
    }

    /// Set up `log`'s global logger with all the configured dispatches.
    ///
    /// This function will panic if it is called more than once, or if another library has already
    /// initialized a global logger.
    ///
    /// This function will set the global maximum log level to `Trace`. To override this, call
    /// [`log::set_max_level`] after this function.
    ///
    /// Alternatively, you can obtain a [`Logger`] instance by calling [`LoggerBuilder::build`], and
    /// then call [`log::set_boxed_logger`] manually.
    ///
    /// # Panics
    ///
    /// Panic if the global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth::builder().apply();
    /// ```
    pub fn apply(self) {
        self.try_apply()
            .expect("LoggerBuilder::apply must be called before the global logger initialized");
    }
}

/// A builder for configuring a log dispatch, including filters and appenders.
///
/// # Examples
///
/// ```
/// use logforth::LevelFilter;
/// use logforth::append;
///
/// logforth::builder()
///     .dispatch(|d| {
///         d.filter(LevelFilter::Info)
///             .append(append::Stdout::default())
///     })
///     .apply();
/// ```
#[derive(Debug)]
pub struct DispatchBuilder<const APPEND: bool> {
    filters: Vec<Box<dyn Filter>>,
    diagnostics: Vec<Box<dyn Diagnostic>>,
    appends: Vec<Box<dyn Append>>,
}

impl DispatchBuilder<false> {
    fn new() -> Self {
        DispatchBuilder {
            filters: vec![],
            diagnostics: vec![],
            appends: vec![],
        }
    }

    /// Add a filter to this dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::LevelFilter;
    /// use logforth::append;
    ///
    /// logforth::builder()
    ///     .dispatch(|d| {
    ///         d.filter(LevelFilter::Error)
    ///             .append(append::Stderr::default())
    ///     })
    ///     .apply();
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
    /// use logforth::LevelFilter;
    /// use logforth::append;
    /// use logforth::diagnostic;
    ///
    /// logforth::builder()
    ///     .dispatch(|d| {
    ///         d.filter(LevelFilter::Error)
    ///             .diagnostic(diagnostic::ThreadLocalDiagnostic::default())
    ///             .append(append::Stderr::default())
    ///     })
    ///     .apply();
    /// ```
    pub fn diagnostic(mut self, diagnostic: impl Into<Box<dyn Diagnostic>>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}

impl DispatchBuilder<true> {
    fn build(self) -> Dispatch {
        Dispatch::new(self.filters, self.diagnostics, self.appends)
    }
}

impl<const APPEND: bool> DispatchBuilder<APPEND> {
    /// Add an appender to this dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append;
    ///
    /// logforth::builder()
    ///     .dispatch(|d| d.append(append::Stdout::default()))
    ///     .apply();
    /// ```
    pub fn append(mut self, append: impl Into<Box<dyn Append>>) -> DispatchBuilder<true> {
        self.appends.push(append.into());
        DispatchBuilder {
            filters: self.filters,
            diagnostics: self.diagnostics,
            appends: self.appends,
        }
    }
}
