use std::time::Duration;

use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global, Task};

use super::events::EventId;
use super::planner::{Planner, Step};
use super::schedule::Reminder;
use super::settings::ReminderSettings;
use super::sound;
use crate::clock::{clock_time, now_ms, ClockTime, LocalZone};
use crate::settings::Settings;

/// Asks the view to show a reminder window. Sent on the UI thread; the sound
/// has already been started when it is sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowReminder {
    /// What starts, in catalogue order; never empty.
    pub events: Vec<EventId>,
    pub start_ms: i64,
    pub starts_at: ClockTime,
    pub is_test: bool,
}

impl ShowReminder {
    /// Time left until the start; zero once it has started.
    pub fn starts_in(&self) -> Duration {
        Duration::from_millis((self.start_ms - now_ms()).max(0) as u64)
    }
}

/// Plans the reminders and sends [`ShowReminder`] when one is due. Lives on the
/// UI thread and sleeps on a GPUI timer between reminders; nothing here runs on
/// the meter's threads.
pub struct Reminders {
    config: ReminderSettings,
    planner: Planner,
    schedule: Option<Task<()>>,
    follows_clock: bool,
    play_sound: fn(&App),
}

impl EventEmitter<ShowReminder> for Reminders {}

struct RemindersHandle(Entity<Reminders>);

impl Global for RemindersHandle {}

impl Reminders {
    /// `follows_clock` false keeps the schedule off, leaving only
    /// [`Self::test_reminder`]; `play_sound` runs for each reminder shown while
    /// the `sound` setting is on.
    pub fn new(settings: &Entity<Settings>, follows_clock: bool, play_sound: fn(&App), cx: &mut Context<Self>) -> Self {
        cx.observe(settings, |this, settings, cx| {
            let changed = {
                let next = &settings.read(cx).get().reminders;
                (*next != this.config).then(|| next.clone())
            };
            if let Some(next) = changed {
                this.config = next;
                this.replan(cx);
            }
        })
        .detach();
        let mut reminders = Self {
            config: settings.read(cx).get().reminders.clone(),
            planner: Planner::default(),
            schedule: None,
            follows_clock,
            play_sound,
        };
        reminders.replan(cx);
        reminders
    }

    /// Shows a sample reminder now, as if Shugo Festival were about to start
    /// after the chosen lead time, whatever the switches say.
    pub fn test_reminder(&mut self, cx: &mut Context<Self>) {
        let start_ms = now_ms() + i64::from(self.config.lead_minutes) * 60_000;
        self.send(vec![EventId::ShugoFestival], start_ms, true, cx);
    }

    fn replan(&mut self, cx: &mut Context<Self>) {
        self.schedule = None;
        if !self.follows_clock || self.config.scheduled_events().is_empty() {
            return;
        }
        self.schedule = Some(cx.spawn(async move |this, cx| loop {
            let Ok(Some(wait)) = this.update(cx, |this, cx| this.advance(cx)) else {
                break;
            };
            cx.background_executor().timer(wait).await;
        }));
    }

    /// Shows everything due and returns how long to sleep, or `None` when
    /// nothing is scheduled.
    fn advance(&mut self, cx: &mut Context<Self>) -> Option<Duration> {
        loop {
            match self.planner.step(&LocalZone, &self.config, now_ms()) {
                Step::Show(Reminder { events, start_ms, .. }) => self.send(events, start_ms, false, cx),
                Step::Wait(wait) => return Some(wait),
                Step::Idle => return None,
            }
        }
    }

    fn send(&self, events: Vec<EventId>, start_ms: i64, is_test: bool, cx: &mut Context<Self>) {
        if self.config.sound {
            (self.play_sound)(cx);
        }
        let starts_at = clock_time(&LocalZone, start_ms);
        cx.emit(ShowReminder { events, start_ms, starts_at, is_test });
    }
}

