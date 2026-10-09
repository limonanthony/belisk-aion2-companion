use std::sync::Arc;
use std::time::Duration;

use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global, Task};
use meter_core::logger::AppLogger;

use super::events::EventId;
use super::planner::{Planner, Step};
use super::schedule::Reminder;
use super::settings::ReminderSettings;
use super::sound;
use crate::clock::{clock_time, now_ms, ClockTime, LocalZone};
use crate::decision_log::DecisionLog;
use crate::settings::Settings;

/// A wake later than this after it was due is written to the decision log.
const LATE_WAKE_MS: i64 = 5_000;

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
    now_ms: fn() -> i64,
    /// When the sleep the loop is in should end; checked at the next wake.
    wake_due_ms: Option<i64>,
    log: DecisionLog,
}

impl EventEmitter<ShowReminder> for Reminders {}

struct RemindersHandle(Entity<Reminders>);

impl Global for RemindersHandle {}

impl Reminders {
    /// `follows_clock` false keeps the schedule off, leaving only
    /// [`Self::test_reminder`]; `play_sound` runs for each reminder shown while
    /// the `sound` setting is on; `now_ms` is the clock the schedule follows.
    pub fn new(
        settings: &Entity<Settings>,
        follows_clock: bool,
        play_sound: fn(&App),
        now_ms: fn() -> i64,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(settings, |this, settings, cx| {
            let (reminders, decision_log) = {
                let all = settings.read(cx).get();
                (all.reminders.clone(), all.capture.decision_log)
            };
            let was_logging = this.log.enabled();
            this.log.set_enabled(decision_log);
            if reminders != this.config {
                this.config = reminders;
                this.replan(cx);
            } else if !was_logging {
                this.log_plan();
            }
        })
        .detach();
        let mut log = DecisionLog::new("reminders", None);
        log.set_enabled(settings.read(cx).get().capture.decision_log);
        let mut reminders = Self {
            config: settings.read(cx).get().reminders.clone(),
            planner: Planner::default(),
            schedule: None,
            follows_clock,
            play_sound,
            now_ms,
            wake_due_ms: None,
            log,
        };
        reminders.replan(cx);
        reminders
    }

    /// Where the decision lines go; the core's logger exists only once the
    /// meter has started, so the plan made before it is written now.
    pub fn log_to(&mut self, logger: Arc<AppLogger>) {
        self.log.attach(logger);
        self.log_plan();
    }

    /// What became of the card a [`ShowReminder`] asked for, written by the
    /// window layer that handled it.
    pub fn log_card(&self, outcome: impl FnOnce() -> String) {
        self.log.event("card", outcome);
    }

    /// Shows a sample reminder now, as if Shugo Festival were about to start
    /// after the chosen lead time, whatever the switches say.
    pub fn test_reminder(&mut self, cx: &mut Context<Self>) {
        let start_ms = (self.now_ms)() + i64::from(self.config.lead_minutes) * 60_000;
        self.send(vec![EventId::ShugoFestival], start_ms, true, cx);
    }

