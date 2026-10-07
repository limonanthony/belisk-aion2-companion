//! What the reminders service does at a given moment: show a reminder, sleep,
//! or stand idle. Pure and driven by the clock reading it is given, so a PC
//! that slept or a clock that jumped is handled by reading the clock again.

use std::time::Duration;

use super::schedule::{next_reminder, Reminder};
use super::settings::{ReminderSettings, LEAD_MINUTES_OPTIONS};
use crate::clock::Zone;

/// The longest sleep between two clock readings. A sleeping PC or a changed
/// clock is noticed within this long, since the timer cannot be trusted across
/// either.
pub const MAX_NAP: Duration = Duration::from_secs(30);

const LONGEST_LEAD_MS: i64 = LEAD_MINUTES_OPTIONS[LEAD_MINUTES_OPTIONS.len() - 1] as i64 * 60_000;

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Show this reminder now; step again for the next.
    Show(Reminder),
    /// Nothing is due; read the clock again after this long.
    Wait(Duration),
    /// Nothing is scheduled (the master switch is off or no event is on); a
    /// settings change is the only thing that can change that.
    Idle,
}

#[derive(Debug, Default)]
pub struct Planner {
    /// The start of the last reminder shown, so a replan within the lead time
    /// does not show the same one again.
    last_shown_start_ms: Option<i64>,
    /// The reminder the last step waited for; none once it was shown or when
    /// nothing is scheduled.
    upcoming: Option<Reminder>,
    /// The reminder the last step waited for whose event had started by the
    /// time of the next step, so it was never shown.
    missed: Option<Reminder>,
}

impl Planner {
    pub fn upcoming(&self) -> Option<&Reminder> {
        self.upcoming.as_ref()
    }

    /// The reminder the last step found already started without having shown
    /// it, once.
    pub fn take_missed(&mut self) -> Option<Reminder> {
        self.missed.take()
    }

