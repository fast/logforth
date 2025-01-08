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
use logforth::filter::CustomFilter;
use logforth::filter::FilterResult;
use logforth::layout::CustomLayout;

fn main() {
    logforth::builder()
        .dispatch(|d| {
            d.filter(CustomFilter::new(|metadata| {
                if metadata.level() < log::Level::Info {
                    FilterResult::Accept
                } else {
                    FilterResult::Reject
                }
            }))
            .append(
                append::Stdout::default().with_layout(CustomLayout::new(|record, _| {
                    Ok(format!("[Alert] {}", record.args()).into_bytes())
                })),
            )
        })
        .apply();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
}
