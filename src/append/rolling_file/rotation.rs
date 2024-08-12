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

use jiff::RoundMode;
use jiff::ToSpan;
use jiff::Unit;
use jiff::Zoned;
use jiff::ZonedRound;

/// Defines a fixed period for rolling of a log file.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Rotation {
    /// Minutely Rotation
    Minutely,
    /// Hourly Rotation
    Hourly,
    /// Daily Rotation
    Daily,
    /// No Time Rotation
    Never,
}

impl Rotation {
    pub fn next_date_timestamp(&self, current_date: &Zoned) -> Option<usize> {
        let timestamp_round = ZonedRound::new().mode(RoundMode::Trunc);

        let next_date = match *self {
            Rotation::Never => return None,
            Rotation::Minutely => {
                (current_date + 1.minute()).round(timestamp_round.smallest(Unit::Minute))
            }
            Rotation::Hourly => {
                (current_date + 1.hour()).round(timestamp_round.smallest(Unit::Hour))
            }
            Rotation::Daily => (current_date + 1.day()).round(timestamp_round.smallest(Unit::Day)),
        };
        let next_date =
            next_date.expect("invalid time; this is a bug in logforth rolling file appender");
        Some(next_date.timestamp().as_millisecond() as usize)
    }

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
    use std::str::FromStr;

    use jiff::Timestamp;
    use jiff::Zoned;

    use super::Rotation;

    #[test]
    fn test_next_date_timestamp() {
        let current_date = Zoned::from_str("2024-08-10T17:12:52+08[+08]").unwrap();

        assert_eq!(Rotation::Never.next_date_timestamp(&current_date), None);

        let expected_date = "2024-08-10T17:13:00+08".parse::<Timestamp>().unwrap();
        assert_eq!(
            Rotation::Minutely.next_date_timestamp(&current_date),
            Some(expected_date.as_millisecond() as usize)
        );

        let expected_date = "2024-08-10T18:00:00+08".parse::<Timestamp>().unwrap();
        assert_eq!(
            Rotation::Hourly.next_date_timestamp(&current_date),
            Some(expected_date.as_millisecond() as usize)
        );

        let expected_date = "2024-08-11T00:00:00+08".parse::<Timestamp>().unwrap();
        assert_eq!(
            Rotation::Daily.next_date_timestamp(&current_date),
            Some(expected_date.as_millisecond() as usize)
        );
    }
}
