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

use jiff::Timestamp;

#[derive(Debug)]
pub enum Clock {
    DefaultClock,
    #[cfg(test)]
    ManualClock(Timestamp),
}

impl Clock {
    pub fn now(&self) -> Timestamp {
        match self {
            Clock::DefaultClock => Timestamp::now(),
            #[cfg(test)]
            Clock::ManualClock(now) => *now,
        }
    }
}
