//! The soul tracker card (docs/gpui-mockups/approved/pets.html, values in
//! tokens.md, "Soul tracker card"): the reminder card's twin in its own
//! non-activating window, 6 px above the meter while it is open, under the
//! toolbar while it is closed, and beyond a reminder card that is up. It
//! shows up to three pets in the order their souls came, each with its level
//! and its souls toward the next one, and goes 10 s after its last update, as
//! the timer ring beside the × shows; the pointer over it pauses that time
//! and a click dismisses it. While the meter is locked the card is
//! click-through and shows no ×. When it goes it closes its window, as the
//! reminder card does; the next gain opens a new card.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, relative, rgb, rgba, size, Animation, AnimationExt as _,
    AnyElement, App, Context, Div, ElementId, FontWeight, Hsla, Pixels, Render, Role, SharedString, Size, Subscription,
    Task, Window,
};
use meter_core::pets::{PetState, MAX_LEVEL};

use super::badge::level_badge;
use super::gain::LatestGain;
use super::list::souls_of_needed;
use super::service::{self, ShowTestCard};
use crate::assets::Icon;
use crate::i18n::fl;
use crate::meter::{LockChanged, OverlaySettings};
use crate::overlay;
use crate::reminders::{card_width, Countdown, MeterLink, TimerRing, FADE, GAP};
use crate::secondary::chrome::{self, Look, Scale, FAINT, GOOD, MUTED, TEXT};
use crate::settings;

pub const SHOWN_FOR: Duration = Duration::from_secs(10);
/// The ring moves a degree per tick, the slowest that keeps its arc smooth.
const TICK: Duration = Duration::from_nanos(SHOWN_FOR.as_nanos() as u64 / 360);
const MOST_PETS: usize = 3;
/// The card with one pet, border included, at 100%; each further pet adds
/// its block and the 7 px above it.
const ONE_PET_HEIGHT: f32 = 100.;
const PET_HEIGHT: f32 = 57.;
const CHIP_GOOD: u32 = 0x86e3a824;
const BAR_TRACK: u32 = crate::style::TRACK;
const BAR_FROM: u32 = 0xfff0e124;
const BAR_TO: u32 = 0xfff0e180;

/// What the card is asked to show.
#[derive(Clone, Debug, PartialEq)]
pub enum CardRequest {
    /// Soul gains Settings › Pets lets through, oldest first.
    Gains(Vec<LatestGain>),
    Test(ShowTestCard),
}

impl CardRequest {
    fn pets(&self) -> usize {
        match self {
            Self::Gains(gains) => gains.len().clamp(1, MOST_PETS),
            Self::Test(_) => 1,
        }
    }
}

/// What a gain changed beyond the souls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Reached(u32),
    /// The gain took the pet from level 0: it is obtained.
    Unlocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardPet {
    pub pet_id: u32,
    pub state: PetState,
    pub change: Option<Change>,
}

impl CardPet {
    fn of(gain: &LatestGain) -> Self {
        let change = gain.reached_a_level().then_some(if gain.level_before == 0 {
            Change::Unlocked
        } else {
            Change::Reached(gain.level)
        });
        Self { pet_id: gain.pet_id, state: PetState { level: gain.level, souls: gain.souls }, change }
    }
}

/// The pets on the card, in arrival order, and whether it is the test card.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub pets: Vec<CardPet>,
    pub test: bool,
}

impl Shown {
    /// A pet already on the card is updated where it is; a newcomer goes at
    /// the end, and a fourth takes the oldest one's place. A real gain on
    /// the test card replaces it.
    fn apply(&mut self, request: &CardRequest) {
        match request {
            CardRequest::Test(test) => {
                *self = Shown { pets: vec![CardPet { pet_id: test.pet_id, state: test.state, change: None }], test: true };
            }
            CardRequest::Gains(gains) => {
                if self.test {
                    *self = Shown::default();
                }
                for gain in gains {
                    let pet = CardPet::of(gain);
                    match self.pets.iter_mut().find(|shown| shown.pet_id == pet.pet_id) {
                        Some(shown) => *shown = pet,
                        None => {
                            if self.pets.len() == MOST_PETS {
                                self.pets.remove(0);
                            }
                            self.pets.push(pet);
                        }
                    }
                }
            }
        }
    }
}

