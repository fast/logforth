// Copyright 2024 CratesLand Developers
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
    ManualClock(ManualClock),
}

impl Clock {
    pub fn now(&self) -> Timestamp {
        match self {
            Clock::DefaultClock => Timestamp::now(),
            #[cfg(test)]
            Clock::ManualClock(clock) => clock.now(),
        }
    }

    #[cfg(test)]
    pub fn set_now(&mut self, new_time: Timestamp) {
        if let Clock::ManualClock(clock) = self {
            clock.set_now(new_time);
        }
    }
}

/// The time could be reset.
#[derive(Debug)]
#[cfg(test)]
pub struct ManualClock {
    now: Timestamp,
}

#[cfg(test)]
impl ManualClock {
    pub fn new(now: Timestamp) -> ManualClock {
        ManualClock { now }
    }

    fn now(&self) -> Timestamp {
        self.now
    }

    pub fn set_now(&mut self, now: Timestamp) {
        self.now = now;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_manual_clock_adjusting() {
        let now = Timestamp::from_str("2023-01-01T12:00:00Z").unwrap();
        let mut clock = ManualClock { now };
        assert_eq!(clock.now(), now);

        let now = Timestamp::from_str("2024-01-01T12:00:00Z").unwrap();
        clock.set_now(now);
        assert_eq!(clock.now(), now);
    }
}
