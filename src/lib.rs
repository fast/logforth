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

//! # A versatile and extensible logging implementation
//!
//! ## Usage
//!
//! Add the dependencies to your `Cargo.toml` with:
//!
//! ```shell
//! cargo add log
//! cargo add logforth
//! ```
//!
//! Here, [`log`] is the logging facade and `logforth` is the logging implementation.
//!
//! Then, you can use the logger with:
//!
//! ```rust
//! use log::LevelFilter;
//! use logforth::append;
//! use logforth::layout::TextLayout;
//! use logforth::Dispatch;
//! use logforth::Logger;
//!
//! Logger::new()
//!     .dispatch(
//!         Dispatch::new()
//!             .filter(LevelFilter::Trace)
//!             .append(append::Stdout::default()),
//!     )
//!     .apply()
//!     .unwrap();
//!
//! log::error!("Hello error!");
//! log::warn!("Hello warn!");
//! log::info!("Hello info!");
//! log::debug!("Hello debug!");
//! log::trace!("Hello trace!");
//! ```
//!
//! Read more demos under the [examples](https://github.com/fast/logforth/tree/main/examples) directory.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod append;
pub mod filter;
pub mod layout;
mod logger;

pub use append::Append;
pub use filter::Filter;
pub use layout::Layout;
pub use logger::Dispatch;
pub use logger::Logger;
