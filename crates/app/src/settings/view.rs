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

use meter_core::config::CaptureBackendPriority;
use meter_core::events::MeterEvent;
use meter_core::preflight::Weight;
use gpui::{
    div, point, prelude::*, px, relative, rgb, rgba, size, Animation, AnimationExt as _, AnyElement, App, Bounds,
    Context, Div, Entity, FocusHandle, FontFeatures, FontWeight, KeyDownEvent, MouseButton, MouseMoveEvent, Pixels,
    Point, Render, Role, ScrollHandle, SharedString, Subscription, Task, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowHandle, WindowKind, WindowOptions,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE};

use super::files::{self, Document, Folder};
use super::model::{
    end_fight_after_label, global, AppSettings, CountMode, DetailPlacement, Settings, END_FIGHT_AFTER_OPTIONS,
};
use super::runtime::{CaptureCheck, MeterTile, ReplayProgress, RuntimeModel, StorageState};
use super::widgets::{
    self, alert, backdrop, badge, button, description, dimmed_row, disabled_select, group, group_title, icon, list_row,
    note, page_head, row, scroll_thumb, segmented, select_menu, select_trigger, side_item, slider, spinner, switch, tile,
    ButtonKind, Focus, Handler, Reveal, SliderSpec, Tone, BODY_FONT, CANVAS, DISPLAY_FONT, FAINT, LINE, MUTED,
    OPTION_HEIGHT, POPOVER_BORDER, RAISED, SCRIM, SIDE, TEXT,
};
use crate::assets::Icon;
use crate::format;
use crate::i18n::{self, fl, LanguageChoice};
use crate::meter::{board_width, Layout, LockChanged, MeterModel, MeterView, OverlaySettings, PctMode, PLAYERS_SHOWN_RANGE};
use crate::overlay::Placement;
use crate::reminders::{self, EventId, Rule, EVENTS, LEAD_MINUTES_OPTIONS};
use crate::sample;
use crate::secondary::chrome::Anchor;
use crate::todos::{self, HiddenSummary};

mod capture_check;
mod todos_page;

const WINDOW_SIZE: (f32, f32) = (960., 620.);
const SIDEBAR_WIDTH: f32 = 208.;
const PREVIEW_CARD: f32 = 236.;
const PREVIEW_STAGE_PADDING: f32 = 12.;
/// The approved mockup's preview column. It keeps this width whatever the
/// overlay shows, so the rows beside it never move: a card wider than the
/// stage is drawn at the largest content scale that fits.
const PREVIEW_WIDTH: f32 = 262.;
const OVERLAY_SLIDER_WIDTH: f32 = 120.;
const SLIDER_WIDTH: f32 = 170.;
/// The reset hour's list shows this many hours at a time and scrolls.
const HOURS_SHOWN: usize = 8;
const CONFIRM_FOR: Duration = Duration::from_secs(4);
/// How long "Shown above the meter" stays after a test reminder.
const TEST_SHOWN_FOR: Duration = Duration::from_secs(4);
const DIALOG_FADE: Duration = Duration::from_millis(150);
const CHIP_FADE: Duration = Duration::from_millis(150);
const ALPHA_PER_PERCENT: f64 = 2.55;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Overlay,
    Behaviour,
    Capture,
    Bests,
    Reminders,
    Todos,
    General,
    About,
}

impl Section {
    const ALL: [Self; 8] = [
        Self::Overlay,
        Self::Behaviour,
        Self::Capture,
        Self::Bests,
        Self::Reminders,
        Self::Todos,
        Self::General,
        Self::About,
    ];

    fn label(self) -> String {
        match self {
            Self::Overlay => fl!("settings-section-overlay"),
            Self::Behaviour => fl!("settings-section-behaviour"),
            Self::Capture => fl!("settings-section-capture"),
            Self::Bests => fl!("settings-section-bests"),
            Self::Reminders => fl!("settings-section-reminders"),
            Self::Todos => fl!("settings-section-todos"),
            Self::General => fl!("settings-section-general"),
            Self::About => fl!("settings-section-about"),
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Overlay => Icon::SlidersHorizontal,
            Self::Behaviour => Icon::Activity,
            Self::Capture => Icon::Radio,
            Self::Bests => Icon::Trophy,
            Self::Reminders => Icon::Bell,
            Self::Todos => Icon::ListChecks,
            Self::General => Icon::Languages,
            Self::About => Icon::Info,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Overlay => "nav-overlay",
            Self::Behaviour => "nav-behaviour",
            Self::Capture => "nav-capture",
            Self::Bests => "nav-bests",
            Self::Reminders => "nav-reminders",
            Self::Todos => "nav-todos",
            Self::General => "nav-general",
            Self::About => "nav-about",
        }
    }
}

/// The page's dropdowns. One is open at a time, in `select_open`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Select {
    EndFight,
    LeadTime,
    ResetHour,
    Language,
}

