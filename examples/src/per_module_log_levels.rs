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

fn main() {
    // This is how you can allow trace level logs for everything else while silencing them
    // for the ones you probably don't need (in this case various rerun modules).
    let my_filter = RustLogFilterBuilder::from_default_env()
        .filter_level(LevelFilter::MoreSevereEqual(Level::Trace))
        .filter_module("rerun", LevelFilter::MoreSevereEqual(Level::Warn))
        .filter_module("re_chunk", LevelFilter::MoreSevereEqual(Level::Warn))
        .filter_module("re_log", LevelFilter::MoreSevereEqual(Level::Warn))
        .filter_module("re_log_encoding", LevelFilter::MoreSevereEqual(Level::Warn))
        .filter_module("re_sdk", LevelFilter::MoreSevereEqual(Level::Warn))
        .filter_module("re_sorbet", LevelFilter::MoreSevereEqual(Level::Warn))
        .filter_module("tracing", LevelFilter::MoreSevereEqual(Level::Warn))
        .build();

    logforth::starter_log::builder()
        .dispatch(|d| d.filter(my_filter).append(append::Stdout::default()))
        .apply();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
}
