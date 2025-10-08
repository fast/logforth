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
use crate::logger::log_impl::Dispatch;
use crate::logger::log_impl::set_default_logger;

/// Create a new empty [`LoggerBuilder`] instance for configuring log dispatching.
///
/// # Examples
///
/// ```
/// use logforth_core::append;
///
/// let builder = logforth_core::builder()
///     .dispatch(|d| d.append(append::Stderr::default()))
///     .apply();
/// ```
pub fn builder() -> LoggerBuilder {
    LoggerBuilder { dispatches: vec![] }
}

/// A builder for configuring log dispatching and setting up the global logger.
///
/// # Examples
///
/// ```
/// use logforth_core::append;
///
/// logforth_core::builder()
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
    /// Register a new dispatch with the [`LoggerBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_core::append;
    ///
    /// logforth_core::builder()
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
    /// use logforth_core::record::Record;
    /// use logforth_core::record::RecordBuilder;
    ///
    /// let l = logforth_core::builder().build();
    /// let r = RecordBuilder::default().payload("hello world!").build();
    /// l.log(&r);
    /// ```
    pub fn build(self) -> Logger {
        Logger::new(self.dispatches)
    }

    /// Set up the global logger with all the configured dispatches.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// # Errors
    ///
    /// Return an error if a global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// if logforth_core::builder().try_apply().is_err() {
    ///     eprintln!("failed to set logger");
    /// }
    /// ```
    pub fn try_apply(self) -> Result<(), Logger> {
        set_default_logger(self.build())
    }

    /// Set up the global logger with all the configured dispatches.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// This function will panic if it is called more than once, or if another library has already
    /// initialized a global logger.
    ///
    /// # Panics
    ///
    /// Panic if the global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth_core::builder().apply();
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
/// use logforth_core::append;
/// use logforth_core::record::Level;
/// use logforth_core::record::LevelFilter;
///
/// logforth_core::builder()
///     .dispatch(|d| {
///         d.filter(LevelFilter::MoreSevereEqual(Level::Info))
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
    /// use logforth_core::append;
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    ///
    /// logforth_core::builder()
    ///     .dispatch(|d| {
    ///         d.filter(LevelFilter::MoreSevereEqual(Level::Error))
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
    /// use logforth_core::append;
    /// use logforth_core::diagnostic;
    /// use logforth_core::record::Level;
    /// use logforth_core::record::LevelFilter;
    ///
    /// logforth_core::builder()
    ///     .dispatch(|d| {
    ///         d.filter(LevelFilter::MoreSevereEqual(Level::Error))
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
    /// use logforth_core::append;
    ///
    /// logforth_core::builder()
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
