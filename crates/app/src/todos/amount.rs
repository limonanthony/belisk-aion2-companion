//! What an amount is worth now. The game's regular pool (or a number the
//! player typed) is known at one moment; from there the tracker adds each
//! refill that has happened since, up to the cap. A spend is never guessed:
//! the game says it (and a live meter hears it) when it happens.

use serde::{Deserialize, Serialize};

use super::catalogue::{AmountSpec, Refill};
use super::resets::Resets;

const HOUR_MS: i64 = 3_600_000;

/// What the player typed (or stepped to) and when. The refills count from there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedAmount {
    pub value: u32,
    pub at_ms: i64,
}

/// Where an amount stands at a moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    pub value: u32,
    /// When it reaches the cap; none once it has.
    pub full_at_ms: Option<i64>,
    /// How many refills it gained since it was known.
    pub refills: u32,
}

impl Standing {
    pub fn is_full(&self) -> bool {
        self.full_at_ms.is_none()
    }
}

/// An amount of `value` known at `at_ms`, at `now_ms`. A value at or above the
/// cap is full and stays as it is.
pub fn standing(spec: &AmountSpec, cap: u32, value: u32, at_ms: i64, now_ms: i64, resets: &Resets) -> Standing {
    if value >= cap {
        return Standing { value, full_at_ms: None, refills: 0 };
    }
    let add = spec.refill.add();
    let needed = (cap - value).div_ceil(add);
    let done = refills_between(spec.refill, resets, at_ms, now_ms, needed);
    if done >= needed {
        return Standing { value: cap, full_at_ms: None, refills: done };
    }
    Standing { value: value + add * done, full_at_ms: Some(refill_instant(spec.refill, resets, at_ms, needed)), refills: done }
}

/// The refill after `after_ms`: the one a full amount loses.
pub fn next_refill(spec: &AmountSpec, resets: &Resets, after_ms: i64) -> i64 {
    refill_instant(spec.refill, resets, after_ms, 1)
}

/// How many refills happen in `(from_ms, to_ms]`, counting no further than `limit`.
fn refills_between(refill: Refill, resets: &Resets, from_ms: i64, to_ms: i64, limit: u32) -> u32 {
    if to_ms <= from_ms {
        return 0;
    }
    match refill {
        Refill::EveryHours { hours, .. } => {
            let every = i64::from(hours) * HOUR_MS;
            let first = first_on_grid(resets, every, from_ms);
            if first > to_ms {
                return 0;
            }
            u32::try_from(1 + (to_ms - first) / every).map_or(limit, |count| count.min(limit))
        }
        Refill::AtDailyReset { .. } => {
            let mut at = from_ms;
            let mut count = 0;
            while count < limit {
                at = resets.next_daily(at);
                if at > to_ms {
                    break;
                }
                count += 1;
            }
            count
        }
    }
}

/// The `k`-th refill after `from_ms` (`k` of at least 1).
fn refill_instant(refill: Refill, resets: &Resets, from_ms: i64, k: u32) -> i64 {
    match refill {
        Refill::EveryHours { hours, .. } => {
            let every = i64::from(hours) * HOUR_MS;
            first_on_grid(resets, every, from_ms) + i64::from(k - 1) * every
        }
        Refill::AtDailyReset { .. } => (0..k).fold(from_ms, |at, _| resets.next_daily(at)),
    }
}

