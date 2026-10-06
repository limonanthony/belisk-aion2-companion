//! What an amount the player typed is worth now. The game's counts cannot be
//! read, so the player types one once; from then on the tracker adds each
//! refill that has happened since, up to the cap.

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
}

impl Standing {
    pub fn is_full(&self) -> bool {
        self.full_at_ms.is_none()
    }
}

pub fn standing(spec: &AmountSpec, cap: u32, typed: TypedAmount, now_ms: i64, resets: &Resets) -> Standing {
    let add = spec.refill.add();
    let start = typed.value.min(cap);
    let needed = (cap - start).div_ceil(add);
    let done = refills_between(spec.refill, resets, typed.at_ms, now_ms, needed);
    if done >= needed {
        return Standing { value: cap, full_at_ms: None };
    }
    Standing { value: start + add * done, full_at_ms: Some(refill_instant(spec.refill, resets, typed.at_ms, needed)) }
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
            u32::try_from((to_ms - from_ms) / every).map_or(limit, |count| count.min(limit))
        }
        Refill::DailyAt { .. } | Refill::AtDailyReset { .. } => {
            let mut at = from_ms;
            let mut count = 0;
            while count < limit {
                at = refill_instant(refill, resets, at, 1);
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
        Refill::EveryHours { hours, .. } => from_ms + i64::from(k) * i64::from(hours) * HOUR_MS,
        Refill::DailyAt { hour, .. } => (0..k).fold(from_ms, |at, _| resets.next_daily_at(hour, at)),
        Refill::AtDailyReset { .. } => (0..k).fold(from_ms, |at, _| resets.next_daily_at(resets.hour(), at)),
    }
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

    fn typed(value: u32, at_ms: i64) -> TypedAmount {
        TypedAmount { value, at_ms }
    }

    #[test]
    fn odyle_gains_15_every_three_hours_counted_from_the_time_typed() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let odyle = spec(ODYLE);
        let at = local(5, 10, 20);
        let after = |minutes: i64| standing(&odyle, 560, typed(100, at), at + minutes * 60_000, &resets).value;
        assert_eq!(after(0), 100);
        assert_eq!(after(179), 100, "not yet");
        assert_eq!(after(180), 115);
        assert_eq!(after(6 * 60 + 5), 130);
    }

    #[test]
    fn odyle_stops_at_the_cap_and_membership_raises_it() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let odyle = spec(ODYLE);
        let at = local(5, 10, 0);
        let day_later = at + 24 * HOUR_MS;
        let value = |cap: u32| standing(&odyle, cap, typed(500, at), day_later, &resets);
        assert_eq!(value(560), Standing { value: 560, full_at_ms: None }, "8 refills of 15 would pass 560");
        assert_eq!(value(840).value, 500 + 8 * 15, "membership: still filling");
        assert_eq!(value(840).full_at_ms, Some(at + 23 * 3 * HOUR_MS), "(840 - 500) / 15 rounds up to 23 refills");
    }

    #[test]
    fn the_time_it_fills_does_not_move_while_it_waits() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let odyle = spec(ODYLE);
        let at = local(5, 10, 0);
        let early = standing(&odyle, 560, typed(545, at), at + HOUR_MS, &resets);
        let later = standing(&odyle, 560, typed(545, at), at + 2 * HOUR_MS, &resets);
        assert_eq!(early.full_at_ms, Some(at + 3 * HOUR_MS));
        assert_eq!(later.full_at_ms, early.full_at_ms);
        assert!(standing(&odyle, 560, typed(545, at), at + 3 * HOUR_MS, &resets).is_full());
    }

    #[test]
    fn a_typed_value_above_the_cap_reads_as_full() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let now = local(5, 12, 0);
        assert_eq!(standing(&spec(ODYLE), 560, typed(800, now), now, &resets), Standing { value: 560, full_at_ms: None });
    }

    #[test]
    fn shugo_keys_gain_4_each_day_at_16_00() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let shugo = spec("shugo");
        let typed_at = local(5, 17, 0);
        let value = |now: i64| standing(&shugo, 28, typed(10, typed_at), now, &resets);
        assert_eq!(value(local(6, 15, 59)).value, 10);
        assert_eq!(value(local(6, 16, 0)).value, 14);
        assert_eq!(value(local(7, 16, 0)).value, 18);
        assert_eq!(value(local(6, 16, 0)).full_at_ms, Some(local(5, 16, 0) + 5 * 24 * HOUR_MS), "5 refills: 10 to 28 takes 5");
        assert_eq!(value(local(20, 0, 0)), Standing { value: 28, full_at_ms: None });
    }

    #[test]
    fn a_refill_at_the_typing_minute_does_not_count_twice() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let invasion = spec("invasion");
        let at = local(5, 16, 0);
        assert_eq!(standing(&invasion, 7, typed(2, at), at, &resets).value, 2, "typed at 16:00 sharp");
        assert_eq!(standing(&invasion, 7, typed(2, at), at + 24 * HOUR_MS, &resets).value, 3);
    }

    #[test]
    fn daily_recharges_follow_the_reset_hour_setting() {
        let zone = summer();
        let nightmare = spec("nightmare");
        let typed_at = local(5, 12, 0);
        let at_nine = standing(&nightmare, 14, typed(0, typed_at), local(6, 9, 30), &Resets::new(&zone, 9));
        let at_ten = standing(&nightmare, 14, typed(0, typed_at), local(6, 9, 30), &Resets::new(&zone, 10));
        assert_eq!(at_nine.value, 2);
        assert_eq!(at_ten.value, 0, "the reset is at 10:00 now");
    }

    #[test]
    fn the_full_row_names_the_refill_it_is_about_to_lose() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        assert_eq!(next_refill(&spec("shugo"), &resets, local(5, 12, 0)), local(5, 16, 0));
        assert_eq!(next_refill(&spec("balaur"), &resets, local(5, 9, 0)), local(6, 9, 0));
    }

    #[test]
    fn daily_refills_follow_the_wall_clock_across_a_changeover() {
        let zone = berlin();
        let resets = Resets::new(&zone, 9);
        let shugo = spec("shugo");
        let typed_at = utc(2026, 3, 28, 16, 0);
        let day_of_change = standing(&shugo, 28, typed(0, typed_at), utc(2026, 3, 29, 14, 0), &resets);
        assert_eq!(day_of_change.value, 4, "16:00 CEST is 14:00 UTC, 23 hours after 16:00 CET");
        let before = standing(&shugo, 28, typed(0, typed_at), utc(2026, 3, 29, 13, 59), &resets);
        assert_eq!(before.value, 0);
    }

    #[test]
    fn a_typing_time_in_the_future_adds_nothing() {
        let zone = summer();
        let resets = Resets::new(&zone, 9);
        let now = local(5, 12, 0);
        assert_eq!(standing(&spec(ODYLE), 560, typed(100, now + 5 * HOUR_MS), now, &resets).value, 100);
    }
}
