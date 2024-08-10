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

use time::format_description;
use time::Duration;
use time::OffsetDateTime;
use time::Time;

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
    pub fn next_date_timestamp(&self, current_date: &OffsetDateTime) -> Option<usize> {
        let next_date = match *self {
            Rotation::Minutely => *current_date + Duration::minutes(1),
            Rotation::Hourly => *current_date + Duration::hours(1),
            Rotation::Daily => *current_date + Duration::days(1),
            Rotation::Never => return None,
        };

        Some(self.round_date(&next_date).unix_timestamp() as usize)
    }

    fn round_date(&self, date: &OffsetDateTime) -> OffsetDateTime {
        match *self {
            Rotation::Minutely => {
                let time = Time::from_hms(date.hour(), date.minute(), 0)
                    .expect("invalid time; this is a bug in logforth rolling file appender");
                date.replace_time(time)
            }
            Rotation::Hourly => {
                let time = Time::from_hms(date.hour(), 0, 0)
                    .expect("invalid time; this is a bug in logforth rolling file appender");
                date.replace_time(time)
            }
            Rotation::Daily => {
                let time = Time::from_hms(0, 0, 0)
                    .expect("invalid time; this is a bug in logforth rolling file appender");
                date.replace_time(time)
            }
            Rotation::Never => unreachable!("Rotation::Never is impossible to round."),
        }
    }

    pub fn date_format(&self) -> Vec<format_description::FormatItem<'static>> {
        match *self {
            Rotation::Minutely => format_description::parse("[year]-[month]-[day]-[hour]-[minute]"),
            Rotation::Hourly => format_description::parse("[year]-[month]-[day]-[hour]"),
            Rotation::Daily => format_description::parse("[year]-[month]-[day]"),
            Rotation::Never => format_description::parse("[year]-[month]-[day]"),
        }
        .expect("failed to create a formatter; this is a bug in logforth rolling file appender")
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::Rotation;

    #[test]
    fn test_next_date_timestamp() {
        let current_date = datetime!(2024-08-10 17:12:52 +8);

        assert_eq!(
            Rotation::Minutely.next_date_timestamp(&current_date),
            Some(datetime!(2024-08-10 17:13:00 +8).unix_timestamp() as usize)
        );
        assert_eq!(
            Rotation::Hourly.next_date_timestamp(&current_date),
            Some(datetime!(2024-08-10 18:00:00 +8).unix_timestamp() as usize)
        );
        assert_eq!(
            Rotation::Daily.next_date_timestamp(&current_date),
            Some(datetime!(2024-08-11 00:00:00 +8).unix_timestamp() as usize)
        );
        assert_eq!(Rotation::Never.next_date_timestamp(&current_date), None);
    }
}
