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
use crate::Filter;

/// Create a new empty [builder][Builder].
///
/// At least one dispatch would be added:
///
/// ```rust
/// use log::LevelFilter;
/// use logforth::append;
///
/// logforth::dispatch(|b| {
///     b.filter(LevelFilter::Info)
///         .append(append::Stdout::default())
/// })
/// .apply();
/// ```
///
/// Multiple dispatches can be added:
///
/// ```rust
/// use log::LevelFilter;
/// use logforth::append;
///
/// logforth::dispatch(|b| {
///     b.filter(LevelFilter::Info)
///         .append(append::Stdout::default())
/// })
/// .and_dispatch(|b| {
///     b.filter(LevelFilter::Debug)
///         .append(append::Stderr::default())
/// })
/// .apply();
/// ```
pub fn dispatch<F>(f: F) -> Builder
where
    F: FnOnce(DispatchBuilder<false>) -> DispatchBuilder<true>,
{
    Builder::dispatch(f)
}

/// Create a new [`Builder`] with a default [`Stdout`][append::Stdout] append configured, and
/// respect the `RUST_LOG` environment variable for filtering logs.
///
/// This is a convenient API that you can use as:
///
/// ```rust
/// logforth::stdout().apply();
/// ```
pub fn stdout() -> Builder {
    dispatch(|b| {
        b.filter(EnvFilter::from_default_env())
            .append(append::Stdout::default())
    })
}

/// Create a new [`Builder`] with a default [`Stderr`][append::Stderr] append configured, and
/// respect the `RUST_LOG` environment variable for filtering logs.
///
/// This is a convenient API that you can use as:
///
/// ```rust
/// logforth::stderr().apply();
/// ```
pub fn stderr() -> Builder {
    dispatch(|b| {
        b.filter(EnvFilter::from_default_env())
            .append(append::Stderr::default())
    })
}

/// A builder for configuring the logger. Always constructed via [`dispatch`] for a fluent API.
///
/// ## Examples
///
/// Create a new builder and configure filters and appends:
///
/// ```rust
/// use log::LevelFilter;
/// use logforth::append;
///
/// logforth::dispatch(|b| {
///     b.filter(LevelFilter::Info)
///         .append(append::Stdout::default())
/// })
/// .apply();
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
    /// Create a new logger builder with the first dispatch configured by `f`.
    fn dispatch<F>(f: F) -> Self
    where
        F: FnOnce(DispatchBuilder<false>) -> DispatchBuilder<true>,
    {
        Self {
            dispatches: vec![f(DispatchBuilder::new()).build()],
            max_level: LevelFilter::Trace,
        }
    }

    /// Stage a new dispatch.
    pub fn and_dispatch<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DispatchBuilder<false>) -> DispatchBuilder<true>,
    {
        self.dispatches.push(f(DispatchBuilder::new()).build());
        self
    }

    /// Set the global maximum log level.
    ///
    /// This will be passed to [`log::set_max_level`] on [`Builder::finish`].
    pub fn max_level(mut self, max_level: LevelFilter) -> Self {
        self.max_level = max_level;
        self
    }

    /// Set up the global logger with all the dispatches configured.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// # Errors
    ///
    /// This function will fail if it is called more than once, or if another library has already
    /// initialized a global logger.
    pub fn try_apply(self) -> Result<(), log::SetLoggerError> {
        let logger = Logger::new(self.dispatches);
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(self.max_level);
        Ok(())
    }

    /// Set up the global logger with all the dispatches configured.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// # Panics
    ///
    /// This function will panic if it is called more than once, or if another library has already
    /// initialized a global logger.
    pub fn apply(self) {
        self.try_apply()
            .expect("Builder::apply should not be called after the global logger initialized");
    }
}

#[derive(Debug)]
pub struct DispatchBuilder<const APPEND: bool = true> {
    filters: Vec<Filter>,
    appends: Vec<Box<dyn Append>>,
}

impl DispatchBuilder<false> {
    fn new() -> Self {
        DispatchBuilder {
            filters: vec![],
            appends: vec![],
        }
    }
}

impl DispatchBuilder<true> {
    fn build(self) -> Dispatch {
        Dispatch::new(self.filters, self.appends)
    }
}

impl<const APPEND: bool> DispatchBuilder<APPEND> {
    /// Add a [`Filter`] to the under constructing `Dispatch`.
    pub fn filter(mut self, filter: impl Into<Filter>) -> Self {
        self.filters.push(filter.into());
        self
    }

    /// Add an [`Append`] to the under constructing `Dispatch`.
    pub fn append(mut self, append: impl Append) -> DispatchBuilder<true> {
        self.appends.push(Box::new(append));
        DispatchBuilder {
            filters: self.filters,
            appends: self.appends,
        }
    }
}
