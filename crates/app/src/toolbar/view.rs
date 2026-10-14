//! The toolbar (docs/gpui-mockups/approved/toolbar.html, values in tokens.md,
//! "Hub" and "Toolbar update icon"): grip; DPS meter, PvP meter, To-dos,
//! Pets, Fight history, Reminders, Settings; Odyle Energy, Shugo keys and the warning
//! while other resources are full; the update icon while an update is
//! available, downloading or ready; fold and quit. A lit tool is open.
//! Controls light on the pointer's enter and go on its leave, never on moves
//! inside; tooltips follow after 500 ms. The warning's and the bell's
//! dropdowns and every tooltip are drawn in the toolbar's own window, which
//! grows for them, upward when the toolbar sits low on its screen. A press
//! never takes the keyboard; with the keyboard given to the toolbar
//! (accessibility tools), Left and Right move along it and Enter or Space
//! press.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    div, prelude::*, px, relative, rgb, rgba, size, AnyElement, App, Bounds, Context, CursorStyle, Div, Entity,
    EventEmitter, FocusHandle, FontWeight, Hsla, KeyDownEvent, MouseButton, Pixels, Render, Role, SharedString, Stateful,
    Subscription, Task, Toggled, Window,
};

use super::counts::{Counts, Pinned};
use super::update::{UpdateClick, UpdateIcon};
use super::window;
use super::Hub;
use crate::assets::Icon;
use crate::clock::{clock_time, now_ms, LocalZone};
use crate::i18n::{self, fl};
use crate::overlay::WindowDrag;
use crate::reminders::{self, ReminderSettings};
use crate::secondary::chrome::{self, Look, Scale, FAINT, GAUGE_EMPTY, GOOD, HOT, MUTED, SOFT, TEXT, WARN};
use crate::secondary::{self, WindowKind};
use crate::settings;
use crate::todos;
use crate::update;

const TIP_DELAY: Duration = Duration::from_millis(500);
/// A dropdown closes this long after the pointer left the toolbar.
const DROPDOWN_CLOSE_AFTER: Duration = Duration::from_millis(400);
/// Between the bar and a dropdown or tooltip.
const GAP: f32 = 6.0;
const FULL_DROPDOWN_WIDTH: f32 = 268.0;
const BELL_DROPDOWN_WIDTH: f32 = 236.0;
const ITEM_HEIGHT: f32 = 26.0;
const LABEL_HEIGHT: f32 = 24.0;
/// The divider and the 4 px above and below it.
const DIVIDER_HEIGHT: f32 = 9.0;
/// The popover's 4 px padding and 1 px border, top and bottom.
const POPOVER_FRAME: f32 = 10.0;
const TIP_LINE_HEIGHT: f32 = 15.5;
/// The tooltip's 4 px padding and 1 px border, top and bottom.
const TIP_FRAME: f32 = 10.0;
const NEXT_REMINDERS: usize = 3;

const RULE: u32 = 0xfff0e12e;
const FULL_FILL: u32 = 0xf2c66d1f;
const FULL_HOVER: u32 = 0xf2c66d33;
const WARN_FAINT: u32 = 0xf2c66db3;
const POPOVER: u32 = 0x1e1a16fa;
const POPOVER_BORDER: u32 = 0xffecd624;
const FOCUS_RING: u32 = 0xf3ebe1e6;
/// `chip-good`: good at 0.13; under the pointer at 0.20.
const READY_FILL: u32 = (GOOD << 8) | 0x21;
const READY_HOVER: u32 = (GOOD << 8) | 0x33;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Meter,
    Pvp,
    Todos,
    Pets,
    History,
    Reminders,
    Settings,
}

impl Tool {
    const ALL: [Tool; 7] = [Tool::Meter, Tool::Pvp, Tool::Todos, Tool::Pets, Tool::History, Tool::Reminders, Tool::Settings];

    /// The window the tool is; none for the meter (the hub's) and the
    /// reminders (a dropdown).
    pub fn window(self) -> Option<WindowKind> {
        match self {
            Tool::Pvp => Some(WindowKind::Pvp),
            Tool::Todos => Some(WindowKind::Todos),
            Tool::Pets => Some(WindowKind::Pets),
            Tool::History => Some(WindowKind::History),
            Tool::Settings => Some(WindowKind::Settings),
            Tool::Meter | Tool::Reminders => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Tool::Meter => "tool-meter",
            Tool::Pvp => "tool-pvp",
            Tool::Todos => "tool-todos",
            Tool::Pets => "tool-pets",
            Tool::History => "tool-history",
            Tool::Reminders => "tool-reminders",
            Tool::Settings => "tool-settings",
        }
    }

    fn name(self) -> String {
        match self {
            Tool::Meter => fl!("toolbar-meter"),
            Tool::Pvp => fl!("toolbar-pvp"),
            Tool::Todos => fl!("toolbar-todos"),
            Tool::Pets => fl!("toolbar-pets"),
            Tool::History => fl!("toolbar-history"),
            Tool::Reminders => fl!("toolbar-reminders"),
            Tool::Settings => fl!("toolbar-settings"),
        }
    }
}

/// A control of the bar, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Tool(Tool),
    Odyle,
    Shugo,
    Warn,
    Update,
    Fold,
    Unfold,
    Quit,
}

impl Slot {
    pub fn id(self) -> &'static str {
        match self {
            Slot::Tool(tool) => tool.id(),
            Slot::Odyle => "count-odyle",
            Slot::Shugo => "count-shugo",
            Slot::Warn => "warn",
            Slot::Update => "update",
            Slot::Fold => "fold",
            Slot::Unfold => "unfold",
            Slot::Quit => "quit",
        }
    }

    /// Tools hang their tooltip from their left edge; the counts, the
    /// warning and the end buttons from their right, so it stays over the
    /// bar. Folded, the bar is too short for that.
    fn tip_from_right(self, folded: bool) -> bool {
        !folded && !matches!(self, Slot::Tool(_))
    }
}

