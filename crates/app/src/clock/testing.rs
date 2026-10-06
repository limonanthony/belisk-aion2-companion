//! Time zones for tests: a base offset and the instants it changes at.

use super::wall::{Zone, MINUTE_MS};

pub struct TestZone {
    pub base: i32,
    pub changes: Vec<(i64, i32)>,
}

impl Zone for TestZone {
    fn utc_offset_minutes(&self, unix_ms: i64) -> i32 {
        self.changes.iter().rev().find(|&&(from, _)| unix_ms >= from).map_or(self.base, |&(_, offset)| offset)
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

pub fn utc(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
    (days_from_civil(year, month, day) * 1440 + hour * 60 + minute) * MINUTE_MS
}

/// Berlin in 2026: UTC+1, +2 from 2026-03-29 01:00 UTC, +1 from 2026-10-25 01:00 UTC.
pub fn berlin() -> TestZone {
    TestZone { base: 60, changes: vec![(utc(2026, 3, 29, 1, 0), 120), (utc(2026, 10, 25, 1, 0), 60)] }
}
