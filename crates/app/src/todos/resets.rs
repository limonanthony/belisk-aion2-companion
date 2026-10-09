//! When the daily and weekly resets, and the daily refills, happen. Pure: the
//! time zone comes in through [`Zone`], so the tests run any clock, daylight
//! saving changeovers included.
//!
//! The game's own grid is 07:00 UTC every day, whatever the PC's clock does
//! ([`Resets::following_the_game`]); the player may instead pin the reset to
//! an hour on the PC's clock ([`Resets::new`]).
//!
//! A pinned time the spring changeover skips happens at the end of the gap; a
//! one the autumn changeover repeats happens at its first occurrence only (the
//! same rules as the reminders, `clock::instant_of_wall_minute`).

use crate::clock::{clock_time, instant_of_wall_minute, wall_minute, ClockTime, Zone, DAY_MS};

const WEDNESDAY: i64 = 3;
const GAME_RESET_UTC_HOUR: i64 = 7;

/// Where on the clock a reset falls.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResetClock {
    /// An hour on the PC's clock.
    Local(u8),
    /// The game's own grid, a fixed time of day in UTC.
    Game,
}

/// The reset clock on a zone: daily at it, weekly on Wednesday at it.
#[derive(Clone, Copy)]
pub struct Resets<'a> {
    zone: &'a dyn Zone,
    clock: ResetClock,
}

/// Something that happens at a reset clock, every day or one weekday (0 is
/// Sunday).
#[derive(Clone, Copy)]
struct Recurrence {
    clock: ResetClock,
    weekday: Option<i64>,
}

impl<'a> Resets<'a> {
    /// Resets pinned to `hour` o'clock on the PC's clock.
    pub fn new(zone: &'a dyn Zone, hour: u8) -> Self {
        Self { zone, clock: ResetClock::Local(hour) }
    }

    /// Resets on the game's grid, 07:00 UTC.
    pub fn following_the_game(zone: &'a dyn Zone) -> Self {
        Self { zone, clock: ResetClock::Game }
    }

    pub fn zone(&self) -> &'a dyn Zone {
        self.zone
    }

    /// The PC's clock reading of the daily reset in force at `now_ms`.
    pub fn clock_at(&self, now_ms: i64) -> ClockTime {
        match self.clock {
            ResetClock::Local(hour) => ClockTime { hour, minute: 0 },
            ResetClock::Game => clock_time(self.zone, self.last_daily(now_ms)),
        }
    }

    /// The latest daily reset at or before `now_ms`.
    pub fn last_daily(&self, now_ms: i64) -> i64 {
        self.daily().last_at_or_before(self.zone, now_ms)
    }

    /// The first daily reset after `now_ms`.
    pub fn next_daily(&self, now_ms: i64) -> i64 {
        self.daily().next_after(self.zone, now_ms)
    }

    pub fn last_weekly(&self, now_ms: i64) -> i64 {
        self.weekly().last_at_or_before(self.zone, now_ms)
    }

    pub fn next_weekly(&self, now_ms: i64) -> i64 {
        self.weekly().next_after(self.zone, now_ms)
    }

    fn daily(&self) -> Recurrence {
        Recurrence { clock: self.clock, weekday: None }
    }

    fn weekly(&self) -> Recurrence {
        Recurrence { clock: self.clock, weekday: Some(WEDNESDAY) }
    }
}

impl Recurrence {
    /// The day number the clock reads at `now_ms`: local days on the PC's
    /// clock, UTC days on the game's.
    fn day_of(&self, zone: &dyn Zone, now_ms: i64) -> i64 {
        match self.clock {
            ResetClock::Local(_) => wall_minute(zone, now_ms).div_euclid(1440),
            ResetClock::Game => now_ms.div_euclid(DAY_MS),
        }
    }

