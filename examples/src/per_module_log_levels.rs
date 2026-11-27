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

use logforth::append;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::record::Level;
use logforth::record::LevelFilter;
use logforth::starter_log::LogStarterExt;

mod noisy {
    pub fn run() {
        log::warn!("Hello noisy warn!");
        log::info!("Hello noisy info (filtered out)!");
    }
}

fn main() {
    // Allow trace logs by default while narrowing noisy prefixes to warn.
    let builder = RustLogFilterBuilder::from_default_env().unwrap();
    let my_filter = builder
        .fallback(LevelFilter::MoreSevereEqual(Level::Trace))
        .prefix(
            concat!(module_path!(), "::noisy"),
            LevelFilter::MoreSevereEqual(Level::Warn),
        )
        .build();

    logforth::builder()
        .dispatch(|d| d.filter(my_filter).append(append::Stdout::default()))
        .install();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
    noisy::run();
}
