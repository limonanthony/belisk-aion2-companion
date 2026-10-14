//! What the reminders service does at a given moment: show a card (the
//! reminder before an event, or the start alert when it starts), sleep, or
//! stand idle. Pure and driven by the clock reading it is given, so a PC that
//! slept or a clock that jumped is handled by reading the clock again.

use std::time::Duration;

use super::schedule::{next_reminder, Reminder};
use super::settings::{ReminderSettings, LEAD_MINUTES_OPTIONS};
use crate::clock::Zone;

/// The longest sleep between two clock readings. A sleeping PC or a changed
/// clock is noticed within this long, since the timer cannot be trusted across
/// either.
pub const MAX_NAP: Duration = Duration::from_secs(30);

const LONGEST_LEAD_MS: i64 = LEAD_MINUTES_OPTIONS[LEAD_MINUTES_OPTIONS.len() - 1] as i64 * 60_000;

/// How long after its start a start alert may still be shown; a later wake
/// drops it.
const ALERT_WINDOW_MS: i64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKind {
    /// Shown the lead time before the event starts.
    Reminder,
    /// Shown when the event starts.
    StartAlert,
}

impl CardKind {
    /// How the decision log names the card.
    pub fn name(self) -> &'static str {
        match self {
            Self::Reminder => "reminder",
            Self::StartAlert => "start alert",
        }
    }
}

/// A card of one kind for the events starting at `reminder.start_ms`; for a
/// start alert `reminder.notify_ms` is the start itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    pub kind: CardKind,
    pub reminder: Reminder,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Show this card now; step again for the next.
    Show(Due),
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
    /// The start of the last start alert shown, so a replan within its window
    /// does not show the same one again.
    last_alerted_start_ms: Option<i64>,
    /// The reminder the last step waited for; none once it was shown or when
    /// nothing is scheduled.
    upcoming: Option<Reminder>,
    /// The start alert the last step waited for, under the same rules.
    upcoming_alert: Option<Reminder>,
    /// The cards the last step waited for that were never shown: a reminder
    /// whose event had started by the time of the next step, a start alert
    /// more than its window past its start.
    missed: Vec<Due>,
}

impl Planner {
    pub fn upcoming(&self) -> Option<&Reminder> {
        self.upcoming.as_ref()
    }

    pub fn upcoming_alert(&self) -> Option<&Reminder> {
        self.upcoming_alert.as_ref()
    }

    /// The cards the last step found past their time without having shown
    /// them, once.
    pub fn take_missed(&mut self) -> Vec<Due> {
        std::mem::take(&mut self.missed)
    }

