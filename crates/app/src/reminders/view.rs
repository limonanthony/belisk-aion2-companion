//! The reminder card (docs/gpui-mockups/approved/reminders.html, values in
//! tokens.md, "Reminder card"): its own overlay window on the meter's left
//! edge, or under the toolbar while the meter is closed (toolbar.html) or
//! steps aside between fights, up for 30 s with a timer ring emptying beside
//! the ×. The pointer over the card pauses it; a click dismisses it. While
//! the meter is locked the card is click-through and shows no ×.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    canvas, div, point, prelude::*, px, rgba, size, Animation, AnimationExt as _, AnyElement, App, Bounds, Context,
    ElementId, FontWeight, PathBuilder, Pixels, Render, Role, SharedString, Size, Subscription, Task, Window,
    WindowHandle,
};

use super::events::{EventId, Rule};
use super::planner::CardKind;
use super::schedule::next_reminder;
use super::service::ShowReminder;
use super::settings::ReminderSettings;
use crate::assets::Icon;
use crate::clock::{clock_time, now_ms, ClockTime, LocalZone};
use crate::i18n::{self, fl, join_and};
use crate::meter::{board_width, Layout, LockChanged, MeterView, OverlaySettings};
use crate::overlay;
use crate::secondary::chrome::{self, Look, Scale, FAINT, GAUGE_EMPTY, MUTED, TEXT};
use crate::secondary::Stacking;
use crate::settings;

pub const SHOWN_FOR: Duration = Duration::from_secs(30);
/// The cards' fade in and out.
pub const FADE: Duration = Duration::from_millis(150);
/// The ring moves a degree per tick, the slowest that keeps its arc smooth.
const TICK: Duration = Duration::from_nanos(SHOWN_FOR.as_nanos() as u64 / 360);
/// Between the card and the meter, at 100%.
pub const GAP: f32 = 6.0;
const CAPSULE_WIDTH: f32 = 220.0;
/// The card with one event, border included, at 100%; each further event
/// adds its block and the 7 px above it.
const ONE_EVENT_HEIGHT: f32 = 81.0;
const EVENT_HEIGHT: f32 = 39.0;

/// The meter the card belongs to: its lock, and where the card sits.
pub struct MeterLink {
    pub window: WindowHandle<MeterView>,
    pub placement: Rc<Stacking>,
}

/// A card's time on screen that stands still while paused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countdown {
    total: Duration,
    left: Duration,
    running_since: Option<Instant>,
}

impl Countdown {
    pub fn started(total: Duration, now: Instant) -> Self {
        Self { total, left: total, running_since: Some(now) }
    }

    pub fn left(&self, now: Instant) -> Duration {
        match self.running_since {
            Some(since) => self.left.saturating_sub(now.saturating_duration_since(since)),
            None => self.left,
        }
    }

    pub fn pause(&mut self, now: Instant) {
        if self.running_since.is_some() {
            self.left = self.left(now);
            self.running_since = None;
        }
    }

    pub fn resume(&mut self, now: Instant) {
        self.running_since.get_or_insert(now);
    }

    pub fn is_running(&self) -> bool {
        self.running_since.is_some()
    }

    /// How long to wait for the next frame: a `tick`, or less so the last
    /// one lands when the time runs out.
    pub fn next_tick(&self, now: Instant, tick: Duration) -> Duration {
        let left = self.left(now);
        if self.is_running() && !left.is_zero() { tick.min(left) } else { tick }
    }

    pub fn fraction_left(&self, now: Instant) -> f32 {
        self.left(now).as_secs_f32() / self.total.as_secs_f32()
    }

    /// For dev captures: `fraction` of the time left from `now`.
    #[cfg(feature = "probes")]
    pub fn set_fraction_left(&mut self, fraction: f32, now: Instant) {
        self.left = self.total.mul_f32(fraction.clamp(0.0, 1.0));
        self.running_since = self.running_since.map(|_| now);
    }
}

