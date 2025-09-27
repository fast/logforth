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

use crate::LoggerBuilder;
use crate::append;
use crate::builder;
use crate::filter::env_filter::EnvFilterBuilder;
#[cfg(feature = "layout-text")]
use crate::layout::TextLayout;

/// Create a [`LoggerBuilder`] with a default [`append::Stdout`] appender and an [`EnvFilter`]
/// respecting `RUST_LOG`.
///
/// [`EnvFilter`]: crate::filter::EnvFilter
///
/// # Examples
///
/// ```
/// logforth::stdout().apply();
/// log::error!("This error will be logged to stdout.");
/// ```
pub fn stdout() -> LoggerBuilder {
    let filter = EnvFilterBuilder::from_default_env().build();

    #[cfg(feature = "layout-text")]
    let append = append::Stdout::default().with_layout(TextLayout::default());

    #[cfg(not(feature = "layout-text"))]
    let append = append::Stdout::default();

    builder().dispatch(|d| d.filter(filter).append(append))
}

/// Create a [`LoggerBuilder`] with a default [`append::Stderr`] appender and an [`EnvFilter`]
/// respecting `RUST_LOG`.
///
/// [`EnvFilter`]: crate::filter::EnvFilter
///
/// # Examples
///
/// ```
/// logforth::stderr().apply();
/// log::error!("This info will be logged to stderr.");
/// ```
pub fn stderr() -> LoggerBuilder {
    let filter = EnvFilterBuilder::from_default_env().build();

    #[cfg(feature = "layout-text")]
    let append = append::Stderr::default().with_layout(TextLayout::default());

    #[cfg(not(feature = "layout-text"))]
    let append = append::Stderr::default();

    builder().dispatch(|d| d.filter(filter).append(append))
}
