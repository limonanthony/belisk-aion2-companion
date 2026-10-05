//! The Settings window (docs/gpui-mockups/approved/settings.html): a title
//! bar, the sections in a sidebar, and the open section's rows. Values are
//! read from the `Settings` entity and every change is written back to it at
//! once; the read-only parts come from `RuntimeModel`.
//!
//! Keyboard and pointer behave as in a browser: Tab and Shift+Tab walk the
//! controls in reading order, Enter and Space press, the arrows move
//! sliders, segmented controls, the select's list and the sidebar, Escape
//! closes the open list or dialog and gives the focus back to what opened
//! it, and a press outside every control takes the focus away (or closes the
//! list).

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aether_core::config::CaptureBackendPriority;
use aether_core::events::MeterEvent;
use aether_core::preflight::{self, Weight};
use gpui::{
    div, point, prelude::*, px, relative, rgb, rgba, size, Animation, AnimationExt as _, AnyElement, App, Bounds,
    Context, Div, Entity, FocusHandle, FontFeatures, FontWeight, KeyDownEvent, MouseButton, MouseMoveEvent, Pixels,
    Point, Render, Role, ScrollHandle, SharedString, Subscription, Task, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowHandle, WindowKind, WindowOptions,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE};

use super::files::{self, Document, Folder};
use super::model::{global, AppSettings, CountMode, DetailPlacement, Settings, END_FIGHT_AFTER_OPTIONS};
use super::runtime::{CaptureCheck, MeterTile, ReplayProgress, RuntimeModel, StorageState};
use super::widgets::{
    self, alert, backdrop, badge, button, description, dimmed_row, disabled_select, group, group_title, icon, list_row,
    note, page_head, row, scroll_thumb, segmented, select_menu, select_trigger, side_item, slider, spinner, switch, tile,
    ButtonKind, Focus, Handler, Reveal, SliderSpec, Tone, BODY_FONT, CANVAS, DISPLAY_FONT, FAINT, LINE, MUTED,
    POPOVER_BORDER, RAISED, SCRIM, SIDE, TEXT,
};
use crate::assets::Icon;
use crate::format;
use crate::meter::{board_width, Layout, LockChanged, MeterModel, MeterView, OverlaySettings, PctMode, PLAYERS_SHOWN_RANGE};
use crate::overlay::Placement;
use crate::reminders::{self, EventId, Rule, EVENTS, LEAD_MINUTES_OPTIONS};
use crate::sample;
use crate::secondary::chrome::Anchor;

const WINDOW_SIZE: (f32, f32) = (960., 620.);
const WINDOW_TITLE: &str = "AION 2 DPS Meter Settings";
const SIDEBAR_WIDTH: f32 = 208.;
const PREVIEW_CARD: f32 = 236.;
const PREVIEW_STAGE_PADDING: f32 = 12.;
/// The approved mockup's preview column. It keeps this width whatever the
/// overlay shows, so the rows beside it never move: a card wider than the
/// stage is drawn at the largest content scale that fits.
const PREVIEW_WIDTH: f32 = 262.;
const OVERLAY_SLIDER_WIDTH: f32 = 120.;
const SLIDER_WIDTH: f32 = 170.;
const OPTION_HEIGHT: f32 = 28.;
const CONFIRM_FOR: Duration = Duration::from_secs(4);
/// How long "Shown above the meter" stays after a test reminder.
const TEST_SHOWN_FOR: Duration = Duration::from_secs(4);
const DIALOG_FADE: Duration = Duration::from_millis(150);
const CHIP_FADE: Duration = Duration::from_millis(150);
const ALPHA_PER_PERCENT: f64 = 2.55;

const COUNT_TIP: &str = "With Boss only on and no boss in front of you, every hit is ignored and the overlay stays empty.";
const RESET_TITLE: &str = "Reset settings?";
const RESET_BODY: &str = "Settings go back to their defaults, including the Capture switches. Fight history, personal bests, recordings and boss corrections are kept.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Overlay,
    Behaviour,
    Capture,
    Bests,
    Reminders,
    About,
}

impl Section {
    const ALL: [Self; 6] = [Self::Overlay, Self::Behaviour, Self::Capture, Self::Bests, Self::Reminders, Self::About];

    fn label(self) -> &'static str {
        match self {
            Self::Overlay => "Overlay",
            Self::Behaviour => "Meter behaviour",
            Self::Capture => "Capture",
            Self::Bests => "Personal bests",
            Self::Reminders => "Reminders",
            Self::About => "About",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Overlay => Icon::SlidersHorizontal,
            Self::Behaviour => Icon::Activity,
            Self::Capture => Icon::Radio,
            Self::Bests => Icon::Trophy,
            Self::Reminders => Icon::Bell,
            Self::About => Icon::Info,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Overlay => "nav-overlay",
            Self::Behaviour => "nav-behaviour",
            Self::Capture => "nav-capture",
            Self::Bests => "nav-bests",
            Self::Reminders => "nav-reminders",
            Self::About => "nav-about",
        }
    }
}

/// The page's dropdowns. One is open at a time, in `select_open`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Select {
    EndFight,
    LeadTime,
}

