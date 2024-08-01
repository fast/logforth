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
use logforth::BoxDynFilter;
use logforth::BoxDynLayout;
use logforth::DispatchAppend;
use logforth::FilterResult;
use logforth::Logger;
use logforth::StdoutAppend;

fn main() {
    let layout = BoxDynLayout::new(|record: &log::Record| {
        let message = format!("[box dyn] {}", record.args());
        Ok(message.into_bytes())
        // ...or
        // anyhow::bail!("boom: {}", message)
    });

    let filter = BoxDynFilter::new(|metadata: &log::Metadata| {
        if metadata.level() <= LevelFilter::Info {
            FilterResult::Accept
        } else {
            FilterResult::Reject
        }
    });

    let append = StdoutAppend::default().with_layout(layout);
    let append = DispatchAppend::new(append).filter(filter);
    Logger::new().add_append(append).apply().unwrap();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
}