/// The first step of the every-`every_ms` clock after `after_ms`; the clock
/// starts at the latest daily reset.
fn first_on_grid(resets: &Resets, every_ms: i64, after_ms: i64) -> i64 {
    let start = resets.last_daily(after_ms);
    start + ((after_ms - start) / every_ms + 1) * every_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::testing::{berlin, utc, TestZone};
    use crate::todos::catalogue::{builtin, Kind, ODYLE};

    fn spec(id: &str) -> AmountSpec {
        match builtin(id).unwrap().kind {
            Kind::Amount(spec) => spec,
            other => panic!("{id} is {other:?}"),
        }
    }

    fn summer() -> TestZone {
        TestZone { base: 120, changes: Vec::new() }
    }

    /// Local time in the fixed summer zone; 2026-10-05 is a Monday.
    fn local(day_of_october: i64, hour: i64, minute: i64) -> i64 {
        utc(2026, 10, day_of_october, hour, minute) - 2 * HOUR_MS
    }

    #[test]
    fn odyle_gains_15_every_three_hours_on_the_clock_that_starts_at_the_reset_hour() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let odyle = spec(ODYLE);
        let at = local(5, 10, 20);
        let after = |now: i64| standing(&odyle, 560, 100, at, now, &resets).value;
        assert_eq!(after(at), 100);
        assert_eq!(after(local(5, 11, 59)), 100, "not yet");
        assert_eq!(after(local(5, 12, 0)), 115, "the steps are 09:00, 12:00, 15:00");
        assert_eq!(after(local(5, 15, 0)), 130);
        assert_eq!(standing(&odyle, 560, 100, at, local(5, 15, 0), &resets).refills, 2);
    }

    #[test]
    fn odyle_stops_at_the_cap_and_membership_raises_it() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let odyle = spec(ODYLE);
        let at = local(5, 10, 0);
        let day_later = at + 24 * HOUR_MS;
        let value = |cap: u32| standing(&odyle, cap, 500, at, day_later, &resets);
        assert_eq!(value(560).value, 560, "8 refills of 15 would pass 560");
        assert!(value(560).is_full());
        assert_eq!(value(840).value, 500 + 8 * 15, "membership: still filling");
        assert_eq!(value(840).full_at_ms, Some(local(5, 12, 0) + 22 * 3 * HOUR_MS), "(840 - 500) / 15 rounds up to 23 refills");
    }

    #[test]
    fn the_time_it_fills_does_not_move_while_it_waits() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let odyle = spec(ODYLE);
        let at = local(5, 10, 0);
        let early = standing(&odyle, 560, 545, at, at + HOUR_MS, &resets);
        let later = standing(&odyle, 560, 545, at, at + 2 * HOUR_MS - 1, &resets);
        assert_eq!(early.full_at_ms, Some(local(5, 12, 0)));
        assert_eq!(later.full_at_ms, early.full_at_ms);
        assert!(standing(&odyle, 560, 545, at, local(5, 12, 0), &resets).is_full());
    }

    #[test]
    fn a_value_at_or_above_the_cap_is_full_and_kept() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let now = local(5, 12, 0);
        let kept = standing(&spec(ODYLE), 560, 800, now, now + 48 * HOUR_MS, &resets);
        assert_eq!(kept, Standing { value: 800, full_at_ms: None, refills: 0 });
        assert!(standing(&spec("shugo"), 12, 12, now, now, &resets).is_full());
    }

    #[test]
    fn shugo_keys_gain_3_each_day_at_the_reset_hour() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let shugo = spec("shugo");
        let known_at = local(5, 17, 0);
        let value = |now: i64| standing(&shugo, 12, 3, known_at, now, &resets);
        assert_eq!(value(local(6, 8, 59)).value, 3);
        assert_eq!(value(local(6, 9, 0)).value, 6);
        assert_eq!(value(local(7, 9, 0)).value, 9);
        assert_eq!(value(local(6, 9, 0)).full_at_ms, Some(local(8, 9, 0)), "3 to 12 takes 3 refills");
        assert_eq!(value(local(20, 0, 0)), Standing { value: 12, full_at_ms: None, refills: 3 });
    }

    #[test]
    fn a_refill_at_the_known_minute_does_not_count_twice() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let invasion = spec("invasion");
        let at = local(5, 9, 0);
        assert_eq!(standing(&invasion, 7, 2, at, at, &resets).value, 2, "known at the reset hour sharp");
        assert_eq!(standing(&invasion, 7, 2, at, at + 24 * HOUR_MS, &resets).value, 3);
    }

    #[test]
    fn daily_recharges_follow_the_reset_hour_setting() {
        let zone = summer();
        let nightmare = spec("nightmare");
        let known_at = local(5, 12, 0);
        let at_nine = standing(&nightmare, 14, 0, known_at, local(6, 9, 30), &Resets::new(&zone, 9));
        let at_ten = standing(&nightmare, 14, 0, known_at, local(6, 9, 30), &Resets::new(&zone, 10));
        assert_eq!(at_nine.value, 2);
        assert_eq!(at_ten.value, 0, "the reset is at 10:00 now");
    }

    #[test]
    fn the_full_row_names_the_refill_it_is_about_to_lose() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        assert_eq!(next_refill(&spec("shugo"), &resets, local(5, 12, 0)), local(6, 9, 0));
        assert_eq!(next_refill(&spec("balaur"), &resets, local(5, 9, 0)), local(6, 9, 0));
        assert_eq!(next_refill(&spec(ODYLE), &resets, local(5, 12, 0)), local(5, 15, 0));
    }

    #[test]
    fn daily_refills_follow_the_wall_clock_across_a_changeover() {
        let zone = berlin();
        let resets = Resets::new(&zone, 9);
        let shugo = spec("shugo");
        let known_at = utc(2026, 3, 28, 16, 0);
        let day_of_change = standing(&shugo, 12, 0, known_at, utc(2026, 3, 29, 7, 0), &resets);
        assert_eq!(day_of_change.value, 3, "09:00 CEST is 07:00 UTC, 23 hours after 09:00 CET");
        let before = standing(&shugo, 12, 0, known_at, utc(2026, 3, 29, 6, 59), &resets);
        assert_eq!(before.value, 0);
    }

    #[test]
    fn a_time_in_the_future_adds_nothing() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let now = local(5, 12, 0);
        assert_eq!(standing(&spec(ODYLE), 560, 100, now + 5 * HOUR_MS, now, &resets).value, 100);
    }
}