/// What the card says, fixed when it is shown.
struct Card {
    kind: CardKind,
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
                kind: CardKind::Reminder,
                when: fl!("reminder-test-title").into(),
                at: None,
                events: vec![(fl!("reminder-test-event").into(), next_line(settings).into())],
            };
        }
        let when = match request.kind {
            CardKind::StartAlert => fl!("reminder-under-way"),
            CardKind::Reminder => {
                let minutes = request.starts_in().as_millis().div_ceil(60_000) as i64;
                if minutes == 0 { fl!("reminder-starting-now") } else { fl!("reminder-in-minutes", minutes = minutes) }
            }
        };
        Self {
            kind: request.kind,
            when: when.into(),
            at: Some(request.starts_at.to_string().into()),
            events: rarest_first(&request.events)
                .into_iter()
                .map(|id| (id.name().into(), detail(id, request.starts_at).into()))
                .collect(),
        }
    }

    fn label(&self) -> SharedString {
        let when = self.when.to_string();
        let mut label = match self.kind {
            CardKind::Reminder => fl!("reminder-label", when = when),
            CardKind::StartAlert => fl!("reminder-start-label", when = when),
        };
        if let Some(at) = &self.at {
            label.push_str(&format!(", {at}"));
        }
        for (name, detail) in &self.events {
            label.push_str(&format!(". {}", fl!("reminder-label-event", name = name.to_string(), detail = detail.to_string())));
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

fn detail(id: EventId, starts_at: ClockTime) -> String {
    match id {
        EventId::ShugoFestival => fl!("reminder-detail-shugo-festival"),
        EventId::DimensionalInvasion => fl!("reminder-detail-dimensional-invasion"),
        EventId::RiftPortals => fl!("reminder-detail-rift-portals"),
        EventId::WatcherKaira => fl!("reminder-detail-watcher-kaira"),
        EventId::ArtifactSiege => fl!("reminder-detail-artifact-siege"),
        EventId::AbyssFieldBosses => fl!("reminder-detail-abyss-field-bosses"),
        EventId::GuardianLordNahma => fl!("reminder-detail-guardian-lord-nahma"),
        EventId::ArenaOfTactics if starts_at.hour < 19 => fl!("reminder-detail-arena-of-tactics-early"),
        EventId::ArenaOfTactics => fl!("reminder-detail-arena-of-tactics-late"),
        EventId::RewardKeys => fl!("reminder-detail-reward-keys"),
    }
}

/// The test card's line: the next real reminder, or why there is none.
fn next_line(settings: &ReminderSettings) -> String {
    let scheduled = settings.scheduled_events();
    match next_reminder(&LocalZone, &scheduled, settings.lead_minutes, now_ms()) {
        Some(next) => {
            let names: Vec<String> = rarest_first(&next.events).into_iter().map(EventId::name).collect();
            fl!("reminder-next", names = join_and(&names), time = clock_time(&LocalZone, next.start_ms).to_string())
        }
        None if !settings.enabled => fl!("reminder-off"),
        None => fl!("reminder-none-on"),
    }
}

/// The card's width at 100%: the meter's, or a fixed width beside the
/// capsule, whose own width follows its figures.
pub fn card_width(overlay: &OverlaySettings) -> f32 {
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
    /// What the card was built from, to build it again in another language.
    request: ShowReminder,
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
        let locale = i18n::locale(cx);
        subscriptions.push(cx.observe(&locale, |this: &mut Self, _, cx| {
            let settings = settings::global(cx).read(cx).get().reminders.clone();
            this.card = Card::of(&this.request, &settings);
            cx.notify();
        }));
        let (reminders, overlay) = {
            let settings = store.read(cx).get();
            (settings.reminders.clone(), settings.overlay.clone())
        };
        let ticker = cx.spawn_in(window, async move |this, cx| loop {
            let Ok(wait) = this.read_with(cx, |this, cx| this.countdown.next_tick(cx.background_executor().now(), TICK)) else {
                break;
            };
            cx.background_executor().timer(wait).await;
            if this.update_in(cx, |this, window, cx| this.tick(window, cx)).is_err() {
                break;
            }
        });
        let mut view = Self {
            card: Card::of(request, &reminders),
            request: request.clone(),
            countdown: Countdown::started(SHOWN_FOR, cx.background_executor().now()),
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
        self.request = request.clone();
        let now = cx.background_executor().now();
        self.countdown = Countdown::started(SHOWN_FOR, now);
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
            if let Err(error) = overlay::set_click_through(window, cx, locked) {
                crate::app_log::warn(format!("cannot {} the reminder: {error}", if locked { "lock" } else { "unlock" }));
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
        self.countdown.set_fraction_left(fraction, now);
        cx.notify();
    }

    #[cfg(feature = "probes")]
    pub fn time_left(&self, cx: &App) -> Duration {
        self.countdown.left(cx.background_executor().now())
    }

    fn gap(&self) -> f32 {
        GAP * self.appearance.scale
    }

    /// Keeps the card in its place, and the line and the time-out running.
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

    fn timer_ring(&self, now: Instant) -> TimerRing {
        TimerRing { left: self.countdown.fraction_left(now), paused: self.pointer_over, solo: self.locked }
    }

    fn head(&self, s: Scale, now: Instant, cx: &mut Context<Self>) -> impl IntoElement {
        let dismiss = (!self.locked).then(|| {
            chrome::tool_button("dismiss", Icon::Close, fl!("common-dismiss"), self.pointer_over, s)
                .on_click(cx.listener(|this, _, window, cx| this.dismiss(window, cx)))
        });
        let ring = self.timer_ring(now);
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
            .child(ring.element(s))
            .children(dismiss)
    }

    fn events(&self, s: Scale) -> impl Iterator<Item = gpui::Div> + '_ {
        self.card.events.iter().enumerate().map(move |(index, (name, detail))| {
            div()
                .mt(s.px(if index == 0 { 4. } else { 7. }))
                .pl(s.px(4.))
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

/// The cards' timer ring (tokens.md, "Timer ring"): the time left as an arc
/// over a full-circle track, emptying clockwise from 12 o'clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimerRing {
    /// The time left, 0 to 1.
    pub left: f32,
    /// The pointer is over the card: the arc stands still and brightens.
    pub paused: bool,
    /// No × beside it (the card is locked): the ring takes the ×'s slot.
    pub solo: bool,
}

impl TimerRing {
    const SIDE: f32 = 12.;
    const STROKE: f32 = 2.;

    /// Where the arc starts, in degrees clockwise from 12 o'clock; it always
    /// ends at 12 o'clock. None once no time is left.
    pub fn arc_start(self) -> Option<f32> {
        let left = self.left.clamp(0., 1.);
        (left > 0.).then(|| (1. - left) * 360.)
    }

    pub fn arc_colour(self) -> u32 {
        if self.paused { TEXT } else { MUTED }
    }

    pub fn slot_width(self) -> f32 {
        if self.solo { 22. } else { 16. }
    }

    /// The 16 × 22 px slot, 8 px before the × (the header's gap and 2 px),
    /// or the ×'s own 22 px slot when there is no ×.
    pub fn element(self, s: Scale) -> impl IntoElement {
        div()
            .id("timer-ring")
            .debug_selector(|| "timer-ring".to_string())
            .role(Role::Timer)
            .aria_label(fl!("common-time-left"))
            .flex_none()
            .w(s.px(self.slot_width()))
            .h(s.px(22.))
            .when(!self.solo, |slot| slot.mr(s.px(2.)))
            .flex()
            .items_center()
            .justify_center()
            .child(canvas(|_, _, _| {}, move |bounds, _, window, _| self.paint(bounds, s, window)).size(s.px(Self::SIDE)))
    }

    fn paint(self, bounds: Bounds<Pixels>, s: Scale, window: &mut Window) {
        let centre = bounds.center();
        let radius = s.px((Self::SIDE - Self::STROKE) / 2.);
        let on_circle = |degrees: f32| {
            let radians = degrees.to_radians();
            point(centre.x + radius * radians.sin(), centre.y - radius * radians.cos())
        };
        let arc = |from: f32, to: f32| {
            let mut path = PathBuilder::stroke(s.px(Self::STROKE));
            path.move_to(on_circle(from));
            // Halves, so a full turn has distinct start and end points.
            let middle = (from + to) / 2.;
            for (start, end) in [(from, middle), (middle, to)] {
                path.arc_to(point(radius, radius), px(0.), end - start > 180., true, on_circle(end));
            }
            path.build()
        };
        let mut paint = |from: f32, colour: u32| match arc(from, 360.) {
            Ok(path) => window.paint_path(path, rgba(colour)),
            Err(error) => crate::app_log::warn(format!("cannot draw the card's timer ring: {error:?}")),
        };
        paint(0., GAUGE_EMPTY);
        if let Some(start) = self.arc_start() {
            paint(start, self.arc_colour());
        }
    }
}

impl Render for ReminderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let over = self.pointer_is_over(window);
        self.note_pointer(over, cx);
        let now = cx.background_executor().now();
        let look = self.appearance.look();
        let s = look.s;

        // 9 px above and 12 px below put 14 px between the border and the
        // ink on all four sides: the header's glyphs sit 4 px inside its
        // row, the last line's 2 px inside its line.
        let card = chrome::card(self.appearance.width, look)
            .pt(s.px(9.))
            .pb(s.px(12.))
            .overflow_hidden()
            .id("reminder")
            .debug_selector(|| "reminder".to_string())
            .role(Role::Status)
            .aria_label(self.card.label())
            .child(self.head(s, now, cx))
            .children(self.events(s))
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
    use crate::reminders::EVENTS;
    use crate::settings::{install, AppSettings, Settings};

    fn request(events: Vec<EventId>, hour: u8) -> ShowReminder {
        ShowReminder {
            events,
            start_ms: now_ms() + 60_000,
            starts_at: ClockTime { hour, minute: 0 },
            kind: CardKind::Reminder,
            is_test: false,
        }
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
        assert_eq!(detail(EventId::ArenaOfTactics, ClockTime { hour: 11, minute: 0 }), fl!("reminder-detail-arena-of-tactics-early"));
        assert_eq!(detail(EventId::ArenaOfTactics, ClockTime { hour: 19, minute: 0 }), fl!("reminder-detail-arena-of-tactics-late"));
    }

    #[test]
    fn the_header_rounds_the_time_left_up_to_the_minute() {
        let mut soon = request(vec![EventId::ShugoFestival], 16);
        soon.start_ms = now_ms() + 61_000;
        let card = Card::of(&soon, &ReminderSettings::default());
        assert_eq!(card.when.as_ref(), fl!("reminder-in-minutes", minutes = 2));
        assert_eq!(card.at.as_deref(), Some("16:00"));
    }

    #[test]
    fn a_start_alert_says_under_way_with_the_start_time_and_has_its_own_label() {
        let started = ShowReminder { kind: CardKind::StartAlert, start_ms: now_ms() - 5_000, ..request(vec![EventId::ShugoFestival], 16) };
        let alert = Card::of(&started, &ReminderSettings::default());
        assert_eq!(alert.when.as_ref(), fl!("reminder-under-way"));
        assert_eq!(alert.at.as_deref(), Some("16:00"));
        assert_eq!(alert.events.len(), 1);

        let reminder = Card::of(&request(vec![EventId::ShugoFestival], 16), &ReminderSettings::default());
        assert_ne!(alert.when, reminder.when);
        assert_ne!(alert.label(), reminder.label());
    }

    #[test]
    fn the_test_card_names_the_next_reminder_or_why_there_is_none() {
        let test = ShowReminder { is_test: true, ..request(vec![EventId::ShugoFestival], 16) };
        let card = Card::of(&test, &ReminderSettings::default());
        assert_eq!(card.when.as_ref(), fl!("reminder-test-title"));
        assert_eq!(card.at, None);
        let (off, none_on) = (fl!("reminder-off"), fl!("reminder-none-on"));
        assert!(card.events[0].1 != off && card.events[0].1 != none_on, "the next reminder, got {}", card.events[0].1);

        let switched_off = ReminderSettings { enabled: false, ..ReminderSettings::default() };
        assert_eq!(Card::of(&test, &switched_off).events[0].1.as_ref(), off);

        let mut nothing_on = ReminderSettings::default();
        for info in EVENTS.iter() {
            nothing_on.set_event_enabled(info.id, false);
        }
        assert_eq!(Card::of(&test, &nothing_on).events[0].1.as_ref(), none_on);
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

    #[gpui::test]
    fn a_card_that_timed_out_is_gone_and_released_so_the_next_reminder_opens_a_visible_one(cx: &mut TestAppContext) {
        let (first, handle) = {
            let (view, cx) = open(cx, request(vec![EventId::ShugoFestival], 0));
            let first = view.downgrade();
            let handle = cx.windows()[0];
            drop(view);
            wait(cx, SHOWN_FOR);
            wait(cx, FADE);
            assert!(!is_open(cx), "30 s and the fade are over");
            assert!(cx.update_window(handle, |_, _, _| ()).is_err(), "nothing can still be shown into it");
            (first, handle)
        };
        assert!(first.upgrade().is_none(), "the view is released, so the registry's release hook has run");
        let _ = handle;

        cx.executor().advance_clock(Duration::from_secs(30 * 60));
        cx.run_until_parked();
        let (view, cx) = open(cx, request(vec![EventId::DimensionalInvasion], 0));
        assert!(is_open(cx));
        assert!(cx.debug_bounds("reminder").is_some(), "the new card is drawn");
        assert_eq!(names(&view, cx), vec!["Dimensional Invasion"]);
    }

    #[gpui::test]
    fn a_reminder_arriving_while_the_card_fades_out_keeps_the_window_and_the_new_card(cx: &mut TestAppContext) {
        let (view, cx) = open(cx, request(vec![EventId::ShugoFestival], 16));
        let card = cx.debug_bounds("reminder").expect("the card is drawn");
        cx.simulate_click(card.center(), gpui::Modifiers::none());
        assert!(is_open(cx), "fading out");

        let newer = request(vec![EventId::DimensionalInvasion], 16);
        cx.update(|_, cx| view.update(cx, |view, cx| view.show(&newer, cx)));
        wait(cx, FADE);
        wait(cx, FADE);
        assert!(is_open(cx), "the fade out was cancelled");
        assert_eq!(names(&view, cx), vec!["Dimensional Invasion"]);
    }

    #[test]
    fn the_ring_empties_clockwise_from_twelve_and_always_ends_at_twelve() {
        let ring = |left: f32| TimerRing { left, paused: false, solo: false };
        assert_eq!(ring(1.).arc_start(), Some(0.), "full: the whole turn");
        assert_eq!(ring(0.75).arc_start(), Some(90.), "a quarter gone: the start has moved to 3 o'clock");
        assert_eq!(ring(0.25).arc_start(), Some(270.));
        assert_eq!(ring(0.).arc_start(), None, "no arc once the time is up");
        assert_eq!(ring(1.5).arc_start(), Some(0.));
    }

    #[test]
    fn the_ring_brightens_while_paused_and_takes_the_x_slot_without_an_x() {
        let ring = TimerRing { left: 0.5, paused: false, solo: false };
        assert_eq!(ring.arc_colour(), MUTED);
        assert_eq!(TimerRing { paused: true, ..ring }.arc_colour(), TEXT);
        assert_eq!(ring.slot_width(), 16.);
        assert_eq!(TimerRing { solo: true, ..ring }.slot_width(), 22.);
    }

    #[gpui::test]
    fn the_ring_follows_the_time_left_stops_under_the_pointer_and_takes_the_x_slot_when_locked(cx: &mut TestAppContext) {
        let (view, cx) = open(cx, request(vec![EventId::ShugoFestival], 16));
        let ring = |view: &Entity<ReminderView>, cx: &mut VisualTestContext| {
            cx.update(|_, cx| view.read(cx).timer_ring(cx.background_executor().now()))
        };
        assert!((ring(&view, cx).left - 1.).abs() < 1e-3, "a new card starts full");
        let card = cx.debug_bounds("reminder").expect("the card is drawn");
        assert_eq!(card.size.height, gpui::px(81.), "one event");
        assert_eq!(cx.debug_bounds("timer-ring").map(|slot| slot.size.width), Some(gpui::px(16.)));

        wait(cx, Duration::from_secs(15));
        let half = ring(&view, cx);
        assert!((half.left - 0.5).abs() < 0.01 && !half.paused, "{half:?}");

        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(Some(true), window, cx)));
        wait(cx, Duration::from_secs(5));
        let paused = ring(&view, cx);
        assert!(paused.paused && (paused.left - half.left).abs() < 0.01, "the arc stands still under the pointer: {paused:?}");
        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(None, window, cx)));

        cx.update(|window, cx| view.update(cx, |view, cx| view.set_locked(true, window, cx)));
        cx.run_until_parked();
        assert!(ring(&view, cx).solo);
        assert_eq!(cx.debug_bounds("timer-ring").map(|slot| slot.size.width), Some(gpui::px(22.)), "the ring in the ×'s slot");
    }
}
