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

use std::ops::Range;

use jiff::SignedDuration;
use jiff::Timestamp;
use jiff::ToSpan;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use logforth_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rotation {
    Minutely,
    Hourly,
    Daily,
    Never,
}

#[derive(Debug)]
pub(crate) struct Period {
    pub(crate) key: DateTime,
    pub(crate) bounds: Range<Timestamp>,
}

impl Rotation {
    pub(crate) fn period_at(self, now: Timestamp, tz: &TimeZone) -> Result<Option<Period>, Error> {
        let local = tz.to_datetime(now);
        let (key, span) = match self {
            Self::Never => return Ok(None),
            Self::Daily => (local.start_of_day(), 1.day()),
            Self::Hourly => (local.date().at(local.hour(), 0, 0, 0), 1.hour()),
            Self::Minutely => (
                local.date().at(local.hour(), local.minute(), 0, 0),
                1.minute(),
            ),
        };
        if key.year() < 0 {
            return Err(Error::new("log file periods require a four-digit year"));
        }
        let bounds = key
            .checked_add(span)
            .and_then(|end| self.bounds(now, tz, key, end))
            .map_err(|err| {
                Error::new("failed to determine log rotation period").with_source(err)
            })?;
        Ok(Some(Period { key, bounds }))
    }

    fn bounds(
        self,
        now: Timestamp,
        tz: &TimeZone,
        key: DateTime,
        next: DateTime,
    ) -> Result<Range<Timestamp>, jiff::Error> {
        let offset = tz.to_offset(now);
        let mut start = offset.to_timestamp(key)?;
        let mut end = offset.to_timestamp(next)?;

        // A repeated hour or minute has two occurrences. A day spans offset changes
        // as long as the clock stays on the same date on both sides of the transition.
        let tick = SignedDuration::from_nanos(1);
        for transition in tz.preceding(now.checked_add(tick)?) {
            let at = transition.timestamp();
            if at < start {
                break;
            }
            let before = at.checked_sub(tick)?;
            if self != Self::Daily || tz.to_datetime(before).date() != key.date() {
                start = at;
                break;
            }
            start = tz.to_offset(before).to_timestamp(key)?;
        }
        for transition in tz.following(now) {
            let at = transition.timestamp();
            if at > end {
                break;
            }
            let after = transition.offset();
            if self != Self::Daily || after.to_datetime(at).date() != key.date() {
                end = at;
                break;
            }
            end = after.to_timestamp(next)?;
        }
        Ok(start..end)
    }

    pub(crate) fn date_format(self) -> &'static str {
        match self {
            Self::Minutely => "%F-%H-%M",
            Self::Hourly => "%F-%H",
            Self::Daily | Self::Never => "%F",
        }
    }

    pub(crate) fn parse_period(self, text: &str) -> Option<DateTime> {
        let len = match self {
            Self::Never => return None,
            Self::Daily => 10,
            Self::Hourly => 13,
            Self::Minutely => 16,
        };
        if text.len() != len
            || !text.bytes().enumerate().all(|(i, b)| match i {
                4 | 7 | 10 | 13 => b == b'-',
                _ => b.is_ascii_digit(),
            })
        {
            return None;
        }
        DateTime::strptime(self.date_format(), text).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_calendar_day_spans_the_offset_change() {
        let tz = TimeZone::get("America/New_York").unwrap();
        for (before, after, start, end) in [
            (
                "2026-03-08T01:59:59-05",
                "2026-03-08T03:00:00-04",
                "2026-03-08T00:00:00-05",
                "2026-03-09T00:00:00-04",
            ),
            (
                "2026-11-01T01:59:59-04",
                "2026-11-01T01:00:00-05",
                "2026-11-01T00:00:00-04",
                "2026-11-02T00:00:00-05",
            ),
        ] {
            let expected = start.parse().unwrap()..end.parse().unwrap();
            for now in [before, after] {
                let period = Rotation::Daily
                    .period_at(now.parse().unwrap(), &tz)
                    .unwrap()
                    .unwrap();
                assert_eq!(period.bounds, expected);
            }
        }
    }

    #[test]
    fn repeated_hour_can_start_at_half_past() {
        let tz = TimeZone::get("Australia/Lord_Howe").unwrap();
        let now = "2026-04-05T01:45:00+10:30".parse().unwrap();
        let period = Rotation::Hourly.period_at(now, &tz).unwrap().unwrap();
        assert_eq!(
            period.bounds,
            "2026-04-05T01:30:00+10:30".parse().unwrap()
                .."2026-04-05T02:00:00+10:30".parse().unwrap()
        );
        assert_eq!(
            period.key,
            "2026-04-05T01:00:00".parse::<DateTime>().unwrap()
        );
    }
}
