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

use log::LevelFilter;
use logforth::append;
use logforth::filter::CustomFilter;
use logforth::filter::FilterResult;
use logforth::layout::CustomLayout;
use logforth::Dispatch;
use logforth::Logger;

fn main() {
    Logger::new()
        .dispatch(
            Dispatch::new()
                .filter(CustomFilter::new(|metadata: &log::Metadata| {
                    if metadata.level() > LevelFilter::Info {
                        FilterResult::Accept
                    } else {
                        FilterResult::Reject
                    }
                }))
                .append(append::Stdout::new(CustomLayout::new(|record| {
                    Ok(format!("[system alert] {}", record.args()))
                }))),
        )
        .apply()
        .unwrap();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
}
