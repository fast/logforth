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

//! An example of logging to a single file with async combinator.

use logforth::append::file::FileBuilder;
use logforth::layout::JsonLayout;
use logforth::record::LevelFilter;
use logforth_append_async::AsyncBuilder;

fn main() {
    let file = FileBuilder::new("logs", "my_app_async")
        .filename_suffix("log")
        .layout(JsonLayout::default())
        .build()
        .unwrap();

    let asynchronous = AsyncBuilder::new("logforth-async").append(file).build();

    logforth::starter_log::builder()
        .dispatch(|d| d.filter(LevelFilter::All).append(asynchronous))
        .apply();

    log::error!("Hello single error!");
    log::warn!("Hello single warn!");
    log::info!("Hello single info!");
    log::debug!("Hello single debug!");
    log::trace!("Hello single trace!");

    // ensure all async events buffered are written out
    logforth::core::default_logger().flush();
}