/// Starts the reminders and makes them the app's, for [`global`]. `live` is a
/// real run: the schedule follows the clock and the sound plays. Replay,
/// capture and probe runs pass false, which leaves `test_reminder` silent and
/// the schedule off.
pub fn install(settings: &Entity<Settings>, live: bool, cx: &mut App) {
    let play_sound: fn(&App) = if live { sound::play_notification } else { |_| {} };
    let reminders = cx.new(|cx| Reminders::new(settings, live, play_sound, cx));
    cx.set_global(RemindersHandle(reminders));
}

pub fn global(cx: &App) -> Entity<Reminders> {
    cx.global::<RemindersHandle>().0.clone()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gpui::TestAppContext;

    use super::*;
    use crate::settings::AppSettings;

    static SOUNDS: AtomicUsize = AtomicUsize::new(0);

    fn count_sound(_: &App) {
        SOUNDS.fetch_add(1, Ordering::SeqCst);
    }

    fn settings(cx: &mut TestAppContext) -> Entity<Settings> {
        cx.update(|cx| cx.new(|_| Settings::in_memory(AppSettings::default())))
    }

    fn change(cx: &mut TestAppContext, settings: &Entity<Settings>, change: impl FnOnce(&mut ReminderSettings)) {
        cx.update(|cx| settings.update(cx, |settings, cx| settings.update(cx, |all| change(&mut all.reminders))));
    }

    #[gpui::test]
    fn a_test_reminder_names_shugo_festival_and_starts_after_the_lead_time(cx: &mut TestAppContext) {
        let settings = settings(cx);
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, false, |_| {}, cx)));
        let shown = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = shown.clone();
        cx.update(|cx| {
            cx.subscribe(&reminders, move |_, event: &ShowReminder, _| sink.borrow_mut().push(event.clone())).detach();
            reminders.update(cx, |reminders, cx| reminders.test_reminder(cx));
        });

        let shown = shown.borrow();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].events, vec![EventId::ShugoFestival]);
        assert!(shown[0].is_test);
        let lead = shown[0].starts_in();
        assert!(lead > Duration::from_secs(50) && lead <= Duration::from_secs(60), "{lead:?}");
    }

    #[gpui::test]
    fn the_sound_plays_only_while_the_sound_setting_is_on(cx: &mut TestAppContext) {
        let settings = settings(cx);
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, false, count_sound, cx)));
        let before = SOUNDS.load(Ordering::SeqCst);

        cx.update(|cx| reminders.update(cx, |reminders, cx| reminders.test_reminder(cx)));
        assert_eq!(SOUNDS.load(Ordering::SeqCst), before + 1);

        change(cx, &settings, |reminders| reminders.sound = false);
        cx.update(|cx| reminders.update(cx, |reminders, cx| reminders.test_reminder(cx)));
        assert_eq!(SOUNDS.load(Ordering::SeqCst), before + 1, "muted");
    }

    #[gpui::test]
    fn the_schedule_runs_only_while_reminders_are_on_and_follow_the_clock(cx: &mut TestAppContext) {
        let settings = settings(cx);
        let follows = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, true, |_| {}, cx)));
        let stays_off = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, false, |_| {}, cx)));
        let scheduled = |cx: &mut TestAppContext, reminders: &Entity<Reminders>| {
            cx.update(|cx| reminders.read(cx).schedule.is_some())
        };
        assert!(scheduled(cx, &follows));
        assert!(!scheduled(cx, &stays_off), "replay and probe runs have no schedule");

        change(cx, &settings, |reminders| reminders.enabled = false);
        assert!(!scheduled(cx, &follows), "the master switch stops the timer");

        change(cx, &settings, |reminders| reminders.enabled = true);
        assert!(scheduled(cx, &follows));

        change(cx, &settings, |reminders| {
            reminders.events.shugo_festival = false;
            reminders.events.dimensional_invasion = false;
            reminders.events.rift_portals = false;
        });
        assert!(!scheduled(cx, &follows), "no event left to wait for");
    }
}