impl Select {
    fn key(self) -> &'static str {
        match self {
            Self::EndFight => "end-fight",
            Self::LeadTime => "lead-time",
            Self::ResetHour => "reset-hour",
            Self::Language => "language",
        }
    }

    fn label(self) -> String {
        match self {
            Self::EndFight => fl!("settings-behaviour-end-fight"),
            Self::LeadTime => fl!("settings-reminders-lead-time"),
            Self::ResetHour => fl!("todos-window-daily-reset"),
            Self::Language => fl!("settings-language"),
        }
    }

    fn min_width(self) -> f32 {
        match self {
            Self::EndFight => 150.,
            Self::LeadTime | Self::ResetHour => 130.,
            Self::Language => 170.,
        }
    }

    fn options(self) -> Vec<SharedString> {
        match self {
            Self::EndFight => END_FIGHT_AFTER_OPTIONS.iter().map(|&secs| end_fight_after_label(secs).into()).collect(),
            Self::LeadTime => LEAD_MINUTES_OPTIONS.iter().map(|&minutes| lead_label(minutes)).collect(),
            Self::ResetHour => (0..24u8).map(hour_label).collect(),
            Self::Language => LanguageChoice::ALL.into_iter().map(language_label).collect(),
        }
    }

    /// How many options the open list shows before it scrolls.
    fn shown(self) -> usize {
        match self {
            Self::ResetHour => HOURS_SHOWN,
            Self::EndFight | Self::LeadTime | Self::Language => usize::MAX,
        }
    }

    /// The stored value's option, if it is one.
    fn current(self, settings: &AppSettings) -> Option<usize> {
        match self {
            Self::EndFight => {
                END_FIGHT_AFTER_OPTIONS.iter().position(|&secs| secs == settings.behaviour.end_fight_after_secs)
            }
            Self::LeadTime => LEAD_MINUTES_OPTIONS.iter().position(|&minutes| minutes == settings.reminders.lead_minutes),
            Self::ResetHour => Some(usize::from(settings.todos.reset_hour.min(23))),
            Self::Language => LanguageChoice::ALL.iter().position(|&choice| choice == settings.general.language),
        }
    }

    fn value(self, settings: &AppSettings) -> SharedString {
        match self {
            Self::EndFight => end_fight_after_label(settings.behaviour.end_fight_after_secs).into(),
            Self::LeadTime => lead_label(settings.reminders.lead_minutes),
            Self::ResetHour => hour_label(settings.todos.reset_hour),
            Self::Language => language_label(settings.general.language),
        }
    }

    /// Keeps the option picked. The reset hour goes through the to-do
    /// tracker, which moves its countdowns at once.
    fn apply(self, index: usize, store: &Entity<Settings>, cx: &mut App) {
        match self {
            Self::EndFight => store.update(cx, |settings, cx| {
                settings.update(cx, |all| all.behaviour.end_fight_after_secs = END_FIGHT_AFTER_OPTIONS[index])
            }),
            Self::LeadTime => {
                store.update(cx, |settings, cx| settings.update(cx, |all| all.reminders.lead_minutes = LEAD_MINUTES_OPTIONS[index]))
            }
            Self::ResetHour => todos::global(cx).update(cx, |todos, cx| todos.set_reset_hour(index.min(23) as u8, cx)),
            Self::Language => {
                store.update(cx, |settings, cx| settings.update(cx, |all| all.general.language = LanguageChoice::ALL[index]))
            }
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

/// The confirm dialogs, each opened by one button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dialog {
    ResetSettings,
    ResetTodos,
}

impl Dialog {
    /// The button that opened it, which takes the focus back.
    fn opener(self) -> &'static str {
        match self {
            Self::ResetSettings => "reset-settings",
            Self::ResetTodos => "todos-reset",
        }
    }
}

/// What Settings → To-dos just did, said for 4 s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TodosDone {
    Restored(usize),
    Reset,
}

/// What the To-dos page shows of the to-dos: hidden built-ins, your own
/// to-dos for the dialog, and a failed save.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TodosSeen {
    hidden: HiddenSummary,
    own_total: usize,
    save_error: Option<String>,
}

impl TodosSeen {
    fn of(todos: &todos::Todos) -> Self {
        Self { hidden: todos.hidden_summary(), own_total: todos.own_total(), save_error: todos.save_error().map(str::to_string) }
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
    select_scroll: ScrollHandle,
    tracks: HashMap<SliderKind, Anchor>,
    /// Where the Overlay page's rows and preview sit, for the dev capture's
    /// check that no setting moves the rows.
    #[cfg(feature = "probes")]
    layout_marks: HashMap<&'static str, Anchor>,
    dragging: Option<SliderKind>,
    dialog: Option<Dialog>,
    /// "Reset all" asked for its second click; the task takes the question back.
    confirm_bests: Option<Task<()>>,
    /// A test reminder was just shown; the task takes the note away.
    test_shown: Option<Task<()>>,
    /// What a To-dos button just did; the task takes the badge away.
    todos_done: Option<(TodosDone, Task<()>)>,
    /// What the To-dos page shows of the to-dos, to redraw only when it
    /// changes.
    todos_seen: TodosSeen,
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

        let locale = i18n::locale(cx);
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
            cx.observe(&todos::global(cx), |this, todos, cx| {
                let seen = TodosSeen::of(todos.read(cx));
                if seen != this.todos_seen {
                    this.todos_seen = seen;
                    cx.notify();
                }
            }),
            // A focused control that goes away (a Cancel replay button once
            // the replay ends) leaves the focus with the window, as a page
            // gives it back to its body, so Tab keeps working.
            cx.on_focus_lost(window, |this, window, cx| window.focus(&this.root_focus, cx)),
            cx.observe_in(&locale, window, |this, _, window, cx| {
                window.set_window_title(&fl!("settings-window-title"));
                this.preview_model.update(cx, |model, cx| {
                    model.refresh_words();
                    cx.notify();
                });
            }),
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

        let todos_seen = TodosSeen::of(todos::global(cx).read(cx));
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
            select_scroll: ScrollHandle::new(),
            tracks: HashMap::new(),
            #[cfg(feature = "probes")]
            layout_marks: HashMap::new(),
            dragging: None,
            dialog: None,
            confirm_bests: None,
            test_shown: None,
            todos_done: None,
            todos_seen,
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
        self.todos_done = None;
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
        self.dialog.is_some()
    }

    #[cfg(feature = "probes")]
    pub fn section(&self) -> Section {
        self.section
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
                if self.dialog.is_some() {
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
            "escape" if self.dialog.is_some() => {
                cx.stop_propagation();
                self.close_dialog(window, cx);
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
            (Some(lit), "down") => self.light(select, (lit + 1).min(last)),
            (Some(lit), "up") => self.light(select, lit.saturating_sub(1)),
            (Some(_), "home") => self.light(select, 0),
            (Some(_), "end") => self.light(select, last),
            (Some(lit), "enter" | "space") => self.pick(select, lit, window, cx),
            (Some(_), "escape") => self.select_open = None,
            _ => return,
        }
        cx.stop_propagation();
        window.prevent_default();
        cx.notify();
    }

    /// Lights an option of the open list, scrolling a long list so it shows.
    fn light(&mut self, select: Select, index: usize) {
        self.select_open = Some((select, index));
        let shown = select.shown();
        if select.options().len() <= shown {
            return;
        }
        let top = (-self.select_scroll.offset().y / px(OPTION_HEIGHT)).round().max(0.) as usize;
        let top = if index < top {
            index
        } else if index >= top + shown {
            index + 1 - shown
        } else {
            return;
        };
        self.scroll_select_to(top);
    }

    fn scroll_select_to(&self, top: usize) {
        self.select_scroll.set_offset(point(px(0.), -px(top as f32 * OPTION_HEIGHT)));
    }

    /// Opens the list on the stored value; a long list opens scrolled so it
    /// shows a few options above it.
    fn open_select(&mut self, select: Select, cx: &App) {
        let current = select.current(self.store.read(cx).get()).unwrap_or(0);
        self.select_open = Some((select, current));
        let (count, shown) = (select.options().len(), select.shown());
        if count > shown {
            self.scroll_select_to(current.saturating_sub(3).min(count - shown));
        }
    }

    fn pick(&mut self, select: Select, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        select.apply(index, &self.store, cx);
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

    fn open_dialog(&mut self, dialog: Dialog, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = Some(dialog);
        self.focus_control("dialog-cancel", window, cx);
        cx.notify();
    }

    fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dialog) = self.dialog.take() {
            self.focus_control(dialog.opener(), window, cx);
        }
        cx.notify();
    }

