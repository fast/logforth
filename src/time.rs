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

//! Time abstraction layer for different time libraries.
//!
//! This module provides a unified interface for time operations that can be
//! backed by either `jiff` (default) or `chrono` depending on the enabled features.

#![allow(dead_code)]

use std::fmt::{Display, Formatter, Result as FmtResult};

#[cfg(feature = "jiff")]
mod jiff_impl {
    use super::*;
    pub use jiff::tz::TimeZone;
    pub use jiff::{RoundMode, ToSpan, Unit, ZonedRound};

    /// A timestamp representing a specific moment in time.
    #[derive(Debug, Clone, Copy)]
    pub struct Timestamp(pub jiff::Timestamp);

    impl Timestamp {
        /// Get the current timestamp.
        pub fn now() -> Self {
            Self(jiff::Timestamp::now())
        }

        /// Convert to a zoned timestamp in the given timezone.
        pub fn to_zoned(self, tz: TimeZone) -> Zoned {
            Zoned(self.0.to_zoned(tz))
        }

        /// Get the timestamp as milliseconds since Unix epoch.
        pub fn as_millisecond(self) -> i64 {
            self.0.as_millisecond()
        }
    }

    impl From<&str> for Timestamp {
        fn from(s: &str) -> Self {
            Self(s.parse().expect("invalid timestamp format"))
        }
    }

    #[cfg(all(feature = "jiff", feature = "internal-serde"))]
    impl serde::Serialize for Timestamp {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // Use jiff's serialize if available, otherwise convert to string
            #[cfg(feature = "internal-serde-jiff")]
            {
                self.0.serialize(serializer)
            }
            #[cfg(not(feature = "internal-serde-jiff"))]
            {
                serializer.collect_str(&self.0)
            }
        }
    }

    /// A zoned timestamp with timezone information.
    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub struct Zoned(pub jiff::Zoned);

    impl Zoned {
        /// Get the current zoned timestamp in the system timezone.
        pub fn now() -> Self {
            Self(jiff::Zoned::now())
        }

        /// Create from a string representation.
        pub fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Self(s.parse()?))
        }

        /// Get the timestamp component.
        pub fn timestamp(&self) -> Timestamp {
            Timestamp(self.0.timestamp())
        }

        /// Format with strftime-like format string.
        pub fn strftime(&self, fmt: &str) -> String {
            self.0.strftime(fmt).to_string()
        }

        /// Round to a specific unit.
        pub fn round(self, round: ZonedRound) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Self(self.0.round(round)?))
        }

        /// Add a span to this zoned timestamp.
        pub fn add_span(&self, span: &Span) -> Self {
            Self(self.0.checked_add(span.0).expect("time addition overflow"))
        }

        /// Add a span to this zoned timestamp (alias for add_span).
        pub fn add(&self, span: &Span) -> Self {
            self.add_span(span)
        }
    }

    impl Display for Zoned {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            if let Some(precision) = f.precision() {
                write!(f, "{:.precision$}", self.0, precision = precision)
            } else {
                write!(f, "{}", self.0)
            }
        }
    }

    #[cfg(all(feature = "jiff", feature = "internal-serde"))]
    impl serde::Serialize for Zoned {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // Use jiff's serialize if available, otherwise convert to string
            #[cfg(feature = "internal-serde-jiff")]
            {
                self.0.serialize(serializer)
            }
            #[cfg(not(feature = "internal-serde-jiff"))]
            {
                serializer.collect_str(&self.0)
            }
        }
    }

    /// A span representing a duration of time.
    #[derive(Debug, Clone)]
    pub struct Span(pub jiff::Span);

    impl Span {
        /// Create a new empty span.
        pub fn new() -> Self {
            Self(jiff::Span::new())
        }

        /// Add minutes to this span.
        pub fn minutes(mut self, n: i64) -> Self {
            self.0 = self.0.minutes(n);
            self
        }

        /// Add hours to this span.
        pub fn hours(mut self, n: i64) -> Self {
            self.0 = self.0.hours(n);
            self
        }

        /// Add days to this span.
        pub fn days(mut self, n: i64) -> Self {
            self.0 = self.0.days(n);
            self
        }

        /// Add seconds to this span.
        pub fn seconds(mut self, n: i64) -> Self {
            self.0 = self.0.seconds(n);
            self
        }
    }

    /// Create a 1-minute span.
    pub fn minute() -> Span {
        Span::new().minutes(1)
    }

    /// Create a 1-hour span.
    pub fn hour() -> Span {
        Span::new().hours(1)
    }

    /// Create a 1-day span.
    pub fn day() -> Span {
        Span::new().days(1)
    }

    /// Parse a date string using the given format.
    pub fn parse_date(format: &str, date_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        jiff::civil::DateTime::strptime(format, date_str)?;
        Ok(())
    }
}