/// A pet's name in the language in force, or "Pet <id>" for an id the
/// bundled list does not know.
pub fn pet_name(pet_id: u32) -> String {
    crate::game_data::pet_name(pet_id).unwrap_or_else(|| fl!("pets-unknown", id = pet_id))
}

/// How a pet reads to accessibility tools: its level and souls toward the
/// next level, or that it is at the cap.
pub fn pet_label(pet_id: u32, state: PetState) -> String {
    let name = pet_name(pet_id);
    if state.level >= MAX_LEVEL {
        return fl!("pets-row-max", name = name, level = state.level);
    }
    let (souls, needed) = souls_of_needed(state);
    let next: u32 = state.level + 1;
    fl!("pets-row", name = name, level = state.level, souls = souls, needed = needed, next = next)
}

/// The window's size before its first frame measures the card, in logical
/// pixels.
pub fn expected_size(request: &CardRequest, overlay: &OverlaySettings) -> Size<Pixels> {
    let scale = overlay.scale();
    size(gpui::px(card_width(overlay) * scale), gpui::px((ONE_PET_HEIGHT + PET_HEIGHT * (request.pets() - 1) as f32) * scale))
}

/// Hands `deliver` the soul gains the card shows under Settings › Pets as
/// the game reports them, whether or not the meter is open. Gains already
/// reported when this starts are not handed over.
pub fn watch_gains(cx: &mut App, deliver: impl Fn(Vec<LatestGain>, &mut App) + 'static) -> Subscription {
    let pets = service::global(cx);
    let handled = Cell::new(pets.read(cx).latest_gain().map_or(0, |gain| gain.sequence));
    cx.observe(&pets, move |pets, cx| {
        let latest = pets.read(cx).latest_gain().map_or(0, |gain| gain.sequence);
        if latest == handled.get() {
            return;
        }
        let settings = settings::global(cx).read(cx).get().pets.clone();
        let gains = pets.read(cx).card_gains_after(handled.replace(latest), &settings);
        if !gains.is_empty() {
            deliver(gains, cx);
        }
    })
}

