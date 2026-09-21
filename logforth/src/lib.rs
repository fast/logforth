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

//! Native structured logging with configurable dispatches, filters, diagnostics, and appenders.
//!
//! Build an explicit [`Logger`] and use the native macros. They skip message and
//! field evaluation when metadata filtering rejects the event.
//!
//! ```
//! use logforth::{Level, LevelFilter, append};
//!
//! let logger = logforth::builder()
//!     .dispatch(|d| d
//!         .filter(LevelFilter::MoreSevereEqual(Level::Info))
//!         .append(append::Stderr::default()))
//!     .build();
//! let worker = logger.named("worker");
//! logforth::info!(worker, { "jobs" => 3u64, "healthy" => true }, "batch complete");
//! logger.flush();
//! ```
//!
//! [`log!`] accepts every native severity; [`info!`] and the other convenience
//! macros use the same syntax. Fields borrow values through [`kv::ToValue`].
//! Use [`kv::Value::display`] / [`kv::Value::debug`] for text, or `kv::serde`
//! (feature `serde`) for nested structures. A fields-only event needs no message.
//!
//! [`Logger::named`] shares dispatches and selects a stable target, while source
//! metadata always describes the call site. Build a separate logger for an
//! independent output or filtering policy. Stop producers and flush each shared
//! dispatch graph before shutting down resources used by appenders.
//!
//! The optional `bridge-log` feature forwards dependency logs to a native logger.
//! Existing `starter-log` helpers configure the `log` facade as a compatibility
//! entry point. Native logging does not require either feature.
//!
//! See the [README] for setup, migration, and lifecycle details.
//!
//! [README]: https://github.com/fast/logforth#readme

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub use logforth_core::DispatchBuilder;
pub use logforth_core::Error;
pub use logforth_core::Logger;
pub use logforth_core::LoggerBuilder;
pub use logforth_core::append::Append;
pub use logforth_core::builder;
pub use logforth_core::debug;
pub use logforth_core::diagnostic::Diagnostic;
pub use logforth_core::error;
pub use logforth_core::fatal;
pub use logforth_core::filter::Filter;
pub use logforth_core::info;
pub use logforth_core::kv;
pub use logforth_core::layout::Layout;
pub use logforth_core::log;
pub use logforth_core::record;
pub use logforth_core::record::Level;
pub use logforth_core::record::LevelFilter;
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

/// Core components of the logforth logging framework.
pub mod core {
    // structs
    pub use logforth_core::DispatchBuilder;
    pub use logforth_core::Logger;
    pub use logforth_core::LoggerBuilder;
    // methods
    pub use logforth_core::builder;
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

/// Traps for processing errors.
pub mod trap {
    pub use logforth_core::trap::*;
}

#[cfg(feature = "starter-log")]
pub mod starter_log;
