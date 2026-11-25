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

//! Append log records directly to a single file or to rotating segments.
//!
//! With `FileBuilder::new(dir, "my_app").filename_suffix("log")`, the filename depends on the
//! rotation policy.
//!
//! | Policy          | Example filename             |
//! | :-------------- | :--------------------------- |
//! | No rotation     | `my_app.log`                 |
//! | Daily           | `my_app.2026-11-25.log`      |
//! | Size            | `my_app.1.log`               |
//! | Hourly and size | `my_app.2026-11-25-17.1.log` |
//!
//! Hourly periods include the hour and minutely periods also include the minute, both with two
//! digits. [`FileBuilder::timezone`] selects the timezone for these periods and defaults to the
//! system timezone. The filename is a literal stem. Passing `"my_app.log"` as the stem puts
//! rotation fields after `.log`.
//!
//! [`FileBuilder::rollover_size`] numbers segments from 1 within each period. Files keep the same
//! name throughout their lifetime. On startup, the highest numbered segment in the current period
//! is reused if it has room. [`FileBuilder::max_log_files`] limits the total number of matching
//! files across all periods, including the active file.
//!
//! # Example
//!
//! ```
//! use std::num::NonZeroU64;
//! use std::num::NonZeroUsize;
//!
//! use jiff::tz::TimeZone;
//! use logforth_append_file::FileBuilder;
//! use logforth_core::record::LevelFilter;
//! use logforth_layout_json::JsonLayout;
//!
//! # fn configure(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
//! let rolling = FileBuilder::new(dir, "my_app")
//!     .filename_suffix("log")
//!     .layout(JsonLayout::default())
//!     .rollover_daily()
//!     .timezone(TimeZone::UTC)
//!     .rollover_size(NonZeroU64::try_from(1024 * 1024)?)
//!     .max_log_files(NonZeroUsize::try_from(7)?)
//!     .build()?;
//!
//! let logger = logforth_core::builder()
//!     .dispatch(|d| d.filter(LevelFilter::All).append(rolling))
//!     .build();
//! # Ok(())
//! # }
//! # configure(tempfile::tempdir()?.path())?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub extern crate jiff;

pub use self::append::File;
pub use self::append::FileBuilder;

mod append;
mod clock;
mod rolling;
mod rotation;
