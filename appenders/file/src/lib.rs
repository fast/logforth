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

//! Appender for writing log records to single file or rolling files.
//!
//! # Example
//!
//!```
//! use logforth_append_file::File;
//! use logforth_append_file::FileBuilder;
//! use logforth_core::record::LevelFilter;
//! use logforth_layout_json::JsonLayout;
//!
//! logforth_bridge_log::setup_log_crate();
//!
//! let rolling = FileBuilder::new("logs", "app_log")
//!     .layout(JsonLayout::default())
//!     .rollover_daily()
//!     .build()
//!     .unwrap();
//!
//! logforth_core::builder()
//!     .dispatch(|d| d.filter(LevelFilter::Trace).append(rolling))
//!     .apply();
//!
//! log::info!("This log will be written to a rolling file.");
//! ```

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub use self::append::File;
pub use self::append::FileBuilder;

mod append;
mod clock;
mod rolling;
mod rotation;