    /// A reminder is shown when its time has come and its event has not started
    /// yet: one whose start has passed (the PC slept through it) is dropped.
    pub fn step(&mut self, zone: &impl Zone, settings: &ReminderSettings, now_ms: i64) -> Step {
        let events = settings.scheduled_events();
        self.missed = self
            .upcoming
            .take()
            .filter(|waited_for| waited_for.start_ms <= now_ms && waited_for.events.iter().any(|id| events.contains(id)));
        let after_ms = match self.last_shown_start_ms {
            Some(shown) if shown - now_ms > LONGEST_LEAD_MS => {
                self.last_shown_start_ms = None;
                now_ms
            }
            Some(shown) => shown.max(now_ms),
            None => now_ms,
        };
        let Some(reminder) = next_reminder(zone, &events, settings.lead_minutes, after_ms) else {
            self.upcoming = None;
            return Step::Idle;
        };
        if reminder.notify_ms > now_ms {
            let remaining = Duration::from_millis((reminder.notify_ms - now_ms) as u64);
            self.upcoming = Some(reminder);
            return Step::Wait(remaining.min(MAX_NAP));
        }
        self.upcoming = None;
        self.last_shown_start_ms = Some(reminder.start_ms);
        Step::Show(reminder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reminders::events::EventId::*;

    const SECOND: i64 = 1000;
    const MINUTE: i64 = 60 * SECOND;
    /// 2026-10-05 00:00 on a clock at UTC+2, in Unix milliseconds.
    const MIDNIGHT: i64 = (20_731 * 1440 - 120) * MINUTE;

    struct Summer;

    impl Zone for Summer {
        fn utc_offset_minutes(&self, _: i64) -> i32 {
            120
        }
    }

    fn at(hour: i64, minute: i64, second: i64) -> i64 {
        MIDNIGHT + hour * 60 * MINUTE + minute * MINUTE + second * SECOND
    }

    fn step(planner: &mut Planner, settings: &ReminderSettings, now: i64) -> Step {
        planner.step(&Summer, settings, now)
    }

    #[test]
    fn it_sleeps_until_the_reminder_but_never_longer_than_the_nap() {
        let settings = ReminderSettings::default();
        let mut planner = Planner::default();
        assert_eq!(step(&mut planner, &settings, at(10, 28, 50)), Step::Wait(Duration::from_secs(10)));
        assert_eq!(step(&mut planner, &settings, at(10, 5, 0)), Step::Wait(MAX_NAP));
    }

    #[test]
    fn a_reminder_is_shown_once_when_its_time_comes() {
        let settings = ReminderSettings::default();
        let mut planner = Planner::default();

        let Step::Show(reminder) = step(&mut planner, &settings, at(10, 29, 0)) else {
            panic!("the reminder is due");
        };
        assert_eq!((reminder.events, reminder.start_ms), (vec![DimensionalInvasion], at(10, 30, 0)));

        assert_eq!(step(&mut planner, &settings, at(10, 29, 1)), Step::Wait(MAX_NAP), "next is 11:00 at 10:59");
    }

    #[test]
    fn a_reminder_found_late_is_still_shown_while_its_event_has_not_started() {
        let mut planner = Planner::default();
        let settings = ReminderSettings::default();
        assert!(matches!(step(&mut planner, &settings, at(10, 29, 40)), Step::Show(_)));
    }

    #[test]
    fn a_reminder_whose_start_has_passed_is_dropped_after_the_pc_wakes() {
        let mut planner = Planner::default();
        let settings = ReminderSettings::default();
        assert_eq!(step(&mut planner, &settings, at(10, 30, 5)), Step::Wait(MAX_NAP), "10:30 is gone, 11:00 is next");
        let Step::Show(reminder) = step(&mut planner, &settings, at(10, 59, 0)) else {
            panic!("11:00 is due");
        };
        assert_eq!(reminder.events, vec![ShugoFestival, RiftPortals]);
    }

    #[test]
    fn a_longer_lead_time_shows_the_reminder_earlier() {
        let settings = ReminderSettings { lead_minutes: 5, ..ReminderSettings::default() };
        let mut planner = Planner::default();
        assert_eq!(step(&mut planner, &settings, at(10, 24, 55)), Step::Wait(Duration::from_secs(5)));
        assert!(matches!(step(&mut planner, &settings, at(10, 25, 0)), Step::Show(_)));
    }

    #[test]
    fn nothing_is_shown_with_the_master_switch_off_or_no_event_on() {
        let mut planner = Planner::default();
        let off = ReminderSettings { enabled: false, ..ReminderSettings::default() };
        assert_eq!(step(&mut planner, &off, at(10, 29, 30)), Step::Idle);

        let mut none = ReminderSettings::default();
        for id in [ShugoFestival, DimensionalInvasion, RiftPortals] {
            none.set_event_enabled(id, false);
        }
        assert_eq!(step(&mut planner, &none, at(10, 29, 30)), Step::Idle);
    }

    #[test]
    fn switching_the_master_switch_back_on_inside_the_lead_time_shows_the_pending_reminder() {
        let mut planner = Planner::default();
        let off = ReminderSettings { enabled: false, ..ReminderSettings::default() };
        assert_eq!(step(&mut planner, &off, at(10, 29, 30)), Step::Idle);
        assert!(matches!(step(&mut planner, &ReminderSettings::default(), at(10, 29, 40)), Step::Show(_)));
    }

    #[test]
    fn a_settings_change_after_a_reminder_does_not_show_it_again() {
        let mut planner = Planner::default();
        assert!(matches!(step(&mut planner, &ReminderSettings::default(), at(10, 29, 0)), Step::Show(_)));
        let louder = ReminderSettings { sound: false, lead_minutes: 2, ..ReminderSettings::default() };
        assert_eq!(step(&mut planner, &louder, at(10, 29, 20)), Step::Wait(MAX_NAP));
    }

    #[test]
    fn a_clock_set_back_shows_the_reminders_of_the_new_time() {
        let mut planner = Planner::default();
        let settings = ReminderSettings::default();
        assert!(matches!(step(&mut planner, &settings, at(10, 29, 0)), Step::Show(_)));
        assert!(matches!(step(&mut planner, &settings, at(8, 29, 30)), Step::Show(_)), "08:30 Invasion, an hour and a half back");
    }

    /// Runs the planner the way the service does, sleeping each `Wait`, and
    /// returns when each reminder was shown and the start it announced.
    fn shown_between(zone: &impl Zone, settings: &ReminderSettings, from: i64, until: i64) -> Vec<(i64, i64)> {
        let mut planner = Planner::default();
        let mut shown = Vec::new();
        let mut now = from;
        while now < until {
            match planner.step(zone, settings, now) {
                Step::Show(reminder) => shown.push((now, reminder.start_ms)),
                Step::Wait(wait) => now += wait.as_millis() as i64,
                Step::Idle => break,
            }
        }
        shown
    }

    #[test]
    fn the_midnight_shugo_festival_is_reminded_on_every_kind_of_day() {
        use crate::clock::testing::{berlin, utc};
        let zone = berlin();
        let midnights = [
            ("an ordinary day", utc(2026, 10, 5, 22, 0)),
            ("the day before spring forward", utc(2026, 3, 28, 23, 0)),
            ("the end of the 23-hour day", utc(2026, 3, 29, 22, 0)),
            ("the day before fall back", utc(2026, 10, 24, 22, 0)),
            ("the end of the 25-hour day", utc(2026, 10, 25, 23, 0)),
            ("a year change", utc(2026, 12, 31, 23, 0)),
        ];
        for lead_minutes in LEAD_MINUTES_OPTIONS {
            let settings = ReminderSettings { lead_minutes, ..ReminderSettings::default() };
            for (day, midnight) in midnights {
                let shown = shown_between(&zone, &settings, midnight - 40 * MINUTE, midnight + MINUTE);
                let reminder = shown.iter().find(|&&(_, start)| start == midnight);
                let due = midnight - i64::from(lead_minutes) * MINUTE;
                assert!(
                    reminder.is_some_and(|&(at, _)| at >= due && at < due + MAX_NAP.as_millis() as i64),
                    "{day}, lead {lead_minutes} min: {shown:?} for the start at {midnight}"
                );
            }
        }
    }

    #[test]
    fn a_reminder_whose_event_started_during_a_long_sleep_is_reported_missed_once() {
        let mut planner = Planner::default();
        let settings = ReminderSettings::default();
        assert_eq!(step(&mut planner, &settings, at(10, 20, 0)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), None);

        assert_eq!(step(&mut planner, &settings, at(10, 30, 5)), Step::Wait(MAX_NAP));
        let missed = planner.take_missed().expect("the 10:30 Invasion went by unannounced");
        assert_eq!((missed.events, missed.start_ms), (vec![DimensionalInvasion], at(10, 30, 0)));
        assert_eq!(planner.take_missed(), None);

        assert_eq!(step(&mut planner, &settings, at(10, 30, 35)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), None, "11:00 is still ahead");
    }

    #[test]
    fn a_reminder_that_was_shown_is_not_reported_missed() {
        let mut planner = Planner::default();
        let settings = ReminderSettings::default();
        assert_eq!(step(&mut planner, &settings, at(10, 20, 0)), Step::Wait(MAX_NAP));
        assert!(matches!(step(&mut planner, &settings, at(10, 29, 10)), Step::Show(_)));
        assert_eq!(step(&mut planner, &settings, at(10, 31, 0)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), None);
    }
}
