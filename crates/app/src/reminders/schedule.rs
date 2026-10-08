//! When the next reminder is due. Pure: the time zone comes in through
//! [`Zone`], so the tests run any clock, DST changeovers included.
//!
//! Every rule in `events.rs` is read on the PC's local wall clock. Instants are
//! Unix milliseconds. Daylight-saving changeovers (Berlin: the last Sundays of
//! March and October):
//!
//! - **Hourly events** (Shugo Festival, Dimensional Invasion) follow the real
//!   hour: one start every 60 real minutes, so the 23-hour spring day has 23
//!   and the 25-hour autumn day has 25, and the repeated wall hour is
//!   reminded twice.
//! - **Fixed wall times** (the 3-hourly events, the weekly and daily ones)
//!   happen once. A time the spring changeover skips (02:00–02:59 in Berlin)
//!   happens at the end of the gap, 03:00, and merges with whatever is due then.
//!   A time the autumn changeover repeats (02:00–02:59) happens at its first
//!   occurrence only.
//! - Two events landing on one instant make one reminder, so a changeover
//!   never produces a burst.

use super::events::{EventId, Rule};
use crate::clock::{instant_of_wall_minute, wall_minute, Zone, MINUTE_MS};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reminder {
    /// When the events start.
    pub start_ms: i64,
    /// When the reminder is due: `start_ms` minus the lead time.
    pub notify_ms: i64,
    /// The events starting at `start_ms`, in catalogue order.
    pub events: Vec<EventId>,
}

/// The first start of any of `events` strictly after `after_ms`, with the
/// events sharing that instant; `None` when `events` is empty.
pub fn next_reminder(zone: &impl Zone, events: &[EventId], lead_minutes: u8, after_ms: i64) -> Option<Reminder> {
    let starts: Vec<(EventId, i64)> =
        events.iter().map(|&id| (id, next_start(zone, id.info().rule, after_ms))).collect();
    let start_ms = starts.iter().map(|&(_, start)| start).min()?;
    let mut due: Vec<EventId> = starts.iter().filter(|&&(_, start)| start == start_ms).map(|&(id, _)| id).collect();
    due.sort();
    Some(Reminder { start_ms, notify_ms: start_ms - i64::from(lead_minutes) * MINUTE_MS, events: due })
}

/// The next `count` starts of `events` after `after_ms`, one per event and
/// start, soonest first; events sharing a start come in catalogue order.
pub fn upcoming(zone: &impl Zone, events: &[EventId], after_ms: i64, count: usize) -> Vec<(EventId, i64)> {
    let mut found = Vec::with_capacity(count);
    let mut after = after_ms;
    while found.len() < count {
        let Some(next) = next_reminder(zone, events, 0, after).filter(|next| next.start_ms != i64::MAX) else {
            break;
        };
        found.extend(next.events.iter().map(|&id| (id, next.start_ms)));
        after = next.start_ms;
    }
    found.truncate(count);
    found
}

fn next_start(zone: &impl Zone, rule: Rule, after_ms: i64) -> i64 {
    match rule {
        Rule::Hourly { minute } => next_hourly_start(zone, i64::from(minute), after_ms),
        _ => next_wall_start(zone, rule, after_ms),
    }
}

fn next_hourly_start(zone: &impl Zone, minute: i64, after_ms: i64) -> i64 {
    let mut at = (after_ms.div_euclid(MINUTE_MS) + 1) * MINUTE_MS;
    loop {
        let behind = (minute - wall_minute(zone, at).rem_euclid(60)).rem_euclid(60);
        if behind == 0 {
            return at;
        }
        at += behind * MINUTE_MS;
    }
}

