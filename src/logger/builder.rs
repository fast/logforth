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

use log::LevelFilter;

use super::log_impl::Dispatch;
use super::log_impl::Logger;
use crate::append;
use crate::filter::EnvFilter;
use crate::Append;
use crate::Diagnostic;
use crate::Filter;

/// Creates a new empty [`Builder`] instance for configuring log dispatching.
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
pub fn builder() -> Builder {
    Builder::new()
}

/// Creates a [`Builder`] with a default [`append::Stdout`] appender and an [`env_filter`](https://crates.io/crates/env_filter)
/// respecting `RUST_LOG`.
///
/// # Examples
///
/// ```
/// logforth::stdout().apply();
/// log::error!("This error will be logged to stdout.");
/// ```
pub fn stdout() -> Builder {
    builder().dispatch(|d| {
        d.filter(EnvFilter::from_default_env())
            .append(append::Stdout::default())
    })
}

/// Creates a [`Builder`] with a default [`append::Stderr`] appender and an [`env_filter`](https://crates.io/crates/env_filter)
/// respecting `RUST_LOG`.
///
/// # Examples
///
/// ```
/// logforth::stderr().apply();
/// log::info!("This info will be logged to stderr.");
/// ```
pub fn stderr() -> Builder {
    builder().dispatch(|d| {
        d.filter(EnvFilter::from_default_env())
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
#[must_use = "call `apply` to set the global logger"]
#[derive(Debug)]
pub struct Builder {
    // stashed dispatches
    dispatches: Vec<Dispatch>,

    // default to trace - we need this because the global default is OFF
    max_level: LevelFilter,
}

impl Builder {
    fn new() -> Self {
        Builder {
            dispatches: vec![],
            max_level: LevelFilter::Trace,
        }
    }

    /// Registers a new dispatch with the [`Builder`].
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

    /// Sets the global maximum log level. Default to [`LevelFilter::Trace`].
    ///
    /// This will be passed to `log::set_max_level()`.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth::builder()
    ///     .max_level(log::LevelFilter::Warn)
    ///     .apply();
    /// ```
    pub fn max_level(mut self, max_level: LevelFilter) -> Self {
        self.max_level = max_level;
        self
    }

    /// Sets up the global logger with all the configured dispatches.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if a global logger has already been set.
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
        let logger = Logger::new(self.dispatches);
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(self.max_level);
        Ok(())
    }

    /// Sets up the global logger with all the configured dispatches.
    ///
    /// This function will panic if it is called more than once, or if another library has already
    /// initialized a global logger.
    ///
    /// # Panics
    ///
    /// Panics if the global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth::builder().apply();
    /// ```
    pub fn apply(self) {
        self.try_apply()
            .expect("Builder::apply should not be called after the global logger initialized");
    }
}

/// A builder for configuring a log dispatch, including filters and appenders.
///
/// # Examples
///
/// ```
/// use logforth::append;
///
/// logforth::builder()
///     .dispatch(|d| {
///         d.filter(log::LevelFilter::Info)
///             .append(append::Stdout::default())
///     })
///     .apply();
/// ```
#[derive(Debug)]
pub struct DispatchBuilder<const APPEND: bool> {
    filters: Vec<Filter>,
    diagnostics: Vec<Diagnostic>,
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
    /// use logforth::append;
    ///
    /// logforth::builder()
    ///     .dispatch(|d| {
    ///         d.filter(log::LevelFilter::Error)
    ///             .append(append::Stderr::default())
    ///     })
    ///     .apply();
    /// ```
    pub fn filter(mut self, filter: impl Into<Filter>) -> Self {
        self.filters.push(filter.into());
        self
    }

    /// Add a diagnostic to this dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append;
    /// use logforth::diagnostic;
    ///
    /// logforth::builder()
    ///     .dispatch(|d| {
    ///         d.filter(log::LevelFilter::Error)
    ///             .diagnostic(diagnostic::ThreadLocalDiagnostic::default())
    ///             .append(append::Stderr::default())
    ///     })
    ///     .apply();
    /// ```
    pub fn diagnostic(mut self, diagnostic: impl Into<Diagnostic>) -> Self {
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
    pub fn append(mut self, append: impl Append) -> DispatchBuilder<true> {
        self.appends.push(Box::new(append));
        DispatchBuilder {
            filters: self.filters,
            diagnostics: self.diagnostics,
            appends: self.appends,
        }
    }
}
