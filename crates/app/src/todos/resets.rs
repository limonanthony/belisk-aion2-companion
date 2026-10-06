//! When the daily and weekly resets, and the daily refills, happen. Pure: the
//! time zone comes in through [`Zone`], so the tests run any clock, daylight
//! saving changeovers included.
//!
//! A time the spring changeover skips happens at the end of the gap; a time the
//! autumn changeover repeats happens at its first occurrence only (the same
//! rules as the reminders, `clock::instant_of_wall_minute`).

use crate::clock::{instant_of_wall_minute, wall_minute, Zone};

const WEDNESDAY: i64 = 3;

/// The reset hour on a zone: daily at that hour, weekly on Wednesday at it.
#[derive(Clone, Copy)]
pub struct Resets<'a> {
    zone: &'a dyn Zone,
    hour: u8,
}

/// Something that happens at `hour` o'clock on the local clock, every day or
/// one weekday (0 is Sunday).
#[derive(Clone, Copy)]
struct Recurrence {
    hour: u8,
    weekday: Option<i64>,
}

impl<'a> Resets<'a> {
    pub fn new(zone: &'a dyn Zone, hour: u8) -> Self {
        Self { zone, hour }
    }

    pub fn zone(&self) -> &'a dyn Zone {
        self.zone
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The latest daily reset at or before `now_ms`.
    pub fn last_daily(&self, now_ms: i64) -> i64 {
        self.daily_at(self.hour).last_at_or_before(self.zone, now_ms)
    }

    /// The first daily reset after `now_ms`.
    pub fn next_daily(&self, now_ms: i64) -> i64 {
        self.next_daily_at(self.hour, now_ms)
    }

    pub fn last_weekly(&self, now_ms: i64) -> i64 {
        self.weekly().last_at_or_before(self.zone, now_ms)
    }

    pub fn next_weekly(&self, now_ms: i64) -> i64 {
        self.weekly().next_after(self.zone, now_ms)
    }

    /// The first `hour`:00 on the local clock after `after_ms`.
    pub fn next_daily_at(&self, hour: u8, after_ms: i64) -> i64 {
        self.daily_at(hour).next_after(self.zone, after_ms)
    }

    fn daily_at(&self, hour: u8) -> Recurrence {
        Recurrence { hour, weekday: None }
    }

    fn weekly(&self) -> Recurrence {
        Recurrence { hour: self.hour, weekday: Some(WEDNESDAY) }
    }
}

impl Recurrence {
    fn on_day(&self, zone: &dyn Zone, day: i64) -> Option<i64> {
        let weekday = (day + 4).rem_euclid(7);
        self.weekday
            .is_none_or(|wanted| wanted == weekday)
            .then(|| instant_of_wall_minute(zone, day * 1440 + i64::from(self.hour) * 60))
    }

    fn last_at_or_before(&self, zone: &dyn Zone, now_ms: i64) -> i64 {
        let today = wall_minute(zone, now_ms).div_euclid(1440);
        (0..).find_map(|back| self.on_day(zone, today + 1 - back).filter(|&at| at <= now_ms)).unwrap_or(now_ms)
    }