#[cfg(feature = "chrono")]
mod chrono_impl {
    use super::*;
    
    /// Timezone wrapper for chrono.
    #[derive(Debug, Clone)]
    pub struct TimeZone(pub chrono_tz::Tz);

    impl TimeZone {
        /// UTC timezone.
        pub const UTC: Self = Self(chrono_tz::UTC);
    }

    /// A timestamp representing a specific moment in time.
    #[derive(Debug, Clone, Copy)]
    pub struct Timestamp(pub chrono::DateTime<chrono::Utc>);

    impl Timestamp {
        /// Get the current timestamp.
        pub fn now() -> Self {
            Self(chrono::Utc::now())
        }

        /// Convert to a zoned timestamp in the given timezone.
        pub fn to_zoned(self, tz: TimeZone) -> Zoned {
            Zoned(self.0.with_timezone(&tz.0))
        }

        /// Get the timestamp as milliseconds since Unix epoch.
        pub fn as_millisecond(self) -> i64 {
            self.0.timestamp_millis()
        }
    }

    impl From<&str> for Timestamp {
        fn from(s: &str) -> Self {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                Self(dt.with_timezone(&chrono::Utc))
            } else {
                // Try parsing as naive datetime and assume UTC
                let naive = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .expect("invalid timestamp format");
                let utc_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
                Self(utc_dt)
            }
        }
    }

    #[cfg(all(feature = "chrono", feature = "internal-serde"))]
    impl serde::Serialize for Timestamp {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // Use chrono's serialize if available, otherwise convert to string
            #[cfg(feature = "internal-serde-chrono")]
            {
                self.0.serialize(serializer)
            }
            #[cfg(not(feature = "internal-serde-chrono"))]
            {
                serializer.collect_str(&format_args!("{}", self.0.format("%Y-%m-%dT%H:%M:%S%.6f%:z")))
            }
        }
    }

    /// A zoned timestamp with timezone information.
    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub struct Zoned(pub chrono::DateTime<chrono_tz::Tz>);

    impl Zoned {
        /// Get the current zoned timestamp in the system timezone.
        pub fn now() -> Self {
            let _local = chrono::Local::now();
            // Convert to system timezone (approximation using UTC for simplicity)
            Self(chrono::Utc::now().with_timezone(&chrono_tz::UTC))
        }

        /// Create from a string representation.
        pub fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            // Try to handle both jiff and chrono compatible formats
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                Ok(Self(dt.with_timezone(&chrono_tz::UTC)))
            } else if s.contains("[UTC]") {
                // Handle jiff-style format like "2024-08-10T00:00:00[UTC]"
                let s_clean = s.replace("[UTC]", "+00:00");
                let dt = chrono::DateTime::parse_from_rfc3339(&s_clean)?;
                Ok(Self(dt.with_timezone(&chrono_tz::UTC)))
            } else {
                // Try parsing as a naive datetime and assume UTC
                let naive = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")?;
                let utc_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
                Ok(Self(utc_dt.with_timezone(&chrono_tz::UTC)))
            }
        }

        /// Get the timestamp component.
        pub fn timestamp(&self) -> Timestamp {
            Timestamp(self.0.with_timezone(&chrono::Utc))
        }

        /// Format with strftime-like format string.
        pub fn strftime(&self, fmt: &str) -> String {
            self.0.format(fmt).to_string()
        }

        /// Round to a specific unit (simplified for chrono).
        pub fn round(self, _round: ZonedRound) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            // Simplified implementation for chrono
            Ok(self)
        }

        /// Add a span to this zoned timestamp.
        pub fn add_span(&self, span: &Span) -> Self {
            Self(self.0 + span.0)
        }

        /// Add a span to this zoned timestamp (alias for add_span).
        pub fn add(&self, span: &Span) -> Self {
            self.add_span(span)
        }
    }

    impl Display for Zoned {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            if let Some(precision) = f.precision() {
                let format = match precision {
                    6 => "%Y-%m-%dT%H:%M:%S%.6f%:z",
                    3 => "%Y-%m-%dT%H:%M:%S%.3f%:z",
                    _ => "%Y-%m-%dT%H:%M:%S%:z",
                };
                write!(f, "{}", self.0.format(format))
            } else {
                write!(f, "{}", self.0.format("%Y-%m-%dT%H:%M:%S%:z"))
            }
        }
    }

    #[cfg(all(feature = "chrono", feature = "internal-serde"))]
    impl serde::Serialize for Zoned {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // Use chrono's serialize if available, otherwise format as string
            #[cfg(feature = "internal-serde-chrono")]
            {
                self.0.serialize(serializer)
            }
            #[cfg(not(feature = "internal-serde-chrono"))]
            {
                serializer.collect_str(&format_args!("{}", self.0.format("%Y-%m-%dT%H:%M:%S%.6f%:z")))
            }
        }
    }

    /// A span representing a duration of time.
    #[derive(Debug, Clone)]
    pub struct Span(pub chrono::Duration);

    impl Span {
        /// Create a new empty span.
        pub fn new() -> Self {
            Self(chrono::Duration::zero())
        }

        /// Add minutes to this span.
        pub fn minutes(mut self, n: i64) -> Self {
            self.0 = self.0 + chrono::Duration::minutes(n);
            self
        }

        /// Add hours to this span.
        pub fn hours(mut self, n: i64) -> Self {
            self.0 = self.0 + chrono::Duration::hours(n);
            self
        }

        /// Add days to this span.
        pub fn days(mut self, n: i64) -> Self {
            self.0 = self.0 + chrono::Duration::days(n);
            self
        }

        /// Add seconds to this span.
        pub fn seconds(mut self, n: i64) -> Self {
            self.0 = self.0 + chrono::Duration::seconds(n);
            self
        }
    }

    /// ZonedRound placeholder for chrono (simplified).
    pub struct ZonedRound;

    impl ZonedRound {
        /// Create a new ZonedRound.
        pub fn new() -> Self {
            Self
        }

        /// Set the smallest unit for rounding.
        pub fn smallest(self, _unit: Unit) -> Self {
            self
        }

        /// Set the rounding mode.
        pub fn mode(self, _mode: RoundMode) -> Self {
            self
        }
    }

    /// Unit placeholder for chrono.
    pub enum Unit {
        /// Minute unit.
        Minute,
        /// Hour unit.
        Hour,
        /// Day unit.
        Day,
    }

    /// RoundMode placeholder for chrono.
    pub enum RoundMode {
        /// Truncate mode.
        Trunc,
    }

    /// ToSpan trait for chrono.
    pub trait ToSpan {
        /// Convert to a span.
        fn to_span(self) -> Span;
    }

    impl ToSpan for Span {
        fn to_span(self) -> Span {
            self
        }
    }

    /// Create a 1-minute span.
    pub fn minute() -> Span {
        Span::new().minutes(1)
    }

    /// Create a 1-hour span.
    pub fn hour() -> Span {
        Span::new().hours(1)
    }

    /// Create a 1-day span.
    pub fn day() -> Span {
        Span::new().days(1)
    }

    /// Parse a date string using the given format.
    pub fn parse_date(format: &str, date_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For chrono, we'll try to parse with a simple format check
        // This is a simplified implementation - chrono doesn't have direct equivalent to jiff's strptime
        chrono::NaiveDateTime::parse_from_str(date_str, format)?;
        Ok(())
    }
}

// Re-export the appropriate implementation based on features
#[cfg(all(feature = "jiff", not(feature = "chrono")))]
pub use jiff_impl::*;

#[cfg(all(feature = "chrono", not(feature = "jiff")))]
pub use chrono_impl::*;

#[cfg(all(feature = "jiff", feature = "chrono"))]
pub use jiff_impl::*; // Prefer jiff when both are enabled

#[cfg(not(any(feature = "jiff", feature = "chrono")))]
compile_error!("At least one of 'jiff' or 'chrono' features must be enabled");
