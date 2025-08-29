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

use crate::time::{RoundMode, Unit, Zoned, ZonedRound, minute, hour, day};

/// Rotation policies for rolling files.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Rotation {
    /// Rotate files every minute.
    Minutely,
    /// Rotate files every hour.
    Hourly,
    /// Rotate files every day.
    Daily,
    /// Never rotate files.
    Never,
}

impl Rotation {
    /// Get the next date timestamp based on the current date and rotation policy.
    pub fn next_date_timestamp(&self, current_date: &Zoned) -> Option<usize> {
        let timestamp_round = ZonedRound::new().mode(RoundMode::Trunc);

        let next_date = match *self {
            Rotation::Never => return None,
            Rotation::Minutely => {
                (current_date.add_span(&minute())).round(timestamp_round.smallest(Unit::Minute))
            }
            Rotation::Hourly => {
                (current_date.add_span(&hour())).round(timestamp_round.smallest(Unit::Hour))
            }
            Rotation::Daily => (current_date.add_span(&day())).round(timestamp_round.smallest(Unit::Day)),
        };
        let next_date =
            next_date.expect("invalid time; this is a bug in logforth rolling file appender");
        Some(next_date.timestamp().as_millisecond() as usize)
    }

    /// Get the date format string for the rotation policy.
    pub fn date_format(&self) -> &'static str {
        match *self {
            Rotation::Minutely => "%F-%H-%M",
            Rotation::Hourly => "%F-%H",
            Rotation::Daily => "%F",
            Rotation::Never => "%F",
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "jiff")]
    use crate::time::{Timestamp, Zoned};

    #[allow(unused_imports)]
    use super::Rotation;

    #[test]
    #[cfg(feature = "jiff")] // TODO: Fix chrono implementation to match jiff's precise rounding
    fn test_next_date_timestamp() {
        #[cfg(feature = "jiff")]
        let current_date = Zoned::from_str("2024-08-10T17:12:52+08[+08]").unwrap();
        #[cfg(not(feature = "jiff"))]
        let current_date = Zoned::from_str("2024-08-10T17:12:52+08:00").unwrap();

        assert_eq!(Rotation::Never.next_date_timestamp(&current_date), None);

        #[cfg(feature = "jiff")]
        let _expected_date = Timestamp::from("2024-08-10T17:13:00+08");
        #[cfg(not(feature = "jiff"))]
        let _expected_date = Timestamp::from("2024-08-10T17:13:00+08:00");
        assert_eq!(
            Rotation::Minutely.next_date_timestamp(&current_date),
            Some(_expected_date.as_millisecond() as usize)
        );

        #[cfg(feature = "jiff")]
        let _expected_date = Timestamp::from("2024-08-10T18:00:00+08");
        #[cfg(not(feature = "jiff"))]
        let _expected_date = Timestamp::from("2024-08-10T18:00:00+08:00");
        assert_eq!(
            Rotation::Hourly.next_date_timestamp(&current_date),
            Some(_expected_date.as_millisecond() as usize)
        );

        #[cfg(feature = "jiff")]
        let _expected_date = Timestamp::from("2024-08-11T00:00:00+08");
        #[cfg(not(feature = "jiff"))]
        let _expected_date = Timestamp::from("2024-08-11T00:00:00+08:00");
        assert_eq!(
            Rotation::Daily.next_date_timestamp(&current_date),
            Some(_expected_date.as_millisecond() as usize)
        );
    }
}