pub struct PetCardView {
    shown: Shown,
    countdown: Countdown,
    /// Counts the fades out, so one after the card came back runs anew.
    leaves: u64,
    /// The fade out is under way; the window goes when it ends.
    leaving: Option<Task<()>>,
    locked: bool,
    pointer_over: bool,
    look: Look,
    /// The reminder card's width: the meter's, or fixed beside the capsule.
    width: f32,
    meter: Option<MeterLink>,
    /// The card's size at the last frame, for placing the window as what it
    /// is stacked on moves.
    card_size: Rc<Cell<Option<Size<Pixels>>>>,
    /// Tests and dev captures cannot move a real pointer, so they set it here.
    #[cfg(any(test, feature = "probes"))]
    forced_pointer: Option<bool>,
    _ticker: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl PetCardView {
    /// `meter` is none where there is no meter window (tests): the card then
    /// neither places itself nor follows a lock.
    pub fn new(request: &CardRequest, meter: Option<MeterLink>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = settings::global(cx);
        let mut subscriptions = vec![cx.observe(&store, |this: &mut Self, store, cx| {
            let overlay = &store.read(cx).get().overlay;
            let (look, width) = (Look::of(overlay), card_width(overlay));
            if look.s.0 != this.look.s.0 || look.alpha != this.look.alpha || width != this.width {
                this.look = look;
                this.width = width;
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
        let ticker = cx.spawn_in(window, async move |this, cx| loop {
            let Ok(wait) = this.read_with(cx, |this, cx| this.countdown.next_tick(cx.background_executor().now(), TICK)) else {
                break;
            };
            cx.background_executor().timer(wait).await;
            if this.update_in(cx, |this, window, cx| this.tick(window, cx)).is_err() {
                break;
            }
        });
        let mut shown = Shown::default();
        shown.apply(request);
        let mut view = Self {
            shown,
            countdown: Countdown::started(SHOWN_FOR, cx.background_executor().now()),
            leaves: 0,
            leaving: None,
            locked: false,
            pointer_over: false,
            look: Look::of(&store.read(cx).get().overlay),
            width: card_width(&store.read(cx).get().overlay),
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

    /// More souls, or the test card: the card takes them and its 10 s start
    /// again, a fade under way included.
    pub fn show(&mut self, request: &CardRequest, cx: &mut Context<Self>) {
        self.shown.apply(request);
        let now = cx.background_executor().now();
        self.countdown = Countdown::started(SHOWN_FOR, now);
        if self.pointer_over {
            self.countdown.pause(now);
        }
        self.leaving = None;
        cx.notify();
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn shown(&self) -> &Shown {
        &self.shown
    }

    #[cfg(feature = "probes")]
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    fn set_locked(&mut self, locked: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.meter.is_some() {
            if let Err(error) = overlay::set_click_through(window, locked) {
                crate::app_log::warn(format!("cannot {} the soul tracker card: {error}", if locked { "lock" } else { "unlock" }));
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
        GAP * self.look.s.0
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
        self.leaves += 1;
        self.leaving = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(FADE).await;
            let _ = this.update_in(cx, |_, window, _| window.remove_window());
        }));
        cx.notify();
    }

    fn title(&self) -> String {
        if self.shown.test {
            fl!("pets-card-test")
        } else {
            fl!("pets-card-title")
        }
    }

    fn label(&self) -> SharedString {
        let mut label = self.title();
        for pet in &self.shown.pets {
            label.push_str(". ");
            label.push_str(&pet_label(pet.pet_id, pet.state));
            match pet.change {
                Some(Change::Reached(level)) => {
                    label.push_str(". ");
                    label.push_str(&fl!("pets-card-reached", level = level));
                }
                Some(Change::Unlocked) => {
                    label.push_str(". ");
                    label.push_str(&fl!("pets-card-unlocked"));
                }
                None => {}
            }
        }
        label.into()
    }

    fn timer_ring(&self, now: Instant) -> TimerRing {
        TimerRing { left: self.countdown.fraction_left(now), paused: self.pointer_over, solo: self.locked }
    }

    fn head(&self, s: Scale, now: Instant, cx: &mut Context<Self>) -> Div {
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
            .child(chrome::icon(Icon::PawPrint, 13., rgba(MUTED), s))
            .child(div().flex_none().text_size(s.px(12.5)).font_weight(FontWeight::SEMIBOLD).child(self.title()))
            .child(div().flex_1())
            .child(ring.element(s))
            .children(dismiss)
    }
}

/// One pet's block: the name and a level change, "Lv 2 28/75 → Lv 3", and
/// the bar of souls toward the next level.
fn pet_block(pet: &CardPet, first: bool, s: Scale) -> Div {
    let (souls, needed) = souls_of_needed(pet.state);
    let maxed = pet.state.level >= MAX_LEVEL;
    let fraction = souls as f32 / needed as f32;
    let width = if fraction > 0. { fraction.max(0.04) } else { 0. };
    let chip = pet.change.map(|change| {
        let (glyph, text) = match change {
            Change::Reached(level) => (Icon::ChevronsUp, fl!("pets-card-reached", level = level)),
            Change::Unlocked => (Icon::Sparkles, fl!("pets-card-unlocked")),
        };
        div()
            .flex_none()
            .h(s.px(18.))
            .px(s.px(7.))
            .flex()
            .items_center()
            .gap(s.px(4.))
            .rounded_full()
            .bg(rgba(CHIP_GOOD))
            .text_color(rgb(GOOD))
            .text_size(s.px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(chrome::icon(glyph, 11., rgb(GOOD), s))
            .child(text)
    });
    let next_level: u32 = pet.state.level + 1;
    let next = (!maxed).then(|| {
        div()
            .flex()
            .items_center()
            .gap(s.px(5.))
            .child(chrome::icon(Icon::ArrowRight, 10., rgba(FAINT), s))
            .child(fl!("pets-level", level = next_level))
    });
    div()
        .mt(s.px(if first { 4. } else { 7. }))
        .pl(s.px(4.))
        .pr(s.px(4.))
        .flex()
        .flex_col()
        .child(
            div()
                .h(s.px(18.))
                .flex()
                .items_center()
                .gap(s.px(6.))
                .child(div().flex_1().min_w_0().truncate().text_size(s.px(13.)).font_weight(FontWeight::SEMIBOLD).child(pet_name(pet.pet_id)))
                .children(chip),
        )
        .child(
            div()
                .mt(s.px(4.))
                .h(s.px(16.))
                .flex()
                .items_center()
                .gap(s.px(5.))
                .text_size(s.px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(MUTED))
                .child(level_badge(pet.state.level, None, 16., s))
                .child(figure(souls, needed, rgba(TEXT).into(), s))
                .children(next),
        )
        .child(
            div().mt(s.px(8.)).h(s.px(4.)).rounded(s.px(2.)).bg(rgba(BAR_TRACK)).overflow_hidden().child(
                div()
                    .h_full()
                    .w(relative(width))
                    .rounded(s.px(2.))
                    .bg(linear_gradient(90., linear_color_stop(rgba(BAR_FROM), 0.), linear_color_stop(rgba(BAR_TO), 1.))),
            ),
        )
}

/// "28" at 12.5 px / 600 in `colour` and "/75" at 11 px in `faint`, 1 px
/// apart.
pub fn figure(souls: u32, needed: u32, colour: Hsla, s: Scale) -> Div {
    div()
        .flex_none()
        .flex()
        .items_baseline()
        .gap(s.px(1.))
        .child(div().text_size(s.px(12.5)).font_weight(FontWeight::SEMIBOLD).text_color(colour).child(souls.to_string()))
        .child(div().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(format!("/{needed}")))
}

impl Render for PetCardView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let over = self.pointer_is_over(window);
        self.note_pointer(over, cx);
        let now = cx.background_executor().now();
        let look = self.look;
        let s = look.s;

        // 9 px above and 13 px below put 14 px between the border and the
        // ink above and below, as on the reminder card; the last pet's bar
        // sits 1 px inside its block.
        let card = chrome::card(self.width, look)
            .pt(s.px(9.))
            .pb(s.px(13.))
            .overflow_hidden()
            .id("pet-card")
            .debug_selector(|| "pet-card".to_string())
            .role(Role::Status)
            .aria_label(self.label())
            .child(self.head(s, now, cx))
            .children(self.shown.pets.iter().enumerate().map(|(index, pet)| pet_block(pet, index == 0, s)))
            .when(!self.locked, |card| {
                card.cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.dismiss(window, cx)))
            });
        let card: AnyElement = if cx.reduce_motion() {
            card.into_any_element()
        } else if self.leaving.is_some() {
            card.with_animation(ElementId::NamedInteger("pet-card-out".into(), self.leaves), Animation::new(FADE), |card, t| {
                card.opacity(1. - t)
            })
            .into_any_element()
        } else {
            card.with_animation("pet-card-in", Animation::new(FADE), |card, t| card.opacity(t)).into_any_element()
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
    use std::cell::RefCell;

    use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext, WindowHandle, WindowOptions};
    use meter_core::capture::parser::pet_record::{PetLevel, PetProgress, PetSnapshot, SoulGain};
    use meter_core::identity::KnownCharacter;
    use meter_core::pet_sources::SoulSource;
    use meter_core::pets::{PetContent, PetReport};

    use super::*;
    use crate::pets::gain::GainSource;
    use crate::pets::PetsSettings;
    use crate::settings::{install, AppSettings, Settings};

    const FOSSA: u32 = 1004;
    const SPARKIE: u32 = 1006;
    const KALGOLEM: u32 = 1084;
    const FUNGEN: u32 = 1086;
    /// Drops souls of Blue Kalgolem.
    const BLUE_KALGOLEM_MOB: u32 = 2_100_454;
    const BLUE_KALGOLEM: u32 = 1110;

    fn gain(pet_id: u32, level_before: u32, level: u32, souls: u32) -> LatestGain {
        LatestGain {
            sequence: 1,
            server_id: 1302,
            pet_id,
            amount: 1,
            level_before,
            level,
            souls,
            source: GainSource::Loot,
            captured_at: 100.0,
        }
    }

    fn installed(cx: &mut TestAppContext) -> Entity<Settings> {
        cx.update(|cx| {
            let store = cx.new(|_| Settings::in_memory(AppSettings::default()));
            install(&store, cx);
            service::install(cx);
            store
        })
    }

    fn open(request: CardRequest, cx: &mut TestAppContext) -> (Entity<PetCardView>, &mut VisualTestContext) {
        installed(cx);
        cx.add_window_view(|window, cx| PetCardView::new(&request, None, window, cx))
    }

    fn is_open(cx: &VisualTestContext) -> bool {
        !cx.windows().is_empty()
    }

    fn wait(cx: &mut VisualTestContext, duration: Duration) {
        cx.executor().advance_clock(duration);
        cx.run_until_parked();
    }

    fn pets_on(view: &Entity<PetCardView>, cx: &mut VisualTestContext) -> Vec<(u32, PetState, Option<Change>)> {
        cx.update(|_, cx| view.read(cx).shown().pets.iter().map(|pet| (pet.pet_id, pet.state, pet.change)).collect())
    }

    /// Gains reach the card as `secondary::show_pet_card` hands them over:
    /// the open card takes them; once it has closed, a new card opens with
    /// them. The cell holds the card now open, or the one that closed.
    fn deliver_to(cx: &mut VisualTestContext) -> (Subscription, Rc<Cell<WindowHandle<PetCardView>>>) {
        let card = Rc::new(Cell::new(cx.window_handle().downcast::<PetCardView>().expect("the card's window")));
        let current = card.clone();
        let watch = cx.cx.update(|cx| {
            watch_gains(cx, move |gains, cx| {
                let request = CardRequest::Gains(gains);
                if current.get().update(cx, |card, _, cx| card.show(&request, cx)).is_err() {
                    let opened = cx
                        .open_window(WindowOptions::default(), |window, cx| cx.new(|cx| PetCardView::new(&request, None, window, cx)))
                        .expect("a new card's window");
                    current.set(opened);
                }
            })
        });
        (watch, card)
    }

    /// Whether `card`'s window is open, and the pets on it.
    fn card_state(card: WindowHandle<PetCardView>, cx: &mut VisualTestContext) -> Option<Vec<u32>> {
        card.update(&mut cx.cx, |card, _, _| card.shown().pets.iter().map(|pet| pet.pet_id).collect()).ok()
    }

    fn record_now(reports: Vec<PetReport>, cx: &mut VisualTestContext) {
        cx.cx.update(|cx| crate::pets::record(cx, reports));
        cx.run_until_parked();
    }

    fn report(content: PetContent) -> PetReport {
        PetReport {
            character: KnownCharacter { name: "Alder".to_string(), server_id: 1302, class: "RANGER".to_string(), key: None },
            captured_at: 100.0,
            content,
        }
    }

    fn souls(source: SoulSource, gains: &[(u32, u32)]) -> PetReport {
        report(PetContent::SoulGains { gains: gains.iter().map(|&(pet_id, amount)| SoulGain { pet_id, amount }).collect(), source })
    }

    #[test]
    fn a_gain_reads_as_a_level_reached_an_unlock_or_souls_only() {
        assert_eq!(CardPet::of(&gain(FOSSA, 1, 2, 4)).change, Some(Change::Reached(2)));
        assert_eq!(CardPet::of(&gain(FOSSA, 0, 1, 0)).change, Some(Change::Unlocked));
        assert_eq!(CardPet::of(&gain(FOSSA, 2, 2, 28)).change, None);
        let maxed = CardPet::of(&LatestGain { source: GainSource::MaxedLoot, ..gain(KALGOLEM, 3, 3, 0) });
        assert_eq!((maxed.change, souls_of_needed(maxed.state)), (None, (75, 75)), "Lv 3 · 75/75, no level change");
    }

    #[test]
    fn up_to_three_pets_in_arrival_order_a_known_one_updates_in_place_and_a_fourth_takes_the_oldest_place() {
        let mut shown = Shown::default();
        shown.apply(&CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27), gain(SPARKIE, 1, 1, 3), gain(FUNGEN, 0, 0, 2)]));
        shown.apply(&CardRequest::Gains(vec![gain(FOSSA, 2, 2, 28)]));
        assert_eq!(shown.pets.iter().map(|pet| (pet.pet_id, pet.state.souls)).collect::<Vec<_>>(), vec![(FOSSA, 28), (SPARKIE, 3), (FUNGEN, 2)]);

        shown.apply(&CardRequest::Gains(vec![gain(KALGOLEM, 3, 3, 0)]));
        assert_eq!(shown.pets.iter().map(|pet| pet.pet_id).collect::<Vec<_>>(), vec![SPARKIE, FUNGEN, KALGOLEM]);
    }

    #[test]
    fn a_real_gain_replaces_the_test_card() {
        let mut shown = Shown::default();
        shown.apply(&CardRequest::Test(ShowTestCard { pet_id: FOSSA, state: PetState { level: 2, souls: 27 } }));
        assert!(shown.test);
        shown.apply(&CardRequest::Gains(vec![gain(SPARKIE, 1, 1, 3)]));
        assert!(!shown.test);
        assert_eq!(shown.pets.iter().map(|pet| pet.pet_id).collect::<Vec<_>>(), vec![SPARKIE]);
    }

    #[gpui::test]
    fn the_card_goes_10_s_after_its_last_update_and_a_new_gain_starts_the_10_s_again(cx: &mut TestAppContext) {
        let (view, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        wait(cx, Duration::from_secs(8));
        cx.update(|_, cx| view.update(cx, |view, cx| view.show(&CardRequest::Gains(vec![gain(FOSSA, 2, 2, 28)]), cx)));
        assert_eq!(pets_on(&view, cx), vec![(FOSSA, PetState { level: 2, souls: 28 }, None)]);

        wait(cx, Duration::from_secs(9));
        assert!(is_open(cx), "the gain started the 10 s again");
        wait(cx, Duration::from_secs(1) + TICK);
        wait(cx, FADE);
        assert!(!is_open(cx), "10 s after the last gain, then the fade, its window closed");
    }

    #[gpui::test]
    fn the_pointer_over_the_card_pauses_its_time(cx: &mut TestAppContext) {
        let (view, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        wait(cx, Duration::from_secs(6));
        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(Some(true), window, cx)));
        wait(cx, Duration::from_secs(30));
        assert!(is_open(cx), "paused under the pointer");

        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(Some(false), window, cx)));
        wait(cx, Duration::from_secs(3));
        assert!(is_open(cx), "4 s were left when the pointer came");
        wait(cx, Duration::from_secs(1) + TICK);
        wait(cx, FADE);
        assert!(!is_open(cx));
    }

