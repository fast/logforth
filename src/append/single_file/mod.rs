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

//! Appender for writing log records to a file.
//!
//! # Example
//!
//!```
//! use logforth::append::single_file;
//! use logforth::append::single_file::SingleFile;
//! use logforth::append::single_file::SingleFileBuilder;
//! use logforth::layout::JsonLayout;
//!
//! let (file_writer, _guard) = SingleFileBuilder::new("/path/to/flile.log")
//!     .layout(JsonLayout::default())
//!     .build()
//!     .unwrap();
//!
//! logforth::builder()
//!     .dispatch(|d| d.filter(log::LevelFilter::Trace).append(file_writer))
//!     .apply();
//!
//! log::info!("This log will be written to a file.");
//! ```

pub use append::SingleFile;
pub use append::SingleFileBuilder;

mod append;
mod single;
