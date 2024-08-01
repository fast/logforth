// Copyright 2024 tison <wander4096@gmail.com>
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

use log::LevelFilter;
use logforth::append;
use logforth::filter;
use logforth::layout;
use logforth::logger::Dispatch;
use logforth::logger::Logger;

fn main() {
    Logger::new()
        .dispatch(
            Dispatch::new()
                .filter(LevelFilter::Trace)
                .layout(layout::Text::default())
                .append(append::Stdout),
        )
        .apply()
        .unwrap();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
}