/// Walks the rule's wall times from a day before `after_ms`, since a skipped
/// time lands up to a changeover's width later than its wall reading. Instants
/// never decrease along the walk, so the first one past `after_ms` is the next.
fn next_wall_start(zone: &impl Zone, rule: Rule, after_ms: i64) -> i64 {
    let mut wall = wall_minute(zone, after_ms) - 1440;
    loop {
        let Some(start) = rule.first_at_or_after(wall) else {
            return i64::MAX;
        };
        let at = instant_of_wall_minute(zone, start);
        if at > after_ms {
            return at;
        }
        wall = start + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::clock_time;
    use crate::clock::testing::{berlin, utc, TestZone};
    use crate::reminders::events::EventId::*;

    fn fixed_summer_zone() -> TestZone {
        TestZone { base: 120, changes: Vec::new() }
    }

    /// Local times in the summer-time zone, so tests read as the clock on the wall.
    /// 2026-10-05 is a Monday.
    fn local(day_of_october: i64, hour: i64, minute: i64) -> i64 {
        utc(2026, 10, day_of_october, hour, minute) - 120 * MINUTE_MS
    }

    fn next(events: &[EventId], lead: u8, after: i64) -> Reminder {
        next_reminder(&fixed_summer_zone(), events, lead, after).expect("a reminder")
    }

    fn starts(zone: &TestZone, events: &[EventId], from: i64, until: i64) -> Vec<(i64, Vec<EventId>)> {
        let mut found = Vec::new();
        let mut after = from;
        while let Some(reminder) = next_reminder(zone, events, 1, after).filter(|reminder| reminder.start_ms < until) {
            after = reminder.start_ms;
            found.push((reminder.start_ms, reminder.events));
        }
        found
    }

    #[test]
    fn the_next_hourly_start_is_this_hour_or_the_next() {
        let at_ten_past = local(5, 10, 10) + 20_000;
        assert_eq!(next(&[ShugoFestival], 1, at_ten_past).start_ms, local(5, 11, 0));
        assert_eq!(next(&[DimensionalInvasion], 1, at_ten_past).start_ms, local(5, 10, 30));
    }

    #[test]
    fn a_start_exactly_now_is_past_and_the_following_one_is_next() {
        assert_eq!(next(&[DimensionalInvasion], 1, local(5, 10, 30)).start_ms, local(5, 11, 30));
        assert_eq!(next(&[DimensionalInvasion], 1, local(5, 10, 30) - 1).start_ms, local(5, 10, 30));
    }

    #[test]
    fn the_next_three_starts_list_each_event_at_its_own_time() {
        let found = upcoming(&fixed_summer_zone(), &[ShugoFestival, DimensionalInvasion], local(5, 10, 10), 3);
        assert_eq!(
            found,
            vec![(DimensionalInvasion, local(5, 10, 30)), (ShugoFestival, local(5, 11, 0)), (DimensionalInvasion, local(5, 11, 30))]
        );
        assert!(upcoming(&fixed_summer_zone(), &[], local(5, 10, 10), 3).is_empty());
    }

    #[test]
    fn the_reminder_comes_the_lead_time_before_the_start() {
        let reminder = next(&[DimensionalInvasion], 5, local(5, 10, 0));
        assert_eq!((reminder.notify_ms, reminder.start_ms), (local(5, 10, 25), local(5, 10, 30)));
    }

    #[test]
    fn the_next_start_after_the_last_hour_of_a_day_is_just_after_midnight() {
        let reminder = next(&[DimensionalInvasion], 1, local(5, 23, 45));
        assert_eq!(reminder.start_ms, local(6, 0, 30));
    }

    #[test]
    fn the_lead_time_crosses_midnight_and_the_end_of_the_week() {
        let monday_midnight = next(&[ShugoFestival], 5, local(4, 23, 57));
        assert_eq!(monday_midnight.start_ms, local(5, 0, 0), "Sunday 4 Oct to Monday 5 Oct");
        assert_eq!(monday_midnight.notify_ms, local(4, 23, 55));

        let tuesday_midnight = next(&[ShugoFestival], 3, local(5, 23, 59) + 30_000);
        assert_eq!((tuesday_midnight.notify_ms, tuesday_midnight.start_ms), (local(5, 23, 57), local(6, 0, 0)));
    }

    #[test]
    fn rift_portals_run_every_three_hours_from_two_in_the_morning() {
        let found = starts(&fixed_summer_zone(), &[RiftPortals], local(5, 0, 0), local(6, 0, 0));
        let hours: Vec<i64> = found.iter().map(|(start, _)| (start - local(5, 0, 0)) / (60 * MINUTE_MS)).collect();
        assert_eq!(hours, vec![2, 5, 8, 11, 14, 17, 20, 23]);
        assert_eq!(next(&[RiftPortals], 1, local(5, 23, 30)).start_ms, local(6, 2, 0));
    }

    #[test]
    fn watcher_kaira_runs_every_three_hours_from_one_in_the_morning() {
        let found = starts(&fixed_summer_zone(), &[WatcherKaira], local(5, 0, 0), local(6, 0, 0));
        let hours: Vec<i64> = found.iter().map(|(start, _)| (start - local(5, 0, 0)) / (60 * MINUTE_MS)).collect();
        assert_eq!(hours, vec![1, 4, 7, 10, 13, 16, 19, 22]);
    }

    #[test]
    fn events_starting_in_the_same_minute_make_one_reminder() {
        let defaults = [ShugoFestival, DimensionalInvasion, RiftPortals];
        assert_eq!(next(&defaults, 1, local(5, 10, 31)).events, vec![ShugoFestival, RiftPortals], "11:00");
        assert_eq!(next(&defaults, 1, local(5, 10, 0)).events, vec![DimensionalInvasion], "10:30 alone");
        let monday_night = [ShugoFestival, ArtifactSiege, DimensionalInvasion, AbyssFieldBosses];
        assert_eq!(next(&monday_night, 1, local(5, 20, 31)).events, vec![ShugoFestival, ArtifactSiege]);
        assert_eq!(next(&monday_night, 1, local(5, 21, 1)).events, vec![DimensionalInvasion, AbyssFieldBosses]);
    }

    #[test]
    fn a_weekly_event_waits_for_its_days_across_the_week() {
        assert_eq!(next(&[GuardianLordNahma], 1, local(5, 12, 0)).start_ms, local(9, 21, 0), "Mon to Fri");
        assert_eq!(next(&[GuardianLordNahma], 1, local(9, 21, 0)).start_ms, local(11, 21, 0), "Fri to Sun");
        assert_eq!(next(&[GuardianLordNahma], 1, local(11, 21, 0)).start_ms, local(16, 21, 0), "Sun to next Fri");
        assert_eq!(next(&[ArtifactSiege], 1, local(10, 22, 0)).start_ms, local(12, 21, 0), "Sat to Mon");
        assert_eq!(next(&[ArtifactSiege], 1, local(12, 21, 0)).start_ms, local(15, 21, 0), "Mon to Thu");
    }

    #[test]
    fn a_daily_event_lists_its_times_then_wraps_to_tomorrow() {
        assert_eq!(next(&[ArenaOfTactics], 1, local(5, 0, 0)).start_ms, local(5, 11, 0));
        assert_eq!(next(&[ArenaOfTactics], 1, local(5, 11, 0)).start_ms, local(5, 19, 0));
        assert_eq!(next(&[ArenaOfTactics], 1, local(5, 19, 0)).start_ms, local(6, 11, 0));
        assert_eq!(next(&[RewardKeys], 1, local(5, 16, 0)).start_ms, local(6, 16, 0));
    }

    #[test]
    fn no_enabled_event_means_no_reminder() {
        assert_eq!(next_reminder(&fixed_summer_zone(), &[], 1, local(5, 0, 0)), None);
    }

    #[test]
    fn the_clock_time_is_read_on_the_local_clock() {
        assert_eq!(clock_time(&fixed_summer_zone(), local(5, 21, 30)).to_string(), "21:30");
        assert_eq!(clock_time(&fixed_summer_zone(), local(5, 0, 5)).to_string(), "00:05");
    }

    #[test]
    fn a_zone_with_a_half_hour_offset_still_reads_hourly_events_on_its_own_clock() {
        let india = TestZone { base: 330, changes: Vec::new() };
        let after = utc(2026, 10, 5, 12, 0);
        let reminder = next_reminder(&india, &[ShugoFestival], 1, after).unwrap();
        assert_eq!(reminder.start_ms, utc(2026, 10, 5, 12, 30), "17:30 local is 12:00 UTC, so :00 local is :30 UTC");
    }

    #[test]
    fn spring_forward_hourly_events_keep_the_real_hour_with_no_gap_and_no_duplicate() {
        let zone = berlin();
        let day = starts(&zone, &[ShugoFestival], utc(2026, 3, 28, 23, 0) - 1, utc(2026, 3, 29, 23, 0));
        assert_eq!(day.len(), 24, "23 real hours between the first and last start of the day");
        assert!(day.windows(2).all(|pair| pair[1].0 - pair[0].0 == 60 * MINUTE_MS));
        let clocks: Vec<String> = day.iter().take(5).map(|(start, _)| clock_time(&zone, *start).to_string()).collect();
        assert_eq!(clocks, ["00:00", "01:00", "03:00", "04:00", "05:00"], "02:00 does not exist");
    }

    #[test]
    fn spring_forward_moves_a_skipped_wall_time_to_the_end_of_the_gap_and_merges() {
        let zone = berlin();
        let events = [ShugoFestival, RiftPortals, DimensionalInvasion];
        let found = starts(&zone, &events, utc(2026, 3, 29, 0, 0), utc(2026, 3, 29, 3, 0));
        let seen: Vec<(String, Vec<EventId>)> =
            found.into_iter().map(|(start, events)| (clock_time(&zone, start).to_string(), events)).collect();
        assert_eq!(
            seen,
            vec![
                ("01:30".to_string(), vec![DimensionalInvasion]),
                ("03:00".to_string(), vec![ShugoFestival, RiftPortals]),
                ("03:30".to_string(), vec![DimensionalInvasion]),
                ("04:00".to_string(), vec![ShugoFestival]),
                ("04:30".to_string(), vec![DimensionalInvasion]),
            ],
            "the skipped 02:00 Rift happens at 03:00 with Shugo Festival, once; the hourly Invasion keeps every real :30"
        );
    }

    #[test]
    fn fall_back_hourly_events_run_twice_in_the_repeated_hour() {
        let zone = berlin();
        let day = starts(&zone, &[ShugoFestival], utc(2026, 10, 24, 22, 0) - 1, utc(2026, 10, 25, 23, 0));
        assert!(day.windows(2).all(|pair| pair[1].0 - pair[0].0 == 60 * MINUTE_MS), "one start per real hour");
        let clocks: Vec<String> = day.iter().take(6).map(|(start, _)| clock_time(&zone, *start).to_string()).collect();
        assert_eq!(clocks, ["00:00", "01:00", "02:00", "02:00", "03:00", "04:00"]);
    }

    #[test]
    fn fall_back_fixed_wall_times_happen_once_at_their_first_occurrence() {
        let zone = berlin();
        let found = starts(&zone, &[RiftPortals], utc(2026, 10, 24, 22, 0), utc(2026, 10, 25, 23, 0));
        let at_two: Vec<_> = found.iter().filter(|(start, _)| clock_time(&zone, *start).hour == 2).collect();
        assert_eq!(at_two.len(), 1);
        assert_eq!(at_two[0].0, utc(2026, 10, 25, 0, 0), "02:00 CEST, not the repeated 02:00 CET at 01:00 UTC");
        let gaps: Vec<i64> = found.windows(2).map(|pair| (pair[1].0 - pair[0].0) / (60 * MINUTE_MS)).collect();
        assert_eq!(gaps.iter().filter(|&&hours| hours == 4).count(), 1, "the repeated hour is the only stretch longer than 3 h");
    }

    #[test]
    fn a_weekly_time_inside_the_skipped_hour_falls_on_the_end_of_the_gap() {
        let zone = TestZone { base: 60, changes: vec![(utc(2026, 10, 5, 20, 0), 120)] };
        let reminder = next_reminder(&zone, &[ArtifactSiege], 1, utc(2026, 10, 5, 18, 0)).unwrap();
        assert_eq!(reminder.start_ms, utc(2026, 10, 5, 20, 0));
        assert_eq!(clock_time(&zone, reminder.start_ms).to_string(), "22:00", "the clock jumped from 21:00 to 22:00");
    }
}