    fn on_day(&self, zone: &dyn Zone, day: i64) -> Option<i64> {
        let weekday = (day + 4).rem_euclid(7);
        self.weekday.is_none_or(|wanted| wanted == weekday).then(|| match self.clock {
            ResetClock::Local(hour) => instant_of_wall_minute(zone, day * 1440 + i64::from(hour) * 60),
            ResetClock::Game => day * DAY_MS + GAME_RESET_UTC_HOUR * 3_600_000,
        })
    }

    fn last_at_or_before(&self, zone: &dyn Zone, now_ms: i64) -> i64 {
        let today = self.day_of(zone, now_ms);
        (0..).find_map(|back| self.on_day(zone, today + 1 - back).filter(|&at| at <= now_ms)).unwrap_or(now_ms)
    }

    fn next_after(&self, zone: &dyn Zone, now_ms: i64) -> i64 {
        let today = self.day_of(zone, now_ms);
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
    fn the_next_daily_reset_follows_the_chosen_reset_hour() {
        let zone = summer();
        let resets = Resets::new(&zone, 16);
        assert_eq!(resets.next_daily(local(5, 10, 0)), local(5, 16, 0));
        assert_eq!(resets.next_daily(local(5, 16, 0)), local(6, 16, 0));
    }

    #[test]
    fn the_games_daily_reset_is_07_utc_whatever_the_clock() {
        let zone = berlin();
        let resets = Resets::following_the_game(&zone);
        assert_eq!(resets.last_daily(utc(2026, 10, 5, 12, 0)), utc(2026, 10, 5, 7, 0), "09:00 CEST");
        assert_eq!(resets.last_daily(utc(2026, 12, 5, 12, 0)), utc(2026, 12, 5, 7, 0), "08:00 CET");
        assert_eq!(resets.last_daily(utc(2026, 12, 5, 6, 59)), utc(2026, 12, 4, 7, 0));
        assert_eq!(resets.next_daily(utc(2026, 12, 5, 7, 0)), utc(2026, 12, 6, 7, 0));
    }

    #[test]
    fn the_games_reset_reads_09_in_summer_and_08_in_winter_on_a_berlin_clock() {
        let zone = berlin();
        let resets = Resets::following_the_game(&zone);
        assert_eq!(resets.clock_at(utc(2026, 10, 5, 12, 0)).to_string(), "09:00");
        assert_eq!(resets.clock_at(utc(2026, 12, 5, 12, 0)).to_string(), "08:00");
    }

    #[test]
    fn the_games_reset_does_not_move_across_a_changeover() {
        let zone = berlin();
        let resets = Resets::following_the_game(&zone);
        let before = resets.last_daily(utc(2026, 10, 25, 6, 0));
        assert_eq!(before, utc(2026, 10, 24, 7, 0), "09:00 CEST the day before the autumn changeover");
        assert_eq!(resets.next_daily(utc(2026, 10, 25, 6, 0)), utc(2026, 10, 25, 7, 0), "08:00 CET, the same game instant");
        assert_eq!(resets.next_daily(before) - before, 24 * HOUR);
    }

    #[test]
    fn the_games_weekly_reset_is_wednesday_07_utc() {
        let zone = berlin();
        let resets = Resets::following_the_game(&zone);
        assert_eq!(resets.last_weekly(utc(2026, 10, 27, 12, 0)), utc(2026, 10, 21, 7, 0));
        assert_eq!(resets.next_weekly(utc(2026, 10, 27, 12, 0)), utc(2026, 10, 28, 7, 0), "08:00 CET");
        assert_eq!(resets.last_weekly(utc(2026, 12, 9, 6, 59)), utc(2026, 12, 2, 7, 0));
    }

    #[test]
    fn a_pinned_hour_still_moves_with_the_clock() {
        let zone = berlin();
        let resets = Resets::new(&zone, 9);
        assert_eq!(resets.last_daily(utc(2026, 10, 5, 12, 0)), utc(2026, 10, 5, 7, 0));
        assert_eq!(resets.last_daily(utc(2026, 12, 5, 12, 0)), utc(2026, 12, 5, 8, 0));
    }
}
