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

use jiff::Zoned;

#[derive(Debug)]
pub enum Clock {
    DefaultClock,
    #[cfg(test)]
    ManualClock(ManualClock),
}

impl Clock {
    pub fn now(&self) -> Zoned {
        match self {
            Clock::DefaultClock => Zoned::now(),
            #[cfg(test)]
            Clock::ManualClock(clock) => clock.now(),
        }
    }

    #[cfg(test)]
    pub fn set_now(&mut self, now: Zoned) {
        if let Clock::ManualClock(clock) = self {
            clock.set_now(now);
        }
    }
}

/// The time could be reset.
#[derive(Debug)]
#[cfg(test)]
pub struct ManualClock {
    now: Zoned,
}

#[cfg(test)]
impl ManualClock {
    pub fn new(now: Zoned) -> ManualClock {
        ManualClock { now }
    }

    fn now(&self) -> Zoned {
        self.now.clone()
    }

    pub fn set_now(&mut self, now: Zoned) {
        self.now = now;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_manual_clock_adjusting() {
        let now = Zoned::from_str("2024-08-10T17:12:52+08[+08]").unwrap();
        let mut clock = ManualClock { now: now.clone() };
        assert_eq!(clock.now(), now);

        let now = Zoned::from_str("2024-01-01T12:00:00+08[+08]").unwrap();
        clock.set_now(now.clone());
        assert_eq!(clock.now(), now);
    }
}
