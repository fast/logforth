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

use logforth::append::syslog::SyslogBuilder;
use logforth::record::LevelFilter;

fn main() {
    logforth::bridge::setup_log_crate();

    let append = SyslogBuilder::tcp_well_known().unwrap().build();
    logforth::builder()
        .dispatch(|d| d.filter(LevelFilter::Trace).append(append))
        .apply();

    let repeat = 1;

    for i in 0..repeat {
        log::error!("Hello syslog error!");
        log::warn!("Hello syslog warn!");
        log::info!("Hello syslog info!");
        log::debug!("Hello syslog debug!");
        log::trace!("Hello syslog trace!");

        if i + 1 < repeat {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
