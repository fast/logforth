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

use time::OffsetDateTime;

#[derive(Debug)]
pub enum Clock {
    DefaultClock,
    #[cfg(test)]
    ManualClock(ManualClock),
}

impl Clock {
    pub fn now(&self) -> OffsetDateTime {
        match self {
            Clock::DefaultClock => OffsetDateTime::now_utc(),
            #[cfg(test)]
            Clock::ManualClock(clock) => clock.now(),
        }
    }

    #[cfg(test)]
    pub fn set_now(&mut self, new_time: OffsetDateTime) {
        if let Clock::ManualClock(clock) = self {
            clock.set_now(new_time);
        }
    }
}

/// The time could be reset.
#[derive(Debug)]
#[cfg(test)]
pub struct ManualClock {
    fixed_time: OffsetDateTime,
}

#[cfg(test)]
impl ManualClock {
    pub fn new(fixed_time: OffsetDateTime) -> ManualClock {
        ManualClock { fixed_time }
    }

    fn now(&self) -> OffsetDateTime {
        self.fixed_time
    }

    pub fn set_now(&mut self, new_time: OffsetDateTime) {
        self.fixed_time = new_time;
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_manual_clock_adjusting() {
        let mut clock = ManualClock {
            fixed_time: datetime!(2023-01-01 12:00:00 UTC),
        };
        assert_eq!(clock.now(), datetime!(2023-01-01 12:00:00 UTC));

        clock.set_now(datetime!(2024-01-01 12:00:00 UTC));
        assert_eq!(clock.now(), datetime!(2024-01-01 12:00:00 UTC));
    }
}
