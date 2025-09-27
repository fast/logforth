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

#![deny(missing_docs)]

//! Logforth is a flexible logging framework for Rust applications, providing easy log dispatching
//! and configuration.
//!
//! # Overview
//!
//! Logforth allows you to set up multiple log dispatches with different filters and appenders. You
//! can configure global log levels, use built-in appenders for stdout, stderr, files, or create
//! custom appenders. It provides out-of-the-box integrations with the `log` crate.
//!
//! # Examples
//!
//! Simple setup with default stdout appender:
//!
//! ```
//! logforth::bridge::setup_log_crate();
//! logforth::stdout().apply();
//!
//! log::info!("This is an info message.");
//! ```
//!
//! Advanced setup with custom filters and multiple appenders:
//!
//! ```
//! use logforth::append;
//! use logforth::record::LevelFilter;
//!
//! logforth::bridge::setup_log_crate();
//! logforth::builder()
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::Error)
//!             .append(append::Stderr::default())
//!     })
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::Info)
//!             .append(append::Stdout::default())
//!     })
//!     .apply();
//!
//! log::error!("Error message.");
//! log::info!("Info message.");
//! ```

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod append;
pub mod bridge;
pub mod diagnostic;
pub mod filter;
pub mod kv;
pub mod layout;
pub mod record;
pub mod str;

pub use self::append::Append;
pub use self::diagnostic::Diagnostic;
pub use self::filter::Filter;
pub use self::layout::Layout;

mod error;
pub use self::error::*;

mod logger;
pub use self::logger::*;
