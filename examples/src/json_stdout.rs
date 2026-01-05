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
use logforth::layout::JsonLayout;

fn main() {
    logforth::starter_log::builder()
        .dispatch(|d| d.append(append::Stdout::default().with_layout(JsonLayout::default())))
        .apply();

    log::info!("This is an info message.");
    log::debug!("This debug message will not be printed by default.");
}