/// What the toolbar asks the app for; the hub does it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarRequest {
    Toggle(Tool),
    /// A count: opens the Resources window, or closes it while open.
    ToggleResources,
    OpenResources,
    PauseReminders(bool),
    ReminderSettings,
    Update(UpdateClick),
    Quit,
}

impl EventEmitter<ToolbarRequest> for ToolbarView {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dropdown {
    /// The warning's: the other resources that are full.
    Full,
    /// The bell's: the next reminders, pause, and the reminder settings.
    Bell,
}

/// The appearance the view draws with, and the part of the reminder settings
/// it shows: what a settings change may change here.
#[derive(Clone, Copy, PartialEq)]
struct Shown {
    alpha: u8,
    scale: f32,
    paused: bool,
}

impl Shown {
    fn of(settings: &settings::AppSettings) -> Self {
        Self { alpha: settings.overlay.background_alpha, scale: settings.overlay.scale(), paused: !settings.reminders.enabled }
    }

    fn look(self) -> Look {
        Look { s: Scale(self.scale), alpha: self.alpha }
    }
}

/// A row of the bell's dropdown.
#[derive(Clone, Debug, PartialEq)]
struct NextReminder {
    name: String,
    at: String,
}

pub struct ToolbarView {
    hub: Entity<Hub>,
    counts: Counts,
    shown: Shown,
    folded: bool,
    /// The control under the pointer, from its enter and leave.
    hover: Option<Slot>,
    /// The control whose tooltip is up, once the pointer has rested on it.
    tip: Option<Slot>,
    tip_up: bool,
    tip_timer: Option<Task<()>>,
    /// The open dropdown, and whether it opens upward.
    dropdown: Option<(Dropdown, bool)>,
    next_reminders: Vec<NextReminder>,
    /// The update icon; none while there is nothing to do or know.
    update: Option<UpdateIcon>,
    dropdown_close: Option<Task<()>>,
    focus: FocusHandle,
    /// The control the keyboard is on.
    focused: Option<Slot>,
    /// Where each control was last drawn, for the dropdowns and tooltips
    /// hanging from it.
    slots: Rc<RefCell<Vec<(Slot, Bounds<Pixels>)>>>,
    /// Where the bar sat in the window at the last frame.
    bar_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The pointer is on the toolbar's window, its dropdown included.
    pointer_over: bool,
    drag: WindowDrag,
    /// Dev captures cannot put the toolbar low on a real screen, so they say
    /// where dropdowns and tooltips open.
    #[cfg(any(test, feature = "probes"))]
    forced_up: Option<bool>,
    /// Tests and dev captures say whether you are fighting.
    #[cfg(any(test, feature = "probes"))]
    forced_fight: Option<bool>,
    _subscriptions: Vec<Subscription>,
}

impl ToolbarView {
    pub fn new(hub: Entity<Hub>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let todos = todos::global(cx);
        let store = settings::global(cx);
        let locale = i18n::locale(cx);
        let updates = update::global(cx);
        let mut subscriptions = vec![
            cx.observe(&hub, |_, _, cx| cx.notify()),
            cx.observe(&todos, |this: &mut Self, todos, cx| this.recount(&todos, cx)),
            cx.observe(&locale, |this: &mut Self, _, cx| {
                let todos = todos::global(cx);
                this.recount(&todos, cx);
                cx.notify();
            }),
            cx.observe(&store, |this: &mut Self, store, cx| {
                let shown = Shown::of(store.read(cx).get());
                if shown != this.shown {
                    this.shown = shown;
                    cx.notify();
                }
            }),
            cx.observe(&updates, |this: &mut Self, updates, cx| {
                let icon = UpdateIcon::of(updates.read(cx).state());
                this.follow_updates(icon, cx);
            }),
        ];
        if let Some(open_windows) = secondary::open_windows(cx) {
            subscriptions.push(cx.observe(&open_windows, |_, _, cx| cx.notify()));
        }
        Self {
            hub,
            counts: Counts::of(todos.read(cx), now_ms()),
            shown: Shown::of(store.read(cx).get()),
            folded: false,
            hover: None,
            tip: None,
            tip_up: false,
            tip_timer: None,
            dropdown: None,
            next_reminders: Vec::new(),
            update: UpdateIcon::of(updates.read(cx).state()),
            dropdown_close: None,
            focus: cx.focus_handle(),
            focused: None,
            slots: Rc::default(),
            bar_bounds: Rc::default(),
            pointer_over: false,
            drag: WindowDrag::default(),
            #[cfg(any(test, feature = "probes"))]
            forced_up: None,
            #[cfg(any(test, feature = "probes"))]
            forced_fight: None,
            _subscriptions: subscriptions,
        }
    }

    fn recount(&mut self, todos: &Entity<todos::Todos>, cx: &mut Context<Self>) {
        let counts = Counts::of(todos.read(cx), now_ms());
        if counts != self.counts {
            self.counts = counts;
            cx.notify();
        }
    }

    /// The icon changes only when what it shows does: a download redraws it
    /// once per percent. When it goes, the pointer and the keyboard leave it.
    fn follow_updates(&mut self, icon: Option<UpdateIcon>, cx: &mut Context<Self>) {
        if icon == self.update {
            return;
        }
        self.update = icon;
        if self.update.is_none() {
            if self.hover == Some(Slot::Update) {
                self.clear_pointer_state();
            }
            if self.focused == Some(Slot::Update) {
                self.focused = Some(if self.folded { Slot::Unfold } else { Slot::Fold });
            }
        }
        cx.notify();
    }

    fn in_fight(&self, cx: &App) -> bool {
        #[cfg(any(test, feature = "probes"))]
        if let Some(fighting) = self.forced_fight {
            return fighting;
        }
        self.hub.read(cx).in_fight(cx)
    }

