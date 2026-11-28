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

//! Core pipeline primitives, structured records, and extension traits for Logforth.
//!
//! `logforth-core` defines the foundational contracts shared across the Logforth ecosystem with
//! zero mandatory third-party dependencies: explicit [`Logger`] channels, [`Record`](record::Record)
//! metadata and payloads, zero-allocation [`kv`] structured fields, native logging macros, and the
//! five extension traits ([`Filter`], [`Diagnostic`], [`Layout`], [`Append`], and [`ErrorSink`]).
//!
//! Most applications can use the `logforth` facade crate, which re-exports this crate alongside
//! optional appender, layout, filter, diagnostic, and bridge crates. Depend on `logforth-core`
//! directly when implementing an extension trait, accepting an injected [`Logger`] in a library, or
//! assembling split `logforth-*` crates explicitly.
//!
//! # Getting started
//!
//! Build an explicit [`Logger`] with one or more [`DispatchBuilder`] branches and emit events with
//! the native structured logging macros:
//!
//! ```
//! use logforth_core::Logger;
//! use logforth_core::append;
//! use logforth_core::diagnostic::ThreadLocalDiagnostic;
//! use logforth_core::record::Level;
//! use logforth_core::record::LevelFilter;
//!
//! let logger = logforth_core::builder()
//!     .dispatch(|d| {
//!         d.filter(LevelFilter::MoreSevereEqual(Level::Info))
//!             .diagnostic(ThreadLocalDiagnostic::default())
//!             .append(append::Stderr::default())
//!     })
//!     .build();
//!
//! let _scope = ThreadLocalDiagnostic::scope([("request_id", "req-42")]);
//! logforth_core::info!(logger, ["port" = 8080, "tls" = true], "server started");
//! logger.flush();
//!
//! // Libraries can default to a zero-allocation no-op logger when none is supplied:
//! const NOOP: Logger = Logger::noop();
//! logforth_core::debug!(NOOP, "discarded without allocation");
//! ```
//!
//! # API map
//!
//! | Area        | Primary APIs                                                                                          | Purpose                                                                                 |
//! | ----------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
//! | Pipeline    | [`Logger`], [`LoggerBuilder`], [`DispatchBuilder`], [`builder`]                                       | Assemble and share immutable multi-branch logging pipelines without global singletons.  |
//! | Macros      | [`log!`], [`trace!`], [`debug!`], [`info!`], [`warn!`], [`error!`], [`fatal!`]                        | Emit structured `"key" = value` fields and formatted payloads to a [`Logger`].          |
//! | Records     | [`record::Record`], [`record::RecordBuilder`], [`record::RecordOwned`], [`record::Level`]             | Represent stack-borrowed log events, owned cross-thread snapshots, and 24 severities.   |
//! | Key-values  | [`kv::Key`], [`kv::Value`], [`kv::LogValue`], [`kv::ValueView`], [`kv::ValueOwned`], [`kv::Visitor`]  | Capture borrowed or owned structured scalars, byte slices, lists, and maps.             |
//! | Filtering   | [`Filter`], [`filter::FilterResult`], [`record::LevelFilter`]                                         | Prune disabled levels via precomputed bitmasks and evaluate per-record filter chains.   |
//! | Diagnostics | [`Diagnostic`], [`diagnostic::StaticDiagnostic`], [`diagnostic::ThreadLocalDiagnostic`]               | Enrich records with static, scoped thread-local, or composite contextual key-values.    |
//! | Formatting  | [`Layout`], [`layout::PlainTextLayout`]                                                               | Append formatted record bytes into reusable caller buffers without framing newlines.    |
//! | Appending   | [`Append`], [`append::Stdout`], [`append::Stderr`], [`append::Testing`]                               | Deliver structured records or formatted byte streams to output destinations.            |
//! | Errors      | [`Error`], [`ErrorSink`], [`error::StderrErrorSink`], [`error::BlackHoleErrorSink`]                   | Propagate single-word operational errors into best-effort terminal sinks without panic. |
//!
//! # Pipeline and safety contracts
//!
//! - **No ambient global state**: [`Logger`] is a pointer-sized [`Clone`] handle over a shared
//!   pipeline; `logforth-core` installs no process-wide singleton.
//! - **DAG sharing**: [`Filter`], [`Diagnostic`], [`Layout`], [`Append`], and [`ErrorSink`] forward
//!   through `Arc<T>` and `&'static T` (and `&T` for [`Diagnostic`]) so stateful components can be
//!   shared across branches or loggers. Implementations of [`Append`] and [`ErrorSink`] must never
//!   route log records back into their owning [`Logger`].
//! - **Non-panicking operational errors**: Runtime formatting and I/O failures return [`Error`] and
//!   route to the active [`ErrorSink`] rather than crashing the host application.
//! - **Reserved key namespace**: Keys prefixed with `logforth.` in [`kv`] are reserved by Logforth.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub mod append;
pub mod diagnostic;
pub mod error;
pub mod filter;
pub mod kv;
pub mod layout;
pub mod record;

pub use self::append::Append;
pub use self::diagnostic::Diagnostic;
pub use self::error::Error;
pub use self::error::ErrorSink;
pub use self::filter::Filter;
pub use self::layout::Layout;

mod logger;
pub use self::logger::*;

mod macros;
#[doc(hidden)]
pub use self::macros::__private;
#[doc(hidden)]
pub use self::macros::__private::log as __private_log;

mod str;
