//! Reading the local wall clock from Unix milliseconds and back, daylight-saving
//! changeovers included. Pure: the time zone comes in through [`Zone`], so the
//! tests run any clock.

use std::fmt;

pub const MINUTE_MS: i64 = 60_000;
pub const DAY_MS: i64 = 86_400_000;

/// The PC's time zone.
pub trait Zone {
    /// Local time minus UTC, in minutes, at `unix_ms`.
    fn utc_offset_minutes(&self, unix_ms: i64) -> i32;
}

/// A time of day on the local clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockTime {
    pub hour: u8,
    pub minute: u8,
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

/// The local clock reading at `unix_ms`.
pub fn clock_time(zone: &(impl Zone + ?Sized), unix_ms: i64) -> ClockTime {
    let minute_of_day = wall_minute(zone, unix_ms).rem_euclid(1440);
    ClockTime { hour: (minute_of_day / 60) as u8, minute: (minute_of_day % 60) as u8 }
}

/// Minutes on the local wall clock since 1970-01-01 00:00.
pub fn wall_minute(zone: &(impl Zone + ?Sized), unix_ms: i64) -> i64 {
    unix_ms.div_euclid(MINUTE_MS) + i64::from(zone.utc_offset_minutes(unix_ms))
}

/// The instant a wall-clock minute happens: its first occurrence when the
/// clock repeats it, the end of the gap when the clock skips it.
pub fn instant_of_wall_minute(zone: &(impl Zone + ?Sized), wall: i64) -> i64 {
    let wall_ms = wall * MINUTE_MS;
    let from_offset = |offset: i32| wall_ms - i64::from(offset) * MINUTE_MS;
    let before = from_offset(zone.utc_offset_minutes(wall_ms - DAY_MS));
    let after = from_offset(zone.utc_offset_minutes(wall_ms + DAY_MS));
    let reads_as_wall = |at: i64| wall_minute(zone, at) == wall;
    match (reads_as_wall(before), reads_as_wall(after)) {
        (true, true) => before.min(after),
        (true, false) => before,
        (false, true) => after,
        (false, false) => before,
    }
}
