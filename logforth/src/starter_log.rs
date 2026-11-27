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

//! Starter presets and extension traits for installing Logforth as the global [`log`] logger.
//!
//! Use [`stdout`], [`stderr`], or [`testing`] for single-sink setups driven by `RUST_LOG`, or
//! import [`LogStarterExt`] to call `.install()` / `.try_install()` directly on a [`LoggerBuilder`]
//! or [`Logger`] constructed via [`builder`].

use logforth_bridge_log::LogBridge;

use crate::Append;
use crate::Error;
use crate::ErrorSink;
use crate::Filter;
use crate::Layout;
use crate::Logger;
use crate::LoggerBuilder;
use crate::append;
use crate::builder;
use crate::error::StderrErrorSink;
use crate::filter::rustlog::RustLogFilterBuilder;

mod sealed {
    pub trait Sealed {}
}

/// Extension trait for installing a Logforth [`Logger`], [`LoggerBuilder`], [`LogStarter`], or
/// [`LogBridge`] as the global [`log`] logger.
///
/// # Examples
///
/// ```
/// use logforth::append;
/// use logforth::starter_log::LogStarterExt;
///
/// logforth::builder()
///     .dispatch(|d| d.append(append::Stderr::default()))
///     .install();
/// ```
pub trait LogStarterExt: sealed::Sealed + Sized {
    /// Install `self` as the global [`log`] logger and update [`log::set_max_level`] from the
    /// active levels.
    ///
    /// # Errors
    ///
    /// Returns an error if a global [`log`] logger has already been set or if filter
    /// initialization fails.
    fn try_install(self) -> Result<(), Error>;

    /// Install `self` as the global [`log`] logger and update [`log::set_max_level`] from the
    /// active levels.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::try_install`] returns an error.
    fn install(self) {
        self.try_install()
            .expect("failed to install global log logger");
    }
}

impl sealed::Sealed for LogBridge {}

impl LogStarterExt for LogBridge {
    fn try_install(self) -> Result<(), Error> {
        let max_level = self.max_level();
        log::set_boxed_logger(Box::new(self))
            .map_err(|err| Error::new("logging system has already been setup").with_source(err))?;
        log::set_max_level(max_level);
        Ok(())
    }
}

impl sealed::Sealed for Logger {}

impl LogStarterExt for Logger {
    fn try_install(self) -> Result<(), Error> {
        LogBridge::new(self).try_install()
    }
}

impl sealed::Sealed for LoggerBuilder {}

impl LogStarterExt for LoggerBuilder {
    fn try_install(self) -> Result<(), Error> {
        self.build().try_install()
    }
}

impl sealed::Sealed for LogStarter {}

impl LogStarterExt for LogStarter {
    fn try_install(self) -> Result<(), Error> {
        LogStarter::try_install(self)
    }
}

enum StarterSink {
    Stdout(append::Stdout),
    Stderr(append::Stderr),
    Testing(append::Testing),
}

impl StarterSink {
    fn into_append(self, layout: Box<dyn Layout>) -> Box<dyn Append> {
        match self {
            Self::Stdout(a) => Box::new(a.with_layout(layout)),
            Self::Stderr(a) => Box::new(a.with_layout(layout)),
            Self::Testing(a) => Box::new(a.with_layout(layout)),
        }
    }
}

/// Create a starter preset writing to [`append::Stdout`] and respecting `RUST_LOG`.
///
/// # Examples
///
/// ```
/// logforth::starter_log::stdout().install();
/// log::error!("This error will be logged to stdout.");
/// ```
pub fn stdout() -> LogStarter {
    LogStarter::new(StarterSink::Stdout(append::Stdout::default()))
}

/// Create a starter preset writing to [`append::Stderr`] and respecting `RUST_LOG`.
///
/// # Examples
///
/// ```
/// logforth::starter_log::stderr().install();
/// log::error!("This error will be logged to stderr.");
/// ```
pub fn stderr() -> LogStarter {
    LogStarter::new(StarterSink::Stderr(append::Stderr::default()))
}

/// Create a starter preset writing to [`append::Testing`] and respecting `RUST_LOG`.
///
/// # Examples
///
/// ```
/// logforth::starter_log::testing().install();
/// log::error!("This error will be logged to stderr and respect test output capture.");
/// ```
pub fn testing() -> LogStarter {
    LogStarter::new(StarterSink::Testing(append::Testing::default()))
}

