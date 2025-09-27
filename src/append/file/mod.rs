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

//! Appender for writing log records to rolling files.
//!
//! # Example
//!
//!```
//! use logforth::append::file;
//! use logforth::append::file::File;
//! use logforth::append::file::FileBuilder;
//! use logforth::layout::JsonLayout;
//! use logforth::record::LevelFilter;
//!
//! let rolling_file = FileBuilder::new("logs", "app_log")
//!     .layout(JsonLayout::default())
//!     .rollover_daily()
//!     .build()
//!     .unwrap();
//!
//! logforth::builder()
//!     .dispatch(|d| d.filter(LevelFilter::Trace).append(rolling_file))
//!     .setup_log_crate();
//!
//! log::info!("This log will be written to a rolling file.");
//! ```

pub use self::append::File;
pub use self::append::FileBuilder;

mod append;
mod clock;
mod rolling;
mod rotation;
