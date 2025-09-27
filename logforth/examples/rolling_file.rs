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

//! An example of logging to rolling files in JSON format.

use logforth::append::file::FileBuilder;
use logforth::layout::JsonLayout;
use logforth::record::LevelFilter;

fn main() {
    logforth::bridge::setup_log_crate();

    let rolling = FileBuilder::new("logs", "my_app")
        .layout(JsonLayout::default())
        .rollover_daily()
        .build()
        .unwrap();

    logforth::builder()
        .dispatch(|d| d.filter(LevelFilter::Trace).append(rolling))
        .apply();

    let repeat = 1;

    for i in 0..repeat {
        log::error!("Hello error!");
        log::warn!("Hello warn!");
        log::info!("Hello info!");
        log::debug!("Hello debug!");
        log::trace!("Hello trace!");

        if i + 1 < repeat {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
