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

//! File appender for writing log records to a single file or rotating segment files.
//!
//! [`File`] formats each record with a configured [`Layout`](logforth_core::Layout), appends a
//! trailing newline, and writes directly to an append-mode file. Construct an instance with
//! [`FileBuilder`].
//!
//! # Segment naming
//!
//! Given `FileBuilder::new(dir, "my_app").filename_suffix("log")`, segment filenames depend on the
//! configured rotation policy:
//!
//! | Policy          | Example filename                |
//! | :-------------- | :------------------------------ |
//! | No rotation     | `my_app.log`                    |
//! | Minutely        | `my_app.2026-11-25-17-05.log`   |
//! | Hourly          | `my_app.2026-11-25-17.log`      |
//! | Daily           | `my_app.2026-11-25.log`         |
//! | Size            | `my_app.1.log`                  |
//! | Hourly and size | `my_app.2026-11-25-17.1.log`    |
//!
//! * `filename` is treated as a literal stem. Configure an extension via
//!   [`FileBuilder::filename_suffix`] so period and sequence fields appear before the extension.
//! * [`FileBuilder::timezone`] selects the timezone used for rotation boundaries and period fields
//!   (default: [`TimeZone::system()`](jiff::tz::TimeZone::system)).
//! * Segment files keep their initial name for their entire lifetime and are never renamed on
//!   rollover.
//! * [`FileBuilder::rollover_size`] numbers segments in ascending order starting from `1` (oldest)
//!   within each period. On startup, [`FileBuilder::build`] resumes the highest-numbered segment in
//!   the active period if its size is below the limit, or opens the next sequence number.
//!
//! # Retention and error handling
//!
//! * [`FileBuilder::max_log_files`] limits the total number of matching regular files across all
//!   periods, including the active segment. Non-matching files, directories, symlinks, and the
//!   active segment itself are never removed.
//! * If opening the next segment or pruning old segments fails at runtime, [`File`] records a
//!   `1`-second retry cooldown, appends the current record to the active segment, and returns the
//!   rollover or cleanup [`Error`](logforth_core::Error).
//!
//! # Examples
//!
//! ```
//! use std::num::NonZeroU64;
//! use std::num::NonZeroUsize;
//!
//! use logforth_append_file::FileBuilder;
//! use logforth_append_file::jiff::tz::TimeZone;
//! use logforth_core::record::LevelFilter;
//! use logforth_layout_json::JsonLayout;
//!
//! # let temp_dir = tempfile::tempdir()?;
//! # let log_dir = temp_dir.path();
//! let file = FileBuilder::new(log_dir, "my_app")
//!     .filename_suffix("log")
//!     .layout(JsonLayout::default())
//!     .rollover_daily()
//!     .timezone(TimeZone::UTC)
//!     .rollover_size(NonZeroU64::new(10 * 1024 * 1024).unwrap())
//!     .max_log_files(NonZeroUsize::new(7).unwrap())
//!     .build()?;
//!
//! let logger = logforth_core::builder()
//!     .dispatch(|d| d.filter(LevelFilter::All).append(file))
//!     .build();
//! # drop(logger);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub extern crate jiff;

mod append;
pub use self::append::File;
pub use self::append::FileBuilder;

mod clock;
mod rolling;
mod rotation;