    #[gpui::test]
    fn a_click_on_the_card_dismisses_it(cx: &mut TestAppContext) {
        let (_, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        let card = cx.debug_bounds("pet-card").expect("the card is drawn");
        cx.simulate_click(card.center(), gpui::Modifiers::none());
        assert!(is_open(cx), "it fades out first");
        wait(cx, FADE);
        assert!(!is_open(cx));
    }

    #[gpui::test]
    fn the_test_card_says_so_in_its_header(cx: &mut TestAppContext) {
        let (view, cx) = open(CardRequest::Test(ShowTestCard { pet_id: FOSSA, state: PetState { level: 2, souls: 27 } }), cx);
        let label = cx.update(|_, cx| view.read(cx).label().to_string());
        assert!(label.starts_with(&fl!("pets-card-test")), "{label}");
        assert!(label.contains("Fossa"), "{label}");
    }

    /// The real gains path, with the meter never opened: the game's messages
    /// reach the collection, Settings › Pets filters them, and what is left
    /// reaches the card.
    #[gpui::test]
    fn settings_pets_decide_which_gains_reach_the_card_with_the_meter_closed(cx: &mut TestAppContext) {
        let store = installed(cx);
        let delivered = Rc::new(RefCell::new(Vec::new()));
        let seen = delivered.clone();
        let _watch = cx.update(|cx| watch_gains(cx, move |gains, _| seen.borrow_mut().push(gains.iter().map(|gain| gain.pet_id).collect::<Vec<_>>())));
        let pets = cx.update(|cx| service::global(cx));
        let record = |reports: Vec<PetReport>, cx: &mut TestAppContext| cx.update(|cx| crate::pets::record(cx, reports));
        let set = |change: fn(&mut PetsSettings), cx: &mut TestAppContext| {
            cx.update(|cx| store.update(cx, |settings, cx| settings.update(cx, |all| change(&mut all.pets))))
        };
        let snapshot = PetSnapshot {
            levels: vec![PetLevel { pet_id: FOSSA, level: 2 }, PetLevel { pet_id: BLUE_KALGOLEM, level: 3 }],
            progress: vec![PetProgress { pet_id: FOSSA, progress: 27 }],
            counters: Vec::new(),
        };
        record(vec![report(PetContent::Snapshot(snapshot))], cx);
        assert!(delivered.borrow().is_empty(), "a snapshot is not a gain");

        record(vec![souls(SoulSource::Loot, &[(FOSSA, 1)])], cx);
        record(vec![report(PetContent::CounterGains { gains: Vec::new(), looted_mobs: vec![BLUE_KALGOLEM_MOB] })], cx);
        record(vec![souls(SoulSource::BlackMarket, &[(SPARKIE, 5)])], cx);
        assert_eq!(*delivered.borrow(), vec![vec![FOSSA], vec![BLUE_KALGOLEM], vec![SPARKIE]], "the defaults show every gain");

        delivered.borrow_mut().clear();
        set(|pets| pets.black_market = false, cx);
        record(vec![souls(SoulSource::BlackMarket, &[(SPARKIE, 5)])], cx);
        set(|pets| pets.maxed_pets = false, cx);
        record(vec![report(PetContent::CounterGains { gains: Vec::new(), looted_mobs: vec![BLUE_KALGOLEM_MOB] })], cx);
        set(|pets| pets.only_pinned = true, cx);
        record(vec![souls(SoulSource::Loot, &[(FOSSA, 1)])], cx);
        cx.update(|cx| pets.update(cx, |pets, cx| pets.pin(FOSSA, cx)));
        record(vec![souls(SoulSource::Loot, &[(FOSSA, 1)])], cx);
        set(|pets| pets.soul_tracker = false, cx);
        record(vec![souls(SoulSource::Loot, &[(FOSSA, 1)])], cx);
        assert_eq!(
            *delivered.borrow(),
            vec![vec![FOSSA]],
            "no black-market buy, no pet at Lv 3, no unpinned pet, then nothing with the master switch off"
        );
    }

    #[gpui::test]
    fn gains_reaching_an_open_card_add_their_pets_and_restart_its_time(cx: &mut TestAppContext) {
        let (view, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        let (_watch, _) = deliver_to(cx);
        wait(cx, Duration::from_secs(7));
        cx.update(|_, cx| {
            let snapshot = PetSnapshot { levels: vec![PetLevel { pet_id: SPARKIE, level: 1 }], progress: vec![PetProgress { pet_id: SPARKIE, progress: 24 }], counters: Vec::new() };
            crate::pets::record(cx, vec![report(PetContent::Snapshot(snapshot)), souls(SoulSource::Loot, &[(SPARKIE, 2)])]);
        });
        assert_eq!(
            pets_on(&view, cx),
            vec![(FOSSA, PetState { level: 2, souls: 27 }, None), (SPARKIE, PetState { level: 2, souls: 1 }, Some(Change::Reached(2)))]
        );
        wait(cx, Duration::from_secs(9));
        assert!(is_open(cx), "the new gain restarted the 10 s");
    }

    #[gpui::test]
    fn the_ring_empties_over_10_s_refills_on_a_gain_and_stops_under_the_pointer(cx: &mut TestAppContext) {
        let (view, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        let ring = |view: &Entity<PetCardView>, cx: &mut VisualTestContext| {
            cx.update(|_, cx| view.read(cx).timer_ring(cx.background_executor().now()))
        };
        let card = cx.debug_bounds("pet-card").expect("the card is drawn");
        assert_eq!(card.size.height, gpui::px(100.), "one pet");
        assert!(cx.debug_bounds("timer-ring").is_some());

        wait(cx, Duration::from_secs(5));
        assert!((ring(&view, cx).left - 0.5).abs() < 0.01, "half of the 10 s");
        cx.update(|_, cx| view.update(cx, |view, cx| view.show(&CardRequest::Gains(vec![gain(FOSSA, 2, 2, 28)]), cx)));
        assert!((ring(&view, cx).left - 1.).abs() < 1e-3, "a gain refills it at once");

        cx.update(|window, cx| view.update(cx, |view, cx| view.force_pointer(Some(true), window, cx)));
        wait(cx, Duration::from_secs(3));
        let paused = ring(&view, cx);
        assert!(paused.paused && (paused.left - 1.).abs() < 1e-3, "{paused:?}");
    }

    fn snapshot_of_three(cx: &mut VisualTestContext) {
        let snapshot = PetSnapshot {
            levels: vec![PetLevel { pet_id: FOSSA, level: 2 }, PetLevel { pet_id: SPARKIE, level: 1 }, PetLevel { pet_id: FUNGEN, level: 1 }],
            progress: Vec::new(),
            counters: Vec::new(),
        };
        record_now(vec![report(PetContent::Snapshot(snapshot))], cx);
    }

    /// The card closes 10 s after its last gain and the fade; a gain in its
    /// last moments or during the fade keeps it, with the pet added.
    #[gpui::test]
    fn a_gain_in_the_last_moments_or_during_the_fade_keeps_the_card(cx: &mut TestAppContext) {
        let (_, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        let (_watch, card) = deliver_to(cx);
        snapshot_of_three(cx);

        wait(cx, Duration::from_millis(9_900));
        record_now(vec![souls(SoulSource::Loot, &[(SPARKIE, 1)])], cx);
        assert!(card_state(card.get(), cx).is_some_and(|pets| pets.contains(&SPARKIE)), "at 9.9 s");
        wait(cx, Duration::from_secs(9));
        assert!(card_state(card.get(), cx).is_some(), "and its 10 s started again");

        wait(cx, Duration::from_secs(1) + TICK * 2);
        let fading = card.get().update(&mut cx.cx, |card, _, _| card.leaving.is_some()).unwrap_or(false);
        assert!(fading, "fading");
        record_now(vec![souls(SoulSource::Loot, &[(FUNGEN, 1)])], cx);
        wait(cx, FADE * 2);
        assert!(card_state(card.get(), cx).is_some_and(|pets| pets.contains(&FUNGEN)), "a gain during the fade keeps the card");

        wait(cx, SHOWN_FOR + TICK * 2 + FADE);
        assert_eq!(card_state(card.get(), cx), None, "10 s after the last gain and the fade, its window closed");
        assert!(cx.windows().is_empty());
    }

    /// After the card closed, the next gain opens a new card, at once, with
    /// that pet alone; that card closes in its turn 10 s later.
    #[gpui::test]
    fn a_gain_after_the_card_closed_opens_a_new_card_with_that_pet_alone(cx: &mut TestAppContext) {
        let (_, cx) = open(CardRequest::Gains(vec![gain(FOSSA, 2, 2, 27)]), cx);
        let (_watch, card) = deliver_to(cx);
        snapshot_of_three(cx);
        let first = card.get();
        wait(cx, SHOWN_FOR + TICK * 2 + FADE);
        assert_eq!(card_state(first, cx), None, "the first card closed");

        record_now(vec![souls(SoulSource::Loot, &[(SPARKIE, 1)])], cx);
        assert_ne!(card.get(), first, "a new window");
        assert_eq!(card_state(card.get(), cx), Some(vec![SPARKIE]), "with that pet alone, at once");
        assert_eq!(cx.windows().len(), 1);

        wait(cx, SHOWN_FOR - TICK * 4);
        assert!(card_state(card.get(), cx).is_some(), "its full 10 s");
        wait(cx, TICK * 6 + FADE);
        assert_eq!(card_state(card.get(), cx), None, "then it closes too");
    }
}