    #[cfg(test)]
    pub fn is_folded(&self) -> bool {
        self.folded
    }

    #[cfg(test)]
    pub(super) fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn dropdown(&self) -> Option<Dropdown> {
        self.dropdown.map(|(dropdown, _)| dropdown)
    }

    #[cfg(test)]
    pub fn tip(&self) -> Option<Slot> {
        self.tip
    }

    /// The tooltip's lines as drawn now.
    #[cfg(test)]
    pub fn tip_lines(&self, cx: &App) -> Option<(SharedString, Option<SharedString>)> {
        self.tip.map(|slot| self.tip_text(slot, cx))
    }

    pub fn unfold(&mut self, cx: &mut Context<Self>) {
        if self.folded {
            self.folded = false;
            self.clear_pointer_state();
            cx.notify();
        }
    }

    fn fold(&mut self, cx: &mut Context<Self>) {
        self.folded = true;
        self.dropdown = None;
        self.clear_pointer_state();
        if matches!(self.focused, Some(slot) if !self.slots_in_order().contains(&slot)) {
            self.focused = Some(Slot::Unfold);
        }
        cx.notify();
    }

    fn clear_pointer_state(&mut self) {
        self.hover = None;
        self.tip = None;
        self.tip_timer = None;
    }

    /// The controls of the bar as drawn, left to right.
    fn slots_in_order(&self) -> Vec<Slot> {
        if self.folded {
            let mut slots = vec![Slot::Unfold];
            if self.counts.full_total > 0 {
                slots.push(Slot::Warn);
            }
            if self.update.is_some() {
                slots.push(Slot::Update);
            }
            return slots;
        }
        let mut slots: Vec<Slot> = Tool::ALL.iter().map(|&tool| Slot::Tool(tool)).collect();
        slots.extend([Slot::Odyle, Slot::Shugo]);
        if !self.counts.others.is_empty() {
            slots.push(Slot::Warn);
        }
        if self.update.is_some() {
            slots.push(Slot::Update);
        }
        slots.extend([Slot::Fold, Slot::Quit]);
        slots
    }

    fn is_lit(&self, slot: Slot, cx: &App) -> bool {
        match slot {
            Slot::Tool(Tool::Reminders) => self.dropdown() == Some(Dropdown::Bell),
            Slot::Tool(tool) => self.hub.read(cx).is_lit(tool, cx),
            Slot::Odyle | Slot::Shugo => secondary::is_open(WindowKind::Resources, cx),
            Slot::Warn => self.dropdown() == Some(Dropdown::Full),
            Slot::Update | Slot::Fold | Slot::Unfold | Slot::Quit => false,
        }
    }

    // --- Pointer ---