    fn next_after(&self, zone: &dyn Zone, now_ms: i64) -> i64 {
        let today = wall_minute(zone, now_ms).div_euclid(1440);
        (0..).find_map(|ahead| self.on_day(zone, today - 1 + ahead).filter(|&at| at > now_ms)).unwrap_or(now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::clock_time;
    use crate::clock::testing::{berlin, utc, TestZone};

    const HOUR: i64 = 60 * 60_000;

    fn summer() -> TestZone {
        TestZone { base: 120, changes: Vec::new() }
    }

    /// Local time in the fixed summer zone; 2026-10-05 is a Monday.
    fn local(day_of_october: i64, hour: i64, minute: i64) -> i64 {
        utc(2026, 10, day_of_october, hour, minute) - 2 * HOUR
    }

    #[test]
    fn the_daily_reset_is_the_latest_reset_hour_on_or_before_now() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        assert_eq!(resets.last_daily(local(5, 10, 0)), local(5, 9, 0));
        assert_eq!(resets.last_daily(local(5, 9, 0)), local(5, 9, 0), "exactly at the reset it has happened");
        assert_eq!(resets.last_daily(local(5, 8, 59)), local(4, 9, 0));
        assert_eq!(resets.next_daily(local(5, 9, 0)), local(6, 9, 0), "the next one is strictly after");
        assert_eq!(resets.next_daily(local(5, 8, 0)), local(5, 9, 0));
    }

    #[test]
    fn the_weekly_reset_is_wednesday_at_the_reset_hour() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        assert_eq!(resets.last_weekly(local(7, 9, 0)), local(7, 9, 0), "Wednesday 7 Oct at 09:00");
        assert_eq!(resets.last_weekly(local(7, 8, 59)), local(7, 9, 0) - 7 * 24 * HOUR, "still last Wednesday");
        assert_eq!(resets.last_weekly(local(12, 12, 0)), local(7, 9, 0), "Monday 12 Oct");
        assert_eq!(resets.next_weekly(local(7, 9, 0)), local(14, 9, 0));
        assert_eq!(resets.next_weekly(local(5, 12, 0)), local(7, 9, 0), "Monday to Wednesday");
        assert_eq!(resets.next_weekly(local(8, 12, 0)), local(14, 9, 0), "Thursday to next Wednesday");
    }

    #[test]
    fn midnight_hours_work_across_the_day_boundary() {
        let zone = summer();
        let resets = Resets::new(&zone, 0);
        assert_eq!(resets.last_daily(local(5, 0, 0)), local(5, 0, 0));
        assert_eq!(resets.last_daily(local(4, 23, 59)), local(4, 0, 0));
        assert_eq!(resets.next_daily(local(5, 23, 59)), local(6, 0, 0));
    }

    #[test]
    fn a_reset_hour_inside_the_spring_gap_happens_at_the_end_of_the_gap() {
        let zone = berlin();
        let resets = Resets::new(&zone, 2);
        let reset = resets.last_daily(utc(2026, 3, 29, 12, 0));
        assert_eq!(reset, utc(2026, 3, 29, 1, 0), "02:00 does not exist, the clock jumps to 03:00");
        assert_eq!(clock_time(&zone, reset).to_string(), "03:00");
        assert_eq!(resets.next_daily(utc(2026, 3, 28, 12, 0)), utc(2026, 3, 29, 1, 0));
    }

    #[test]
    fn a_reset_hour_the_autumn_repeats_happens_once_at_its_first_occurrence() {
        let zone = berlin();
        let resets = Resets::new(&zone, 2);
        let first = utc(2026, 10, 25, 0, 0);
        assert_eq!(resets.last_daily(utc(2026, 10, 25, 1, 30)), first, "02:30 CET is after the first 02:00");
        assert_eq!(resets.next_daily(first), utc(2026, 10, 26, 1, 0), "the repeated 02:00 CET is not a second reset");
    }

    #[test]
    fn the_day_of_a_changeover_keeps_the_reset_at_the_wall_hour() {
        let zone = berlin();
        let resets = Resets::new(&zone, 9);
        let before = resets.last_daily(utc(2026, 3, 29, 6, 0));
        assert_eq!(before, utc(2026, 3, 28, 8, 0), "09:00 CET the day before");
        let after = resets.next_daily(utc(2026, 3, 29, 6, 0));
        assert_eq!(after, utc(2026, 3, 29, 7, 0), "09:00 CEST, 23 hours later");
    }

    #[test]
    fn the_weekly_reset_is_found_across_a_changeover() {
        let zone = berlin();
        let resets = Resets::new(&zone, 9);
        let reset = resets.last_weekly(utc(2026, 10, 27, 12, 0));
        assert_eq!(reset, utc(2026, 10, 21, 7, 0), "Wednesday 21 Oct 09:00 CEST");
        assert_eq!(resets.next_weekly(utc(2026, 10, 27, 12, 0)), utc(2026, 10, 28, 8, 0), "09:00 CET after the changeover");
    }

    #[test]
    fn a_daily_refill_hour_is_independent_of_the_reset_hour() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        assert_eq!(resets.next_daily_at(16, local(5, 10, 0)), local(5, 16, 0));
        assert_eq!(resets.next_daily_at(16, local(5, 16, 0)), local(6, 16, 0));
    }
}
