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

//! Starter configurations for quickly setting up logforth with the `log` crate

use crate::Append;
use crate::Error;
use crate::Filter;
use crate::Layout;
use crate::append;
use crate::core::DispatchBuilder;
use crate::core::LoggerBuilder;
use crate::filter::env_filter::EnvFilterBuilder;

/// A builder for setting up logforth with the `log` crate.
pub struct LogStarterBuilder {
    builder: LoggerBuilder,
}

/// Create a new empty [`LogStarterBuilder`] instance for configuring logforth setups.
///
/// # Examples
///
/// ```
/// use logforth::append;
///
/// let builder = logforth::starter_log::builder()
///     .dispatch(|d| d.append(append::Stderr::default()))
///     .apply();
/// ```
pub fn builder() -> LogStarterBuilder {
    use crate::core::builder;
    LogStarterBuilder { builder: builder() }
}

impl LogStarterBuilder {
    /// Register a new dispatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append;
    ///
    /// logforth::starter_log::builder()
    ///     .dispatch(|d| d.append(append::Stderr::default()))
    ///     .apply();
    /// ```
    pub fn dispatch<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DispatchBuilder<false>) -> DispatchBuilder<true>,
    {
        self.builder = self.builder.dispatch(f);
        self
    }

    /// Set up the global logger with all the configured dispatches.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// This function will set the global maximum log level to `Trace`. To override this, call
    /// `log::set_max_level` after this function.
    ///
    /// # Errors
    ///
    /// Return an error if a global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Err(err) = logforth::starter_log::builder().try_apply() {
    ///     eprintln!("failed to set logger: {err}");
    /// }
    /// ```
    pub fn try_apply(self) -> Result<(), Error> {
        self.builder
            .try_apply()
            .map_err(|_| Error::new("logforth default logger has been already setup"))?;

        logforth_bridge_log::try_setup()
            .map_err(|_| Error::new("log global logger has been already setup"))?;

        Ok(())
    }

    /// Set up the global logger with all the configured dispatches.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// This function will panic if it is called more than once, or if another library has already
    /// initialized a global logger.
    ///
    /// This function will set the global maximum log level to `Trace`. To override this, call
    /// `log::set_max_level` after this function.
    ///
    /// # Panics
    ///
    /// Panic if the global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth::starter_log::builder().apply();
    /// ```
    pub fn apply(self) {
        self.try_apply()
            .expect("LogStarterBuilder::apply must be called before the global logger initialized");
    }
}

enum StdStream {
    Stdout(append::Stdout),
    Stderr(append::Stderr),
}

/// A builder for setting up logforth with the `log` crate, using standard output/error streams.
pub struct LogStarterStdStreamBuilder {
    append: StdStream,
    filter: Box<dyn Filter>,
    layout: Box<dyn Layout>,
}

/// Create a starter builder with a default [`append::Stdout`] appender and an [`EnvFilter`]
/// respecting `RUST_LOG`.
///
/// [`EnvFilter`]: crate::filter::EnvFilter
///
/// # Examples
///
/// ```
/// logforth::starter_log::stdout().apply();
/// log::error!("This error will be logged to stdout.");
/// ```
pub fn stdout() -> LogStarterStdStreamBuilder {
    LogStarterStdStreamBuilder {
        append: StdStream::Stdout(append::Stdout::default()),
        filter: default_filter(),
        layout: default_layout(),
    }
}

/// Create a starter builder with a default [`append::Stderr`] appender and an [`EnvFilter`]
/// respecting `RUST_LOG`.
///
/// [`EnvFilter`]: crate::filter::EnvFilter
///
/// # Examples
///
/// ```
/// logforth::starter_log::stderr().apply();
/// log::error!("This error will be logged to stderr.");
/// ```
pub fn stderr() -> LogStarterStdStreamBuilder {
    LogStarterStdStreamBuilder {
        append: StdStream::Stderr(append::Stderr::default()),
        filter: default_filter(),
        layout: default_layout(),
    }
}

fn default_filter() -> Box<dyn Filter> {
    Box::new(EnvFilterBuilder::from_default_env().build())
}

fn default_layout() -> Box<dyn Layout> {
    #[cfg(feature = "layout-text")]
    {
        use crate::layout::TextLayout;
        Box::new(TextLayout::default())
    }

    #[cfg(not(feature = "layout-text"))]
    {
        use crate::layout::PlainTextLayout;
        Box::new(PlainTextLayout::default())
    }
}

impl LogStarterStdStreamBuilder {
    /// Set the layout for the StdStream appender.
    ///
    /// # Examples
    ///
    /// ```
    /// # use logforth::layout::PlainTextLayout;
    /// logforth::starter_log::stderr()
    ///     .layout(PlainTextLayout::default())
    ///     .apply();
    /// log::error!("This error will be logged to stderr.");
    /// ```
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Set the layout for the StdStream appender.
    ///
    /// # Examples
    ///
    /// ```
    /// # use logforth::record::LevelFilter;
    /// # use logforth::record::Level;
    /// logforth::starter_log::stdout().filter(LevelFilter::MoreSevereEqual(Level::Warn)).apply();
    /// log::info!("This info message will be ignored.");
    pub fn filter(mut self, filter: impl Into<Box<dyn Filter>>) -> Self {
        self.filter = filter.into();
        self
    }

    /// Set up the global logger with the configured std stream dispatch.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// This function will set the global maximum log level to `Trace`. To override this, call
    /// `log::set_max_level` after this function.
    ///
    /// # Errors
    ///
    /// Return an error if a global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Err(err) = logforth::starter_log::stdout().try_apply() {
    ///     eprintln!("failed to set logger: {err}");
    /// }
    /// ```
    pub fn try_apply(self) -> Result<(), Error> {
        let Self {
            append,
            filter,
            layout,
        } = self;

        let append: Box<dyn Append> = match append {
            StdStream::Stdout(a) => Box::new(a.with_layout(layout)),
            StdStream::Stderr(a) => Box::new(a.with_layout(layout)),
        };

        builder()
            .dispatch(|d| d.filter(filter).append(append))
            .try_apply()
    }

    /// Set up the global logger with the configured std stream dispatch.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur
    /// before initialization will be ignored.
    ///
    /// This function will panic if it is called more than once, or if another library has already
    /// initialized a global logger.
    ///
    /// This function will set the global maximum log level to `Trace`. To override this, call
    /// `log::set_max_level` after this function.
    ///
    /// # Panics
    ///
    /// Panic if the global logger has already been set.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth::starter_log::stdout().apply();
    /// ```
    pub fn apply(self) {
        self.try_apply().expect(
            "LogStarterStdStreamBuilder::apply must be called before the global logger initialized",
        );
    }
}