    fn confirm_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.dialog {
            Some(Dialog::ResetSettings) => {
                self.store.update(cx, |settings, cx| settings.reset(cx));
                self.runtime.update(cx, |runtime, cx| runtime.load_storage(cx));
            }
            Some(Dialog::ResetTodos) => {
                todos::global(cx).update(cx, |todos, cx| todos.reset_completely(cx));
                self.say_todos_done(TodosDone::Reset, cx);
            }
            None => return,
        }
        self.close_dialog(window, cx);
    }

    fn switch_row(
        &mut self,
        key: &'static str,
        label: String,
        text: Option<String>,
        on: bool,
        apply: fn(&mut AppSettings, bool),
        window: &Window,
        cx: &mut App,
    ) -> Div {
        let focus = self.focus_of(key, true, window, cx);
        let control = switch(key, label.clone(), on, &focus, Some(self.set(apply)));
        row(label, text.map(description), control)
    }

    #[allow(clippy::too_many_arguments)]
    fn segmented_row(
        &mut self,
        key: &'static str,
        label: impl IntoElement,
        name: String,
        text: Option<AnyElement>,
        options: [SharedString; 2],
        selected: usize,
        apply: fn(&mut AppSettings, usize),
        window: &Window,
        cx: &mut App,
    ) -> Div {
        let focus = self.focus_of(key, true, window, cx);
        row(label, text, segmented(key, name, &options, selected, &focus, self.set(apply)))
    }

    fn slider_row(
        &mut self,
        kind: SliderKind,
        label: String,
        text: String,
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
        let control = slider(kind.key(), label.clone(), spec, kind.text(spec.value), width, &focus, &track, on_change, on_press);
        row(label, Some(description(text)), control)
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
                fl!("settings-overlay-opacity"),
                fl!("settings-overlay-opacity-text"),
                OVERLAY_SLIDER_WIDTH,
                settings,
                window,
                cx,
            ),
            self.slider_row(
                SliderKind::Scale,
                fl!("settings-overlay-scale"),
                fl!("settings-overlay-scale-text"),
                OVERLAY_SLIDER_WIDTH,
                settings,
                window,
                cx,
            ),
            self.segmented_row(
                "layout",
                fl!("settings-overlay-layout"),
                fl!("settings-overlay-layout"),
                Some(description(fl!("settings-overlay-layout-text"))),
                [fl!("settings-overlay-layout-full").into(), fl!("settings-overlay-layout-capsule").into()],
                usize::from(overlay.layout == Layout::Capsule),
                |all, index| all.overlay.layout = if index == 0 { Layout::Full } else { Layout::Capsule },
                window,
                cx,
            ),
            self.segmented_row(
                "percentage",
                fl!("settings-overlay-percentage"),
                fl!("settings-overlay-percentage"),
                Some(description(fl!("settings-overlay-percentage-text"))),
                [fl!("settings-overlay-contribution").into(), fl!("settings-overlay-share").into()],
                usize::from(overlay.pct_mode == PctMode::Share),
                |all, index| all.overlay.pct_mode = if index == 0 { PctMode::Contribution } else { PctMode::Share },
                window,
                cx,
            ),
        ]);
        let shown = group(vec![
            self.switch_row(
                "boss-hp",
                fl!("settings-overlay-boss-hp"),
                None,
                overlay.show_boss_hp,
                |all, on| all.overlay.show_boss_hp = on,
                window,
                cx,
            ),
            self.switch_row(
                "player-name",
                fl!("settings-overlay-player-name"),
                Some(fl!("settings-overlay-player-name-text")),
                overlay.show_player_name,
                |all, on| all.overlay.show_player_name = on,
                window,
                cx,
            ),
            self.switch_row(
                "mask-nicknames",
                fl!("settings-overlay-mask"),
                Some(fl!("settings-overlay-mask-text")),
                overlay.mask_nicknames,
                |all, on| all.overlay.mask_nicknames = on,
                window,
                cx,
            ),
            self.switch_row("server", fl!("common-server"), None, overlay.show_server, |all, on| all.overlay.show_server = on, window, cx),
            self.switch_row(
                "combat-power",
                fl!("settings-overlay-combat-power"),
                Some(fl!("settings-overlay-combat-power-text")),
                overlay.show_combat_power,
                |all, on| all.overlay.show_combat_power = on,
                window,
                cx,
            ),
            self.switch_row(
                "total-damage",
                fl!("settings-overlay-total-damage"),
                None,
                overlay.show_damage,
                |all, on| all.overlay.show_damage = on,
                window,
                cx,
            ),
            self.switch_row("dps", fl!("common-dps"), None, overlay.show_dps, |all, on| all.overlay.show_dps = on, window, cx),
        ]);
        let fights = group(vec![
            self.switch_row(
                "fight-summary",
                fl!("settings-overlay-fight-summary"),
                Some(fl!("settings-overlay-fight-summary-text")),
                overlay.show_fight_summary,
                |all, on| all.overlay.show_fight_summary = on,
                window,
                cx,
            ),
            self.switch_row(
                "pace",
                fl!("settings-overlay-pace"),
                Some(fl!("settings-overlay-pace-text")),
                overlay.show_personal_best,
                |all, on| all.overlay.show_personal_best = on,
                window,
                cx,
            ),
        ]);
        let lock = self.lock_row(window, cx);
        let detail = self.segmented_row(
            "detail-opens",
            fl!("settings-overlay-detail-opens"),
            fl!("settings-overlay-detail-opens"),
            Some(description(fl!("settings-overlay-detail-opens-text"))),
            [fl!("settings-overlay-beside").into(), fl!("settings-overlay-centred").into()],
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
            .child(group_title(fl!("settings-overlay-look"), true))
            .child(look)
            .child(group_title(fl!("settings-overlay-shown"), false))
            .child(shown)
            .child(group_title(fl!("settings-overlay-boss-fights"), false))
            .child(fights)
            .child(group_title(fl!("settings-overlay-window"), false))
            .child(group(vec![lock, detail]));
        #[cfg(feature = "probes")]
        let rows = rows.relative().child(self.layout_marks.entry("overlay-rows").or_default().marker());
        let preview = self.preview_column(cx);
        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-overlay"), Some(fl!("settings-overlay-lead").into())))
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
            note(Icon::Info, fl!("settings-overlay-lock-unavailable"), rgba(FAINT))
        });
        let text = div()
            .flex()
            .flex_col()
            .child(description(fl!("settings-overlay-lock-text")))
            .children(reason)
            .into_any_element();
        let label = fl!("settings-overlay-lock");
        let control = switch("lock", label.clone(), self.locked, &focus, on_toggle);
        row(label, Some(text), control)
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
            .child(group_title(fl!("settings-preview"), true))
            .child(stage)
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(11.5))
                    .text_color(rgba(FAINT))
                    .child(fl!("settings-preview-note")),
            )
    }

    fn behaviour_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let behaviour = &settings.behaviour;
        let count_label = div().flex().items_center().gap(px(6.)).child(fl!("settings-behaviour-count")).child(
            div()
                .id("count-info")
                .child(icon(Icon::Info, 14., rgba(FAINT)))
                .tooltip(widgets::tooltip(fl!("settings-behaviour-count-tip"))),
        );
        let count = self.segmented_row(
            "count",
            count_label,
            fl!("settings-behaviour-count"),
            Some(description(fl!("settings-behaviour-count-text"))),
            [fl!("settings-behaviour-all-targets").into(), fl!("settings-behaviour-boss-only").into()],
            usize::from(behaviour.count == CountMode::BossOnly),
            |all, index| all.behaviour.count = if index == 0 { CountMode::AllTargets } else { CountMode::BossOnly },
            window,
            cx,
        );
        let counted = group(vec![
            count,
            self.switch_row(
                "possible-bosses",
                fl!("settings-behaviour-possible-bosses"),
                Some(fl!("settings-behaviour-possible-bosses-text")),
                behaviour.detect_possible_bosses,
                |all, on| all.behaviour.detect_possible_bosses = on,
                window,
                cx,
            ),
            self.switch_row(
                "training-dummy",
                fl!("settings-behaviour-dummy"),
                Some(fl!("settings-behaviour-dummy-text")),
                behaviour.training_dummy_only,
                |all, on| all.behaviour.training_dummy_only = on,
                window,
                cx,
            ),
            self.switch_row(
                "hide-unknown",
                fl!("settings-behaviour-hide-unknown"),
                Some(fl!("settings-behaviour-hide-unknown-text")),
                behaviour.hide_unknown_players,
                |all, on| all.behaviour.hide_unknown_players = on,
                window,
                cx,
            ),
            self.slider_row(
                SliderKind::Players,
                fl!("settings-behaviour-players-shown"),
                fl!("settings-behaviour-players-shown-text"),
                SLIDER_WIDTH,
                settings,
                window,
                cx,
            ),
        ]);

        let trigger = self.select_control(Select::EndFight, settings, true, window, cx);
        let between = group(vec![row(
            fl!("settings-behaviour-end-fight"),
            Some(description(fl!("settings-behaviour-end-fight-text"))),
            trigger,
        )]);

        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-behaviour"), Some(fl!("settings-behaviour-lead").into())))
            .child(group_title(fl!("settings-behaviour-counted"), true))
            .child(counted)
            .child(group_title(fl!("settings-behaviour-between"), false))
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

        let meter_label = fl!("settings-capture-meter");
        let meter_tile = match &status.meter {
            MeterTile::Running { via } => tile(
                Icon::Activity,
                meter_label,
                fl!("settings-status-running"),
                via.map(|via| fl!("settings-capture-via", backend = via).into()),
                Tone::Good,
            ),
            MeterTile::Stopped => tile(Icon::Activity, meter_label, fl!("settings-status-stopped"), None, Tone::Neutral),
            MeterTile::Unavailable(reason) => {
                tile(Icon::Activity, meter_label, fl!("settings-status-not-running"), Some(reason.clone().into()), Tone::Bad)
            }
        };
        let game_tile = if status.game_found {
            tile(Icon::Radio, fl!("settings-capture-game"), fl!("settings-capture-found"), status.game_address.clone().map(Into::into), Tone::Good)
        } else {
            tile(Icon::Radio, fl!("settings-capture-game"), fl!("settings-capture-waiting-game"), None, Tone::Warn)
        };
        let character_tile = match &status.character {
            Some(name) => tile(Icon::User, fl!("common-character"), settings.overlay.display_name(name), None, Tone::Good),
            None => tile(
                Icon::User,
                fl!("common-character"),
                fl!("settings-capture-not-identified"),
                Some(fl!("meter-unidentified-hint").into()),
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
        let server_tile = tile(Icon::Server, fl!("common-server"), server_text, None, server_tone);
        let tiles = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(div().flex().gap(px(8.)).child(meter_tile).child(game_tile))
            .child(div().flex().gap(px(8.)).child(character_tile).child(server_tile));

        let backend = self.segmented_row(
            "backend",
            fl!("settings-capture-backend"),
            fl!("settings-capture-backend"),
            Some(
                div()
                    .flex()
                    .flex_col()
                    .child(description(fl!("settings-capture-backend-text")))
                    .child(note(Icon::Info, fl!("settings-capture-next-start"), rgba(FAINT)))
                    .into_any_element(),
            ),
            [fl!("settings-capture-npcap-first").into(), fl!("settings-capture-windivert-first").into()],
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
                        .child(fl!(
                            "settings-capture-recording-progress",
                            count = progress.packets,
                            packets = format::grouped(progress.packets as f64),
                            size = format::file_size(progress.bytes)
                        )),
                )
                .child(button(
                    "recording",
                    fl!("settings-capture-stop-recording"),
                    Some(Icon::Stop),
                    ButtonKind::Danger,
                    false,
                    &focus,
                    Some(Rc::new(move |(), _, cx| runtime_entity.update(cx, |runtime, cx| runtime.stop_recording(cx)))),
                )),
            None => div().child(button(
                "recording",
                fl!("settings-capture-start-recording"),
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
            .child(description(fl!("settings-capture-packet-recording-text")))
            .children(recording_error.map(|error| note(Icon::TriangleAlert, error, rgb(widgets::BAD))))
            .into_any_element();
        let recording_group = group(vec![
            self.switch_row(
                "record-unfamiliar",
                fl!("settings-capture-record-unfamiliar"),
                Some(fl!("settings-capture-record-unfamiliar-text")),
                settings.capture.record_unfamiliar_servers,
                |all, on| all.capture.record_unfamiliar_servers = on,
                window,
                cx,
            ),
            row(fl!("settings-capture-packet-recording"), Some(recording_text), recording_control),
        ]);

        let mut recording_rows: Vec<Div> = Vec::new();
        for (index, item) in recordings.iter().enumerate() {
            let replaying = matches!(&replay, ReplayProgress::Running { name } if *name == item.name);
            let mut line = format!("{} \u{b7} {}", item.when, format::file_size(item.bytes));
            if item.unfamiliar_server {
                line.push_str(&format!(" \u{b7} {}", fl!("settings-capture-unfamiliar-server")));
            }
            let runtime_entity = self.runtime.clone();
            let actions = if replaying {
                let focus = self.focus_of("cancel-replay", true, window, cx);
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(badge(
                        Some(spinner(("replaying", index), 12., rgb(widgets::WARN), animate)),
                        fl!("settings-capture-replaying"),
                        Tone::Warn,
                    ))
                    .child(button(
                        "cancel-replay",
                        fl!("settings-capture-cancel-replay"),
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
                    fl!("settings-capture-replay"),
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
            recording_rows.push(list_row(
                Some(Icon::FileText),
                fl!("settings-capture-no-recordings"),
                fl!("settings-capture-no-recordings-line"),
            ));
        }
        let replay_note = match &replay {
            ReplayProgress::Failed(error) => {
                Some(note(Icon::TriangleAlert, fl!("settings-capture-replay-failed", error = error.as_str()), rgb(widgets::BAD)))
            }
            ReplayProgress::Finished { packets } => {
                let count = *packets;
                Some(note(
                    Icon::CircleCheck,
                    fl!("settings-capture-replayed", count = count, packets = format::grouped(count as f64)),
                    rgb(widgets::GOOD),
                ))
            }
            ReplayProgress::Idle | ReplayProgress::Running { .. } => None,
        };

        let mobs = group(vec![self.switch_row(
            "collect-mobs",
            fl!("settings-capture-collect-mobs"),
            Some(fl!("settings-capture-collect-mobs-text")),
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
            .child(description(fl!("settings-capture-meter-log-text")))
            .children(self.error_under("log-folder"))
            .into_any_element();
        let log = group(vec![
            self.switch_row(
                "decision-log",
                fl!("settings-capture-decision-log"),
                Some(fl!("settings-capture-decision-log-text")),
                settings.capture.decision_log,
                |all, on| all.capture.decision_log = on,
                window,
                cx,
            ),
            row(
                fl!("settings-capture-meter-log"),
                Some(log_text),
                button(
                    "log-folder",
                    fl!("settings-capture-open-log-folder"),
                    Some(Icon::FolderOpen),
                    ButtonKind::Secondary,
                    false,
                    &folder_focus,
                    Some(open_logs),
                ),
            ),
        ]);

        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-capture"), Some(fl!("settings-capture-lead").into())))
            .child(group_title(fl!("settings-capture-connection"), true))
            .child(tiles)
            .child(group_title(fl!("settings-section-capture"), false))
            .child(capture)
            .child(group_title(fl!("settings-capture-recording"), false))
            .child(recording_group)
            .child(
                group_title(fl!("settings-capture-recordings"), false).child(
                    div()
                        .font_weight(FontWeight::NORMAL)
                        .text_color(rgba(FAINT))
                        .child(fl!("settings-capture-recordings-note")),
                ),
            )
            .child(group(recording_rows))
            .children(replay_note)
            .child(group_title(fl!("settings-capture-mob-data"), false))
            .child(mobs)
            .child(group_title(fl!("settings-capture-log"), false))
            .child(log)
    }

    fn capture_check_row(&mut self, check: &CaptureCheck, animate: bool, window: &Window, cx: &mut Context<Self>) -> Div {
        let focus = self.focus_of("capture-check", true, window, cx);
        let running = matches!(check, CaptureCheck::Running);
        let runtime = self.runtime.clone();
        let rerun: Option<Handler<()>> = (!running).then(|| -> Handler<()> {
            Rc::new(move |(), _, cx| runtime.update(cx, |runtime, cx| runtime.run_capture_check(cx)))
        });
        let mut text = div().flex().flex_col().child(description(fl!("settings-capture-check-text")));
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
                        .child(fl!("settings-capture-checking")),
                );
            }
            CaptureCheck::Done(report) => {
                let summary_tone = if report.ready { rgb(widgets::GOOD) } else { rgb(widgets::BAD) };
                let summary_icon = if report.ready { Icon::CircleCheck } else { Icon::TriangleAlert };
                text = text.child(note(summary_icon, capture_check::summary(report), summary_tone)).children(report.checks.iter().map(
                    |check| {
                        let (glyph, colour) = match (check.ok, check.weight) {
                            (true, _) => (Icon::CircleCheck, rgb(widgets::GOOD)),
                            (false, Weight::Required) => (Icon::TriangleAlert, rgb(widgets::BAD)),
                            (false, Weight::Optional) => (Icon::TriangleAlert, rgb(widgets::WARN)),
                        };
                        let line = capture_check::line(check, report);
                        let raw_detail = line.raw_detail.map(|detail| div().pl(px(19.)).child(description(detail)));
                        div().flex().flex_col().child(note(glyph, line.text, colour)).children(raw_detail)
                    },
                ));
            }
        }
        row(
            fl!("settings-capture-check"),
            Some(text.into_any_element()),
            button("capture-check", fl!("settings-capture-rerun"), Some(Icon::ShieldCheck), ButtonKind::Secondary, false, &focus, rerun),
        )
    }

    fn bests_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let bests = self.runtime.read(cx).bests.clone();
        let head = page_head(fl!("settings-section-bests"), Some(fl!("settings-bests-lead").into()));
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
                        .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(fl!("settings-bests-none")))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(rgba(MUTED))
                                .child(fl!("settings-bests-none-line")),
                        ),
                );
            return div().flex().flex_col().child(head).child(div().mt(px(18.)).child(group(vec![empty])));
        }

        let bosses = distinct(bests.iter().map(|best| best.boss.as_str()));
        let characters = distinct(bests.iter().map(|best| best.character.as_str()));
        let count_text = fl!("settings-bests-count", bosses = bosses, characters = characters);
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
            if confirming { fl!("settings-bests-confirm") } else { fl!("settings-bests-reset-all") },
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
            actions = actions.child(button("bests-cancel", fl!("common-cancel"), None, ButtonKind::Ghost, false, &cancel_focus, Some(cancel)));
        }

        let header = |text: String, width: Option<f32>, right: bool| {
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
            .child(header(fl!("settings-bests-boss"), None, false))
            .child(header(fl!("common-character"), Some(150.), false))
            .child(header(fl!("settings-bests-best-dps"), Some(100.), true))
            .child(header(fl!("settings-bests-date"), Some(120.), true));
        let rows = bests.iter().enumerate().map(|(index, best)| {
            let character = settings.overlay.display_name(&best.character);
            let dps = format::compact(best.dps);
            div()
                .id(("best", index))
                .role(Role::Row)
                .aria_label(SharedString::from(fl!(
                    "settings-bests-row",
                    boss = best.boss.as_str(),
                    character = character.as_str(),
                    dps = dps.as_str(),
                    when = best.when.as_str()
                )))
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
            .aria_label(fl!("settings-section-bests"))
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
            fl!("settings-reminders-master"),
            Some(fl!("settings-reminders-master-text")),
            on,
            |all, on| all.reminders.enabled = on,
            window,
            cx,
        );
        let lead = self.select_control(Select::LeadTime, settings, on, window, cx);
        let sound_focus = self.focus_of("reminder-sound", true, window, cx);
        let sound_toggle = on.then(|| self.set(|all: &mut AppSettings, on| all.reminders.sound = on));
        let sound = switch("reminder-sound", fl!("settings-reminders-sound"), reminders.sound, &sound_focus, sound_toggle);
        let test = self.test_reminder_control(on, window, cx);
        let how = group(vec![
            master,
            reminder_row(on, fl!("settings-reminders-lead-time"), fl!("settings-reminders-lead-time-text"), lead),
            reminder_row(on, fl!("settings-reminders-sound"), fl!("settings-reminders-sound-text"), sound),
            reminder_row(on, fl!("reminder-test-title"), fl!("settings-reminders-test-text"), test),
        ]);

        let mut page = div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-reminders"), Some(fl!("settings-reminders-lead").into())))
            .child(group_title(fl!("settings-reminders-how"), true))
            .child(how);
        let titles = [
            fl!("settings-reminders-every-hour"),
            fl!("settings-reminders-every-3-hours"),
            fl!("settings-reminders-weekly"),
            fl!("settings-reminders-daily"),
        ];
        for (group_index, title) in titles.into_iter().enumerate() {
            let mut rows = Vec::new();
            for info in EVENTS.iter().filter(|info| how_often(info.rule) == group_index) {
                let id = info.id;
                let (key, label, text) = event_row(id);
                let focus = self.focus_of(key, true, window, cx);
                let toggle = on.then(|| self.set(move |all: &mut AppSettings, on| all.reminders.set_event_enabled(id, on)));
                let control = switch(key, label.clone(), reminders.event_enabled(id), &focus, toggle);
                rows.push(reminder_row(on, label, text, control));
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
            let text = fl!("settings-reminders-test-shown");
            let note = badge(Some(icon(Icon::Check, 12., rgb(widgets::GOOD)).into_any_element()), text.clone(), Tone::Good)
                .id("test-shown")
                .debug_selector(|| "test-shown".to_string())
                .role(Role::Status)
                .aria_label(text);
            let note = if cx.reduce_motion() {
                note.into_any_element()
            } else {
                note.with_animation("test-shown-fade", Animation::new(CHIP_FADE), |note, t| note.opacity(t)).into_any_element()
            };
            div().absolute().top_0().bottom_0().right(relative(1.)).pr(px(10.)).flex().items_center().child(note)
        });
        div().relative().flex().items_center().children(shown).child(button(
            "test-reminder",
            fl!("settings-reminders-show-test"),
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

    fn general_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let language = self.select_control(Select::Language, settings, true, window, cx);
        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-general"), None))
            .child(group_title(fl!("settings-general-app"), true))
            .child(group(vec![row(fl!("settings-language"), Some(description(fl!("settings-language-text"))), language)]))
    }

    fn about_page(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let storage = match &self.runtime.read(cx).storage {
            StorageState::NotLoaded | StorageState::Loading => fl!("settings-about-counting"),
            StorageState::Loaded(summary) => fl!(
                "settings-about-storage-line",
                settings = format::file_size(summary.settings_bytes),
                records = summary.history_records,
                history = format::file_size(summary.history_bytes),
                files = summary.recordings,
                recordings = format::file_size(summary.recordings_bytes)
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
                        .child(description(fl!("settings-about-description")))
                        .children(self.error_under("readme"))
                        .into_any_element(),
                ),
                button(
                    "readme",
                    fl!("settings-about-source"),
                    Some(Icon::External),
                    ButtonKind::Secondary,
                    false,
                    &readme_focus,
                    Some(open_readme),
                ),
            ),
            row(
                fl!("settings-about-licence"),
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .child(description(fl!("settings-about-licence-text")))
                        .children(self.error_under("licence"))
                        .into_any_element(),
                ),
                button(
                    "licence",
                    fl!("settings-about-read-licence"),
                    Some(Icon::Scale),
                    ButtonKind::Secondary,
                    false,
                    &licence_focus,
                    Some(open_licence),
                ),
            ),
        ]);

        let credit = |title: String, tag: Option<String>, text: String| {
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
                "Helveticxa/Aether-Aion2-DPS-meter-Global".to_string(),
                Some(fl!("settings-about-forked-from")),
                fl!("settings-about-aether"),
            ),
            credit("ZDYoung0519/NOIA2".to_string(), Some(fl!("settings-about-upstream")), fl!("settings-about-noia2")),
            credit("Kuroukihime/AIon2-Dps-Meter".to_string(), None, fl!("settings-about-kuroukihime")),
            credit(
                fl!("settings-about-references"),
                None,
                "TK-open-public/Aion2-Dps-Meter \u{b7} taengu/Aion2-Dps-Meter \u{b7} p62003/aletheia_AION2_DPS_Meter".to_string(),
            ),
            credit(fl!("settings-about-built-with"), None, fl!("settings-about-built-with-text")),
            credit("Npcap".to_string(), None, fl!("settings-about-npcap")),
        ]);

        let folder_focus = self.focus_of("data-folder", true, window, cx);
        let open_data = self.button_handler(cx, |this, _, cx| this.hand_over("data-folder", files::open_folder(Folder::Data), cx));
        let reset_focus = self.focus_of("reset-settings", true, window, cx);
        let reset = self.button_handler(cx, |this, window, cx| this.open_dialog(Dialog::ResetSettings, window, cx));
        let storage_group = group(vec![
            row(
                fl!("settings-about-stored"),
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .child(description(storage))
                        .children(self.error_under("data-folder"))
                        .into_any_element(),
                ),
                button(
                    "data-folder",
                    fl!("settings-about-open-folder"),
                    Some(Icon::FolderOpen),
                    ButtonKind::Secondary,
                    false,
                    &folder_focus,
                    Some(open_data),
                ),
            ),
            row(
                fl!("settings-about-reset"),
                Some(description(fl!("settings-about-reset-text"))),
                button("reset-settings", fl!("settings-about-reset-button"), None, ButtonKind::Danger, false, &reset_focus, Some(reset)),
            ),
        ]);

        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-about"), None))
            .child(group_title("AION 2 DPS Meter", true))
            .child(meter)
            .child(group_title(fl!("settings-about-credits"), false))
            .child(credits)
            .child(group_title(fl!("settings-about-storage"), false))
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
        let (dot, status): (gpui::Hsla, String) = match &self.runtime.read(cx).capture.meter {
            MeterTile::Running { .. } => (rgb(widgets::GOOD).into(), fl!("settings-status-running")),
            MeterTile::Stopped => (rgba(FAINT).into(), fl!("settings-status-stopped")),
            MeterTile::Unavailable(_) => (rgb(widgets::BAD).into(), fl!("settings-status-not-running")),
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
                    .aria_label(fl!("settings-sections"))
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
                            .child(fl!("settings-sidebar-title")),
                    )
                    .children(items),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("meter-status")
                    .role(Role::Status)
                    .aria_label(fl!("settings-meter-status", status = status.as_str()))
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
            notices.push(alert(fl!("settings-set-aside"), notice.to_string(), Tone::Warn));
        }
        if let Some(error) = store.save_error() {
            notices.push(alert(fl!("settings-not-saved"), error.to_string(), Tone::Bad));
        }
        notices
    }

    fn dialog_layer(&mut self, dialog: Dialog, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let cancel_focus = self.focus_of("dialog-cancel", false, window, cx);
        let reset_focus = self.focus_of("dialog-reset", false, window, cx);
        let cancel = self.button_handler(cx, |this, window, cx| this.close_dialog(window, cx));
        let reset = self.button_handler(cx, |this, window, cx| this.confirm_dialog(window, cx));
        let (title, paragraphs, confirm): (SharedString, Vec<SharedString>, String) = match dialog {
            Dialog::ResetSettings => {
                (fl!("settings-reset-title").into(), vec![fl!("settings-reset-body").into()], fl!("settings-about-reset"))
            }
            Dialog::ResetTodos => (
                fl!("settings-todos-reset-title").into(),
                vec![todos::copy::reset_dialog_body(self.todos_seen.own_total).into(), todos::copy::reset_dialog_keeps().into()],
                fl!("settings-todos-reset-confirm"),
            ),
        };
        let panel = div()
            .id("reset-dialog")
            .debug_selector(|| "reset-dialog".to_string())
            .role(Role::AlertDialog)
            .aria_label(title.clone())
            .aria_description(SharedString::from(paragraphs.iter().map(SharedString::as_ref).collect::<Vec<&str>>().join(" ")))
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
            .child(div().font_family(DISPLAY_FONT).text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(title))
            .children(paragraphs.into_iter().enumerate().map(|(index, paragraph)| {
                div().mt(px(if index == 0 { 8. } else { 6. })).text_color(rgba(MUTED)).child(paragraph)
            }))
            .child(
                div()
                    .mt(px(18.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(button("dialog-cancel", fl!("common-cancel"), None, ButtonKind::Secondary, false, &cancel_focus, Some(cancel)))
                    .child(button("dialog-reset", confirm, None, ButtonKind::Danger, false, &reset_focus, Some(reset))),
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
            .on_any_mouse_down(cx.listener(|this, _, window, cx| this.close_dialog(window, cx)))
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
        let height = px(options.len().min(select.shown()) as f32 * OPTION_HEIGHT + 10.);
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
        let menu = select_menu(
            below,
            select.label(),
            &options,
            current,
            lit,
            pick,
            hover,
            !cx.reduce_motion(),
            Some((&self.select_scroll, select.shown())),
        );
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
            Section::Todos => self.todos_page(&settings, window, cx),
            Section::General => self.general_page(&settings, window, cx),
            Section::About => self.about_page(window, cx),
        };
        let sidebar = self.sidebar(window, cx);
        let dialog = self.dialog.map(|dialog| self.dialog_layer(dialog, window, cx));
        let select = self.select_layer(window, cx);
        let notices = self.notices(cx);
        let section = self.section;

        div()
            .id("settings")
            .role(Role::Window)
            .aria_label(fl!("settings-window-title"))
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
    fl!("settings-reminders-minutes-before", minutes = minutes).into()
}

fn language_label(choice: LanguageChoice) -> SharedString {
    let language = choice.resolve().autonym();
    match choice {
        LanguageChoice::System => fl!("settings-language-system", language = language).into(),
        LanguageChoice::English | LanguageChoice::French => language.into(),
    }
}

fn hour_label(hour: u8) -> SharedString {
    format!("{hour:02}:00").into()
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
fn event_row(id: EventId) -> (&'static str, String, String) {
    match id {
        EventId::ShugoFestival => ("event-shugo", id.name(), fl!("settings-event-shugo-text")),
        EventId::DimensionalInvasion => ("event-invasion", id.name(), fl!("settings-event-invasion-text")),
        EventId::RiftPortals => ("event-rifts", id.name(), fl!("settings-event-rifts-text")),
        EventId::WatcherKaira => ("event-kaira", id.name(), fl!("settings-event-kaira-text")),
        EventId::ArtifactSiege => ("event-siege", id.name(), fl!("settings-event-siege-text")),
        EventId::AbyssFieldBosses => ("event-abyss", id.name(), fl!("settings-event-abyss-text")),
        EventId::GuardianLordNahma => ("event-nahma", id.name(), fl!("settings-event-nahma-text")),
        EventId::ArenaOfTactics => ("event-arena", fl!("settings-event-arena-opens"), fl!("settings-event-arena-text")),
        EventId::RewardKeys => ("event-keys", id.name(), fl!("settings-event-keys-text")),
    }
}

fn reminder_row(enabled: bool, label: String, text: String, control: impl IntoElement) -> Div {
    if enabled {
        row(label, Some(description(text)), control)
    } else {
        dimmed_row(label, Some(description(text)), control)
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
            title: Some(fl!("settings-window-title").into()),
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
    use crate::i18n::Language;
    use crate::settings::{connect, install};

    struct Opened<'a> {
        view: Entity<SettingsView>,
        store: Entity<Settings>,
        meter: Entity<MeterModel>,
        cx: &'a mut VisualTestContext,
    }

    /// The real view in GPUI's test platform, on settings in memory (in
    /// English, whatever Windows' language) or in `data_dir`, connected to an
    /// overlay model as the app connects it.
    fn open<'a>(cx: &'a mut TestAppContext, data_dir: Option<&Path>) -> Opened<'a> {
        let (store, meter, runtime) = cx.update(|cx| {
            let store = cx.new(|_| match data_dir {
                Some(dir) => Settings::load(Some(dir)),
                None => {
                    let mut english = AppSettings::default();
                    english.general.language = LanguageChoice::English;
                    Settings::in_memory(english)
                }
            });
            install(&store, cx);
            reminders::install(&store, false, cx);
            todos::install(&store, false, cx);
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
            "nav-todos",
            "nav-general",
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

    #[gpui::test]
    fn the_todos_page_sets_the_hour_restores_and_resets_through_its_dialog(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        let store = window.store.clone();
        window.cx.update(|_, cx| {
            store.update(cx, |settings, cx| {
                settings.update(cx, |all| {
                    all.known_characters = vec![meter_core::identity::KnownCharacter {
                        name: "Alder".to_string(),
                        server_id: 1001,
                        class: "RANGER".to_string(),
                        key: None,
                    }]
                })
            })
        });
        window.show(Section::Todos);
        window.tab_to("reset-hour");
        window.press("down");
        assert!(window.select_open());
        window.press("up");
        window.press("enter");
        assert_eq!(window.settings().todos.reset_hour, 8, "the hour before 09:00");
        let todos = window.cx.update(|_, cx| todos::global(cx));
        assert_eq!(window.cx.update(|_, cx| todos.read(cx).reset_hour()), 8, "the tracker follows at once");

        window.tab_to("membership");
        window.press("space");
        assert!(window.settings().todos.membership);

        window.press("tab");
        assert_eq!(window.focused().as_deref(), Some("todos-restore"), "Duty Commands start hidden, so Restore can act");
        window.cx.update(|_, cx| {
            todos.update(cx, |todos, cx| {
                let id = todos.selected().expect("a character");
                todos.hide(id, "trial", cx);
                todos.add_own(id, todos::NewOwnTodo { name: "Mine".into(), kind: todos::OwnKind::Tick, cadence: todos::Cadence::Daily }, cx).expect("added");
            })
        });
        window.tab_to("todos-restore");
        window.press("enter");
        let view = window.view.clone();
        assert_eq!(window.cx.update(|_, cx| view.read(cx).todos_done_text()), Some(todos::copy::to_dos_back(2)));
        assert_eq!(window.cx.update(|_, cx| todos.read(cx).hidden_summary().hidden), 0);

        window.tab_to("todos-reset");
        window.press("enter");
        assert!(window.dialog_open());
        assert_eq!(window.focused().as_deref(), Some("dialog-cancel"));
        window.press("escape");
        assert!(!window.dialog_open());
        assert_eq!(window.focused().as_deref(), Some("todos-reset"));
        assert_eq!(window.cx.update(|_, cx| todos.read(cx).own_total()), 1, "Escape resets nothing");
        window.press("enter");
        window.press("tab");
        window.press("enter");
        assert!(!window.dialog_open());
        assert_eq!(window.cx.update(|_, cx| todos.read(cx).own_total()), 0);
        assert_eq!(window.cx.update(|_, cx| view.read(cx).todos_done_text()), Some(fl!("settings-todos-reset-done")));
        assert!(window.settings().todos.membership, "the membership stays");
    }

    #[gpui::test]
    fn choosing_french_by_keyboard_switches_the_settings_and_the_meter_and_back(cx: &mut TestAppContext) {
        let mut window = open(cx, None);
        let mut fight = sample::boss_fight(60.0, 500);
        if let Some(target) = fight.last_target_info.as_mut() {
            target.target_name = None;
        }
        let meter = window.meter.clone();
        window.cx.update(|_, cx| {
            meter.update(cx, |model, cx| {
                model.apply(MeterEvent::DpsSnapshot(sample::indexed(fight)), Instant::now());
                cx.notify();
            })
        });
        let header = |window: &mut Opened| {
            let meter = window.meter.clone();
            window.cx.update(|_, cx| meter.read(cx).header_name().map(|name| name.to_string()))
        };
        assert_eq!(header(&mut window).as_deref(), Some("Target 90001"));

        window.show(Section::General);
        window.tab_to("language");
        window.press("down");
        assert!(window.select_open(), "the list opens on English");
        window.press("down");
        window.press("enter");
        assert_eq!(window.settings().general.language, LanguageChoice::French);
        assert_eq!(i18n::language(), Language::French);
        assert_eq!(header(&mut window).as_deref(), Some("Cible 90001"), "the meter's prepared words follow");
        assert_eq!(Section::General.label(), "Général");
        assert_eq!(window.focused().as_deref(), Some("language"));

        window.press("down");
        window.press("up");
        window.press("enter");
        assert_eq!(window.settings().general.language, LanguageChoice::English);
        assert_eq!(i18n::language(), Language::English);
        assert_eq!(header(&mut window).as_deref(), Some("Target 90001"));
    }

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let dir = std::env::temp_dir().join(format!("aether-settings-view-{name}-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp folder");
        dir
    }
}