    fn replan(&mut self, cx: &mut Context<Self>) {
        self.schedule = None;
        self.wake_due_ms = None;
        self.log_plan();
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

    fn log_plan(&self) {
        self.log.event("service", || {
            if !self.follows_clock {
                return "stopped: this run does not follow the clock".to_string();
            }
            let events = self.config.scheduled_events();
            if events.is_empty() {
                return "stopped: the master switch is off or no event is on".to_string();
            }
            format!("started: {events:?}, {} min ahead, sound {}", self.config.lead_minutes, self.config.sound)
        });
    }

    /// Shows everything due and returns how long to sleep, or `None` when
    /// nothing is scheduled.
    fn advance(&mut self, cx: &mut Context<Self>) -> Option<Duration> {
        let woke_at = (self.now_ms)();
        if let Some(due) = self.wake_due_ms.take().filter(|due| woke_at - due > LATE_WAKE_MS) {
            self.log.event("late wake", || {
                format!("expected {} ({due} ms), woke {} ({woke_at} ms): {} ms late", clock_time(&LocalZone, due), clock_time(&LocalZone, woke_at), woke_at - due)
            });
        }
        loop {
            let now = (self.now_ms)();
            let step = self.planner.step(&LocalZone, &self.config, now);
            if let Some(missed) = self.planner.take_missed() {
                self.log.event("dropped", || {
                    format!(
                        "{:?} starting at {} ({} ms) had started when the clock was read at {} ({now} ms); its reminder was due {}",
                        missed.events,
                        clock_time(&LocalZone, missed.start_ms),
                        missed.start_ms,
                        clock_time(&LocalZone, now),
                        clock_time(&LocalZone, missed.notify_ms),
                    )
                });
            }
            match step {
                Step::Show(Reminder { events, start_ms, .. }) => self.send(events, start_ms, false, cx),
                Step::Wait(wait) => {
                    let upcoming = self.planner.upcoming();
                    self.log.on_change("next", || match upcoming {
                        Some(next) => format!(
                            "{:?} at {} ({} ms), reminder due {}",
                            next.events,
                            clock_time(&LocalZone, next.start_ms),
                            next.start_ms,
                            clock_time(&LocalZone, next.notify_ms),
                        ),
                        None => "none".to_string(),
                    });
                    self.wake_due_ms = Some(now + wait.as_millis() as i64);
                    return Some(wait);
                }
                Step::Idle => {
                    self.log.event("service", || "idle: nothing is scheduled".to_string());
                    return None;
                }
            }
        }
    }

    fn send(&self, events: Vec<EventId>, start_ms: i64, is_test: bool, cx: &mut Context<Self>) {
        let starts_at = clock_time(&LocalZone, start_ms);
        self.log.event("card", || {
            format!("due for {events:?} starting at {starts_at} ({start_ms} ms), test {is_test}, sound {}", self.config.sound)
        });
        if self.config.sound {
            (self.play_sound)(cx);
        }
        cx.emit(ShowReminder { events, start_ms, starts_at, is_test });
    }
}

/// Starts the reminders and makes them the app's, for [`global`]. `live` is a
/// real run: the schedule follows the clock and the sound plays. Replay,
/// capture and probe runs pass false, which leaves `test_reminder` silent and
/// the schedule off.
pub fn install(settings: &Entity<Settings>, live: bool, cx: &mut App) {
    let play_sound: fn(&App) = if live { sound::play_notification } else { |_| {} };
    let reminders = cx.new(|cx| Reminders::new(settings, live, play_sound, now_ms, cx));
    cx.set_global(RemindersHandle(reminders));
}

/// Sends the reminders' decision lines to the core's log.
pub fn log_to(logger: Arc<AppLogger>, cx: &mut App) {
    global(cx).update(cx, |reminders, _| reminders.log_to(logger));
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
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, false, |_| {}, now_ms, cx)));
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
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, false, count_sound, now_ms, cx)));
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
        let follows = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, true, |_| {}, now_ms, cx)));
        let stays_off = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, false, |_| {}, now_ms, cx)));
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

    thread_local! {
        static CLOCK: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
    }
    const MINUTE: i64 = 60_000;

    fn fake_now() -> i64 {
        CLOCK.get()
    }

    fn next_local_midnight(after_ms: i64) -> i64 {
        let mut at = (after_ms / MINUTE + 1) * MINUTE;
        while clock_time(&LocalZone, at).to_string() != "00:00" {
            at += MINUTE;
        }
        at
    }

    #[gpui::test]
    fn the_service_shows_the_midnight_shugo_festival_once_and_the_next_invasion_once(cx: &mut TestAppContext) {
        let midnight = next_local_midnight(now_ms());
        CLOCK.set(midnight - 12 * MINUTE);
        let settings = settings(cx);
        change(cx, &settings, |reminders| reminders.lead_minutes = 5);
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, true, |_| {}, fake_now, cx)));
        let shown = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = shown.clone();
        cx.update(|cx| {
            cx.subscribe(&reminders, move |_, event: &ShowReminder, _| sink.borrow_mut().push((fake_now(), event.clone())))
                .detach();
        });
        cx.run_until_parked();

        for step in 0..76 {
            if step == 10 {
                change(cx, &settings, |reminders| reminders.sound = false);
            }
            CLOCK.set(CLOCK.get() + 30_000);
            cx.executor().advance_clock(Duration::from_secs(30));
            cx.run_until_parked();
        }

        let shown: Vec<(i64, i64, Vec<EventId>)> =
            shown.borrow().iter().map(|(at, request)| (*at, request.start_ms, request.events.clone())).collect();
        let due = |start: i64| (start - 5 * MINUTE, start - 5 * MINUTE + 30_000);
        assert_eq!(shown.len(), 2, "one card per reminder: {shown:?}");
        assert_eq!((shown[0].1, shown[0].2.clone()), (midnight, vec![EventId::ShugoFestival]));
        assert!((due(midnight).0..=due(midnight).1).contains(&shown[0].0), "shown at {}", shown[0].0);
        assert_eq!((shown[1].1, shown[1].2.clone()), (midnight + 30 * MINUTE, vec![EventId::DimensionalInvasion]));
        assert!((due(midnight + 30 * MINUTE).0..=due(midnight + 30 * MINUTE).1).contains(&shown[1].0));
    }

    #[gpui::test]
    fn changing_settings_while_waiting_leaves_one_loop_and_one_card(cx: &mut TestAppContext) {
        let midnight = next_local_midnight(now_ms());
        CLOCK.set(midnight - 12 * MINUTE);
        let settings = settings(cx);
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, true, |_| {}, fake_now, cx)));
        let shown = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = shown.clone();
        cx.update(|cx| {
            cx.subscribe(&reminders, move |_, event: &ShowReminder, _| sink.borrow_mut().push(event.start_ms)).detach();
        });
        for lead in [2, 3, 5, 1, 5] {
            change(cx, &settings, |reminders| reminders.lead_minutes = lead);
        }
        cx.run_until_parked();

        for _ in 0..40 {
            CLOCK.set(CLOCK.get() + 30_000);
            cx.executor().advance_clock(Duration::from_secs(30));
            cx.run_until_parked();
        }

        assert_eq!(*shown.borrow(), vec![midnight]);
    }

    type Lines = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

    /// Reminders on the fake clock, started before a logger exists, then given
    /// one, with the decision log on.
    fn logging_reminders(cx: &mut TestAppContext, name: &str) -> (Entity<Reminders>, Lines) {
        let settings = settings(cx);
        cx.update(|cx| {
            settings.update(cx, |settings, cx| {
                settings.update(cx, |all| {
                    all.capture.decision_log = true;
                    all.reminders.lead_minutes = 5;
                })
            })
        });
        let reminders = cx.update(|cx| cx.new(|cx| Reminders::new(&settings, true, |_| {}, fake_now, cx)));
        let lines: Lines = Lines::default();
        let sink = lines.clone();
        let path = std::env::temp_dir().join(format!("belisk-reminders-{}-{name}.log", std::process::id()));
        let logger = Arc::new(AppLogger::new(path, Box::new(move |event| sink.lock().unwrap().push(event.message))));
        cx.update(|cx| reminders.update(cx, |reminders, _| reminders.log_to(logger)));
        cx.run_until_parked();
        (reminders, lines)
    }

    #[gpui::test]
    fn the_decision_log_gets_the_start_line_even_though_the_logger_came_later(cx: &mut TestAppContext) {
        CLOCK.set(next_local_midnight(now_ms()) - 12 * MINUTE);
        let (_reminders, lines) = logging_reminders(cx, "start");

        let lines = lines.lock().unwrap();
        assert_eq!(
            lines[0],
            "[reminders] service: started: [ShugoFestival, DimensionalInvasion, RiftPortals], 5 min ahead, sound true"
        );
        assert!(lines[1].starts_with("[reminders] next: [ShugoFestival] at 00:00"), "{lines:?}");
        assert!(lines[1].ends_with("reminder due 23:55"), "{lines:?}");
    }

    #[gpui::test]
    fn a_card_that_is_due_and_a_reminder_that_was_slept_through_are_both_written(cx: &mut TestAppContext) {
        let midnight = next_local_midnight(now_ms());
        CLOCK.set(midnight - 12 * MINUTE);
        let (_reminders, lines) = logging_reminders(cx, "late");

        CLOCK.set(midnight - 5 * MINUTE + 1_000);
        cx.executor().advance_clock(Duration::from_secs(30));
        cx.run_until_parked();
        CLOCK.set(midnight + 40 * MINUTE);
        cx.executor().advance_clock(Duration::from_secs(30));
        cx.run_until_parked();

        let lines = lines.lock().unwrap().join("\n");
        assert!(lines.contains("[reminders] card: due for [ShugoFestival] starting at 00:00"), "{lines}");
        assert!(lines.contains("[reminders] late wake: expected 23:48"), "{lines}");
        assert!(lines.contains("[reminders] late wake: expected 23:55"), "{lines}");
        assert!(lines.contains("[reminders] dropped: [DimensionalInvasion] starting at 00:30"), "{lines}");
    }
}