    fn hover_slot(&mut self, slot: Slot, hovered: bool, window: &Window, cx: &mut Context<Self>) {
        if hovered {
            self.hover = Some(slot);
            self.tip = None;
            let needed = (GAP + self.tip_height(slot, cx)) * self.shown.scale * window.scale_factor();
            let up = self.opens_upward(needed, window);
            self.tip_timer = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TIP_DELAY).await;
                let _ = this.update(cx, |this, cx| {
                    this.tip = this.hover;
                    this.tip_up = up;
                    this.tip_timer = None;
                    cx.notify();
                });
            }));
        } else if self.hover == Some(slot) {
            self.clear_pointer_state();
        }
        cx.notify();
    }

    /// The pointer left the toolbar and its dropdown: the dropdown goes
    /// after a moment, as the meter's menu does; coming back keeps it.
    fn pointer_on_toolbar(&mut self, over: bool, cx: &mut Context<Self>) {
        if over {
            self.dropdown_close = None;
            return;
        }
        if self.dropdown.is_none() {
            return;
        }
        self.dropdown_close = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DROPDOWN_CLOSE_AFTER).await;
            let _ = this.update(cx, |this, cx| {
                this.dropdown = None;
                this.dropdown_close = None;
                cx.notify();
            });
        }));
    }

    fn opens_upward(&self, needed: f32, window: &Window) -> bool {
        #[cfg(any(test, feature = "probes"))]
        if let Some(up) = self.forced_up {
            return up;
        }
        window::room(window).is_some_and(|(bar_bottom, work_bottom)| {
            window::opens_upward(bar_bottom, work_bottom, needed.ceil() as i32)
        })
    }

    // --- Actions ---

    fn press(&mut self, slot: Slot, window: &Window, cx: &mut Context<Self>) {
        self.clear_pointer_state();
        match slot {
            Slot::Tool(Tool::Reminders) => self.toggle_dropdown(Dropdown::Bell, window, cx),
            Slot::Tool(tool) => {
                self.dropdown = None;
                cx.emit(ToolbarRequest::Toggle(tool));
            }
            Slot::Odyle | Slot::Shugo => {
                self.dropdown = None;
                cx.emit(ToolbarRequest::ToggleResources);
            }
            Slot::Warn if self.folded => self.unfold(cx),
            Slot::Warn => self.toggle_dropdown(Dropdown::Full, window, cx),
            Slot::Update => {
                if let Some(icon) = &self.update {
                    self.dropdown = None;
                    cx.emit(ToolbarRequest::Update(icon.click()));
                }
            }
            Slot::Fold => self.fold(cx),
            Slot::Unfold => self.unfold(cx),
            Slot::Quit => cx.emit(ToolbarRequest::Quit),
        }
        cx.notify();
    }

    fn toggle_dropdown(&mut self, dropdown: Dropdown, window: &Window, cx: &mut Context<Self>) {
        if self.dropdown() == Some(dropdown) {
            self.dropdown = None;
            return;
        }
        if dropdown == Dropdown::Bell {
            self.next_reminders = next_reminders(&settings::global(cx).read(cx).get().reminders);
        }
        let needed = (GAP + self.dropdown_height(dropdown)) * self.shown.scale * window.scale_factor();
        self.dropdown = Some((dropdown, self.opens_upward(needed, window)));
        self.dropdown_close = None;
    }

    fn close_dropdown(&mut self, cx: &mut Context<Self>) {
        self.dropdown = None;
        self.dropdown_close = None;
        cx.notify();
    }

    fn pick(&mut self, request: ToolbarRequest, cx: &mut Context<Self>) {
        self.close_dropdown(cx);
        cx.emit(request);
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let order = self.slots_in_order();
        let at = self.focused.and_then(|slot| order.iter().position(|&each| each == slot));
        match event.keystroke.key.as_str() {
            "left" => self.focused = Some(order[at.map_or(0, |at| at.saturating_sub(1))]),
            "right" => self.focused = Some(order[at.map_or(0, |at| (at + 1).min(order.len() - 1))]),
            "home" => self.focused = order.first().copied(),
            "end" => self.focused = order.last().copied(),
            "enter" | "space" => match self.focused {
                Some(slot) => self.press(slot, window, cx),
                None => return,
            },
            "escape" if self.dropdown.is_some() => self.close_dropdown(cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    // --- Sizes the window must make room for ---

    fn tip_height(&self, slot: Slot, cx: &App) -> f32 {
        let lines = if self.tip_text(slot, cx).1.is_some() { 2. } else { 1. };
        TIP_FRAME + TIP_LINE_HEIGHT * lines
    }

    fn dropdown_height(&self, dropdown: Dropdown) -> f32 {
        let rows = match dropdown {
            Dropdown::Full => self.counts.others.len() + 1,
            Dropdown::Bell => self.next_reminders.len().max(1) + 2,
        };
        POPOVER_FRAME + LABEL_HEIGHT + DIVIDER_HEIGHT + ITEM_HEIGHT * rows as f32
    }

    // --- Drawing ---

    fn tip_text(&self, slot: Slot, cx: &App) -> (SharedString, Option<SharedString>) {
        let paused = self.shown.paused;
        match slot {
            Slot::Tool(Tool::Meter) => {
                let hub = self.hub.read(cx);
                let line = if hub.meter_is_aside(cx) {
                    fl!("toolbar-meter-aside-line", seconds = super::PEEK_SECS)
                } else if hub.meter_is_open() {
                    fl!("toolbar-meter-open-line")
                } else {
                    fl!("toolbar-meter-closed-line")
                };
                (Tool::Meter.name().into(), Some(line.into()))
            }
            Slot::Tool(Tool::Reminders) if paused => {
                (Tool::Reminders.name().into(), Some(fl!("toolbar-reminders-paused").into()))
            }
            Slot::Tool(tool) => (tool.name().into(), None),
            Slot::Odyle => (self.counts.odyle.name.clone().into(), Some(self.counts.odyle.tip.clone().into())),
            Slot::Shugo => (self.counts.shugo.name.clone().into(), Some(self.counts.shugo.tip.clone().into())),
            Slot::Warn => (fl!("toolbar-more-full", count = self.counts.others.len()).into(), None),
            Slot::Update => match &self.update {
                Some(icon) => {
                    let (name, line) = icon.tip(self.in_fight(cx));
                    (name.into(), Some(line.into()))
                }
                None => (SharedString::default(), None),
            },
            Slot::Fold => (fl!("toolbar-fold").into(), None),
            Slot::Unfold => (fl!("toolbar-unfold").into(), None),
            Slot::Quit => (fl!("toolbar-quit").into(), Some(fl!("toolbar-quit-line").into())),
        }
    }

    /// The common part of every control: enter and leave, a press that
    /// neither drags nor takes the keyboard, the click, and the focus ring.
    fn control(&self, slot: Slot, window: &Window, s: Scale, cx: &Context<Self>) -> Stateful<Div> {
        let ring = self.focused == Some(slot) && self.focus.is_focused(window) && window.last_input_was_keyboard();
        div()
            .id(slot.id())
            .debug_selector(move || slot.id().to_string())
            .relative()
            .flex_none()
            .h(s.px(26.))
            .flex()
            .items_center()
            .rounded(s.px(7.))
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, window, cx| this.hover_slot(slot, *hovered, window, cx)))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(move |this, _, window, cx| this.press(slot, window, cx)))
            .when(ring, |control| {
                control.child(
                    div()
                        .absolute()
                        .top(s.px(-3.))
                        .left(s.px(-3.))
                        .right(s.px(-3.))
                        .bottom(s.px(-3.))
                        .rounded(s.px(10.))
                        .border_2()
                        .border_color(rgba(FOCUS_RING)),
                )
            })
    }

    fn tool_button(&self, tool: Tool, window: &Window, s: Scale, cx: &Context<Self>) -> AnyElement {
        let slot = Slot::Tool(tool);
        let lit = self.is_lit(slot, cx);
        let hovered = self.hover == Some(slot);
        let paused = tool == Tool::Reminders && self.shown.paused;
        let glyph = match tool {
            Tool::Meter => Icon::Swords,
            Tool::Pvp => Icon::Crosshair,
            Tool::Todos => Icon::ListChecks,
            Tool::Pets => Icon::PawPrint,
            Tool::History => Icon::Crown,
            Tool::Reminders if paused => Icon::BellOff,
            Tool::Reminders => Icon::Bell,
            Tool::Settings => Icon::Settings,
        };
        let colour = if lit || hovered {
            TEXT
        } else if paused {
            FAINT
        } else {
            MUTED
        };
        let (name, line) = self.tip_text(slot, cx);
        self.control(slot, window, s, cx)
            .w(s.px(26.))
            .justify_center()
            .role(Role::Button)
            .aria_label(name)
            .when_some(line, |button, line| button.aria_description(line))
            .when(tool != Tool::Reminders, |button| button.aria_toggled(if lit { Toggled::True } else { Toggled::False }))
            .when(tool == Tool::Reminders, |button| button.aria_expanded(lit))
            .when(lit, |button| button.bg(rgba(HOT)))
            .when(hovered && !lit, |button| button.bg(rgba(SOFT)))
            .child(chrome::icon(glyph, 14., rgba(colour), s).when(lit, |icon| icon.mt(s.px(-3.))))
            .when(lit, |button| {
                button.child(div().absolute().bottom(s.px(2.)).left(s.px(11.)).size(s.px(4.)).rounded_full().bg(rgba(colour)))
            })
            .into_any_element()
    }

    fn end_button(&self, slot: Slot, glyph: Icon, window: &Window, s: Scale, cx: &Context<Self>) -> AnyElement {
        let hovered = self.hover == Some(slot);
        let (name, line) = self.tip_text(slot, cx);
        self.control(slot, window, s, cx)
            .w(s.px(20.))
            .justify_center()
            .role(Role::Button)
            .aria_label(name)
            .when_some(line, |button, line| button.aria_description(line))
            .when(hovered, |button| button.bg(rgba(SOFT)))
            .child(chrome::icon(glyph, 12., rgba(if hovered { TEXT } else { FAINT }), s))
            .into_any_element()
    }

    fn count(&self, slot: Slot, pinned: &Pinned, window: &Window, s: Scale, cx: &Context<Self>) -> AnyElement {
        let glyph = if slot == Slot::Odyle { Icon::Zap } else { Icon::Key };
        let full = pinned.figure.is_some_and(|figure| figure.full);
        let hovered = self.hover == Some(slot);
        let lit = self.is_lit(slot, cx);
        let fill = match (lit, full, hovered) {
            (true, _, _) => Some(HOT),
            (false, true, true) => Some(FULL_HOVER),
            (false, true, false) => Some(FULL_FILL),
            (false, false, true) => Some(SOFT),
            (false, false, false) => None,
        };
        let figure: AnyElement = match pinned.figure {
            Some(figure) => div()
                .flex()
                .items_baseline()
                .child(
                    div()
                        .text_size(s.px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if full { rgb(WARN) } else { rgba(TEXT) })
                        .child(figure.value.to_string()),
                )
                .child(
                    div()
                        .ml(s.px(1.))
                        .text_size(s.px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(if full { WARN_FAINT } else { FAINT }))
                        .child(format!("/{}", figure.cap)),
                )
                .when(figure.extra > 0, |row| {
                    row.child(
                        div()
                            .ml(s.px(3.))
                            .text_size(s.px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgba(MUTED))
                            .child(format!("+{}", figure.extra)),
                    )
                })
                .into_any_element(),
            None => div()
                .text_size(s.px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(FAINT))
                .child("\u{2014}")
                .into_any_element(),
        };
        self.control(slot, window, s, cx)
            .px(s.px(7.))
            .gap(s.px(5.))
            .role(Role::Button)
            .aria_label(SharedString::from(pinned.label.clone()))
            .when_some(fill, |count, fill| count.bg(rgba(fill)))
            .child(chrome::icon(glyph, 13., if full { rgb(WARN) } else { rgba(MUTED) }, s))
            .child(figure)
            .into_any_element()
    }

    fn warn_button(&self, window: &Window, s: Scale, cx: &Context<Self>) -> AnyElement {
        let count = if self.folded { self.counts.full_total } else { self.counts.others.len() };
        let open = self.dropdown() == Some(Dropdown::Full);
        let hovered = self.hover == Some(Slot::Warn);
        let label = if self.folded {
            fl!("toolbar-folded-full-label", count = count)
        } else {
            fl!("toolbar-more-full-label", count = count)
        };
        self.control(Slot::Warn, window, s, cx)
            .px(s.px(7.))
            .gap(s.px(4.))
            .role(Role::Button)
            .aria_label(label)
            .when(!self.folded, |button| button.aria_expanded(open))
            .bg(rgba(if open || hovered { FULL_HOVER } else { FULL_FILL }))
            .text_size(s.px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(WARN))
            .child(chrome::icon(Icon::TriangleAlert, 13., rgb(WARN), s))
            .child(count.to_string())
            .into_any_element()
    }

    /// Available: the download icon in text on the soft fill. Downloading,
    /// paused or checking: the icon in muted, 2 px higher, over a 2 px
    /// progress line. Ready: rotate-cw in good on chip-good. Nothing pulses.
    fn update_button(&self, icon: &UpdateIcon, window: &Window, s: Scale, cx: &Context<Self>) -> AnyElement {
        let hovered = self.hover == Some(Slot::Update);
        let (name, line) = self.tip_text(Slot::Update, cx);
        let progress = icon.progress();
        let (fill, colour, glyph): (Option<Hsla>, Hsla, Icon) = match icon {
            UpdateIcon::Available { .. } => (Some(rgba(if hovered { HOT } else { SOFT }).into()), rgba(TEXT).into(), Icon::Download),
            UpdateIcon::Downloading { .. } | UpdateIcon::Verifying { .. } => {
                (hovered.then(|| rgba(SOFT).into()), rgba(if hovered { TEXT } else { MUTED }).into(), Icon::Download)
            }
            UpdateIcon::Ready { .. } => (Some(rgba(if hovered { READY_HOVER } else { READY_FILL }).into()), rgb(GOOD).into(), Icon::RotateCw),
        };
        self.control(Slot::Update, window, s, cx)
            .w(s.px(26.))
            .justify_center()
            .role(Role::Button)
            .aria_label(name)
            .when_some(line, |button, line| button.aria_description(line))
            .when_some(fill, |button, fill| button.bg(fill))
            .child(chrome::icon(glyph, 14., colour, s).when(progress.is_some(), |icon| icon.mt(s.px(-2.))))
            .when_some(progress, |button, (fraction, paused)| {
                button.child(
                    div()
                        .id("update-progress")
                        .absolute()
                        .left(s.px(5.))
                        .right(s.px(5.))
                        .bottom(s.px(4.))
                        .h(s.px(2.))
                        .rounded(s.px(1.))
                        .overflow_hidden()
                        .bg(rgba(GAUGE_EMPTY))
                        .role(Role::ProgressIndicator)
                        .aria_min_numeric_value(0.)
                        .aria_max_numeric_value(100.)
                        .aria_numeric_value(f64::from(fraction) * 100.)
                        .child(div().h_full().w(relative(fraction)).rounded(s.px(1.)).bg(rgba(if paused { FAINT } else { MUTED }))),
                )
            })
            .into_any_element()
    }

    fn rule(s: Scale) -> AnyElement {
        div().flex_none().w(s.px(1.)).h(s.px(16.)).mx(s.px(5.)).bg(rgba(RULE)).into_any_element()
    }

    fn grip(&self, s: Scale) -> AnyElement {
        div()
            .flex_none()
            .w(s.px(14.))
            .h(s.px(26.))
            .flex()
            .items_center()
            .justify_center()
            .cursor(CursorStyle::OpenHand)
            .child(chrome::icon(Icon::Grip, 12., rgba(FAINT), s))
            .into_any_element()
    }

    fn bar(&self, window: &Window, s: Scale, cx: &Context<Self>) -> Stateful<Div> {
        let mut children: Vec<(Option<Slot>, AnyElement)> = vec![(None, self.grip(s))];
        if self.folded {
            children.push((Some(Slot::Unfold), self.end_button(Slot::Unfold, Icon::ChevronsRight, window, s, cx)));
            if self.counts.full_total > 0 {
                children.push((Some(Slot::Warn), self.warn_button(window, s, cx)));
            }
            if let Some(icon) = &self.update {
                children.push((Some(Slot::Update), self.update_button(icon, window, s, cx)));
            }
        } else {
            for tool in Tool::ALL {
                children.push((Some(Slot::Tool(tool)), self.tool_button(tool, window, s, cx)));
            }
            children.push((None, Self::rule(s)));
            children.push((Some(Slot::Odyle), self.count(Slot::Odyle, &self.counts.odyle, window, s, cx)));
            children.push((Some(Slot::Shugo), self.count(Slot::Shugo, &self.counts.shugo, window, s, cx)));
            if !self.counts.others.is_empty() {
                children.push((Some(Slot::Warn), self.warn_button(window, s, cx)));
            }
            children.push((None, Self::rule(s)));
            if let Some(icon) = &self.update {
                children.push((Some(Slot::Update), self.update_button(icon, window, s, cx)));
            }
            children.push((Some(Slot::Fold), self.end_button(Slot::Fold, Icon::ChevronsLeft, window, s, cx)));
            children.push((Some(Slot::Quit), self.end_button(Slot::Quit, Icon::Close, window, s, cx)));
        }
        let order: Vec<Option<Slot>> = children.iter().map(|(slot, _)| *slot).collect();
        let slots = self.slots.clone();
        let bar = chrome::frame(self.shown.look(), 12.)
            .flex_none()
            .flex()
            .items_center()
            .gap(s.px(2.))
            .p(s.px(4.))
            .on_children_prepainted(move |bounds, _, _| {
                let drawn = order.iter().zip(bounds).filter_map(|(slot, bounds)| slot.map(|slot| (slot, bounds))).collect();
                *slots.borrow_mut() = drawn;
            })
            .id("toolbar")
            .debug_selector(|| "toolbar".to_string())
            .role(Role::Toolbar)
            .aria_label(fl!("toolbar-label"))
            .children(children.into_iter().map(|(_, child)| child));
        self.drag.handle_around_controls(bar)
    }

    fn slot_bounds(&self, slot: Slot) -> Option<Bounds<Pixels>> {
        self.slots.borrow().iter().find(|(each, _)| *each == slot).map(|(_, bounds)| *bounds)
    }

    /// The tooltip under (or over) its control.
    fn tooltip(&self, slot: Slot, s: Scale, cx: &App) -> Option<(Hanging, f32)> {
        let bounds = self.slot_bounds(slot)?;
        let (title, line) = self.tip_text(slot, cx);
        let height = self.tip_height(slot, cx);
        let tip = div()
            .flex_none()
            .py(s.px(4.))
            .px(s.px(8.))
            .rounded(s.px(6.))
            .border_1()
            .border_color(rgba(POPOVER_BORDER))
            .bg(rgba(POPOVER))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .line_height(s.px(TIP_LINE_HEIGHT))
            .whitespace_nowrap()
            .id("tooltip")
            .role(Role::Tooltip)
            .child(div().text_color(rgba(TEXT)).child(title))
            .children(line.map(|line| div().text_color(rgba(MUTED)).child(line)))
            .into_any_element();
        let align = if slot.tip_from_right(self.folded) { Align::Right(bounds.right()) } else { Align::Left(bounds.left()) };
        Some((Hanging { element: tip, align, up: self.tip_up }, height))
    }

    fn dropdown_element(&self, dropdown: Dropdown, up: bool, s: Scale, cx: &Context<Self>) -> Option<(Hanging, f32)> {
        let align = match dropdown {
            Dropdown::Full => Align::Right(self.slot_bounds(Slot::Warn)?.right()),
            Dropdown::Bell => Align::Left(self.slot_bounds(Slot::Tool(Tool::Reminders))?.left()),
        };
        let element = match dropdown {
            Dropdown::Full => self.full_dropdown(s, cx),
            Dropdown::Bell => self.bell_dropdown(s, cx),
        };
        Some((Hanging { element, align, up }, self.dropdown_height(dropdown)))
    }

    fn dropdown_frame(width: f32, label: SharedString, glyph: Icon, s: Scale) -> Stateful<Div> {
        div()
            .id("dropdown")
            .flex_none()
            .w(s.px(width))
            .occlude()
            .p(s.px(4.))
            .flex()
            .flex_col()
            .rounded(s.px(9.))
            .border_1()
            .border_color(rgba(POPOVER_BORDER))
            .bg(rgba(POPOVER))
            .text_size(s.px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgba(TEXT))
            .role(Role::Menu)
            .aria_label(label.clone())
            .child(
                div()
                    .flex_none()
                    .h(s.px(LABEL_HEIGHT))
                    .px(s.px(8.))
                    .flex()
                    .items_center()
                    .gap(s.px(6.))
                    .text_size(s.px(11.))
                    .text_color(rgba(FAINT))
                    .child(chrome::icon(glyph, 12., rgba(FAINT), s))
                    .child(label),
            )
    }

    fn divider(s: Scale) -> Div {
        div().flex_none().h(s.px(1.)).mx(s.px(4.)).my(s.px(4.)).bg(rgba(RULE))
    }

    fn full_dropdown(&self, s: Scale, cx: &Context<Self>) -> AnyElement {
        let rows = self.counts.others.iter().enumerate().map(|(index, line)| {
            let label = fl!(
                "toolbar-refills-label",
                name = line.name.clone(),
                value = line.value,
                cap = line.cap,
                time = line.refills_at.clone()
            );
            chrome::menu_item(("full", index), false, s)
                .debug_selector(move || format!("full-{index}"))
                .gap(s.px(6.))
                .role(Role::MenuItem)
                .aria_label(label)
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(cx.listener(|this, _, _, cx| this.pick(ToolbarRequest::OpenResources, cx)))
                .child(div().flex_1().min_w_0().truncate().child(line.name.clone()))
                .child(
                    div()
                        .flex_none()
                        .w(s.px(42.))
                        .flex()
                        .justify_end()
                        .items_baseline()
                        .child(
                            div()
                                .text_size(s.px(12.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(WARN))
                                .child(line.value.to_string()),
                        )
                        .child(div().text_size(s.px(11.)).text_color(rgba(WARN_FAINT)).child(format!("/{}", line.cap))),
                )
                .child(
                    div()
                        .flex_none()
                        .w(s.px(74.))
                        .flex()
                        .justify_end()
                        .items_baseline()
                        .gap(s.px(3.))
                        .text_size(s.px(11.))
                        .text_color(rgba(FAINT))
                        .child(fl!("toolbar-refills"))
                        .child(div().font_weight(FontWeight::SEMIBOLD).text_color(rgba(MUTED)).child(line.refills_at.clone())),
                )
        });
        let all = chrome::menu_item("all-resources", false, s)
            .debug_selector(|| "all-resources".to_string())
            .gap(s.px(6.))
            .role(Role::MenuItem)
            .aria_label(fl!("toolbar-all-resources"))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(|this, _, _, cx| this.pick(ToolbarRequest::OpenResources, cx)))
            .child(div().flex_none().size(s.px(12.)).child(chrome::icon(Icon::Package, 12., rgba(MUTED), s)))
            .child(div().flex_1().min_w_0().truncate().child(fl!("toolbar-all-resources")))
            .child(chrome::icon(Icon::ChevronRight, 12., rgba(FAINT), s));
        Self::dropdown_frame(FULL_DROPDOWN_WIDTH, fl!("toolbar-full-heading").into(), Icon::TriangleAlert, s)
            .debug_selector(|| "full-dropdown".to_string())
            .children(rows)
            .child(Self::divider(s))
            .child(all)
            .into_any_element()
    }

    fn bell_dropdown(&self, s: Scale, cx: &Context<Self>) -> AnyElement {
        let paused = self.shown.paused;
        let rows: Vec<Div> = if self.next_reminders.is_empty() {
            vec![div()
                .flex_none()
                .h(s.px(ITEM_HEIGHT))
                .px(s.px(8.))
                .flex()
                .items_center()
                .text_size(s.px(11.5))
                .text_color(rgba(MUTED))
                .child(fl!("toolbar-no-reminders"))]
        } else {
            self.next_reminders
                .iter()
                .map(|next| {
                    div()
                        .flex_none()
                        .h(s.px(ITEM_HEIGHT))
                        .px(s.px(8.))
                        .flex()
                        .items_center()
                        .gap(s.px(6.))
                        .child(div().flex_1().min_w_0().truncate().child(next.name.clone()))
                        .child(div().flex_none().font_weight(FontWeight::SEMIBOLD).text_color(rgba(MUTED)).child(next.at.clone()))
                })
                .collect()
        };
        let pause = chrome::menu_item("pause-reminders", false, s)
            .debug_selector(|| "pause-reminders".to_string())
            .gap(s.px(6.))
            .role(Role::MenuItemCheckBox)
            .aria_label(fl!("toolbar-pause-reminders"))
            .aria_toggled(if paused { Toggled::True } else { Toggled::False })
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(move |this, _, _, cx| this.pick(ToolbarRequest::PauseReminders(!paused), cx)))
            .child(
                div()
                    .flex_none()
                    .size(s.px(12.))
                    .when(paused, |lead| lead.child(chrome::icon(Icon::Check, 12., rgba(MUTED), s))),
            )
            .child(div().flex_1().min_w_0().truncate().child(fl!("toolbar-pause-reminders")));
        let reminder_settings = chrome::menu_item("reminder-settings", false, s)
            .debug_selector(|| "reminder-settings".to_string())
            .gap(s.px(6.))
            .role(Role::MenuItem)
            .aria_label(fl!("toolbar-reminder-settings"))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(|this, _, _, cx| this.pick(ToolbarRequest::ReminderSettings, cx)))
            .child(div().flex_none().size(s.px(12.)).child(chrome::icon(Icon::Settings, 12., rgba(MUTED), s)))
            .child(div().flex_1().min_w_0().truncate().child(fl!("toolbar-reminder-settings")))
            .child(chrome::icon(Icon::ChevronRight, 12., rgba(FAINT), s));
        Self::dropdown_frame(BELL_DROPDOWN_WIDTH, fl!("toolbar-next-reminders").into(), Icon::Bell, s)
            .debug_selector(|| "bell-dropdown".to_string())
            .children(rows)
            .child(Self::divider(s))
            .child(pause)
            .child(reminder_settings)
            .into_any_element()
    }

    /// For dev captures: the pointer resting on `slot`, its tooltip up.
    #[cfg(feature = "probes")]
    pub fn force_tip(&mut self, slot: Option<Slot>, cx: &mut Context<Self>) {
        self.hover = slot;
        self.tip = slot;
        self.tip_up = self.forced_up.unwrap_or(false);
        self.tip_timer = None;
        cx.notify();
    }

    /// For dev captures: a dropdown open, upward or not as forced.
    #[cfg(feature = "probes")]
    pub fn force_dropdown(&mut self, dropdown: Option<Dropdown>, cx: &mut Context<Self>) {
        self.clear_pointer_state();
        if dropdown == Some(Dropdown::Bell) {
            self.next_reminders = next_reminders(&settings::global(cx).read(cx).get().reminders);
        }
        self.dropdown = dropdown.map(|dropdown| (dropdown, self.forced_up.unwrap_or(false)));
        self.dropdown_close = None;
        cx.notify();
    }

    #[cfg(feature = "probes")]
    pub fn force_up(&mut self, up: Option<bool>) {
        self.forced_up = up;
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn force_fight(&mut self, fighting: Option<bool>, cx: &mut Context<Self>) {
        self.forced_fight = fighting;
        cx.notify();
    }

    /// For dev captures: the keyboard given to the toolbar, on `slot`, as an
    /// accessibility tool would give it.
    #[cfg(feature = "probes")]
    pub fn take_keyboard(&mut self, slot: Slot, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = Some(slot);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    #[cfg(feature = "probes")]
    pub fn set_folded(&mut self, folded: bool, cx: &mut Context<Self>) {
        if folded {
            self.fold(cx);
        } else {
            self.unfold(cx);
        }
    }

    /// Where a control was last drawn, in the window's logical pixels.
    #[cfg(feature = "probes")]
    pub fn centre_of(&self, slot: Slot) -> Option<gpui::Point<Pixels>> {
        self.slot_bounds(slot).map(|bounds| bounds.center())
    }
}

/// How a dropdown or tooltip hangs from its control: its left edge on the
/// control's left, or its right edge on the control's right (pushed right
/// when the window's left edge is in the way).
#[derive(Clone, Copy)]
enum Align {
    Left(Pixels),
    Right(Pixels),
}

struct Hanging {
    element: AnyElement,
    align: Align,
    up: bool,
}

/// The next reminders switched on, paused or not: "Shugo Festival 22:00".
fn next_reminders(settings: &ReminderSettings) -> Vec<NextReminder> {
    reminders::upcoming(&LocalZone, &settings.switched_on_events(), now_ms(), NEXT_REMINDERS)
        .into_iter()
        .map(|(event, start_ms)| NextReminder { name: event.name(), at: clock_time(&LocalZone, start_ms).to_string() })
        .collect()
}

impl Render for ToolbarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let over = window.is_window_hovered();
        if over != self.pointer_over {
            self.pointer_over = over;
            self.pointer_on_toolbar(over, cx);
        }
        let look = self.shown.look();
        let s = look.s;
        let bar = self.bar(window, s, cx);
        let hanging = match self.dropdown {
            Some((dropdown, up)) => self.dropdown_element(dropdown, up, s, cx),
            None => self.tip.and_then(|slot| self.tooltip(slot, s, cx)),
        };
        let lift = hanging.as_ref().filter(|(hanging, _)| hanging.up).map(|(_, height)| s.px(height + GAP));
        // The bar's 4 px padding and 1 px border around 26 px controls,
        // until it has been drawn.
        let bar_height = self.bar_bounds.get().map_or(s.px(36.), |bar| bar.size.height);
        let hanging = hanging.map(|(hanging, _)| {
            let container = match hanging.align {
                Align::Left(left) => div().absolute().left(left).child(hanging.element),
                Align::Right(right) => div().absolute().left_0().min_w(right).flex().justify_end().child(hanging.element),
            };
            if lift.is_some() {
                container.top_0()
            } else {
                container.top(bar_height + s.px(GAP))
            }
        });
        let scrim = self.dropdown.is_some().then(|| {
            div().id("dropdown-scrim").absolute().inset_0().occlude().on_any_mouse_down(cx.listener(|this, _, _, cx| {
                this.close_dropdown(cx);
                cx.stop_propagation();
            }))
        });
        let has_hanging = hanging.is_some();
        let has_scrim = scrim.is_some();
        let bar_bounds = self.bar_bounds.clone();
        chrome::surface(look)
            .when_some(lift, |root, lift| root.pt(lift))
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bar) = bounds.first().copied() else {
                    return;
                };
                let popup = has_hanging.then(|| bounds.get(1 + usize::from(has_scrim))).flatten();
                let wanted = bounds_union(&bar, popup);
                let shift = bar_bounds.replace(Some(bar)).map_or(px(0.), |before| bar.origin.y - before.origin.y);
                window::refit(window, wanted, shift, cx);
            })
            .id("toolbar-root")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .child(bar)
            .children(scrim)
            .children(hanging)
            .child(self.drag.listener())
    }
}

/// The size from the window's corner that holds the bar and what hangs from it.
fn bounds_union(bar: &Bounds<Pixels>, popup: Option<&Bounds<Pixels>>) -> gpui::Size<Pixels> {
    popup.into_iter().fold(size(bar.right(), bar.bottom()), |wanted, popup| {
        size(wanted.width.max(popup.right()), wanted.height.max(popup.bottom()))
    })
}