impl Select {
    fn key(self) -> &'static str {
        match self {
            Self::EndFight => "end-fight",
            Self::LeadTime => "lead-time",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::EndFight => "End a fight after",
            Self::LeadTime => "Remind me",
        }
    }

    fn min_width(self) -> f32 {
        match self {
            Self::EndFight => 150.,
            Self::LeadTime => 130.,
        }
    }

    fn options(self) -> Vec<SharedString> {
        match self {
            Self::EndFight => END_FIGHT_AFTER_OPTIONS.iter().map(|(_, label)| SharedString::new_static(label)).collect(),
            Self::LeadTime => LEAD_MINUTES_OPTIONS.iter().map(|&minutes| lead_label(minutes)).collect(),
        }
    }

    /// The stored value's option, if it is one.
    fn current(self, settings: &AppSettings) -> Option<usize> {
        match self {
            Self::EndFight => {
                END_FIGHT_AFTER_OPTIONS.iter().position(|(secs, _)| *secs == settings.behaviour.end_fight_after_secs)
            }
            Self::LeadTime => LEAD_MINUTES_OPTIONS.iter().position(|&minutes| minutes == settings.reminders.lead_minutes),
        }
    }

    fn value(self, settings: &AppSettings) -> SharedString {
        match self {
            Self::EndFight => end_fight_label(settings.behaviour.end_fight_after_secs),
            Self::LeadTime => lead_label(settings.reminders.lead_minutes),
        }
    }

    fn apply(self, settings: &mut AppSettings, index: usize) {
        match self {
            Self::EndFight => settings.behaviour.end_fight_after_secs = END_FIGHT_AFTER_OPTIONS[index].0,
            Self::LeadTime => settings.reminders.lead_minutes = LEAD_MINUTES_OPTIONS[index],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SliderKind {
    Opacity,
    Scale,
    Players,
}

impl SliderKind {
    fn key(self) -> &'static str {
        match self {
            Self::Opacity => "opacity",
            Self::Scale => "scale",
            Self::Players => "players",
        }
    }

    /// Opacity in whole percent, the scale as a factor, players as a count.
    fn spec(self, settings: &AppSettings) -> SliderSpec {
        let overlay = &settings.overlay;
        match self {
            Self::Opacity => SliderSpec {
                min: 0.,
                max: 100.,
                step: 1.,
                page: 10.,
                value: (f64::from(overlay.background_alpha) / ALPHA_PER_PERCENT).round(),
            },
            Self::Scale => SliderSpec {
                min: 0.7,
                max: 1.5,
                step: 0.05,
                page: 0.25,
                value: (f64::from(overlay.scale()) * 20.).round() / 20.,
            },
            Self::Players => SliderSpec {
                min: *PLAYERS_SHOWN_RANGE.start() as f64,
                max: *PLAYERS_SHOWN_RANGE.end() as f64,
                step: 1.,
                page: 5.,
                value: overlay.players_shown as f64,
            },
        }
    }

    fn text(self, value: f64) -> SharedString {
        match self {
            Self::Opacity => format!("{value:.0}%").into(),
            Self::Scale => format!("{:.0}%", value * 100.).into(),
            Self::Players => format!("{value:.0}").into(),
        }
    }

    fn apply(self, settings: &mut AppSettings, value: f64) {
        let overlay = &mut settings.overlay;
        match self {
            Self::Opacity => overlay.background_alpha = (value * ALPHA_PER_PERCENT).round().clamp(0., 255.) as u8,
            Self::Scale => overlay.content_scale = ((value * 100.).round() / 100.) as f32,
            Self::Players => overlay.players_shown = value.round() as usize,
        }
    }
}

pub struct SettingsView {
    store: Entity<Settings>,
    runtime: Entity<RuntimeModel>,
    /// The overlay, for the lock; none when Settings runs without one.
    meter_window: Option<WindowHandle<MeterView>>,
    locked: bool,
    can_unlock: bool,
    preview_model: Entity<MeterModel>,
    preview: Entity<MeterView>,
    section: Section,
    /// Holds the focus when no control does, as a page's body would.
    root_focus: FocusHandle,
    controls: HashMap<String, FocusHandle>,
    scroll: ScrollHandle,
    reveal: Rc<Cell<bool>>,
    /// This select's list is open, with this option lit.
    select_open: Option<(Select, usize)>,
    select_anchor: Anchor,
    tracks: HashMap<SliderKind, Anchor>,
    /// Where the Overlay page's rows and preview sit, for the dev capture's
    /// check that no setting moves the rows.
    #[cfg(feature = "probes")]
    layout_marks: HashMap<&'static str, Anchor>,
    dragging: Option<SliderKind>,
    reset_dialog_open: bool,
    /// "Reset all" asked for its second click; the task takes the question back.
    confirm_bests: Option<Task<()>>,
    /// A test reminder was just shown; the task takes the note away.
    test_shown: Option<Task<()>>,
    /// What an action handed to Windows could not do, under its control.
    action_error: Option<(&'static str, String)>,
    figures: FontFeatures,
    _subscriptions: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(
        runtime: Entity<RuntimeModel>,
        meter_window: Option<WindowHandle<MeterView>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let store = global(cx);
        let overlay = store.read(cx).get().overlay.clone();
        let preview_model = cx.new(|_| sample_meter(preview_settings(overlay)));
        let preview = cx.new(|cx| MeterView::preview(preview_model.clone(), preview_zoom(), cx));

        let mut subscriptions = vec![
            cx.observe(&store, |this, store, cx| {
                let overlay = preview_settings(store.read(cx).get().overlay.clone());
                this.preview_model.update(cx, |model, cx| {
                    if model.settings != overlay {
                        model.set_overlay_settings(overlay);
                        cx.notify();
                    }
                });
                cx.notify();
            }),
            cx.observe(&runtime, |_, _, cx| cx.notify()),
            // A focused control that goes away (a Cancel replay button once
            // the replay ends) leaves the focus with the window, as a page
            // gives it back to its body, so Tab keeps working.
            cx.on_focus_lost(window, |this, window, cx| window.focus(&this.root_focus, cx)),
        ];
        let (mut locked, mut can_unlock) = (false, false);
        if let Some(meter_window) = meter_window {
            if let Ok((is_locked, unlockable)) = meter_window.read_with(cx, |view, _| (view.is_locked(), view.can_unlock())) {
                (locked, can_unlock) = (is_locked, unlockable);
            }
            if let Ok(meter_view) = meter_window.entity(cx) {
                subscriptions.push(cx.subscribe(&meter_view, |this, _, event: &LockChanged, cx| {
                    this.locked = event.0;
                    cx.notify();
                }));
            }
        }

        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);
        Self {
            store,
            runtime,
            meter_window,
            locked,
            can_unlock,
            preview_model,
            preview,
            section: Section::Overlay,
            root_focus,
            controls: HashMap::new(),
            scroll: ScrollHandle::new(),
            reveal: Rc::new(Cell::new(false)),
            select_open: None,
            select_anchor: Anchor::default(),
            tracks: HashMap::new(),
            #[cfg(feature = "probes")]
            layout_marks: HashMap::new(),
            dragging: None,
            reset_dialog_open: false,
            confirm_bests: None,
            test_shown: None,
            action_error: None,
            figures: FontFeatures(Arc::new(vec![("tnum".into(), 1)])),
            _subscriptions: subscriptions,
        }
    }

    pub fn show_section(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section == section {
            return;
        }
        self.section = section;
        self.select_open = None;
        self.dragging = None;
        self.confirm_bests = None;
        self.test_shown = None;
        self.action_error = None;
        self.scroll.set_offset(point(px(0.), px(0.)));
        if section == Section::About {
            self.runtime.update(cx, |runtime, cx| runtime.load_storage(cx));
        }
        cx.notify();
    }

    fn handle(&mut self, key: &str, cx: &mut App) -> FocusHandle {
        if let Some(handle) = self.controls.get(key) {
            return handle.clone();
        }
        let handle = cx.focus_handle().tab_stop(true);
        self.controls.insert(key.to_string(), handle.clone());
        handle
    }

    /// `in_page`: the control scrolls with the page, which then brings it
    /// into view when the keyboard moves the focus to it.
    fn focus_of(&mut self, key: &str, in_page: bool, window: &Window, cx: &mut App) -> Focus {
        let handle = self.handle(key, cx);
        let focused = handle.is_focused(window);
        Focus {
            ring: focused && window.last_input_was_keyboard(),
            reveal: (focused && in_page).then(|| Reveal { scroll: self.scroll.clone(), pending: self.reveal.clone() }),
            handle,
        }
    }

    fn focus_control(&mut self, key: &str, window: &mut Window, cx: &mut App) {
        let handle = self.handle(key, cx);
        window.focus(&handle, cx);
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn focused_control(&self, window: &Window) -> Option<String> {
        self.controls.iter().find(|(_, handle)| handle.is_focused(window)).map(|(key, _)| key.clone())
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn select_is_open(&self) -> bool {
        self.select_open.is_some()
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn reset_dialog_is_open(&self) -> bool {
        self.reset_dialog_open
    }

    /// Where a slider's track, the select, the Overlay page's rows or its
    /// preview sit in the window, for dev captures that post presses to them
    /// or check that they hold still.
    #[cfg(feature = "probes")]
    pub fn bounds_of(&self, key: &str) -> Option<Bounds<Pixels>> {
        if key == "end-fight" {
            return self.select_anchor.bounds();
        }
        if let Some(mark) = self.layout_marks.get(key) {
            return mark.bounds();
        }
        let kind = [SliderKind::Opacity, SliderKind::Scale, SliderKind::Players].into_iter().find(|kind| kind.key() == key)?;
        self.tracks.get(&kind)?.bounds()
    }

    fn set<T: 'static>(&self, apply: impl Fn(&mut AppSettings, T) + 'static) -> Handler<T> {
        let store = self.store.clone();
        Rc::new(move |value, _, cx| store.update(cx, |settings, cx| settings.update(cx, |all| apply(all, value))))
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        match event.keystroke.key.as_str() {
            "tab" if !modifiers.control && !modifiers.alt && !modifiers.platform => {
                cx.stop_propagation();
                if self.reset_dialog_open {
                    let next = if self.handle("dialog-cancel", cx).is_focused(window) { "dialog-reset" } else { "dialog-cancel" };
                    self.focus_control(next, window, cx);
                } else {
                    self.select_open = None;
                    if modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    self.reveal.set(true);
                }
                cx.notify();
            }
            "escape" if self.select_open.is_some() => {
                cx.stop_propagation();
                if let Some((select, _)) = self.select_open.take() {
                    self.focus_control(select.key(), window, cx);
                }
                cx.notify();
            }
            "escape" if self.reset_dialog_open => {
                cx.stop_propagation();
                self.close_reset_dialog(window, cx);
            }
            _ => {}
        }
    }

    fn press_slider(&mut self, kind: SliderKind, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.dragging = Some(kind);
        self.drag_slider(kind, position.x, cx);
    }

    fn drag_slider(&mut self, kind: SliderKind, x: Pixels, cx: &mut Context<Self>) {
        let Some(bounds) = self.tracks.get(&kind).and_then(Anchor::bounds) else {
            return;
        };
        let value = kind.spec(self.store.read(cx).get()).at(widgets::fraction_at(bounds, x));
        self.store.update(cx, |settings, cx| settings.update(cx, |all| kind.apply(all, value)));
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.dragging else {
            return;
        };
        if event.pressed_button == Some(MouseButton::Left) {
            self.drag_slider(kind, event.position.x, cx);
        } else {
            self.dragging = None;
        }
    }

    /// A press that no control or open list took: the focus goes back to the
    /// window, as a click on a page's background does.
    fn blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.select_open = None;
        window.focus(&self.root_focus, cx);
        cx.notify();
    }

    fn on_select_key(&mut self, select: Select, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let key = event.keystroke.key.as_str();
        let last = select.options().len() - 1;
        let lit = self.select_open.filter(|(open, _)| *open == select).map(|(_, lit)| lit);
        match (lit, key) {
            (None, "down" | "up" | "enter" | "space") => self.open_select(select, cx),
            (Some(lit), "down") => self.select_open = Some((select, (lit + 1).min(last))),
            (Some(lit), "up") => self.select_open = Some((select, lit.saturating_sub(1))),
            (Some(_), "home") => self.select_open = Some((select, 0)),
            (Some(_), "end") => self.select_open = Some((select, last)),
            (Some(lit), "enter" | "space") => self.pick(select, lit, window, cx),
            (Some(_), "escape") => self.select_open = None,
            _ => return,
        }
        cx.stop_propagation();
        window.prevent_default();
        cx.notify();
    }

    fn open_select(&mut self, select: Select, cx: &App) {
        let current = select.current(self.store.read(cx).get());
        self.select_open = Some((select, current.unwrap_or(0)));
    }

    fn pick(&mut self, select: Select, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.store.update(cx, |settings, cx| settings.update(cx, |all| select.apply(all, index)));
        self.select_open = None;
        self.focus_control(select.key(), window, cx);
        cx.notify();
    }

    /// The select's trigger, or the same box inert while `enabled` is false.
    fn select_control(&mut self, select: Select, settings: &AppSettings, enabled: bool, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let value = select.value(settings);
        if !enabled {
            return disabled_select(select.key(), select.label(), value, select.min_width());
        }
        let focus = self.focus_of(select.key(), true, window, cx);
        let open = self.select_open.is_some_and(|(open, _)| open == select);
        select_trigger(select.key(), select.label(), value, open, select.min_width(), &focus, &self.select_anchor)
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                if matches!(event, gpui::ClickEvent::Keyboard(_)) {
                    return;
                }
                if this.select_open.is_some() {
                    this.select_open = None;
                } else {
                    this.open_select(select, cx);
                }
                this.focus_control(select.key(), window, cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event, window, cx| this.on_select_key(select, event, window, cx)))
            .into_any_element()
    }

    fn open_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_dialog_open = true;
        self.focus_control("dialog-cancel", window, cx);
        cx.notify();
    }

    fn close_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_dialog_open = false;
        self.focus_control("reset-settings", window, cx);
        cx.notify();
    }

    fn reset_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.store.update(cx, |settings, cx| settings.reset(cx));
        self.runtime.update(cx, |runtime, cx| runtime.load_storage(cx));
        self.close_reset_dialog(window, cx);
    }

    fn switch_row(
        &mut self,
        key: &'static str,
        label: &'static str,
        text: Option<&'static str>,
        on: bool,
        apply: fn(&mut AppSettings, bool),
        window: &Window,
        cx: &mut App,
    ) -> Div {
        let focus = self.focus_of(key, true, window, cx);
        row(label, text.map(description), switch(key, label, on, &focus, Some(self.set(apply))))
    }

    #[allow(clippy::too_many_arguments)]
    fn segmented_row(
        &mut self,
        key: &'static str,
        label: impl IntoElement,
        name: &'static str,
        text: Option<AnyElement>,
        options: &[&'static str],
        selected: usize,
        apply: fn(&mut AppSettings, usize),
        window: &Window,
        cx: &mut App,
    ) -> Div {
        let focus = self.focus_of(key, true, window, cx);
        row(label, text, segmented(key, name, options, selected, &focus, self.set(apply)))
    }

    fn slider_row(
        &mut self,
        kind: SliderKind,
        label: &'static str,
        text: &'static str,
        width: f32,
        settings: &AppSettings,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let spec = kind.spec(settings);
        let focus = self.focus_of(kind.key(), true, window, cx);
        let track = self.tracks.entry(kind).or_default().clone();
        let on_change = self.set(move |all: &mut AppSettings, value: f64| kind.apply(all, value));
        let view = cx.entity().downgrade();
        let on_press: Handler<Point<Pixels>> = Rc::new(move |position, _, cx| {
            let _ = view.update(cx, |view, cx| view.press_slider(kind, position, cx));
        });
        row(
            label,
            Some(description(text)),
            slider(kind.key(), label, spec, kind.text(spec.value), width, &focus, &track, on_change, on_press),
        )
    }

    fn button_handler(&self, cx: &Context<Self>, act: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) -> Handler<()> {
        let view = cx.entity().downgrade();
        Rc::new(move |(), window, cx| {
            let _ = view.update(cx, |view, cx| act(view, window, cx));
        })
    }

    /// The result of handing a folder or document to Windows, kept under the
    /// control that asked.
    fn hand_over(&mut self, control: &'static str, result: Result<(), String>, cx: &mut Context<Self>) {
        self.action_error = result.err().map(|error| (control, error));
        cx.notify();
    }

    fn error_under(&self, control: &'static str) -> Option<Div> {
        self.action_error
            .as_ref()
            .filter(|(key, _)| *key == control)
            .map(|(_, error)| note(Icon::TriangleAlert, error.clone(), rgb(widgets::BAD)))
    }

    fn overlay_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let overlay = &settings.overlay;
        let look = group(vec![
            self.slider_row(
                SliderKind::Opacity,
                "Background opacity",
                "Lower is more see-through. The text keeps its own plates.",
                OVERLAY_SLIDER_WIDTH,
                settings,
                window,
                cx,
            ),
            self.slider_row(SliderKind::Scale, "Content scale", "70 to 150%.", OVERLAY_SLIDER_WIDTH, settings, window, cx),
            self.segmented_row(
                "layout",
                "Layout",
                "Layout",
                Some(description("Capsule is one line: your DPS, place and time. It opens on pointer over.")),
                &["Full", "Capsule"],
                usize::from(overlay.layout == Layout::Capsule),
                |all, index| all.overlay.layout = if index == 0 { Layout::Full } else { Layout::Capsule },
                window,
                cx,
            ),
            self.segmented_row(
                "percentage",
                "Percentage",
                "Percentage",
                Some(description("Contribution is based on the visible list; share on the whole fight.")),
                &["Contribution", "Share"],
                usize::from(overlay.pct_mode == PctMode::Share),
                |all, index| all.overlay.pct_mode = if index == 0 { PctMode::Contribution } else { PctMode::Share },
                window,
                cx,
            ),
        ]);
        let shown = group(vec![
            self.switch_row("boss-hp", "Boss HP gauge", None, overlay.show_boss_hp, |all, on| all.overlay.show_boss_hp = on, window, cx),
            self.switch_row(
                "player-name",
                "Player name",
                Some("Off hides names; icons and figures stay."),
                overlay.show_player_name,
                |all, on| all.overlay.show_player_name = on,
                window,
                cx,
            ),
            self.switch_row(
                "mask-nicknames",
                "Mask nicknames",
                Some("Shows Mi****e instead of the name, for streams and screenshots."),
                overlay.mask_nicknames,
                |all, on| all.overlay.mask_nicknames = on,
                window,
                cx,
            ),
            self.switch_row("server", "Server", None, overlay.show_server, |all, on| all.overlay.show_server = on, window, cx),
            self.switch_row(
                "combat-power",
                "Combat power",
                Some("Beside the name, where the game reported it."),
                overlay.show_combat_power,
                |all, on| all.overlay.show_combat_power = on,
                window,
                cx,
            ),
            self.switch_row("total-damage", "Total damage", None, overlay.show_damage, |all, on| all.overlay.show_damage = on, window, cx),
            self.switch_row("dps", "DPS", None, overlay.show_dps, |all, on| all.overlay.show_dps = on, window, cx),
        ]);
        let fights = group(vec![
            self.switch_row(
                "fight-summary",
                "Fight summary",
                Some("When a boss dies: your DPS and place, top skills and personal best. One click copies it."),
                overlay.show_fight_summary,
                |all, on| all.overlay.show_fight_summary = on,
                window,
                cx,
            ),
            self.switch_row(
                "pace",
                "Pace against your personal best",
                Some("After 10 s of a boss fight, ahead or behind your best on that boss."),
                overlay.show_personal_best,
                |all, on| all.overlay.show_personal_best = on,
                window,
                cx,
            ),
        ]);
        let lock = self.lock_row(window, cx);
        let detail = self.segmented_row(
            "detail-opens",
            "Player detail opens",
            "Player detail opens",
            Some(description("Beside the overlay, or in the middle of the screen.")),
            &["Beside", "Centred"],
            usize::from(settings.windows.detail_opens == DetailPlacement::Centred),
            |all, index| {
                all.windows.detail_opens = if index == 0 { DetailPlacement::Beside } else { DetailPlacement::Centred }
            },
            window,
            cx,
        );
        let rows = div()
            .debug_selector(|| "overlay-rows".to_string())
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(group_title("Look", true))
            .child(look)
            .child(group_title("Shown on each row", false))
            .child(shown)
            .child(group_title("Boss fights", false))
            .child(fights)
            .child(group_title("Window", false))
            .child(group(vec![lock, detail]));
        #[cfg(feature = "probes")]
        let rows = rows.relative().child(self.layout_marks.entry("overlay-rows").or_default().marker());
        let preview = self.preview_column(cx);
        div()
            .flex()
            .flex_col()
            .child(page_head("Overlay", Some("How the meter overlay looks. Changes show in the overlay at once.")))
            .child(div().flex().items_start().gap(px(24.)).child(rows).child(preview))
    }

    fn lock_row(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let focus = self.focus_of("lock", true, window, cx);
        let on_toggle: Option<Handler<bool>> = self.meter_window.filter(|_| self.can_unlock).map(|meter_window| {
            Rc::new(move |on, _: &mut Window, cx: &mut App| {
                if let Err(error) = meter_window.update(cx, |view, window, cx| view.set_locked(on, window, cx)) {
                    eprintln!("cannot reach the overlay to {}: {error:#}", if on { "lock it" } else { "unlock it" });
                }
            }) as Handler<bool>
        });
        let reason = (self.meter_window.is_some() && !self.can_unlock).then(|| {
            note(Icon::Info, "Alt+CapsLock could not be registered, so nothing could unlock the overlay.", rgba(FAINT))
        });
        let text = div()
            .flex()
            .flex_col()
            .child(description("Clicks pass through to the game and the controls hide. Alt+CapsLock switches it."))
            .children(reason)
            .into_any_element();
        row("Lock overlay", Some(text), switch("lock", "Lock overlay", self.locked, &focus, on_toggle))
    }

    fn preview_column(&mut self, cx: &Context<Self>) -> Div {
        let stage = div()
            .id("preview")
            .debug_selector(|| "preview".to_string())
            .relative()
            .overflow_hidden()
            .rounded(px(10.))
            .py(px(14.))
            .px(px(PREVIEW_STAGE_PADDING))
            .child(backdrop())
            .child(div().relative().child(self.preview.clone()))
            .child(
                div()
                    .id("preview-shield")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.blur(window, cx))),
            );
        #[cfg(feature = "probes")]
        let stage = stage.child(self.layout_marks.entry("preview").or_default().marker());
        div()
            .flex_none()
            .w(px(PREVIEW_WIDTH))
            .flex()
            .flex_col()
            .child(group_title("Preview", true))
            .child(stage)
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(11.5))
                    .text_color(rgba(FAINT))
                    .child("Sample data. Follows every change on this page."),
            )
    }

    fn behaviour_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let behaviour = &settings.behaviour;
        let count_label = div().flex().items_center().gap(px(6.)).child("Count").child(
            div()
                .id("count-info")
                .child(icon(Icon::Info, 14., rgba(FAINT)))
                .tooltip(widgets::tooltip(COUNT_TIP)),
        );
        let count = self.segmented_row(
            "count",
            count_label,
            "Count",
            Some(description("All targets records every mob you hit; Boss only ignores everything that is not a boss.")),
            &["All targets", "Boss only"],
            usize::from(behaviour.count == CountMode::BossOnly),
            |all, index| all.behaviour.count = if index == 0 { CountMode::AllTargets } else { CountMode::BossOnly },
            window,
            cx,
        );
        let counted = group(vec![
            count,
            self.switch_row(
                "possible-bosses",
                "Detect possible bosses",
                Some("Treats very high-HP monsters missing from the boss table as bosses. Useful in new dungeons."),
                behaviour.detect_possible_bosses,
                |all, on| all.behaviour.detect_possible_bosses = on,
                window,
                cx,
            ),
            self.switch_row(
                "training-dummy",
                "Training dummy only",
                Some("Counts only damage on your own training dummy."),
                behaviour.training_dummy_only,
                |all, on| all.behaviour.training_dummy_only = on,
                window,
                cx,
            ),
            self.switch_row(
                "hide-unknown",
                "Hide unknown players",
                Some("Leaves out players the meter could not name."),
                behaviour.hide_unknown_players,
                |all, on| all.behaviour.hide_unknown_players = on,
                window,
                cx,
            ),
            self.slider_row(SliderKind::Players, "Players shown", "5 to 20.", SLIDER_WIDTH, settings, window, cx),
        ]);

        let trigger = self.select_control(Select::EndFight, settings, true, window, cx);
        let between = group(vec![row(
            "End a fight after",
            Some(description("With no hit of yours for this long, the fight is saved to history and the meter clears.")),
            trigger,
        )]);

        div()
            .flex()
            .flex_col()
            .child(page_head("Meter behaviour", Some("What the meter counts, and what it does between fights.")))
            .child(group_title("What is counted", true))
            .child(counted)
            .child(group_title("Between fights", false))
            .child(between)
    }

    fn capture_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let runtime = self.runtime.read(cx);
        let status = runtime.capture.clone();
        let check = runtime.capture_check.clone();
        let recording = runtime.recording;
        let recording_error = runtime.recording_error.clone();
        let recordings = runtime.recordings.clone();
        let replay = runtime.replay.clone();
        let has_meter = runtime.has_meter();
        let can_replay = runtime.can_replay();
        let animate = !cx.reduce_motion();

        let meter_tile = match &status.meter {
            MeterTile::Running { via } => tile(Icon::Activity, "Meter", "Running", via.map(|via| format!("via {via}").into()), Tone::Good),
            MeterTile::Stopped => tile(Icon::Activity, "Meter", "Stopped", None, Tone::Neutral),
            MeterTile::Unavailable(reason) => tile(Icon::Activity, "Meter", "Not running", Some(reason.clone().into()), Tone::Bad),
        };
        let game_tile = if status.game_found {
            tile(Icon::Radio, "Game connection", "Found", status.game_address.clone().map(Into::into), Tone::Good)
        } else {
            tile(Icon::Radio, "Game connection", "Waiting for the game", None, Tone::Warn)
        };
        let character_tile = match &status.character {
            Some(name) => tile(Icon::User, "Character", settings.overlay.display_name(name), None, Tone::Good),
            None => tile(
                Icon::User,
                "Character",
                "Not identified yet",
                Some("Start before login, or relog via character select".into()),
                Tone::Warn,
            ),
        };
        let servers = status.servers.join(", ");
        let server_text = match (status.region, servers.is_empty()) {
            (Some(region), false) => format!("{region} · {servers}"),
            (Some(region), true) => region.to_string(),
            (None, false) => servers,
            (None, true) => "\u{2014}".to_string(),
        };
        let server_tone = if status.region.is_some() || !status.servers.is_empty() { Tone::Good } else { Tone::Neutral };
        let server_tile = tile(Icon::Server, "Server", server_text, None, server_tone);
        let tiles = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(div().flex().gap(px(8.)).child(meter_tile).child(game_tile))
            .child(div().flex().gap(px(8.)).child(character_tile).child(server_tile));

        let backend = self.segmented_row(
            "backend",
            "Backend priority",
            "Backend priority",
            Some(
                div()
                    .flex()
                    .flex_col()
                    .child(description(
                        "Which capture method is tried first; the other is the fallback. Npcap first: some anti-cheats dislike a loaded WinDivert driver.",
                    ))
                    .child(note(Icon::Info, "Applies at next start.", rgba(FAINT)))
                    .into_any_element(),
            ),
            &["Npcap first", "WinDivert first"],
            usize::from(settings.capture.backend_priority == CaptureBackendPriority::WinDivertFirst),
            |all, index| {
                all.capture.backend_priority =
                    if index == 0 { CaptureBackendPriority::NpcapFirst } else { CaptureBackendPriority::WinDivertFirst }
            },
            window,
            cx,
        );
        let check_row = self.capture_check_row(&check, animate, window, cx);
        let capture = group(vec![backend, check_row]);

        let focus = self.focus_of("recording", true, window, cx);
        let runtime_entity = self.runtime.clone();
        let recording_control = match recording {
            Some(progress) => div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_color(rgb(widgets::BAD))
                        .font_family(DISPLAY_FONT)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icon(Icon::Record, 10., rgb(widgets::BAD)))
                        .child(format!(
                            "{} packets \u{b7} {}",
                            format::grouped(progress.packets as f64),
                            format::file_size(progress.bytes)
                        )),
                )
                .child(button(
                    "recording",
                    "Stop recording",
                    Some(Icon::Stop),
                    ButtonKind::Danger,
                    false,
                    &focus,
                    Some(Rc::new(move |(), _, cx| runtime_entity.update(cx, |runtime, cx| runtime.stop_recording(cx)))),
                )),
            None => div().child(button(
                "recording",
                "Start recording",
                Some(Icon::Record),
                ButtonKind::Secondary,
                false,
                &focus,
                has_meter.then(|| -> Handler<()> {
                    Rc::new(move |(), _, cx| runtime_entity.update(cx, |runtime, cx| runtime.start_recording(cx)))
                }),
            )),
        };
        let recording_text = div()
            .flex()
            .flex_col()
            .child(description("Writes this session's raw packets to a file, for protocol work."))
            .children(recording_error.map(|error| note(Icon::TriangleAlert, error, rgb(widgets::BAD))))
            .into_any_element();
        let recording_group = group(vec![
            self.switch_row(
                "record-unfamiliar",
                "Record unfamiliar servers",
                Some("The first two minutes on a server no profile matches. The newest five are kept and never leave this PC."),
                settings.capture.record_unfamiliar_servers,
                |all, on| all.capture.record_unfamiliar_servers = on,
                window,
                cx,
            ),
            row("Packet recording", Some(recording_text), recording_control),
        ]);

        let mut recording_rows: Vec<Div> = Vec::new();
        for (index, item) in recordings.iter().enumerate() {
            let replaying = matches!(&replay, ReplayProgress::Running { name } if *name == item.name);
            let mut line = format!("{} \u{b7} {}", item.when, format::file_size(item.bytes));
            if item.unfamiliar_server {
                line.push_str(" \u{b7} unfamiliar server");
            }
            let runtime_entity = self.runtime.clone();
            let actions = if replaying {
                let focus = self.focus_of("cancel-replay", true, window, cx);
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(badge(Some(spinner(("replaying", index), 12., rgb(widgets::WARN), animate)), "Replaying", Tone::Warn))
                    .child(button(
                        "cancel-replay",
                        "Cancel replay",
                        None,
                        ButtonKind::Secondary,
                        true,
                        &focus,
                        Some(Rc::new(move |(), _, cx| runtime_entity.update(cx, |runtime, cx| runtime.cancel_replay(cx)))),
                    ))
            } else {
                let key = format!("replay-{index}");
                let focus = self.focus_of(&key, true, window, cx);
                let item = item.clone();
                div().child(button(
                    key,
                    "Replay",
                    Some(Icon::Play),
                    ButtonKind::Secondary,
                    true,
                    &focus,
                    can_replay.then(|| -> Handler<()> {
                        Rc::new(move |(), _, cx| runtime_entity.update(cx, |runtime, cx| runtime.replay_recording(&item, cx)))
                    }),
                ))
            };
            recording_rows.push(list_row(Some(Icon::FileText), item.name.clone(), line).child(actions));
        }
        if recording_rows.is_empty() {
            recording_rows.push(list_row(Some(Icon::FileText), "No recordings yet", "Start one above, or let an unfamiliar server start one."));
        }
        let replay_note = match &replay {
            ReplayProgress::Failed(error) => Some(note(Icon::TriangleAlert, format!("The replay failed: {error}"), rgb(widgets::BAD))),
            ReplayProgress::Finished { packets } => Some(note(
                Icon::CircleCheck,
                format!("Replayed {} packets.", format::grouped(*packets as f64)),
                rgb(widgets::GOOD),
            )),
            ReplayProgress::Idle | ReplayProgress::Running { .. } => None,
        };

        let mobs = group(vec![self.switch_row(
            "collect-mobs",
            "Collect mob data",
            Some("Records every mob you fight (code, HP, counts) into mobs.json, to help name the ones the bundled list misses. No player data. Boss corrections are saved either way."),
            settings.capture.collect_mob_data,
            |all, on| all.capture.collect_mob_data = on,
            window,
            cx,
        )]);

        let folder_focus = self.focus_of("log-folder", true, window, cx);
        let open_logs = self.button_handler(cx, |this, _, cx| this.hand_over("log-folder", files::open_folder(Folder::Logs), cx));
        let log_text = div()
            .flex()
            .flex_col()
            .child(description("For when something does not look right."))
            .children(self.error_under("log-folder"))
            .into_any_element();
        let log = group(vec![
            self.switch_row(
                "decision-log",
                "Decision log",
                Some("Writes the meter's decisions (which board, which boss, who you are) to the log file. Lines may include player names; the file stays on this PC."),
                settings.capture.decision_log,
                |all, on| all.capture.decision_log = on,
                window,
                cx,
            ),
            row(
                "Meter log",
                Some(log_text),
                button("log-folder", "Open log folder", Some(Icon::FolderOpen), ButtonKind::Secondary, false, &folder_focus, Some(open_logs)),
            ),
        ]);

        div()
            .flex()
            .flex_col()
            .child(page_head(
                "Capture",
                Some("How the meter sees the game's traffic. Everything here is detected; there is nothing to enter."),
            ))
            .child(group_title("Connection", true))
            .child(tiles)
            .child(group_title("Capture", false))
            .child(capture)
            .child(group_title("Recording", false))
            .child(recording_group)
            .child(
                group_title("Recordings", false).child(
                    div()
                        .font_weight(FontWeight::NORMAL)
                        .text_color(rgba(FAINT))
                        .child("Replay runs one through the live meter, which has to be running."),
                ),
            )
            .child(group(recording_rows))
            .children(replay_note)
            .child(group_title("Mob data", false))
            .child(mobs)
            .child(group_title("Log", false))
            .child(log)
    }

    fn capture_check_row(&mut self, check: &CaptureCheck, animate: bool, window: &Window, cx: &mut Context<Self>) -> Div {
        let focus = self.focus_of("capture-check", true, window, cx);
        let running = matches!(check, CaptureCheck::Running);
        let runtime = self.runtime.clone();
        let rerun: Option<Handler<()>> = (!running).then(|| -> Handler<()> {
            Rc::new(move |(), _, cx| runtime.update(cx, |runtime, cx| runtime.run_capture_check(cx)))
        });
        let mut text = div().flex().flex_col().child(description("Administrator rights, Npcap and WinDivert, as at start-up."));
        match check {
            CaptureCheck::NotRun => {}
            CaptureCheck::Running => {
                text = text.child(
                    div()
                        .mt(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(rgba(MUTED))
                        .child(spinner("checking", 13., rgba(MUTED), animate))
                        .child("Checking\u{2026}"),
                );
            }
            CaptureCheck::Done(report) => {
                let summary_tone = if report.ready { rgb(widgets::GOOD) } else { rgb(widgets::BAD) };
                let summary_icon = if report.ready { Icon::CircleCheck } else { Icon::TriangleAlert };
                text = text.child(note(summary_icon, report.summary.clone(), summary_tone)).children(report.checks.iter().map(
                    |check: &preflight::Check| {
                        let (glyph, colour) = match (check.ok, check.weight) {
                            (true, _) => (Icon::CircleCheck, rgb(widgets::GOOD)),
                            (false, Weight::Required) => (Icon::TriangleAlert, rgb(widgets::BAD)),
                            (false, Weight::Optional) => (Icon::TriangleAlert, rgb(widgets::WARN)),
                        };
                        note(glyph, format!("{}: {}", check.label, check.detail), colour)
                    },
                ));
            }
        }
        row(
            "Capture check",
            Some(text.into_any_element()),
            button("capture-check", "Re-run", Some(Icon::ShieldCheck), ButtonKind::Secondary, false, &focus, rerun),
        )
    }

    fn bests_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let bests = self.runtime.read(cx).bests.clone();
        let head = page_head(
            "Personal bests",
            Some("Your best DPS per boss and character. Boss fights of 15 s or more count. Kept when history is deleted."),
        );
        if bests.is_empty() {
            let empty = div()
                .flex()
                .items_center()
                .gap(px(12.))
                .min_h(px(96.))
                .px(px(16.))
                .child(icon(Icon::Trophy, 16., rgba(FAINT)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child("None yet"))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(rgba(MUTED))
                                .child("Beat a boss for 15 s or more and your best appears here, per character."),
                        ),
                );
            return div().flex().flex_col().child(head).child(div().mt(px(18.)).child(group(vec![empty])));
        }

        let bosses = distinct(bests.iter().map(|best| best.boss.as_str()));
        let characters = distinct(bests.iter().map(|best| best.character.as_str()));
        let count_text = format!(
            "{bosses} {}, {characters} {}",
            if bosses == 1 { "boss" } else { "bosses" },
            if characters == 1 { "character" } else { "characters" }
        );
        let confirming = self.confirm_bests.is_some();
        let reset_focus = self.focus_of("bests-reset", true, window, cx);
        let reset = if confirming {
            self.button_handler(cx, |this, _, cx| {
                this.confirm_bests = None;
                this.runtime.update(cx, |runtime, cx| runtime.reset_personal_bests(cx));
                cx.notify();
            })
        } else {
            self.button_handler(cx, |this, _, cx| {
                this.confirm_bests = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(CONFIRM_FOR).await;
                    let _ = this.update(cx, |this, cx| {
                        this.confirm_bests = None;
                        cx.notify();
                    });
                }));
                cx.notify();
            })
        };
        let mut actions = div().flex().items_center().gap(px(8.)).child(button(
            "bests-reset",
            if confirming { "Click again to reset" } else { "Reset all" },
            Some(Icon::Trash),
            if confirming { ButtonKind::Danger } else { ButtonKind::Secondary },
            false,
            &reset_focus,
            Some(reset),
        ));
        if confirming {
            let cancel_focus = self.focus_of("bests-cancel", true, window, cx);
            let cancel = self.button_handler(cx, |this, window, cx| {
                this.confirm_bests = None;
                this.focus_control("bests-reset", window, cx);
                cx.notify();
            });
            actions = actions.child(button("bests-cancel", "Cancel", None, ButtonKind::Ghost, false, &cancel_focus, Some(cancel)));
        }

        let header = |text: &'static str, width: Option<f32>, right: bool| {
            div()
                .when_some(width, |cell, width| cell.w(px(width)).flex_none())
                .when(width.is_none(), |cell| cell.flex_1().min_w_0())
                .flex()
                .when(right, |cell| cell.justify_end())
                .child(text)
        };
        let table_head = div()
            .h(px(34.))
            .px(px(16.))
            .flex()
            .items_center()
            .gap(px(16.))
            .text_size(px(12.))
            .text_color(rgba(FAINT))
            .child(header("Boss", None, false))
            .child(header("Character", Some(150.), false))
            .child(header("Best DPS", Some(100.), true))
            .child(header("Date", Some(120.), true));
        let rows = bests.iter().enumerate().map(|(index, best)| {
            let character = settings.overlay.display_name(&best.character);
            let dps = format::compact(best.dps);
            div()
                .id(("best", index))
                .role(Role::Row)
                .aria_label(SharedString::from(format!("{}, {character}, {dps} per second, {}", best.boss, best.when)))
                .h(px(40.))
                .px(px(16.))
                .flex()
                .items_center()
                .gap(px(16.))
                .border_t_1()
                .border_color(rgba(LINE))
                .child(div().flex_1().min_w_0().truncate().child(best.boss.clone()))
                .child(div().w(px(150.)).flex_none().truncate().child(character))
                .child(
                    div()
                        .w(px(100.))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .items_baseline()
                        .child(div().font_family(DISPLAY_FONT).text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(dps))
                        .child(div().text_color(rgba(FAINT)).child("/s")),
                )
                .child(div().w(px(120.)).flex_none().flex().justify_end().text_color(rgba(MUTED)).child(best.when.clone()))
        });
        let table = div()
            .id("bests-table")
            .role(Role::Table)
            .aria_label("Personal bests")
            .flex()
            .flex_col()
            .child(table_head)
            .children(rows);
        let card = div()
            .mt(px(18.))
            .flex()
            .flex_col()
            .rounded(px(10.))
            .border_1()
            .border_color(rgba(LINE))
            .bg(rgb(widgets::SURFACE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .pt(px(10.))
                    .px(px(16.))
                    .pb(px(4.))
                    .child(div().flex_1().text_size(px(12.)).text_color(rgba(MUTED)).child(count_text))
                    .child(actions),
            )
            .child(table);
        div().flex().flex_col().child(head).child(card)
    }

    fn reminders_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let reminders = &settings.reminders;
        let on = reminders.enabled;
        let master = self.switch_row(
            "reminders",
            "Event reminders",
            Some("A small card above the meter before each event switched on below. It goes by itself after 30 s; a click dismisses it."),
            on,
            |all, on| all.reminders.enabled = on,
            window,
            cx,
        );
        let lead = self.select_control(Select::LeadTime, settings, on, window, cx);
        let sound_focus = self.focus_of("reminder-sound", true, window, cx);
        let sound_toggle = on.then(|| self.set(|all: &mut AppSettings, on| all.reminders.sound = on));
        let sound = switch("reminder-sound", "Sound", reminders.sound, &sound_focus, sound_toggle);
        let test = self.test_reminder_control(on, window, cx);
        let how = group(vec![
            master,
            reminder_row(on, "Remind me", "How long before the event starts.", lead),
            reminder_row(on, "Sound", "Windows' Chimes sound, once per reminder.", sound),
            reminder_row(on, "Test reminder", "Shows a reminder above the meter now, with its sound.", test),
        ]);

        let mut page = div()
            .flex()
            .flex_col()
            .child(page_head(
                "Reminders",
                Some("A reminder before timed events, so you can get there in time. Times follow this PC's clock and are set for the European servers, which run on Berlin time."),
            ))
            .child(group_title("How you are reminded", true))
            .child(how);
        for (group_index, title) in ["Every hour", "Every 3 hours", "Weekly", "Daily"].into_iter().enumerate() {
            let mut rows = Vec::new();
            for info in EVENTS.iter().filter(|info| how_often(info.rule) == group_index) {
                let id = info.id;
                let (key, label, text) = event_row(id);
                let focus = self.focus_of(key, true, window, cx);
                let toggle = on.then(|| self.set(move |all: &mut AppSettings, on| all.reminders.set_event_enabled(id, on)));
                rows.push(reminder_row(on, label, text, switch(key, label, reminders.event_enabled(id), &focus, toggle)));
            }
            page = page.child(group_title(title, false)).child(group(rows));
        }
        page
    }

    /// "Show a test reminder", and the note that it went above the meter,
    /// which Settings may be covering. The note hangs to the button's left
    /// without taking room, so the row keeps its height and the button stays
    /// under the pointer.
    fn test_reminder_control(&mut self, enabled: bool, window: &Window, cx: &mut Context<Self>) -> Div {
        let focus = self.focus_of("test-reminder", true, window, cx);
        let press = enabled.then(|| self.button_handler(cx, |this, _, cx| this.show_test_reminder(cx)));
        let shown = self.test_shown.is_some().then(|| {
            let note = badge(Some(icon(Icon::Check, 12., rgb(widgets::GOOD)).into_any_element()), "Shown above the meter", Tone::Good)
                .id("test-shown")
                .debug_selector(|| "test-shown".to_string())
                .role(Role::Status)
                .aria_label("Shown above the meter");
            let note = if cx.reduce_motion() {
                note.into_any_element()
            } else {
                note.with_animation("test-shown-fade", Animation::new(CHIP_FADE), |note, t| note.opacity(t)).into_any_element()
            };
            div().absolute().top_0().bottom_0().right(relative(1.)).pr(px(10.)).flex().items_center().child(note)
        });
        div().relative().flex().items_center().children(shown).child(button(
            "test-reminder",
            "Show a test reminder",
            Some(Icon::Play),
            ButtonKind::Secondary,
            false,
            &focus,
            press,
        ))
    }

    fn show_test_reminder(&mut self, cx: &mut Context<Self>) {
        reminders::global(cx).update(cx, |reminders, cx| reminders.test_reminder(cx));
        self.test_shown = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TEST_SHOWN_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.test_shown = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn test_note_is_shown(&self) -> bool {
        self.test_shown.is_some()
    }

    fn about_page(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let storage = match &self.runtime.read(cx).storage {
            StorageState::NotLoaded | StorageState::Loading => "Counting\u{2026}".to_string(),
            StorageState::Loaded(summary) => format!(
                "Settings {} \u{b7} Fight history {} {}, {} \u{b7} Recordings {} {}, {}",
                format::file_size(summary.settings_bytes),
                summary.history_records,
                if summary.history_records == 1 { "record" } else { "records" },
                format::file_size(summary.history_bytes),
                summary.recordings,
                if summary.recordings == 1 { "file" } else { "files" },
                format::file_size(summary.recordings_bytes),
            ),
        };

        let readme_focus = self.focus_of("readme", true, window, cx);
        let open_readme = self.button_handler(cx, |this, _, cx| this.hand_over("readme", files::open_document(Document::Readme), cx));
        let licence_focus = self.focus_of("licence", true, window, cx);
        let open_licence = self.button_handler(cx, |this, _, cx| this.hand_over("licence", files::open_document(Document::Licence), cx));
        let version_label = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child("AION 2 DPS Meter")
            .child(
                div()
                    .font_family(DISPLAY_FONT)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgba(FAINT))
                    .child(env!("CARGO_PKG_VERSION")),
            );
        let meter = group(vec![
            row(
                version_label,
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .child(description("A real-time DPS meter for AION 2, built for the global servers."))
                        .children(self.error_under("readme"))
                        .into_any_element(),
                ),
                button("readme", "Source and notice", Some(Icon::External), ButtonKind::Secondary, false, &readme_focus, Some(open_readme)),
            ),
            row(
                "Licence: GPL-3.0-only",
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .child(description(
                            "Free software: you may share and change it under the GNU General Public License, version 3.",
                        ))
                        .children(self.error_under("licence"))
                        .into_any_element(),
                ),
                button("licence", "Read the licence", Some(Icon::Scale), ButtonKind::Secondary, false, &licence_focus, Some(open_licence)),
            ),
        ]);

        let credit = |title: &'static str, tag: Option<&'static str>, text: &'static str| {
            div()
                .flex()
                .flex_col()
                .min_h(px(46.))
                .py(px(8.))
                .px(px(16.))
                .child(
                    div().flex().items_center().gap(px(6.)).font_weight(FontWeight::MEDIUM).child(title).children(tag.map(|tag| {
                        div()
                            .h(px(20.))
                            .px(px(7.))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .border_1()
                            .border_color(rgba(widgets::LINE_STRONG))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgba(MUTED))
                            .child(tag)
                    })),
                )
                .child(div().text_size(px(12.)).text_color(rgba(MUTED)).child(text))
        };
        let credits = group(vec![
            credit(
                "Helveticxa/Aether-Aion2-DPS-meter-Global",
                Some("Forked from"),
                "This meter is a fork of Aether, by Helveticxa. GPL-3.0-only.",
            ),
            credit(
                "ZDYoung0519/NOIA2",
                Some("Upstream"),
                "By zdyoung. Aether, which this meter is forked from, is itself a fork of NOIA2: the capture pipeline, packet parsers, overlay and game-data catalogues come from it. GPL-3.0-only.",
            ),
            credit(
                "Kuroukihime/AIon2-Dps-Meter",
                None,
                "English monster and boss names, the Fighter's skill names, and the server codes. GPL-3.0.",
            ),
            credit(
                "References",
                None,
                "TK-open-public/Aion2-Dps-Meter \u{b7} taengu/Aion2-Dps-Meter \u{b7} p62003/aletheia_AION2_DPS_Meter",
            ),
            credit(
                "Built with",
                None,
                "GPUI (Apache-2.0) \u{b7} Lucide icons (ISC) \u{b7} Barlow Semi Condensed (SIL OFL 1.1) \u{b7} WinDivert, the bundled fallback capture driver",
            ),
            credit("Npcap", None, "Not bundled with the meter: you install it from npcap.com, under its own licence."),
        ]);

        let folder_focus = self.focus_of("data-folder", true, window, cx);
        let open_data = self.button_handler(cx, |this, _, cx| this.hand_over("data-folder", files::open_folder(Folder::Data), cx));
        let reset_focus = self.focus_of("reset-settings", true, window, cx);
        let reset = self.button_handler(cx, |this, window, cx| this.open_reset_dialog(window, cx));
        let storage_group = group(vec![
            row(
                "Stored on this PC",
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .child(description(storage))
                        .children(self.error_under("data-folder"))
                        .into_any_element(),
                ),
                button("data-folder", "Open folder", Some(Icon::FolderOpen), ButtonKind::Secondary, false, &folder_focus, Some(open_data)),
            ),
            row(
                "Reset settings",
                Some(description("Settings go back to their defaults.")),
                button("reset-settings", "Reset settings\u{2026}", None, ButtonKind::Danger, false, &reset_focus, Some(reset)),
            ),
        ]);

        div()
            .flex()
            .flex_col()
            .child(page_head("About", None))
            .child(group_title("AION 2 DPS Meter", true))
            .child(meter)
            .child(group_title("Credits", false))
            .child(credits)
            .child(group_title("Storage", false))
            .child(storage_group)
    }

    fn title_bar(&self) -> Div {
        let control = |id: &'static str, glyph: Icon, area: WindowControlArea, hover: u32| {
            div()
                .id(id)
                .w(px(46.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .window_control_area(area)
                .hover(move |style| style.bg(rgba(hover)))
                .child(icon(glyph, 14., rgba(MUTED)))
        };
        div()
            .flex_none()
            .h(px(36.))
            .pl(px(12.))
            .flex()
            .items_center()
            .bg(rgb(SIDE))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .window_control_area(WindowControlArea::Drag)
                    .font_family(DISPLAY_FONT)
                    .text_size(px(13.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(
                        div()
                            .size(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .bg(rgb(widgets::CREAM))
                            .child(icon(Icon::Gauge, 14., rgb(widgets::INK))),
                    )
                    .child("AION 2 DPS Meter"),
            )
            .child(control("minimise", Icon::Minus, WindowControlArea::Min, widgets::SOFT))
            .child(control("close", Icon::Close, WindowControlArea::Close, 0xc42b1cff))
    }

    fn sidebar(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let mut items = Vec::with_capacity(Section::ALL.len());
        for section in Section::ALL {
            let focus = self.focus_of(section.key(), false, window, cx);
            items.push(
                side_item(section.key(), section.icon(), section.label(), section == self.section, &focus)
                    .on_click(cx.listener(move |this, _, _, cx| this.show_section(section, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        let at = Section::ALL.iter().position(|candidate| *candidate == section).unwrap_or(0);
                        let next = match event.keystroke.key.as_str() {
                            "down" => Section::ALL[(at + 1) % Section::ALL.len()],
                            "up" => Section::ALL[(at + Section::ALL.len() - 1) % Section::ALL.len()],
                            "home" => Section::ALL[0],
                            "end" => Section::ALL[Section::ALL.len() - 1],
                            _ => return,
                        };
                        cx.stop_propagation();
                        this.show_section(next, cx);
                        this.focus_control(next.key(), window, cx);
                    })),
            );
        }
        let (dot, status): (gpui::Hsla, &str) = match &self.runtime.read(cx).capture.meter {
            MeterTile::Running { .. } => (rgb(widgets::GOOD).into(), "Running"),
            MeterTile::Stopped => (rgba(FAINT).into(), "Stopped"),
            MeterTile::Unavailable(_) => (rgb(widgets::BAD).into(), "Not running"),
        };
        div()
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .flex()
            .flex_col()
            .pt(px(8.))
            .px(px(10.))
            .pb(px(10.))
            .bg(rgb(SIDE))
            .child(
                div()
                    .id("sections")
                    .role(Role::TabList)
                    .aria_label("Settings sections")
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .mt(px(6.))
                            .mb(px(4.))
                            .px(px(10.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgba(FAINT))
                            .child("Settings"),
                    )
                    .children(items),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("meter-status")
                    .role(Role::Status)
                    .aria_label(SharedString::from(format!("Meter: {status}")))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .py(px(9.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .bg(rgba(widgets::SOFT))
                    .text_size(px(12.))
                    .child(div().size(px(7.)).rounded_full().bg(dot))
                    .child(status),
            )
    }

    fn notices(&self, cx: &App) -> Vec<Div> {
        let store = self.store.read(cx);
        let mut notices = Vec::new();
        if let Some(notice) = store.load_notice() {
            notices.push(alert("The stored settings were set aside", notice.to_string(), Tone::Warn));
        }
        if let Some(error) = store.save_error() {
            notices.push(alert("Settings could not be saved", error.to_string(), Tone::Bad));
        }
        notices
    }

    fn reset_dialog(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let cancel_focus = self.focus_of("dialog-cancel", false, window, cx);
        let reset_focus = self.focus_of("dialog-reset", false, window, cx);
        let cancel = self.button_handler(cx, |this, window, cx| this.close_reset_dialog(window, cx));
        let reset = self.button_handler(cx, |this, window, cx| this.reset_settings(window, cx));
        let panel = div()
            .id("reset-dialog")
            .role(Role::AlertDialog)
            .aria_label(RESET_TITLE)
            .aria_description(RESET_BODY)
            .w(px(400.))
            .pt(px(20.))
            .px(px(20.))
            .pb(px(16.))
            .occlude()
            .rounded(px(12.))
            .border_1()
            .border_color(rgba(POPOVER_BORDER))
            .bg(rgb(RAISED))
            .shadow(widgets::dialog_shadow())
            .child(div().font_family(DISPLAY_FONT).text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(RESET_TITLE))
            .child(div().mt(px(8.)).text_color(rgba(MUTED)).child(RESET_BODY))
            .child(
                div()
                    .mt(px(18.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(button("dialog-cancel", "Cancel", None, ButtonKind::Secondary, false, &cancel_focus, Some(cancel)))
                    .child(button("dialog-reset", "Reset settings", None, ButtonKind::Danger, false, &reset_focus, Some(reset))),
            );
        let scrim = div()
            .id("dialog-scrim")
            .debug_selector(|| "dialog-scrim".to_string())
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(SCRIM))
            .on_any_mouse_down(cx.listener(|this, _, window, cx| this.close_reset_dialog(window, cx)))
            .child(panel);
        if cx.reduce_motion() {
            scrim.into_any_element()
        } else {
            scrim.with_animation("dialog-fade", Animation::new(DIALOG_FADE), |scrim, t| scrim.opacity(t)).into_any_element()
        }
    }

    /// The select's list over everything, and the surface under it that
    /// closes it on a press anywhere else.
    fn select_layer(&self, window: &Window, cx: &mut Context<Self>) -> Option<Vec<AnyElement>> {
        let (select, lit) = self.select_open?;
        let trigger = self.select_anchor.bounds()?;
        let options = select.options();
        let height = px(options.len() as f32 * OPTION_HEIGHT + 10.);
        let room_below = window.viewport_size().height - trigger.bottom() - px(12.);
        let below = if height <= room_below {
            trigger
        } else {
            Bounds::new(point(trigger.left(), trigger.top() - height - px(8.)), size(trigger.size.width, px(0.)))
        };
        let view = cx.entity().downgrade();
        let pick: Handler<usize> = Rc::new({
            let view = view.clone();
            move |index, window, cx| {
                let _ = view.update(cx, |this, cx| this.pick(select, index, window, cx));
            }
        });
        let hover: Handler<usize> = Rc::new(move |index, _, cx| {
            let _ = view.update(cx, |this, cx| {
                if this.select_open.is_some_and(|(_, lit)| lit != index) {
                    this.select_open = Some((select, index));
                    cx.notify();
                }
            });
        });
        let scrim = div()
            .id("select-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .on_any_mouse_down(cx.listener(|this, _, window, cx| this.blur(window, cx)))
            .into_any_element();
        let current = select.current(self.store.read(cx).get());
        let menu = select_menu(below, select.label(), &options, current, lit, pick, hover, !cx.reduce_motion());
        Some(vec![scrim, menu])
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.store.read(cx).get().clone();
        let page = match self.section {
            Section::Overlay => self.overlay_page(&settings, window, cx),
            Section::Behaviour => self.behaviour_page(&settings, window, cx),
            Section::Capture => self.capture_page(&settings, window, cx),
            Section::Bests => self.bests_page(&settings, window, cx),
            Section::Reminders => self.reminders_page(&settings, window, cx),
            Section::About => self.about_page(window, cx),
        };
        let sidebar = self.sidebar(window, cx);
        let dialog = self.reset_dialog_open.then(|| self.reset_dialog(window, cx));
        let select = self.select_layer(window, cx);
        let notices = self.notices(cx);
        let section = self.section;

        div()
            .id("settings")
            .role(Role::Window)
            .aria_label(WINDOW_TITLE)
            .track_focus(&self.root_focus)
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(CANVAS))
            .font_family(BODY_FONT)
            .font_features(self.figures.clone())
            .text_size(px(13.))
            .line_height(relative(1.4))
            .text_color(rgba(TEXT))
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .capture_any_mouse_up(cx.listener(|this, _, _, _| this.dragging = None))
            .child(self.title_bar())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(sidebar)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .relative()
                            .child(
                                div()
                                    .id("page")
                                    .debug_selector(|| "page".to_string())
                                    .role(Role::TabPanel)
                                    .aria_label(section.label())
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.scroll)
                                    .pt(px(24.))
                                    .px(px(30.))
                                    .pb(px(30.))
                                    // A scroll container lays its content out at
                                    // max-content width, and GPUI measures text
                                    // unwrapped there: the page takes the
                                    // container's width instead, so text wraps.
                                    .child(div().w_full().flex().flex_col().children(notices).child(page)),
                            )
                            .child(scroll_thumb(self.scroll.clone())),
                    ),
            )
            .children(select.into_iter().flatten())
            .children(dialog)
    }
}

fn lead_label(minutes: u8) -> SharedString {
    format!("{minutes} min before").into()
}

/// Which of the page's event groups a rule belongs to: every hour, every 3
/// hours, weekly, daily.
fn how_often(rule: Rule) -> usize {
    match rule {
        Rule::Hourly { .. } => 0,
        Rule::EveryHours { .. } => 1,
        Rule::Weekly { .. } => 2,
        Rule::Daily { .. } => 3,
    }
}

/// An event's switch: its key, label and the sentence giving its times and
/// place.
fn event_row(id: EventId) -> (&'static str, &'static str, &'static str) {
    match id {
        EventId::ShugoFestival => (
            "event-shugo",
            "Shugo Festival",
            "At :00. The minigames, with 3 min to join; which games are open shows only in the game.",
        ),
        EventId::DimensionalInvasion => {
            ("event-invasion", "Dimensional Invasion", "At :30. Somewhere in the field, with 3 min to join.")
        }
        EventId::RiftPortals => (
            "event-rifts",
            "Rift portals",
            "02:00, 05:00, 08:00, 11:00, 14:00, 17:00, 20:00 and 23:00. Altgard, Verteron, Morheim and Eltnen.",
        ),
        EventId::WatcherKaira => (
            "event-kaira",
            "Watcher Kaira",
            "01:00, 04:00, 07:00, 10:00, 13:00, 16:00, 19:00 and 22:00. Field boss in Chaotic Lower Reshanta.",
        ),
        EventId::ArtifactSiege => {
            ("event-siege", "Artifact Siege", "Monday, Thursday and Saturday at 21:00. Chaotic Lower Reshanta.")
        }
        EventId::AbyssFieldBosses => (
            "event-abyss",
            "Abyss field bosses",
            "Monday, Thursday and Saturday at 21:30. Three Executors in Lower Reshanta, three more bosses in Middle Reshanta.",
        ),
        EventId::GuardianLordNahma => {
            ("event-nahma", "Guardian Lord Nahma", "Friday and Sunday at 21:00. Lv 65, and the Enraged Lv 80.")
        }
        EventId::ArenaOfTactics => (
            "event-arena",
            "Arena of Tactics opens",
            "11:00 and 19:00, when the 10 vs 10 queue opens (until 14:00 and 21:00).",
        ),
        EventId::RewardKeys => ("event-keys", "Reward keys refill", "16:00. Shugo Festival and Dimensional Invasion keys."),
    }
}

fn reminder_row(enabled: bool, label: &'static str, text: &'static str, control: impl IntoElement) -> Div {
    if enabled {
        row(label, Some(description(text)), control)
    } else {
        dimmed_row(label, Some(description(text)), control)
    }
}

/// The open select's label for a stored value: an option's, or the minutes
/// of a value set outside the page.
fn end_fight_label(secs: u64) -> SharedString {
    match END_FIGHT_AFTER_OPTIONS.iter().find(|(option, _)| *option == secs) {
        Some((_, label)) => SharedString::new_static(label),
        None if secs % 60 == 0 => format!("{} min", secs / 60).into(),
        None => format!("{:.1} min", secs as f64 / 60.).into(),
    }
}

fn distinct<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    let mut seen: Vec<&str> = values.collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len()
}

fn preview_zoom() -> f32 {
    PREVIEW_CARD / board_width(&OverlaySettings::default())
}

/// The overlay settings the preview draws: the user's, with the content
/// scale lowered to the largest whose card fits the stage.
fn preview_settings(mut overlay: OverlaySettings) -> OverlaySettings {
    let room = PREVIEW_WIDTH - 2. * PREVIEW_STAGE_PADDING;
    let fitting = room / (board_width(&overlay) * preview_zoom());
    overlay.content_scale = overlay.scale().min(fitting);
    overlay
}

fn sample_meter(overlay: OverlaySettings) -> MeterModel {
    let mut model = MeterModel::new(overlay, None);
    model.apply(MeterEvent::DpsSnapshot(sample::boss_fight(221.0, 624)), Instant::now());
    model
}

/// Brings the open Settings window back to the front. Not GPUI's
/// `activate_window`: that presses Alt with `SendInput`, and the app injects
/// no input. The request comes from a click in our own meter, so Windows
/// normally grants the foreground; when it refuses, Windows itself flashes
/// the taskbar button instead.
pub fn bring_forward(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Opens the Settings window: a normal window with our own title bar, in the
/// middle of the primary screen, activated. GPUI shows it with
/// `SetWindowPlacement` (`SW_SHOWNORMAL`), never through its `activate`.
/// Off screen (dev captures) it is shown past every monitor, unactivated and
/// out of the taskbar.
pub fn open_window(
    placement: Placement,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<SettingsView>,
    cx: &mut App,
) -> Result<WindowHandle<SettingsView>, String> {
    let (width, height) = WINDOW_SIZE;
    let on_screen = placement == Placement::Screen;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(width), px(height)), cx))),
        titlebar: Some(TitlebarOptions {
            title: Some(WINDOW_TITLE.into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        focus: on_screen,
        show: on_screen,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: false,
        is_minimizable: true,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    };
    let handle = cx.open_window(options, build).map_err(|error| format!("cannot open the Settings window: {error:#}"))?;
    #[cfg(feature = "probes")]
    if placement == Placement::OffScreen {
        let shown = handle
            .update(cx, |_, window, _| -> Result<(), String> {
                use windows::Win32::UI::WindowsAndMessaging::{WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW};
                let hwnd = crate::overlay::hwnd(window)?;
                crate::overlay::update_ex_style(hwnd, |style| {
                    (style & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0
                });
                let scale = window.scale_factor();
                // GPUI keeps the size it created the hidden window with until
                // it resizes the window itself.
                window.resize(size(px(width), px(height)));
                crate::overlay::off_screen::show(hwnd, width * scale, height * scale)
            })
            .map_err(|error| format!("cannot reach the Settings window: {error:#}"))
            .and_then(|shown| shown);
        if let Err(error) = shown {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            return Err(error);
        }
    }
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use gpui::{KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext};

    use super::*;
    use crate::settings::{connect, install};

    struct Opened<'a> {
        view: Entity<SettingsView>,
        store: Entity<Settings>,
        meter: Entity<MeterModel>,
        cx: &'a mut VisualTestContext,
    }

    /// The real view in GPUI's test platform, on settings in memory or in
    /// `data_dir`, connected to an overlay model as the app connects it.
    fn open<'a>(cx: &'a mut TestAppContext, data_dir: Option<&Path>) -> Opened<'a> {
        let (store, meter, runtime) = cx.update(|cx| {
            let store = cx.new(|_| match data_dir {
                Some(dir) => Settings::load(Some(dir)),
                None => Settings::in_memory(AppSettings::default()),
            });
            install(&store, cx);
            reminders::install(&store, false, cx);
            let overlay = store.read(cx).get().overlay.clone();
            let meter = cx.new(|_| MeterModel::new(overlay, None));
            connect(&store, &meter, None, cx);
            let runtime = cx.new(|cx| RuntimeModel::new(meter.clone(), cx));
            (store, meter, runtime)
        });
        let (view, cx) = cx.add_window_view(|window, cx| SettingsView::new(runtime, None, window, cx));
        Opened { view, store, meter, cx }
    }

    impl Opened<'_> {
        /// Key down then up, as a real press: buttons and switches act on the
        /// release, as in a browser.
        fn press(&mut self, key: &str) {
            let keystroke = Keystroke::parse(key).expect("a key name");
            self.cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
            self.cx.simulate_event(KeyUpEvent { keystroke });
        }

        fn focused(&mut self) -> Option<String> {
            let view = self.view.clone();
            self.cx.update(|window, cx| view.read(cx).focused_control(window))
        }

        fn tab_to(&mut self, key: &str) {
            for _ in 0..60 {
                self.press("tab");
                if self.focused().as_deref() == Some(key) {
                    return;
                }
            }
            panic!("Tab never reached {key}");
        }

        fn bounds(&mut self, selector: &'static str) -> Bounds<Pixels> {
            self.cx.debug_bounds(selector).unwrap_or_else(|| panic!("{selector} is not drawn"))
        }

        fn click(&mut self, selector: &'static str) {
            let centre = self.bounds(selector).center();
            self.cx.simulate_click(centre, Modifiers::none());
        }

        /// A point inside the page that no control covers.
        fn click_background(&mut self) {
            let page = self.bounds("page");
            self.cx.simulate_click(page.origin + point(px(6.), px(6.)), Modifiers::none());
        }

        fn settings(&mut self) -> AppSettings {
            let store = self.store.clone();
            self.cx.update(|_, cx| store.read(cx).get().clone())
        }

        fn ring_shown(&mut self) -> bool {
            self.cx.debug_bounds("focus-ring").is_some()
        }

        fn select_open(&mut self) -> bool {
            let view = self.view.clone();
            self.cx.update(|_, cx| view.read(cx).select_is_open())
        }

        fn dialog_open(&mut self) -> bool {
            let view = self.view.clone();
            self.cx.update(|_, cx| view.read(cx).reset_dialog_is_open())
        }

        fn show(&mut self, section: Section) {
            self.tab_to(section.key());
            self.press("enter");
        }
    }

    #[gpui::test]
    fn tab_walks_the_sidebar_then_the_page_in_reading_order_and_wraps(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        assert_eq!(window.focused(), None, "nothing is focused when the window opens");

        let expected = [
            "nav-overlay",
            "nav-behaviour",
            "nav-capture",
            "nav-bests",
            "nav-reminders",
            "nav-about",
            "opacity",
            "scale",
            "layout",
            "percentage",
            "boss-hp",
            "player-name",
            "mask-nicknames",
            "server",
            "combat-power",
            "total-damage",
            "dps",
            "fight-summary",
            "pace",
            "detail-opens",
            "nav-overlay",
        ];
        let mut walked = Vec::new();
        for _ in expected {
            window.press("tab");
            walked.push(window.focused().unwrap_or_default());
            assert!(window.ring_shown(), "a ring on {:?}", walked.last());
        }
        assert_eq!(walked, expected);

        window.press("shift-tab");
        assert_eq!(window.focused().as_deref(), Some("detail-opens"));
        window.press("shift-tab");
        assert_eq!(window.focused().as_deref(), Some("pace"));
    }

    #[gpui::test]
    fn space_and_enter_switch_a_switch_and_the_overlay_follows(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        window.tab_to("dps");
        assert!(window.settings().overlay.show_dps);

        window.press("space");
        assert!(!window.settings().overlay.show_dps);
        let meter = window.meter.clone();
        assert!(!window.cx.update(|_, cx| meter.read(cx).settings.show_dps), "the overlay's model follows");

        window.press("enter");
        assert!(window.settings().overlay.show_dps);
        assert_eq!(window.focused().as_deref(), Some("dps"));
    }

    #[gpui::test]
    fn arrows_page_keys_home_and_end_move_a_slider(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        window.tab_to("opacity");
        assert_eq!(window.settings().overlay.background_alpha, 150, "59%");

        window.press("right");
        assert_eq!(window.settings().overlay.background_alpha, 153, "60%");
        window.press("left");
        window.press("left");
        assert_eq!(window.settings().overlay.background_alpha, 148, "58%");
        window.press("pageup");
        assert_eq!(window.settings().overlay.background_alpha, 173, "68%");
        window.press("end");
        assert_eq!(window.settings().overlay.background_alpha, 255);
        window.press("home");
        assert_eq!(window.settings().overlay.background_alpha, 0);

        window.press("tab");
        assert_eq!(window.focused().as_deref(), Some("scale"));
        window.press("up");
        assert!((window.settings().overlay.content_scale - 1.05).abs() < 1e-6);
        window.press("pagedown");
        assert!((window.settings().overlay.content_scale - 0.8).abs() < 1e-6);
    }

    #[gpui::test]
    fn a_slider_takes_a_press_on_its_track_and_follows_a_drag(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        let track = window.bounds("opacity");
        let at = |fraction: f32| point(track.left() + track.size.width * fraction, track.center().y);

        window.cx.simulate_mouse_down(at(0.25), MouseButton::Left, Modifiers::none());
        assert_eq!(window.settings().overlay.background_alpha, 64, "25%");
        assert_eq!(window.focused().as_deref(), Some("opacity"));
        assert!(!window.ring_shown(), "no ring after a press with the mouse");

        window.cx.simulate_mouse_move(at(0.75), MouseButton::Left, Modifiers::none());
        assert_eq!(window.settings().overlay.background_alpha, 191, "75%");
        window.cx.simulate_mouse_up(at(0.75), MouseButton::Left, Modifiers::none());
        window.cx.simulate_mouse_move(at(0.5), None, Modifiers::none());
        assert_eq!(window.settings().overlay.background_alpha, 191, "released: the slider stays");
    }

    #[gpui::test]
    fn the_select_opens_moves_picks_and_closes_with_escape(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        window.show(Section::Behaviour);
        window.tab_to("end-fight");
        assert_eq!(window.settings().behaviour.end_fight_after_secs, 300);

        window.press("down");
        assert!(window.select_open());
        window.press("down");
        window.press("enter");
        assert!(!window.select_open());
        assert_eq!(window.settings().behaviour.end_fight_after_secs, 600, "the option after 5 min");
        assert_eq!(window.focused().as_deref(), Some("end-fight"));

        window.press("space");
        assert!(window.select_open());
        window.press("up");
        window.press("escape");
        assert!(!window.select_open());
        assert_eq!(window.settings().behaviour.end_fight_after_secs, 600, "Escape picks nothing");
        assert_eq!(window.focused().as_deref(), Some("end-fight"), "the focus is back on the select");

        window.press("enter");
        assert!(window.select_open());
        window.click("option-0");
        assert!(!window.select_open());
        assert_eq!(window.settings().behaviour.end_fight_after_secs, 60);
    }

    #[gpui::test]
    fn the_reset_dialog_keeps_the_focus_and_escape_gives_it_back(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        let store = window.store.clone();
        window.cx.update(|_, cx| store.update(cx, |settings, cx| settings.update(cx, |all| all.overlay.show_damage = true)));
        window.show(Section::About);
        window.tab_to("reset-settings");

        window.press("enter");
        assert!(window.dialog_open());
        assert_eq!(window.focused().as_deref(), Some("dialog-cancel"));
        window.press("tab");
        assert_eq!(window.focused().as_deref(), Some("dialog-reset"));
        window.press("tab");
        assert_eq!(window.focused().as_deref(), Some("dialog-cancel"), "Tab stays in the dialog");
        window.press("shift-tab");
        assert_eq!(window.focused().as_deref(), Some("dialog-reset"));

        window.press("escape");
        assert!(!window.dialog_open());
        assert_eq!(window.focused().as_deref(), Some("reset-settings"));
        assert!(window.settings().overlay.show_damage, "Escape resets nothing");

        window.press("space");
        assert!(window.dialog_open());
        window.press("tab");
        window.press("enter");
        assert!(!window.dialog_open());
        assert_eq!(window.settings(), AppSettings::default());
        assert_eq!(window.focused().as_deref(), Some("reset-settings"));
    }

    #[gpui::test]
    fn a_press_outside_blurs_the_control_and_closes_the_select(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        window.click("dps");
        assert_eq!(window.focused().as_deref(), Some("dps"), "a click focuses the control");
        assert!(!window.settings().overlay.show_dps, "and presses it");
        assert!(!window.ring_shown());

        window.click_background();
        assert_eq!(window.focused(), None);

        window.click("nav-behaviour");
        window.click("end-fight");
        assert!(window.select_open());
        assert_eq!(window.focused().as_deref(), Some("end-fight"));
        window.click_background();
        assert!(!window.select_open());
        assert_eq!(window.focused(), None);
        assert_eq!(window.settings().behaviour.end_fight_after_secs, 300, "the press outside picked nothing");
    }

    #[gpui::test]
    fn a_press_on_the_scroll_thumb_strip_or_the_empty_sidebar_blurs(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        let page = window.bounds("page");
        let sections = window.bounds("nav-about");
        let strip = point(page.right() - px(5.), page.top() + px(40.));
        let empty_sidebar = point(sections.center().x, sections.bottom() + px(60.));
        for (place, at) in [("the scroll thumb strip", strip), ("the empty sidebar", empty_sidebar)] {
            window.click("dps");
            assert_eq!(window.focused().as_deref(), Some("dps"));
            window.cx.simulate_click(at, Modifiers::none());
            assert_eq!(window.focused(), None, "a press on {place} blurs");
        }
    }

    #[gpui::test]
    fn a_change_reaches_the_overlay_and_is_there_after_a_restart(cx: &mut TestAppContext) {
        let dir = temp_dir("restart");
        let mut window = open(cx, Some(&dir));
        window.tab_to("total-damage");
        window.press("space");
        let meter = window.meter.clone();
        assert!(window.cx.update(|_, cx| meter.read(cx).settings.show_damage), "the overlay shows total damage at once");

        window.cx.executor().advance_clock(Duration::from_secs(1));
        window.cx.run_until_parked();
        let restarted = Settings::load(Some(&dir));
        assert!(restarted.get().overlay.show_damage, "settings.json kept the change");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn overlay_settings_change_the_preview_but_never_move_the_rows(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        let rows = ["overlay-rows", "opacity", "layout", "total-damage", "detail-opens"];
        let rows_before = rows.map(|mark| window.bounds(mark));
        let preview_before = window.bounds("preview");
        let changes: [(&str, fn(&mut OverlaySettings)); 6] = [
            ("Total damage on", |overlay| overlay.show_damage = true),
            ("150%", |overlay| overlay.content_scale = 1.5),
            ("5 players", |overlay| overlay.players_shown = 5),
            ("Capsule", |overlay| overlay.layout = Layout::Capsule),
            ("Server on", |overlay| overlay.show_server = true),
            ("70%", |overlay| overlay.content_scale = 0.7),
        ];
        for (change, apply) in changes {
            let store = window.store.clone();
            window.cx.update(|_, cx| store.update(cx, |settings, cx| settings.update(cx, |all| apply(&mut all.overlay))));
            window.cx.run_until_parked();
            assert_eq!(rows.map(|mark| window.bounds(mark)), rows_before, "{change} moved the rows");
            let preview = window.bounds("preview");
            assert_eq!(
                (preview.origin, preview.size.width),
                (preview_before.origin, preview_before.size.width),
                "{change} moved or resized the preview"
            );
        }
    }

    #[gpui::test]
    fn the_reminders_page_works_from_the_keyboard_and_dims_under_its_master_switch(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        window.show(Section::Reminders);
        window.tab_to("reminders");
        let expected = [
            "reminders",
            "lead-time",
            "reminder-sound",
            "test-reminder",
            "event-shugo",
            "event-invasion",
            "event-rifts",
            "event-kaira",
            "event-siege",
            "event-abyss",
            "event-nahma",
            "event-arena",
            "event-keys",
            "nav-overlay",
        ];
        let mut walked = vec![window.focused().unwrap_or_default()];
        for _ in 1..expected.len() {
            window.press("tab");
            walked.push(window.focused().unwrap_or_default());
        }
        assert_eq!(walked, expected, "the page in reading order, then back to the sidebar");

        window.tab_to("lead-time");
        assert_eq!(window.settings().reminders.lead_minutes, 1);
        window.press("down");
        assert!(window.select_open());
        window.press("end");
        window.press("enter");
        assert!(!window.select_open());
        assert_eq!(window.settings().reminders.lead_minutes, 5, "End lights the last option");
        assert_eq!(window.focused().as_deref(), Some("lead-time"));

        window.tab_to("event-kaira");
        window.press("space");
        assert!(window.settings().reminders.events.watcher_kaira);

        window.tab_to("test-reminder");
        let button_before = window.bounds("test-reminder");
        window.press("enter");
        window.cx.run_until_parked();
        let view = window.view.clone();
        assert!(window.cx.update(|_, cx| view.read(cx).test_note_is_shown()), "the note says where the card went");
        assert!(window.cx.debug_bounds("test-shown").is_some(), "the note is drawn");
        assert_eq!(window.bounds("test-reminder"), button_before, "the note takes no room: the button stays under the pointer");
        window.cx.executor().advance_clock(TEST_SHOWN_FOR);
        window.cx.run_until_parked();
        assert!(!window.cx.update(|_, cx| view.read(cx).test_note_is_shown()), "for 4 s");

        window.tab_to("reminders");
        window.press("space");
        assert!(!window.settings().reminders.enabled);
        window.press("tab");
        assert_eq!(window.focused().as_deref(), Some("nav-overlay"), "the dimmed controls are out of the tab order");
        window.click("event-shugo");
        window.click("lead-time");
        assert!(!window.select_open(), "a dimmed select does not open");
        let reminders = window.settings().reminders;
        assert!(reminders.events.shugo_festival, "a dimmed switch does not react");
        assert!(reminders.events.watcher_kaira && reminders.lead_minutes == 5, "every row keeps its own value");

        window.tab_to("reminders");
        window.press("enter");
        assert!(window.settings().reminders.enabled);
        window.press("tab");
        assert_eq!(window.focused().as_deref(), Some("lead-time"), "back in the tab order");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let dir = std::env::temp_dir().join(format!("aether-settings-view-{name}-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp folder");
        dir
    }
}
