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

fn main() {
    log::set_max_level(LevelFilter::Trace);
    let l = logforth::stdout().build();

    log::error!(logger: l, "Hello error!");
    log::warn!(logger: l, "Hello warn!");
    log::info!(logger: l, "Hello info!");
    log::debug!(logger: l, "Hello debug!");
    log::trace!(logger: l, "Hello trace!");
}
