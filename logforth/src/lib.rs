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

//! Logforth is a flexible logging framework for Rust applications, providing easy log dispatching
//! and configuration.
//!
//! # Overview
//!
//! Logforth allows you to set up multiple log dispatches with different filters and appenders. You
//! can build explicit [`Logger`] instances with built-in appenders, layouts, filters, and
//! diagnostics using zero optional features:
//!
//! ```shell
//! cargo add logforth
//! ```
//!
//! # Examples
//!
//! Configure a native [`Logger`] with multiple dispatches and structured key-value logging:
//!
//! ```
//! use logforth::append;
//! use logforth::record::Level;
//! use logforth::record::LevelFilter;
//!
//! let logger = logforth::builder()
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::MoreSevereEqual(Level::Error))
//!             .append(append::Stderr::default())
//!     })
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::MoreSevereEqual(Level::Info))
//!             .append(append::Stdout::default())
//!     })
//!     .build();
//!
//! logforth::error!(logger, "Error message.");
//! logforth::info!(logger, ["port" = 8080, "tls" = true], "Server started.");
//! logger.flush();
//! ```
//!
//! For crates or applications using the `log` facade, enable the `starter-log` (or `bridge-log`)
//! feature to bridge an explicit [`Logger`] or install a pipeline globally:
//!
//! ```
//! # #[cfg(feature = "starter-log")]
//! # {
//! use logforth::append;
//! use logforth::bridge::log::LogBridge;
//! use logforth::starter_log::LogStarterExt;
//!
//! let logger = logforth::builder()
//!     .dispatch(|d| d.append(append::Stderr::default()))
//!     .build();
//!
//! // Wrap with `LogBridge::new(logger)` for `log::info!(logger: ...)`, or install globally:
//! let _bridge = LogBridge::new(logger.clone());
//! logger.install();
//! log::info!("Info message via log facade.");
//! # }
//! ```
//!
//! See the [README] file for more details and examples.
//!
//! [README]: https://github.com/fast/logforth?tab=readme-ov-file

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub use logforth_core::DispatchBuilder;
pub use logforth_core::Logger;
pub use logforth_core::LoggerBuilder;
pub use logforth_core::append::Append;
pub use logforth_core::builder;
pub use logforth_core::debug;
pub use logforth_core::diagnostic::Diagnostic;
pub use logforth_core::error;
pub use logforth_core::error::Error;
pub use logforth_core::error::ErrorSink;
pub use logforth_core::fatal;
pub use logforth_core::filter::Filter;
pub use logforth_core::info;
pub use logforth_core::kv;
pub use logforth_core::layout::Layout;
pub use logforth_core::log;
pub use logforth_core::record;
pub use logforth_core::trace;
pub use logforth_core::warn;

/// Dispatch log records to various targets.
pub mod append {
    #[cfg(feature = "append-async")]
    pub use logforth_append_async as asynchronous; // `async` is a keyword
    #[cfg(feature = "append-async")]
    pub use logforth_append_async::Async;
    #[cfg(feature = "append-fastrace")]
    pub use logforth_append_fastrace::FastraceEvent;
    #[cfg(feature = "append-file")]
    pub use logforth_append_file as file;
    #[cfg(feature = "append-file")]
    pub use logforth_append_file::File;
    #[cfg(all(unix, feature = "append-journald"))]
    pub use logforth_append_journald::Journald;
    #[cfg(feature = "append-opentelemetry")]
    pub use logforth_append_opentelemetry as opentelemetry;
    #[cfg(feature = "append-opentelemetry")]
    pub use logforth_append_opentelemetry::OpentelemetryLog;
    #[cfg(feature = "append-syslog")]
    pub use logforth_append_syslog as syslog;
    #[cfg(feature = "append-syslog")]
    pub use logforth_append_syslog::Syslog;
    pub use logforth_core::append::*;
}

/// Bridge logforth with other logging frameworks.
pub mod bridge {
    /// Bridge logforth with [`log`].
    ///
    /// [`log`]: https://docs.rs/log
    #[cfg(feature = "bridge-log")]
    pub mod log {
        pub use logforth_bridge_log::*;
    }
}

/// Mapped Diagnostic Context (MDC).
pub mod diagnostic {
    pub use logforth_core::diagnostic::*;
    #[cfg(feature = "diagnostic-fastrace")]
    pub use logforth_diagnostic_fastrace::FastraceDiagnostic;
    #[cfg(feature = "diagnostic-task-local")]
    pub use logforth_diagnostic_task_local as task_local;
    #[cfg(feature = "diagnostic-task-local")]
    pub use logforth_diagnostic_task_local::TaskLocalDiagnostic;
}

/// Filters for log records.
pub mod filter {
    pub use logforth_core::filter::*;
    #[cfg(feature = "filter-rustlog")]
    pub use logforth_filter_rustlog as rustlog;
    #[cfg(feature = "filter-rustlog")]
    pub use logforth_filter_rustlog::RustLogFilter;
}

/// Layouts for formatting log records.
pub mod layout {
    pub use logforth_core::layout::*;
    #[cfg(feature = "layout-google-cloud-logging")]
    pub use logforth_layout_google_cloud_logging::GoogleCloudLoggingLayout;
    #[cfg(feature = "layout-json")]
    pub use logforth_layout_json::JsonLayout;
    #[cfg(feature = "layout-logfmt")]
    pub use logforth_layout_logfmt::LogfmtLayout;
    #[cfg(feature = "layout-text")]
    pub use logforth_layout_text as text;
    #[cfg(feature = "layout-text")]
    pub use logforth_layout_text::TextLayout;
}

#[cfg(feature = "starter-log")]
pub mod starter_log;
