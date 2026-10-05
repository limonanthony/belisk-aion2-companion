//! The reminder card (docs/gpui-mockups/approved/reminders.html, values in
//! tokens.md, "Reminder card"): its own overlay window on the meter's left
//! edge, up for 30 s with a thin line draining the time left. The pointer over
//! the card pauses it; a click dismisses it. While the meter is locked the
//! card is click-through and shows no ×.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, relative, rgba, size, Animation, AnimationExt as _, AnyElement, App, Context, ElementId,
    FontWeight, Pixels, Render, Role, SharedString, Size, Subscription, Task, Window, WindowHandle,
};

use super::clock::{now_ms, LocalZone};
use super::events::{EventId, Rule};
use super::schedule::{clock_time, next_reminder, ClockTime};
use super::service::ShowReminder;
use super::settings::ReminderSettings;
use crate::assets::Icon;
use crate::meter::{board_width, Layout, LockChanged, MeterView, OverlaySettings};
use crate::overlay;
use crate::secondary::chrome::{self, Look, Scale, FAINT, GAUGE_EMPTY, MUTED};
use crate::secondary::OnMeter;
use crate::settings;

pub const SHOWN_FOR: Duration = Duration::from_secs(30);
const FADE: Duration = Duration::from_millis(150);
/// The line loses about a pixel per tick at 100%.
const TICK: Duration = Duration::from_millis(100);
/// Between the card and the meter, at 100%.
pub const GAP: f32 = 6.0;
const CAPSULE_WIDTH: f32 = 220.0;
/// The card with one event, border included, at 100%; each further event
/// adds its block and the 7 px above it.
const ONE_EVENT_HEIGHT: f32 = 84.0;
const EVENT_HEIGHT: f32 = 39.0;

/// The meter the card belongs to: its lock, and the window the card sits on.
pub struct MeterLink {
    pub window: WindowHandle<MeterView>,
    pub placement: Rc<OnMeter>,
}

/// 30 s that stand still while paused.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Countdown {
    left: Duration,
    running_since: Option<Instant>,
}

impl Countdown {
    fn started(now: Instant) -> Self {
        Self { left: SHOWN_FOR, running_since: Some(now) }
    }

    fn left(&self, now: Instant) -> Duration {
        match self.running_since {
            Some(since) => self.left.saturating_sub(now.saturating_duration_since(since)),
            None => self.left,
        }
    }

    fn pause(&mut self, now: Instant) {
        if self.running_since.is_some() {
            self.left = self.left(now);
            self.running_since = None;
        }
    }

    fn resume(&mut self, now: Instant) {
        self.running_since.get_or_insert(now);
    }

    fn is_running(&self) -> bool {
        self.running_since.is_some()
    }

    fn fraction_left(&self, now: Instant) -> f32 {
        self.left(now).as_secs_f32() / SHOWN_FOR.as_secs_f32()
    }
}

/// What the card says, fixed when it is shown.
struct Card {
    when: SharedString,
    /// The start on the clock; none on the test card.
    at: Option<SharedString>,
    /// Name and one line of fact per event, the rarer first.
    events: Vec<(SharedString, SharedString)>,
}

impl Card {
    fn of(request: &ShowReminder, settings: &ReminderSettings) -> Self {
        if request.is_test {
            return Self {
                when: "Test reminder".into(),
                at: None,
                events: vec![("Reminders look like this".into(), next_line(settings).into())],
            };
        }
        let minutes = request.starts_in().as_millis().div_ceil(60_000);
        let when = if minutes == 0 { "Starting now".to_string() } else { format!("In {minutes} min") };
        Self {
            when: when.into(),
            at: Some(request.starts_at.to_string().into()),
            events: rarest_first(&request.events)
                .into_iter()
                .map(|id| (id.info().name.into(), detail(id, request.starts_at).into()))
                .collect(),
        }
    }

    fn label(&self) -> SharedString {
        let mut label = format!("Event reminder: {}", self.when);
        if let Some(at) = &self.at {
            label.push_str(&format!(", {at}"));
        }
        for (name, detail) in &self.events {
            label.push_str(&format!(". {name}: {detail}"));
        }
        label.into()
    }
}

/// Weekly, then daily, then every 3 hours, then hourly; catalogue order
/// within each.
fn rarest_first(events: &[EventId]) -> Vec<EventId> {
    let mut events = events.to_vec();
    events.sort_by_key(|id| match id.info().rule {
        Rule::Weekly { .. } => 0,
        Rule::Daily { .. } => 1,
        Rule::EveryHours { .. } => 2,
        Rule::Hourly { .. } => 3,
    });
    events
}

