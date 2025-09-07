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

use logforth::append::rolling_file::RollingFileBuilder;
use logforth::append::rolling_file::Rotation;
use logforth::layout::JsonLayout;

fn main() {
    let rolling_writer = RollingFileBuilder::new("logs", "my_app")
        .filename_suffix("log")
        .layout(JsonLayout::default())
        .rotation(Rotation::Never)
        .build()
        .unwrap();

    logforth::builder()
        .dispatch(|d| d.filter(log::LevelFilter::Trace).append(rolling_writer))
        .apply();

    let repeat = 1;

    for i in 0..repeat {
        log::error!("Hello single error!");
        log::warn!("Hello single warn!");
        log::info!("Hello single info!");
        log::debug!("Hello single debug!");
        log::trace!("Hello single trace!");

        if i + 1 < repeat {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
