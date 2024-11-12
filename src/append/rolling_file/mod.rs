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
//! use logforth::append::rolling_file;
//! use logforth::append::rolling_file::RollingFile;
//! use logforth::append::rolling_file::RollingFileWriter;
//! use logforth::append::rolling_file::Rotation;
//! use logforth::layout::JsonLayout;
//! use logforth::non_blocking::NonBlockingBuilder;
//!
//! let rolling_writer = RollingFileWriter::builder()
//!     .rotation(Rotation::Daily)
//!     .filename_prefix("app_log")
//!     .build("logs")
//!     .unwrap();
//!
//! let (non_blocking, _guard) = rolling_file::non_blocking_builder().finish(rolling_writer);
//!
//! logforth::builder()
//!     .dispatch(|d| {
//!         d.filter(log::LevelFilter::Trace)
//!             .append(RollingFile::new(non_blocking).with_layout(JsonLayout::default()))
//!     })
//!     .apply();
//!
//! log::info!("This log will be written to a rolling file.");
//! ```

pub use append::RollingFile;
pub use rolling::RollingFileWriter;
pub use rolling::RollingFileWriterBuilder;
pub use rotation::Rotation;

use crate::non_blocking::NonBlockingBuilder;

mod append;
mod clock;
mod rolling;
mod rotation;

/// Create a non-blocking builder for rolling file writers.
pub fn non_blocking_builder() -> NonBlockingBuilder<RollingFileWriter> {
    NonBlockingBuilder::new("logforth-rolling-file")
}
