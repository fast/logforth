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
//! can configure global log levels, use built-in appenders for stdout, stderr, files, or create
//! custom appenders.
//!
//! It provides out-of-the-box integrations with the `log` crate:
//!
//! ```shell
//! cargo add log
//! cargo add logforth -F starter-log
//! ```
//!
//! # Examples
//!
//! Simple setup with default stdout appender:
//!
//! ```
//! logforth::starter_log::stdout().apply();
//!
//! log::info!("This is an info message.");
//! ```
//!
//! Advanced setup with custom filters and multiple appenders:
//!
//! ```
//! use logforth::append;
//! use logforth::record::Level;
//! use logforth::record::LevelFilter;
//!
//! logforth::starter_log::builder()
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::MoreSevereEqual(Level::Error))
//!             .append(append::Stderr::default())
//!     })
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::MoreSevereEqual(Level::Info))
//!             .append(append::Stdout::default())
//!     })
//!     .apply();
//!
//! log::error!("Error message.");
//! log::info!("Info message.");
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub use logforth_core::Error;
pub use logforth_core::append::Append;
pub use logforth_core::diagnostic::Diagnostic;
pub use logforth_core::filter::Filter;
pub use logforth_core::kv;
pub use logforth_core::layout::Layout;
pub use logforth_core::record;

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
    /// [`log`]: https://docs.rs/log/
    #[cfg(feature = "bridge-log")]
    pub mod log {
        #[cfg(feature = "bridge-log")]
        pub use logforth_bridge_log::*;
    }
}

/// Core components of the logforth logging framework.
pub mod core {
    // structs
    pub use logforth_core::DispatchBuilder;
    pub use logforth_core::Logger;
    pub use logforth_core::LoggerBuilder;
    // methods
    pub use logforth_core::builder;
    pub use logforth_core::default_logger;
    pub use logforth_core::set_default_logger;
}

/// Mapped Diagnostic Context (MDC).
pub mod diagnostic {
    pub use logforth_core::diagnostic::*;
    #[cfg(feature = "diagnostic-fastrace")]
    pub use logforth_diagnostic_fastrace::FastraceDiagnostic;
}

/// Filters for log records.
pub mod filter {
    pub use logforth_core::filter::*;
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

/// Traps for processing errors.
pub mod trap {
    pub use logforth_core::trap::*;
}

#[cfg(feature = "bridge-log")]
pub mod starter_log;
