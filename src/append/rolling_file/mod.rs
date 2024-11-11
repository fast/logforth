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
//! use logforth::non_blocking::NonBlockingBuilder;
//! use logforth::append::rolling_file::RollingFile;
//! use logforth::append::rolling_file::RollingFileWriter;
//! use logforth::append::rolling_file::Rotation;
//! use logforth::layout::JsonLayout;
//!
//! let rolling_writer = RollingFileWriter::builder()
//!     .rotation(Rotation::Daily)
//!     .filename_prefix("app_log")
//!     .build("logs")
//!     .unwrap();
//!
//! let (non_blocking, _guard) = NonBlockingBuilder::default().finish(rolling_writer);
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

mod append;
mod clock;
mod rolling;
mod rotation;