    /// A reminder is shown when its time has come and its event has not started
    /// yet: one whose start has passed (the PC slept through it) is dropped. A
    /// start alert is shown from the start until [`ALERT_WINDOW_MS`] after it,
    /// at most once per start.
    pub fn step(&mut self, zone: &impl Zone, settings: &ReminderSettings, now_ms: i64) -> Step {
        let events = settings.scheduled_events();
        let still_on = |waited_for: &Reminder| waited_for.events.iter().any(|id| events.contains(id));
        self.missed.clear();
        if let Some(reminder) = self.upcoming.take().filter(|waited_for| waited_for.start_ms <= now_ms && still_on(waited_for)) {
            self.missed.push(Due { kind: CardKind::Reminder, reminder });
        }
        if let Some(reminder) =
            self.upcoming_alert.take().filter(|waited_for| now_ms - waited_for.start_ms > ALERT_WINDOW_MS && still_on(waited_for))
        {
            self.missed.push(Due { kind: CardKind::StartAlert, reminder });
        }

        let reminder_after_ms = match self.last_shown_start_ms {
            Some(shown) if shown - now_ms > LONGEST_LEAD_MS => {
                self.last_shown_start_ms = None;
                now_ms
            }
            Some(shown) => shown.max(now_ms),
            None => now_ms,
        };
        let reminder = next_reminder(zone, &events, settings.lead_minutes, reminder_after_ms);

        let oldest_alertable_after_ms = now_ms - ALERT_WINDOW_MS - 1;
        let alert_after_ms = match self.last_alerted_start_ms {
            Some(alerted) if alerted - now_ms > ALERT_WINDOW_MS => {
                self.last_alerted_start_ms = None;
                oldest_alertable_after_ms
            }
            Some(alerted) => alerted.max(oldest_alertable_after_ms),
            None => oldest_alertable_after_ms,
        };
        let alert = settings.start_alert.then(|| next_reminder(zone, &events, 0, alert_after_ms)).flatten();

        let reminder_due_ms = reminder.as_ref().map(|reminder| reminder.notify_ms);
        let alert_due_ms = alert.as_ref().map(|alert| alert.start_ms);
        let first_due = match (reminder_due_ms, alert_due_ms) {
            (None, None) => {
                return Step::Idle;
            }
            (Some(reminder_due), Some(alert_due)) if reminder_due <= now_ms && alert_due <= now_ms => {
                Some(if reminder_due <= alert_due { CardKind::Reminder } else { CardKind::StartAlert })
            }
            (Some(reminder_due), _) if reminder_due <= now_ms => Some(CardKind::Reminder),
            (_, Some(alert_due)) if alert_due <= now_ms => Some(CardKind::StartAlert),
            _ => None,
        };

        match first_due {
            Some(CardKind::Reminder) => {
                let reminder = reminder.expect("a reminder is due");
                self.last_shown_start_ms = Some(reminder.start_ms);
                self.upcoming_alert = alert;
                Step::Show(Due { kind: CardKind::Reminder, reminder })
            }
            Some(CardKind::StartAlert) => {
                let alert = alert.expect("a start alert is due");
                self.last_alerted_start_ms = Some(alert.start_ms);
                self.upcoming = reminder;
                Step::Show(Due { kind: CardKind::StartAlert, reminder: alert })
            }
            None => {
                let wake_ms = reminder_due_ms.into_iter().chain(alert_due_ms).min().expect("a card is pending");
                self.upcoming = reminder;
                self.upcoming_alert = alert;
                Step::Wait(Duration::from_millis((wake_ms - now_ms) as u64).min(MAX_NAP))
            }
        }
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

        let Step::Show(Due { kind, reminder }) = step(&mut planner, &settings, at(10, 29, 0)) else {
            panic!("the reminder is due");
        };
        assert_eq!(kind, CardKind::Reminder);
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
        let Step::Show(Due { reminder, .. }) = step(&mut planner, &settings, at(10, 59, 0)) else {
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
                Step::Show(card) => shown.push((now, card.reminder.start_ms)),
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
        assert_eq!(planner.take_missed(), vec![]);

        assert_eq!(step(&mut planner, &settings, at(10, 30, 5)), Step::Wait(MAX_NAP));
        let [missed] = planner.take_missed().try_into().expect("the 10:30 Invasion went by unannounced");
        assert_eq!(missed.kind, CardKind::Reminder);
        assert_eq!((missed.reminder.events, missed.reminder.start_ms), (vec![DimensionalInvasion], at(10, 30, 0)));
        assert_eq!(planner.take_missed(), vec![]);

        assert_eq!(step(&mut planner, &settings, at(10, 30, 35)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), vec![], "11:00 is still ahead");
    }

    #[test]
    fn a_reminder_that_was_shown_is_not_reported_missed() {
        let mut planner = Planner::default();
        let settings = ReminderSettings::default();
        assert_eq!(step(&mut planner, &settings, at(10, 20, 0)), Step::Wait(MAX_NAP));
        assert!(matches!(step(&mut planner, &settings, at(10, 29, 10)), Step::Show(_)));
        assert_eq!(step(&mut planner, &settings, at(10, 31, 0)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), vec![]);
    }

    fn with_start_alert() -> ReminderSettings {
        ReminderSettings { start_alert: true, ..ReminderSettings::default() }
    }

    fn shown_kind(planner: &mut Planner, settings: &ReminderSettings, now: i64) -> Option<(CardKind, i64)> {
        match step(planner, settings, now) {
            Step::Show(card) => Some((card.kind, card.reminder.start_ms)),
            Step::Wait(_) | Step::Idle => None,
        }
    }

    #[test]
    fn the_start_alert_follows_the_reminder_and_is_shown_at_the_start() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 29, 0)), Some((CardKind::Reminder, at(10, 30, 0))));
        assert_eq!(step(&mut planner, &settings, at(10, 29, 1)), Step::Wait(MAX_NAP));
        assert_eq!(step(&mut planner, &settings, at(10, 29, 50)), Step::Wait(Duration::from_secs(10)), "the alert is the next thing due");
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 30, 0)), Some((CardKind::StartAlert, at(10, 30, 0))));
        assert_eq!(step(&mut planner, &settings, at(10, 30, 1)), Step::Wait(MAX_NAP), "11:00's reminder is next");
    }

    #[test]
    fn a_start_alert_is_never_shown_twice_for_one_start_even_after_a_replan() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 30, 0)), Some((CardKind::StartAlert, at(10, 30, 0))));
        let louder = ReminderSettings { sound: false, lead_minutes: 2, ..with_start_alert() };
        for second in [1, 20, 59] {
            assert_eq!(shown_kind(&mut planner, &louder, at(10, 30, second)), None, "{second} s after the start");
        }
        assert_eq!(planner.take_missed(), vec![]);
    }

    #[test]
    fn a_start_alert_found_within_a_minute_of_the_start_is_shown_at_once() {
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &with_start_alert(), at(10, 31, 0)), Some((CardKind::StartAlert, at(10, 30, 0))));

        let mut late = Planner::default();
        assert_eq!(shown_kind(&mut late, &with_start_alert(), at(10, 31, 1)), None, "a second past the window");
    }

    #[test]
    fn turning_the_start_alert_on_inside_the_window_shows_it() {
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &ReminderSettings::default(), at(10, 30, 20)), None);
        assert_eq!(shown_kind(&mut planner, &with_start_alert(), at(10, 30, 30)), Some((CardKind::StartAlert, at(10, 30, 0))));
    }

    #[test]
    fn a_start_alert_slept_through_is_reported_missed_once_alongside_its_reminder() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        assert_eq!(step(&mut planner, &settings, at(10, 20, 0)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), vec![]);

        assert_eq!(step(&mut planner, &settings, at(10, 31, 30)), Step::Wait(MAX_NAP), "11:00 is next");
        let kinds: Vec<CardKind> = planner.take_missed().iter().map(|missed| missed.kind).collect();
        assert_eq!(kinds, vec![CardKind::Reminder, CardKind::StartAlert]);
        assert_eq!(planner.take_missed(), vec![]);
        assert_eq!(step(&mut planner, &settings, at(10, 31, 40)), Step::Wait(MAX_NAP));
        assert_eq!(planner.take_missed(), vec![], "reported once");
    }

    #[test]
    fn a_start_alert_slept_through_within_its_window_is_shown_not_dropped() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 29, 30)), Some((CardKind::Reminder, at(10, 30, 0))));
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 30, 40)), Some((CardKind::StartAlert, at(10, 30, 0))));
        assert_eq!(planner.take_missed(), vec![]);
    }

    #[test]
    fn with_the_start_alert_off_or_the_master_switch_off_only_reminders_come() {
        let mut off = Planner::default();
        let settings = ReminderSettings::default();
        assert!(!settings.start_alert, "off by default");
        assert_eq!(shown_kind(&mut off, &settings, at(10, 29, 0)), Some((CardKind::Reminder, at(10, 30, 0))));
        assert_eq!(shown_kind(&mut off, &settings, at(10, 30, 0)), None);
        assert_eq!(step(&mut off, &settings, at(10, 30, 30)), Step::Wait(MAX_NAP));
        assert_eq!(off.take_missed(), vec![]);

        let mut paused = Planner::default();
        let master_off = ReminderSettings { enabled: false, ..with_start_alert() };
        assert_eq!(step(&mut paused, &master_off, at(10, 30, 0)), Step::Idle);
    }

    #[test]
    fn events_sharing_a_start_make_one_start_alert() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        let Step::Show(Due { kind, reminder }) = step(&mut planner, &settings, at(11, 0, 0)) else {
            panic!("the 11:00 start alert is due");
        };
        assert_eq!((kind, reminder.events), (CardKind::StartAlert, vec![ShugoFestival, RiftPortals]));
        assert_eq!(shown_kind(&mut planner, &settings, at(11, 0, 1)), None);
    }

    #[test]
    fn a_start_alert_follows_the_reminder_of_the_same_start_and_each_is_shown_once() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 59, 59)), Some((CardKind::Reminder, at(11, 0, 0))));
        assert_eq!(shown_kind(&mut planner, &settings, at(11, 0, 0)), Some((CardKind::StartAlert, at(11, 0, 0))));
        assert_eq!(shown_kind(&mut planner, &settings, at(11, 0, 0)), None);
    }

    #[test]
    fn a_clock_set_back_shows_the_start_alerts_of_the_new_time() {
        let settings = with_start_alert();
        let mut planner = Planner::default();
        assert_eq!(shown_kind(&mut planner, &settings, at(10, 30, 0)), Some((CardKind::StartAlert, at(10, 30, 0))));
        assert_eq!(shown_kind(&mut planner, &settings, at(8, 30, 10)), Some((CardKind::StartAlert, at(8, 30, 0))));
    }
}