fn detail(id: EventId, starts_at: ClockTime) -> &'static str {
    match id {
        EventId::ShugoFestival => "3 min to join",
        EventId::DimensionalInvasion => "Somewhere in the field \u{b7} 3 min to join",
        EventId::RiftPortals => "Altgard \u{b7} Verteron \u{b7} Morheim \u{b7} Eltnen",
        EventId::WatcherKaira => "Field boss \u{b7} Chaotic Lower Reshanta",
        EventId::ArtifactSiege => "Chaotic Lower Reshanta",
        EventId::AbyssFieldBosses => "Lower and Middle Reshanta",
        EventId::GuardianLordNahma => "Lv 65, and the Enraged Lv 80",
        EventId::ArenaOfTactics if starts_at.hour < 19 => "10 vs 10 queue open until 14:00",
        EventId::ArenaOfTactics => "10 vs 10 queue open until 21:00",
        EventId::RewardKeys => "Shugo Festival and Invasion keys",
    }
}

/// The test card's line: the next real reminder, or why there is none.
fn next_line(settings: &ReminderSettings) -> String {
    let scheduled = settings.scheduled_events();
    match next_reminder(&LocalZone, &scheduled, settings.lead_minutes, now_ms()) {
        Some(next) => {
            let names: Vec<&str> = rarest_first(&next.events).into_iter().map(|id| id.info().name).collect();
            format!("Next: {} at {}", join_names(&names), clock_time(&LocalZone, next.start_ms))
        }
        None if !settings.enabled => "Event reminders are off".to_string(),
        None => "No event is switched on".to_string(),
    }
}

fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The card's width at 100%: the meter's, or a fixed width beside the
/// capsule, whose own width follows its figures.
fn card_width(overlay: &OverlaySettings) -> f32 {
    if overlay.layout == Layout::Capsule {
        CAPSULE_WIDTH
    } else {
        board_width(overlay)
    }
}

/// The window's size before its first frame measures the card, in logical
/// pixels.
pub fn expected_size(request: &ShowReminder, overlay: &OverlaySettings) -> Size<Pixels> {
    let blocks = if request.is_test { 1 } else { request.events.len().max(1) };
    let scale = overlay.scale();
    size(
        gpui::px(card_width(overlay) * scale),
        gpui::px((ONE_EVENT_HEIGHT + EVENT_HEIGHT * (blocks - 1) as f32) * scale),
    )
}

#[derive(Clone, Copy, PartialEq)]
struct Appearance {
    alpha: u8,
    scale: f32,
    width: f32,
}

impl Appearance {
    fn of(overlay: &OverlaySettings) -> Self {
        Self { alpha: overlay.background_alpha, scale: overlay.scale(), width: card_width(overlay) }
    }

    fn look(self) -> Look {
        Look { s: Scale(self.scale), alpha: self.alpha }
    }
}