/// A single-sink preset builder for installing Logforth into the [`log`] facade.
pub struct LogStarter {
    sink: StarterSink,
    filter: Option<Box<dyn Filter>>,
    layout: Box<dyn Layout>,
    error_sink: Box<dyn ErrorSink>,
}

impl LogStarter {
    fn new(sink: StarterSink) -> Self {
        Self {
            sink,
            filter: None,
            layout: default_layout(),
            error_sink: Box::new(StderrErrorSink::default()),
        }
    }

    /// Override the layout for the preset appender.
    ///
    /// # Examples
    ///
    /// ```
    /// # use logforth::layout::PlainTextLayout;
    /// logforth::starter_log::stderr()
    ///     .layout(PlainTextLayout::default())
    ///     .install();
    /// log::error!("This error will be logged to stderr.");
    /// ```
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Override the default `RUST_LOG` filter for the preset appender.
    ///
    /// # Examples
    ///
    /// ```
    /// # use logforth::record::LevelFilter;
    /// # use logforth::record::Level;
    /// logforth::starter_log::stdout()
    ///     .filter(LevelFilter::MoreSevereEqual(Level::Warn))
    ///     .install();
    /// log::info!("This info message will be ignored.");
    /// ```
    pub fn filter(mut self, filter: impl Into<Box<dyn Filter>>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Configure the [`ErrorSink`] used when the preset appender fails to format, write, or flush.
    ///
    /// Defaults to [`StderrErrorSink`].
    pub fn error_sink(mut self, error_sink: impl Into<Box<dyn ErrorSink>>) -> Self {
        self.error_sink = error_sink.into();
        self
    }

    /// Build and install the preset logger as the global [`log`] logger, updating
    /// [`log::set_max_level`] from the active levels.
    ///
    /// # Errors
    ///
    /// Returns an error if `RUST_LOG` is invalid (when using the default filter) or if a global
    /// [`log`] logger has already been installed.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Err(err) = logforth::starter_log::stdout().try_install() {
    ///     eprintln!("failed to install logger: {err}");
    /// }
    /// ```
    pub fn try_install(self) -> Result<(), Error> {
        <Logger as LogStarterExt>::try_install(self.try_build()?)
    }

    /// Build and install the preset logger as the global [`log`] logger, updating
    /// [`log::set_max_level`] from the active levels.
    ///
    /// # Panics
    ///
    /// Panics if `RUST_LOG` is invalid (when using the default filter) or if a global [`log`]
    /// logger has already been installed.
    ///
    /// # Examples
    ///
    /// ```
    /// logforth::starter_log::stdout().install();
    /// ```
    pub fn install(self) {
        self.try_install()
            .expect("LogStarter::install failed to initialize logger");
    }

    /// Build the configured [`Logger`] without installing it globally.
    ///
    /// # Errors
    ///
    /// Returns an error if `RUST_LOG` is invalid when using the default filter.
    ///
    /// # Examples
    ///
    /// ```
    /// let logger = logforth::starter_log::stdout().try_build().unwrap();
    /// logger.flush();
    /// ```
    pub fn try_build(self) -> Result<Logger, Error> {
        let Self {
            sink,
            filter,
            layout,
            error_sink,
        } = self;

        let filter = match filter {
            Some(filter) => filter,
            None => {
                let builder = RustLogFilterBuilder::from_default_env()?;
                Box::new(builder.build())
            }
        };
        let append = sink.into_append(layout);
        Ok(builder()
            .error_sink(error_sink)
            .dispatch(|d| d.filter(filter).append(append))
            .build())
    }

    /// Build the configured [`Logger`] without installing it globally.
    ///
    /// # Panics
    ///
    /// Panics if `RUST_LOG` is invalid when using the default filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::starter_log::LogStarterExt;
    ///
    /// let logger = logforth::starter_log::stdout().build();
    /// logger.clone().install();
    ///
    /// logger.flush();
    /// ```
    pub fn build(self) -> Logger {
        self.try_build()
            .expect("LogStarter::build failed to initialize filter")
    }
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