pub struct ReminderView {
    card: Card,
    countdown: Countdown,
    /// Counts the cards shown in this window, so a replacement fades in anew.
    shows: u64,
    /// The fade out is under way; the window goes when it ends.
    leaving: Option<Task<()>>,
    locked: bool,
    pointer_over: bool,
    appearance: Appearance,
    meter: Option<MeterLink>,
    /// The card's size at the last frame, for placing the window as the meter
    /// moves.
    card_size: Rc<Cell<Option<Size<Pixels>>>>,
    /// Tests and dev captures cannot move a real pointer, so they set it here.
    #[cfg(any(test, feature = "probes"))]
    forced_pointer: Option<bool>,
    _ticker: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl ReminderView {
    /// `meter` is none where there is no meter window (tests): the card then
    /// neither places itself nor follows a lock.
    pub fn new(request: &ShowReminder, meter: Option<MeterLink>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = settings::global(cx);
        let mut subscriptions = vec![cx.observe(&store, |this: &mut Self, store, cx| {
            let appearance = Appearance::of(&store.read(cx).get().overlay);
            if appearance != this.appearance {
                this.appearance = appearance;
                cx.notify();
            }
        })];
        let mut locked = false;
        if let Some(link) = &meter {
            locked = link.window.read_with(cx, |view, _| view.is_locked()).unwrap_or(false);
            if let Ok(meter_view) = link.window.entity(cx) {
                subscriptions.push(cx.subscribe_in(&meter_view, window, |this, _, event: &LockChanged, window, cx| {
                    this.set_locked(event.0, window, cx)
                }));
            }
        }
        let (reminders, overlay) = {
            let settings = store.read(cx).get();
            (settings.reminders.clone(), settings.overlay.clone())
        };
        let ticker = cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(TICK).await;
            if this.update_in(cx, |this, window, cx| this.tick(window, cx)).is_err() {
                break;
            }
        });
        let mut view = Self {
            card: Card::of(request, &reminders),
            countdown: Countdown::started(cx.background_executor().now()),
            shows: 0,
            leaving: None,
            locked: false,
            pointer_over: false,
            appearance: Appearance::of(&overlay),
            meter,
            card_size: Rc::new(Cell::new(None)),
            #[cfg(any(test, feature = "probes"))]
            forced_pointer: None,
            _ticker: ticker,
            _subscriptions: subscriptions,
        };
        if locked {
            view.set_locked(true, window, cx);
        }
        view
    }

    /// A newer reminder takes the place of the one up, with its own 30 s.
    pub fn show(&mut self, request: &ShowReminder, cx: &mut Context<Self>) {
        let settings = settings::global(cx).read(cx).get().reminders.clone();
        self.card = Card::of(request, &settings);
        let now = cx.background_executor().now();
        self.countdown = Countdown::started(now);
        if self.pointer_over {
            self.countdown.pause(now);
        }
        self.shows += 1;
        self.leaving = None;
        cx.notify();
    }

    #[cfg(feature = "probes")]
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    fn set_locked(&mut self, locked: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.meter.is_some() {
            if let Err(error) = overlay::set_click_through(window, locked) {
                eprintln!("cannot {} the reminder: {error}", if locked { "lock" } else { "unlock" });
                return;
            }
        }
        self.locked = locked;
        self.note_pointer(self.pointer_is_over(window), cx);
        cx.notify();
    }

    fn pointer_is_over(&self, window: &Window) -> bool {
        #[cfg(any(test, feature = "probes"))]
        if let Some(forced) = self.forced_pointer {
            return forced && !self.locked;
        }
        !self.locked && window.is_window_hovered()
    }

    fn note_pointer(&mut self, over: bool, cx: &App) {
        if over == self.pointer_over {
            return;
        }
        self.pointer_over = over;
        let now = cx.background_executor().now();
        if over {
            self.countdown.pause(now);
        } else {
            self.countdown.resume(now);
        }
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn force_pointer(&mut self, over: Option<bool>, window: &mut Window, cx: &mut Context<Self>) {
        self.forced_pointer = over;
        self.note_pointer(self.pointer_is_over(window), cx);
        cx.notify();
    }

    /// For dev captures: the line as it stands with `fraction` of the time
    /// left.
    #[cfg(feature = "probes")]
    pub fn force_time_left(&mut self, fraction: f32, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        self.countdown.left = SHOWN_FOR.mul_f32(fraction.clamp(0.0, 1.0));
        self.countdown.running_since = self.countdown.running_since.map(|_| now);
        cx.notify();
    }

    #[cfg(feature = "probes")]
    pub fn time_left(&self, cx: &App) -> Duration {
        self.countdown.left(cx.background_executor().now())
    }

    fn gap(&self) -> f32 {
        GAP * self.appearance.scale
    }

    /// Keeps the card on the meter, and the line and the time-out running.
    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let (Some(link), Some(card)) = (&self.meter, self.card_size.get()) {
            link.placement.place(card, self.gap(), window, cx);
        }
        if self.leaving.is_some() || !self.countdown.is_running() {
            return;
        }
        if self.countdown.left(cx.background_executor().now()).is_zero() {
            self.dismiss(window, cx);
        } else {
            cx.notify();
        }
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.leaving.is_some() {
            return;
        }
        if cx.reduce_motion() {
            self.leaving = Some(Task::ready(()));
            window.remove_window();
            return;
        }
        self.leaving = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(FADE).await;
            let _ = this.update_in(cx, |_, window, _| window.remove_window());
        }));
        cx.notify();
    }

    fn head(&self, s: Scale, cx: &mut Context<Self>) -> impl IntoElement {
        let dismiss = (!self.locked).then(|| {
            chrome::tool_button("dismiss", Icon::Close, self.pointer_over, s)
                .role(Role::Button)
                .aria_label("Dismiss")
                .on_click(cx.listener(|this, _, window, cx| this.dismiss(window, cx)))
        });
        div()
            .flex_none()
            .h(s.px(22.))
            .pl(s.px(4.))
            .mr(s.px(-2.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .child(chrome::icon(Icon::Bell, 13., rgba(MUTED), s))
            .child(div().flex_none().text_size(s.px(12.5)).font_weight(FontWeight::SEMIBOLD).child(self.card.when.clone()))
            .when_some(self.card.at.clone(), |head, at| {
                head.child(div().flex_none().text_color(rgba(FAINT)).child("\u{b7}")).child(
                    div()
                        .flex_none()
                        .text_size(s.px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(MUTED))
                        .child(at),
                )
            })
            .child(div().flex_1())
            .children(dismiss)
    }

    fn events(&self, s: Scale) -> impl Iterator<Item = gpui::Div> + '_ {
        self.card.events.iter().enumerate().map(move |(index, (name, detail))| {
            div()
                .mt(s.px(if index == 0 { 4. } else { 7. }))
                .pl(s.px(23.))
                .pr(s.px(4.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .line_height(s.px(17.))
                        .text_size(s.px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(name.clone()),
                )
                .child(
                    div()
                        .line_height(s.px(15.))
                        .text_size(s.px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(MUTED))
                        .truncate()
                        .child(detail.clone()),
                )
        })
    }
}

fn time_left_line(fraction: f32, s: Scale) -> impl IntoElement {
    div()
        .mt(s.px(8.))
        .mx(s.px(4.))
        .h(s.px(2.))
        .rounded(s.px(1.))
        .bg(rgba(GAUGE_EMPTY))
        .overflow_hidden()
        .child(div().h_full().w(relative(fraction.clamp(0.0, 1.0))).rounded(s.px(1.)).bg(rgba(FAINT)))
}

impl Render for ReminderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let over = self.pointer_is_over(window);
        self.note_pointer(over, cx);
        let now = cx.background_executor().now();
        let look = self.appearance.look();
        let s = look.s;

        let card = chrome::card(self.appearance.width, look)
            .pt(s.px(8.))
            .pb(s.px(6.))
            .overflow_hidden()
            .id("reminder")
            .debug_selector(|| "reminder".to_string())
            .role(Role::Status)
            .aria_label(self.card.label())
            .child(self.head(s, cx))
            .children(self.events(s))
            .child(time_left_line(self.countdown.fraction_left(now), s))
            .when(!self.locked, |card| {
                card.cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.dismiss(window, cx)))
            });
        let card: AnyElement = if cx.reduce_motion() {
            card.into_any_element()
        } else if self.leaving.is_some() {
            card.with_animation(
                ElementId::NamedInteger("reminder-out".into(), self.shows),
                Animation::new(FADE),
                |card, t| card.opacity(1. - t),
            )
            .into_any_element()
        } else {
            card.with_animation(
                ElementId::NamedInteger("reminder-in".into(), self.shows),
                Animation::new(FADE),
                |card, t| card.opacity(t),
            )
            .into_any_element()
        };

        let card_size = self.card_size.clone();
        let placement = self.meter.as_ref().map(|link| link.placement.clone());
        let gap = self.gap();
        chrome::surface(look)
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(card) = bounds.first() else {
                    return;
                };
                let measured = size(card.origin.x + card.size.width, card.origin.y + card.size.height);
                card_size.set(Some(measured));
                if let Some(placement) = &placement {
                    placement.place(measured, gap, window, cx);
                }
            })
            .child(card)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};

    use super::*;
    use crate::settings::{install, AppSettings, Settings};

    fn request(events: Vec<EventId>, hour: u8) -> ShowReminder {
        ShowReminder { events, start_ms: now_ms() + 60_000, starts_at: ClockTime { hour, minute: 0 }, is_test: false }
    }

    fn open(cx: &mut TestAppContext, first: ShowReminder) -> (Entity<ReminderView>, &mut VisualTestContext) {
        cx.update(|cx| {
            let store = cx.new(|_| Settings::in_memory(AppSettings::default()));
            install(&store, cx);
        });
        cx.add_window_view(|window, cx| ReminderView::new(&first, None, window, cx))
    }

    fn is_open(cx: &VisualTestContext) -> bool {
        !cx.windows().is_empty()
    }

    fn names(view: &Entity<ReminderView>, cx: &mut VisualTestContext) -> Vec<String> {
        cx.update(|_, cx| view.read(cx).card.events.iter().map(|(name, _)| name.to_string()).collect())
    }

    fn wait(cx: &mut VisualTestContext, duration: Duration) {
        cx.executor().advance_clock(duration);
        cx.run_until_parked();
    }

    #[test]
    fn events_at_the_same_minute_come_rarest_first() {
        let order = rarest_first(&[EventId::ShugoFestival, EventId::RiftPortals, EventId::ArenaOfTactics]);
        assert_eq!(order, vec![EventId::ArenaOfTactics, EventId::RiftPortals, EventId::ShugoFestival]);
        let order = rarest_first(&[EventId::ShugoFestival, EventId::ArtifactSiege]);
        assert_eq!(order, vec![EventId::ArtifactSiege, EventId::ShugoFestival]);
    }

    #[test]
    fn the_arena_line_names_the_queue_window_it_opens() {
        assert_eq!(detail(EventId::ArenaOfTactics, ClockTime { hour: 11, minute: 0 }), "10 vs 10 queue open until 14:00");
        assert_eq!(detail(EventId::ArenaOfTactics, ClockTime { hour: 19, minute: 0 }), "10 vs 10 queue open until 21:00");
    }

    #[test]
    fn the_header_rounds_the_time_left_up_to_the_minute() {
        let mut soon = request(vec![EventId::ShugoFestival], 16);
        soon.start_ms = now_ms() + 61_000;
        let card = Card::of(&soon, &ReminderSettings::default());
        assert_eq!(card.when.as_ref(), "In 2 min");
        assert_eq!(card.at.as_deref(), Some("16:00"));
    }

    #[test]
    fn the_test_card_names_the_next_reminder_or_why_there_is_none() {
        let test = ShowReminder { is_test: true, ..request(vec![EventId::ShugoFestival], 16) };
        let card = Card::of(&test, &ReminderSettings::default());
        assert_eq!(card.when.as_ref(), "Test reminder");
        assert_eq!(card.at, None);
        assert!(card.events[0].1.starts_with("Next: "), "{}", card.events[0].1);

        let off = ReminderSettings { enabled: false, ..ReminderSettings::default() };
        assert_eq!(Card::of(&test, &off).events[0].1.as_ref(), "Event reminders are off");
    }

    #[gpui::test]
    fn a_newer_reminder_replaces_the_one_up_with_its_own_30_s(cx: &mut TestAppContext) {
        let (view, cx) = open(cx, request(vec![EventId::ShugoFestival], 16));
        wait(cx, Duration::from_secs(20));
        assert_eq!(names(&view, cx), vec!["Shugo Festival"]);

        let newer = request(vec![EventId::ShugoFestival, EventId::RiftPortals], 2);
        cx.update(|_, cx| view.update(cx, |view, cx| view.show(&newer, cx)));
        assert_eq!(names(&view, cx), vec!["Rift portals", "Shugo Festival"]);

        wait(cx, Duration::from_secs(20));
        assert!(is_open(cx), "the newer card has 10 s left");
        wait(cx, Duration::from_secs(11));
        wait(cx, FADE);
        assert!(!is_open(cx), "and goes when they are over");
    }

    #[gpui::test]
    fn the_card_goes_after_30_s_and_the_pointer_over_it_pauses_the_time(cx: &mut TestAppContext) {
        let (view, cx) = open(cx, request(vec![EventId::DimensionalInvasion], 15));
        wait(cx, Duration::from_secs(20));
        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(Some(true), window, cx)));
        wait(cx, Duration::from_secs(60));
        assert!(is_open(cx), "paused under the pointer");

        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(Some(false), window, cx)));
        wait(cx, Duration::from_secs(9));
        assert!(is_open(cx), "10 s were left when the pointer came");
        wait(cx, Duration::from_secs(2));
        wait(cx, FADE);
        assert!(!is_open(cx), "30 s in all, then the fade");
    }

    #[gpui::test]
    fn a_click_on_the_card_dismisses_it(cx: &mut TestAppContext) {
        let (_, cx) = open(cx, request(vec![EventId::ShugoFestival], 16));
        let card = cx.debug_bounds("reminder").expect("the card is drawn");
        cx.simulate_click(card.center(), gpui::Modifiers::none());
        assert!(is_open(cx), "it fades out first");
        wait(cx, FADE);
        assert!(!is_open(cx));
    }
}
