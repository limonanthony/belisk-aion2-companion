//! The Pets window (docs/gpui-mockups/approved/pets.html, values in
//! tokens.md, "Pets window"): every pet with its level and its souls toward
//! the next level, family tabs, a search, and the Show and Order menu. The
//! pets pinned on the server being played sit on top. What it shows comes
//! from the `Pets` entity, kept up to date whether or not the meter is open;
//! the menu's choice is kept with the collection in `pets.json`.
//!
//! The window never activates, like the other overlays, except while the
//! search field takes the keyboard (a click in it), as the to-do window's
//! fields do; see `secondary::typing`. Escape clears the search, a second
//! Escape (or Enter, a click elsewhere, the pointer leaving the window) gives
//! the keyboard back to the game. Accessibility tools: Tab reaches the
//! search, the filter button, the tabs (Left and Right) and the pins (Up and
//! Down; Space pins).
//!
//! A row's pin appears when the pointer enters the row and goes when it
//! leaves; moves inside the row change nothing. A pinned pet keeps its pin.

mod rows;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::time::Duration;

use gpui::{
    div, prelude::*, px, relative, rgba, AnyElement, App, ClickEvent, Context, Div, Entity, FocusHandle, Focusable as _,
    FontWeight, HighlightStyle, KeyDownEvent, MouseButton, MouseMoveEvent, Pixels, Point, Render, Role, SharedString,
    Stateful, StyledText, Subscription, Task, Toggled, Window,
};
use meter_core::pets::SOULS_PER_LEVEL;
use windows::Win32::Foundation::HWND;

use super::list::{ListChoice, Order, Show, Tab};
use super::service::{self, Pets};
use crate::assets::Icon;
use crate::clock::{clock_time, LocalZone};
use crate::game_data::{self, PetFamily};
use crate::i18n::{self, day_stamp, fl};
use crate::meter::OverlaySettings;
use crate::overlay;
use crate::secondary::chrome::{self, Anchor, Look, Rows, Scale, FAINT, GAUGE_EMPTY, HOT, MUTED, SOFT, TEXT};
use crate::secondary::text_input::{TextInput, TextInputEvent};
use crate::secondary::typing::{self, Edit};
use crate::settings;

const WIDTH: f32 = 370.0;
/// The list scrolls past this height.
const LIST_MAX: f32 = 452.0;
/// How long the pointer may be away before typing ends.
const LEAVE_GRACE: Duration = Duration::from_millis(400);
const MENU_WIDTH: f32 = 232.0;
/// The tallest the window gets at 100%; it fits its card from the first
/// frame on.
pub const EXPECTED_SIZE: (f32, f32) = (370.0, 680.0);

const FOCUS_RING: u32 = 0xf3ebe1e6;
const DIVIDER: u32 = 0xfff0e12e;
const LINK_UNDERLINE: u32 = 0xfff6ec40;
const PROGRESS_FILL: u32 = 0xfff6ec8c;
const OFF_OPACITY: f32 = 0.4;

/// The Show and Order menu's choices, in the order drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuItem {
    Show(Show),
    Order(Order),
}

const MENU: [MenuItem; 5] = [
    MenuItem::Show(Show::All),
    MenuItem::Show(Show::NotOwned),
    MenuItem::Show(Show::NotMaxed),
    MenuItem::Order(Order::Game),
    MenuItem::Order(Order::ClosestToNextLevel),
];

impl MenuItem {
    fn label(self) -> String {
        match self {
            Self::Show(Show::All) => fl!("pets-show-all"),
            Self::Show(Show::NotOwned) => fl!("pets-show-not-owned"),
            Self::Show(Show::NotMaxed) => fl!("pets-show-not-maxed"),
            Self::Order(Order::Game) => fl!("pets-order-game"),
            Self::Order(Order::ClosestToNextLevel) => fl!("pets-order-closest"),
        }
    }

    fn is_current(self, choice: ListChoice) -> bool {
        match self {
            Self::Show(show) => choice.show == show,
            Self::Order(order) => choice.order == order,
        }
    }

    fn chosen(self, choice: ListChoice) -> ListChoice {
        match self {
            Self::Show(show) => ListChoice { show, ..choice },
            Self::Order(order) => ListChoice { order, ..choice },
        }
    }
}

fn tab_name(tab: Tab) -> String {
    match tab {
        Tab::All => fl!("pets-tab-all"),
        Tab::Family(family) => family_name(family),
    }
}

fn family_name(family: PetFamily) -> String {
    match family {
        PetFamily::Cogni => fl!("pets-family-cogni"),
        PetFamily::Fera => fl!("pets-family-fera"),
        PetFamily::Natura => fl!("pets-family-natura"),
        PetFamily::Varian => fl!("pets-family-varian"),
        PetFamily::Special => fl!("pets-family-special"),
    }
}

/// Where the shown collection comes from.
enum Source {
    /// Sent at a login during this run.
    Live { time: String, server: String },
    /// Kept from an earlier run.
    Earlier { stamp: String },
    NotRead,
}

/// A control's keyboard focus as the view sees it this frame.
#[derive(Clone)]
struct Focus {
    handle: FocusHandle,
    /// Focused, and the last input came from the keyboard.
    ring: bool,
}

pub struct PetsView {
    pets: Entity<Pets>,
    overlay: OverlaySettings,
    /// The window's handle, kept so the keyboard can be given back even when
    /// the window is going away.
    hwnd: Option<HWND>,
    edit: Edit,
    search: Entity<TextInput>,
    /// The search was opened from the keyboard, which goes back to the search
    /// box when typing ends.
    search_from_keyboard: bool,
    tab: Tab,
    /// The Show and Order menu is open with this item lit.
    menu: Option<usize>,
    menu_anchor: Anchor,
    /// The row the pointer is in, from its enter and leave only.
    pointer_row: Option<u32>,
    /// After a pin moved a row away from under the pointer, the row now there
    /// keeps its pin hidden until the pointer moves from here.
    held_still: Option<Point<Pixels>>,
    /// The pin Tab reaches in the list; Up and Down move it.
    pin_cursor: Option<u32>,
    /// The pets listed this frame, in order, for Up and Down.
    drawn: Vec<u32>,
    controls: HashMap<String, FocusHandle>,
    /// Each pet's pin, by pet.
    pins: HashMap<u32, FocusHandle>,
    root_focus: FocusHandle,
    rows: Rows,
    /// The list's lines, drawn by `rows` as they come into view.
    lines: Vec<rows::Line>,
    lines_frame: Option<rows::LinesFrame>,
    /// The pet whose row the keyboard brings into view at the next frame.
    reveal_pet: Option<u32>,
    drag: overlay::WindowDrag,
    leave_timer: Option<Task<()>>,
    announcement: Option<SharedString>,
    /// Tests and dev captures give the field the keyboard inside GPUI only:
    /// neither may ask Windows for the foreground.
    #[cfg(any(test, feature = "probes"))]
    keyboard_in_app: bool,
    /// A capture's window is off screen, so no pointer is ever over it.
    #[cfg(any(test, feature = "probes"))]
    pointer_held_over: bool,
    /// Where each row was drawn, for captures that post pointer messages to
    /// rows.
    #[cfg(feature = "probes")]
    row_marks: std::cell::RefCell<HashMap<u32, Anchor>>,
    /// The pets whose rows the list built in the last frame, for the
    /// scrollbar check.
    #[cfg(feature = "probes")]
    built: std::cell::RefCell<Vec<u32>>,
    _subscriptions: Vec<Subscription>,
}

impl PetsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pets = service::global(cx);
        let store = settings::global(cx);
        let overlay = store.read(cx).get().overlay.clone();
        let search = cx.new(|cx| TextInput::new(fl!("pets-search"), cx));
        let search_focus = search.read(cx).focus_handle(cx);
        let locale = i18n::locale(cx);
        let subscriptions = vec![
            cx.observe(&pets, |_, _, cx| cx.notify()),
            cx.observe(&locale, |this, _, cx| this.search.update(cx, |input, cx| input.set_placeholder(fl!("pets-search"), cx))),
            cx.observe(&store, |this, store, cx| {
                let overlay = &store.read(cx).get().overlay;
                if *overlay != this.overlay {
                    this.overlay = overlay.clone();
                    cx.notify();
                }
            }),
            cx.observe(&search, |_, _, cx| cx.notify()),
            cx.subscribe_in(&search, window, |this, search, event: &TextInputEvent, window, cx| match event {
                TextInputEvent::Submitted(_) => this.stop_search(true, window, cx),
                TextInputEvent::Cancelled if !search.read(cx).text().is_empty() => search.update(cx, |input, cx| input.clear(cx)),
                TextInputEvent::Cancelled => this.stop_search(true, window, cx),
            }),
            cx.on_blur(&search_focus, window, |this, window, cx| this.stop_search(false, window, cx)),
            // A focused control that goes away (a pin of a pet a search left
            // out) leaves the focus with the window, so Tab keeps working.
            cx.on_focus_lost(window, |this, window, cx| window.focus(&this.root_focus, cx)),
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.stop_search(false, window, cx);
                }
            }),
        ];
        let hwnd = overlay::hwnd(window).map_err(|error| crate::app_log::warn(format!("the Pets window cannot take the keyboard: {error}"))).ok();
        // Closing the window or quitting mid-search gives the keyboard back.
        cx.on_release(|this, _| this.finish_search()).detach();
        cx.on_app_quit(|this, _| {
            this.finish_search();
            async {}
        })
        .detach();
        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);
        Self {
            pets,
            overlay,
            #[cfg(any(test, feature = "probes"))]
            keyboard_in_app: hwnd.is_none(),
            hwnd,
            edit: Edit::Idle,
            search,
            search_from_keyboard: false,
            tab: Tab::All,
            menu: None,
            menu_anchor: Anchor::default(),
            pointer_row: None,
            held_still: None,
            pin_cursor: None,
            drawn: Vec::new(),
            controls: HashMap::new(),
            pins: HashMap::new(),
            root_focus,
            rows: Rows::new(px(LIST_MAX)),
            lines: Vec::new(),
            lines_frame: None,
            reveal_pet: None,
            drag: overlay::WindowDrag::default(),
            leave_timer: None,
            announcement: None,
            #[cfg(any(test, feature = "probes"))]
            pointer_held_over: false,
            #[cfg(feature = "probes")]
            row_marks: std::cell::RefCell::default(),
            #[cfg(feature = "probes")]
            built: std::cell::RefCell::default(),
            _subscriptions: subscriptions,
        }
    }

    // ---- Focus ----

    fn handle(&mut self, key: &str, cx: &mut App) -> FocusHandle {
        if let Some(handle) = self.controls.get(key) {
            return handle.clone();
        }
        let handle = cx.focus_handle().tab_stop(true);
        self.controls.insert(key.to_string(), handle.clone());
        handle
    }

    fn focus_of(&mut self, key: &str, window: &Window, cx: &mut App) -> Focus {
        let handle = self.handle(key, cx);
        Focus { ring: handle.is_focused(window) && window.last_input_was_keyboard(), handle }
    }

    /// [`Self::focus_of`] for one of a group Tab reaches once: only the
    /// group's `stop` is in the tab order.
    fn roving_focus(&mut self, key: &str, stop: bool, window: &Window, cx: &mut App) -> Focus {
        let focus = self.focus_of(key, window, cx);
        Focus { handle: focus.handle.clone().tab_stop(stop), ..focus }
    }

    fn pin_handle(&mut self, pet_id: u32, cx: &mut App) -> FocusHandle {
        self.pins.entry(pet_id).or_insert_with(|| cx.focus_handle().tab_stop(true)).clone()
    }

    fn focus_control(&mut self, key: &str, window: &mut Window, cx: &mut App) {
        let handle = self.handle(key, cx);
        window.focus(&handle, cx);
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        match event.keystroke.key.as_str() {
            "tab" if !modifiers.control && !modifiers.alt && !modifiers.platform => {
                cx.stop_propagation();
                self.menu = None;
                if modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                cx.notify();
            }
            "escape" if self.menu.is_some() => {
                cx.stop_propagation();
                self.close_menu(true, window, cx);
            }
            _ => {}
        }
    }

    fn announce(&mut self, text: String) {
        self.announcement = Some(text.into());
    }

    // ---- The keyboard exception ----

    #[cfg(any(test, feature = "probes"))]
    fn keyboard_in_app(&self) -> bool {
        self.keyboard_in_app
    }

    #[cfg(not(any(test, feature = "probes")))]
    fn keyboard_in_app(&self) -> bool {
        false
    }

    /// Gives the search field the keyboard. Windows answers a foreground
    /// request only soon after a click on our window, so this is called from
    /// the click or key that opened the search; the window calls run on the
    /// next turn, not inside GPUI's event handling. If the search ended
    /// before the request was answered, the keyboard is given back at once.
    fn start_search(&mut self, from_keyboard: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        if self.edit.is_editing() {
            self.focus_search_field(window, cx);
            return;
        }
        if !self.edit.start() {
            return;
        }
        self.search_from_keyboard = from_keyboard;
        if self.keyboard_in_app() {
            self.edit.began(true);
            self.focus_search_field(window, cx);
            cx.notify();
            return;
        }
        let Some(hwnd) = self.hwnd else {
            self.edit.stop();
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            if !this.read_with(cx, |this, _| this.edit.wants_focus()).unwrap_or(false) {
                return;
            }
            let taken = typing::begin(hwnd);
            if let Err(error) = &taken {
                crate::app_log::warn(error.to_string());
            }
            let going_on = this
                .update_in(cx, |this, window, cx| {
                    let going_on = this.edit.began(taken.is_ok());
                    if going_on {
                        this.focus_search_field(window, cx);
                        cx.notify();
                    }
                    going_on
                })
                .unwrap_or(false);
            if taken.is_ok() && !going_on {
                typing::end(hwnd);
            }
        })
        .detach();
    }

    fn focus_search_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.search.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        // The press that started the search may still move the focus, so it
        // is given again once this frame is drawn.
        cx.on_next_frame(window, move |this, window, cx| {
            if this.edit.is_editing() {
                window.focus(&handle, cx);
            }
        });
    }

    /// Back to non-activating; what was typed stays. `refocus` puts the
    /// keyboard focus back on the search box when the search was opened from
    /// the keyboard. Nothing is given focus: Windows picks the window under
    /// ours.
    fn stop_search(&mut self, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.edit.stop();
        self.leave_timer = None;
        if held {
            if let Some(hwnd) = self.hwnd {
                cx.spawn(async move |_, _| typing::end(hwnd)).detach();
            }
        }
        if refocus {
            if self.search_from_keyboard {
                self.focus_control("search", window, cx);
            } else {
                window.focus(&self.root_focus, cx);
            }
        }
        cx.notify();
    }

    /// The window is closing or the app quitting: no later turn to wait for.
    fn finish_search(&mut self) {
        if let Some(hwnd) = self.hwnd.filter(|_| self.edit.stop()) {
            typing::end(hwnd);
        }
    }

    fn pointer_is_over(&self, window: &Window) -> bool {
        #[cfg(any(test, feature = "probes"))]
        if self.pointer_held_over {
            return true;
        }
        window.is_window_hovered()
    }

    /// Typing ends a moment after the pointer has left the window. GPUI
    /// redraws the window when the pointer enters or leaves it.
    fn watch_pointer(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.edit.is_editing() || self.pointer_is_over(window) {
            self.leave_timer = None;
            return;
        }
        if self.leave_timer.is_some() {
            return;
        }
        self.leave_timer = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(LEAVE_GRACE).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.leave_timer = None;
                if !this.pointer_is_over(window) {
                    this.stop_search(false, window, cx);
                }
            });
        }));
    }

    fn clear_search(&mut self, cx: &mut Context<Self>) {
        self.search.update(cx, |input, cx| input.clear(cx));
    }

    fn query(&self, cx: &App) -> String {
        self.search.read(cx).text().to_string()
    }

    // ---- Tabs ----

    fn select_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab == tab {
            return;
        }
        self.tab = tab;
        self.pointer_row = None;
        self.rows.scroll_to(Pixels::ZERO);
        cx.notify();
    }

    fn on_tab_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let at = Tab::ALL.iter().position(|tab| *tab == self.tab).unwrap_or(0);
        let last = Tab::ALL.len() - 1;
        let next = match event.keystroke.key.as_str() {
            "right" => (at + 1).min(last),
            "left" => at.saturating_sub(1),
            "home" => 0,
            "end" => last,
            _ => return,
        };
        cx.stop_propagation();
        self.select_tab(Tab::ALL[next], cx);
        self.focus_control(&format!("tab:{next}"), window, cx);
    }

    // ---- The Show and Order menu ----

    fn open_menu(&mut self, cx: &mut Context<Self>) {
        let choice = self.pets.read(cx).list_choice();
        self.menu = Some(MENU.iter().position(|item| item.is_current(choice)).unwrap_or(0));
        cx.notify();
    }

    fn close_menu(&mut self, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        if refocus {
            self.focus_control("filter", window, cx);
        }
        cx.notify();
    }

    fn pick(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = MENU.get(index) {
            let choice = item.chosen(self.pets.read(cx).list_choice());
            self.set_choice(choice, cx);
        }
        self.close_menu(true, window, cx);
    }

    fn set_choice(&mut self, choice: ListChoice, cx: &mut Context<Self>) {
        self.rows.scroll_to(Pixels::ZERO);
        self.pets.update(cx, |pets, cx| pets.set_list_choice(choice, cx));
    }

    fn on_menu_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let last = MENU.len() - 1;
        match (self.menu, event.keystroke.key.as_str()) {
            (None, "down" | "up" | "enter" | "space") => self.open_menu(cx),
            (Some(lit), "down") => self.menu = Some((lit + 1).min(last)),
            (Some(lit), "up") => self.menu = Some(lit.saturating_sub(1)),
            (Some(_), "home") => self.menu = Some(0),
            (Some(_), "end") => self.menu = Some(last),
            (Some(lit), "enter" | "space") => self.pick(lit, window, cx),
            (Some(_), "escape") => self.close_menu(true, window, cx),
            _ => return,
        }
        cx.stop_propagation();
        window.prevent_default();
        cx.notify();
    }

    // ---- Pins ----

    fn toggle_pin(&mut self, pet_id: u32, by_keyboard: bool, window: &mut Window, cx: &mut Context<Self>) {
        let pinned = self.pets.read(cx).is_pinned(pet_id);
        let changed = self.pets.update(cx, |pets, cx| if pinned { pets.unpin(pet_id, cx) } else { pets.pin(pet_id, cx) });
        if !changed {
            return;
        }
        let name = super::card::pet_name(pet_id);
        self.announce(if pinned { fl!("pets-unpinned-announce", name = name) } else { fl!("pets-pinned-announce", name = name) });
        self.pin_cursor = Some(pet_id);
        if by_keyboard {
            self.reveal_pet = Some(pet_id);
        } else {
            self.pointer_row = None;
            self.held_still = Some(window.mouse_position());
        }
        cx.notify();
    }

    /// Space and Enter pin on the key's press, as the filter menu and the
    /// toolbar act; the pin's keyboard click is ignored so it does not act
    /// twice. Up, Down, Home and End move along the pins.
    fn on_pin_key(&mut self, pet_id: u32, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() || self.drawn.is_empty() {
            return;
        }
        let at = self.drawn.iter().position(|drawn| *drawn == pet_id).unwrap_or(0);
        let last = self.drawn.len() - 1;
        let next = match event.keystroke.key.as_str() {
            "space" | "enter" => {
                cx.stop_propagation();
                window.prevent_default();
                self.toggle_pin(pet_id, true, window, cx);
                return;
            }
            "down" => (at + 1).min(last),
            "up" => at.saturating_sub(1),
            "home" => 0,
            "end" => last,
            _ => return,
        };
        cx.stop_propagation();
        window.prevent_default();
        let target = self.drawn[next];
        self.pin_cursor = Some(target);
        self.reveal_pet = Some(target);
        let handle = self.pin_handle(target, cx);
        window.focus(&handle, cx);
        cx.notify();
    }

    fn pointer_on_row(&mut self, pet_id: u32, hovered: bool, cx: &mut Context<Self>) {
        let now = if hovered {
            Some(pet_id)
        } else if self.pointer_row == Some(pet_id) {
            None
        } else {
            return;
        };
        if self.pointer_row != now {
            self.pointer_row = now;
            cx.notify();
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.held_still.is_some_and(|at| at != event.position) {
            self.held_still = None;
            cx.notify();
        }
    }

    // ---- Drawing ----

    fn source(&self, cx: &App) -> Source {
        let pets = self.pets.read(cx);
        let (Some(server_id), Some(at)) = (pets.current_server(), pets.collection().and_then(|collection| collection.snapshot_at)) else {
            return Source::NotRead;
        };
        let at_ms = (at * 1000.0) as i64;
        if pets.read_this_run() {
            Source::Live { time: clock_time(&LocalZone, at_ms).to_string(), server: game_data::server_name(&server_id.to_string()) }
        } else {
            Source::Earlier { stamp: day_stamp(&LocalZone, at_ms) }
        }
    }

    fn header(&mut self, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let close_focus = self.focus_of("close", window, cx);
        let close = focusable(
            chrome::tool_button("close", Icon::Close, fl!("common-close"), false, s).on_click(|_, window, _| window.remove_window()),
            &close_focus,
            7.,
            s,
        );
        chrome::header_closed_by(chrome::title(Icon::PawPrint, fl!("pets-window-title"), None, s), Vec::new(), close, &self.drag, s)
    }

    /// "From the game, live · logged in at 19:02 · Ossia": where the values
    /// come from, times and names brighter; an earlier login's line explains
    /// itself in a tooltip.
    fn source_line(&self, source: &Source, s: Scale) -> Stateful<Div> {
        let (text, strong, tip) = match source {
            Source::Live { time, server } => {
                (fl!("pets-source-live", time = time.as_str(), server = server.as_str()), vec![time.clone(), server.clone()], None)
            }
            Source::Earlier { stamp } => (
                fl!("pets-source-earlier", stamp = stamp.as_str()),
                vec![stamp.clone()],
                Some((fl!("pets-source-earlier-tip", stamp = stamp.as_str()), fl!("pets-source-earlier-tip-detail"))),
            ),
            Source::NotRead => (fl!("pets-source-none"), Vec::new(), None),
        };
        let line = div()
            .id("source-line")
            .debug_selector(|| "source-line".to_string())
            .role(Role::Status)
            .aria_label(SharedString::from(text.clone()))
            .flex_none()
            .h(s.px(20.))
            .mt(s.px(-2.))
            .mb(s.px(4.))
            .px(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgba(MUTED))
            .child(chrome::icon(Icon::Gamepad, 12., rgba(FAINT), s))
            .child(div().flex_1().min_w_0().truncate().child(highlighted(text, &strong)));
        match tip {
            Some((text, detail)) => line.tooltip(chrome::tooltip_with_detail(text.into(), Some(detail.into()), s)),
            None => line,
        }
    }

    /// The search field and the filter button; `off` draws them dimmed and
    /// inert, before any collection was read.
    fn find_row(&mut self, off: bool, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let query = self.query(cx);
        let typing = self.edit.is_editing();
        let row = div().flex_none().flex().items_center().gap(s.px(4.)).mb(s.px(6.));
        if off {
            let field = chrome::input_box(false, s)
                .flex_1()
                .min_w_0()
                .pl(s.px(8.))
                .child(chrome::icon(Icon::Search, 12., rgba(FAINT), s))
                .child(div().flex_1().text_color(rgba(FAINT)).font_weight(FontWeight::MEDIUM).child(fl!("pets-search")));
            let filter = div().flex_none().size(s.px(26.)).flex().items_center().justify_center().child(chrome::icon(Icon::ListFilter, 13., rgba(MUTED), s));
            return row.opacity(OFF_OPACITY).child(field).child(filter);
        }
        let search_focus = self.focus_of("search", window, cx);
        let bright = typing || search_focus.ring;
        let clear = (!query.is_empty()).then(|| {
            let group = SharedString::from("clear-search");
            div()
                .id("clear-search")
                .debug_selector(|| "clear-search".to_string())
                .group(group.clone())
                .role(Role::Button)
                .aria_label(fl!("pets-clear-search"))
                .flex_none()
                .size(s.px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(s.px(4.))
                .cursor_pointer()
                .hover(|button| button.bg(rgba(SOFT)))
                .child(chrome::icon(Icon::Close, 12., rgba(MUTED), s).group_hover(group, |icon| icon.text_color(rgba(TEXT))))
                // Clearing keeps the keyboard where it is.
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.clear_search(cx);
                }))
        });
        let field = chrome::input_box(bright, s)
            .flex_1()
            .min_w_0()
            .pl(s.px(8.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_text()
            .id("search")
            .debug_selector(|| "search".to_string())
            .role(Role::TextInput)
            .aria_label(fl!("pets-search"))
            .aria_value(SharedString::from(query.clone()))
            .track_focus(&search_focus.handle)
            .child(chrome::icon(Icon::Search, 12., rgba(FAINT), s))
            .child(self.search.clone())
            .children(clear)
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                this.start_search(matches!(event, ClickEvent::Keyboard(_)), window, cx);
            }));
        let choice = self.pets.read(cx).list_choice();
        let filtering = choice.filters();
        let open = self.menu.is_some();
        let lit = filtering || open;
        let filter_focus = self.focus_of("filter", window, cx);
        let group = SharedString::from("pets-filter");
        let filter = div()
            .id("filter")
            .debug_selector(|| "filter".to_string())
            .group(group.clone())
            .role(Role::Button)
            .aria_label(fl!("pets-filter"))
            .aria_expanded(open)
            .flex_none()
            .size(s.px(26.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(s.px(7.))
            .cursor_pointer()
            .when(lit, |button| button.bg(rgba(HOT)))
            .when(!lit, |button| button.hover(|button| button.bg(rgba(SOFT))))
            .child(self.menu_anchor.marker())
            .child(
                chrome::icon(Icon::ListFilter, 13., rgba(if lit { TEXT } else { MUTED }), s)
                    .when(filtering, |icon| icon.mt(s.px(-3.)))
                    .group_hover(group, |icon| icon.text_color(rgba(TEXT))),
            )
            .when(filtering, |button| {
                button.child(div().absolute().bottom(s.px(2.)).size(s.px(4.)).rounded_full().bg(rgba(TEXT)))
            })
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                if matches!(event, ClickEvent::Keyboard(_)) {
                    return;
                }
                if this.menu.is_some() {
                    this.close_menu(true, window, cx);
                } else {
                    this.open_menu(cx);
                }
            }))
            .on_key_down(cx.listener(Self::on_menu_key));
        row.child(field).child(focusable(filter, &filter_focus, 7., s))
    }

    fn tabs(&mut self, off: bool, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut tabs = chrome::tabs(s).mt_0().id("pet-tabs").role(Role::TabList);
        if off {
            for (index, tab) in Tab::ALL.into_iter().enumerate() {
                tabs = tabs.child(chrome::tab(("tab", index), tab_name(tab), None, tab == self.tab, s));
            }
            return tabs.opacity(OFF_OPACITY);
        }
        for (index, tab) in Tab::ALL.into_iter().enumerate() {
            let on = tab == self.tab;
            let focus = self.roving_focus(&format!("tab:{index}"), on, window, cx);
            let element = chrome::tab(("tab", index), tab_name(tab), None, on, s)
                .debug_selector(move || format!("tab-{index}"))
                .role(Role::Tab)
                .aria_label(tab_name(tab))
                .aria_selected(on)
                .on_click(cx.listener(move |this, _, _, cx| this.select_tab(tab, cx)))
                .on_key_down(cx.listener(Self::on_tab_key));
            tabs = tabs.child(focusable(element, &focus, 6., s).flex_1());
        }
        tabs
    }

    /// "61 of 92 owned · 25 at Lv 3", "Levels 132/276" and the levels bar:
    /// the whole tab, whatever the search and the menu keep.
    fn summary(&self, summary: super::list::Summary, s: Scale) -> Div {
        let owned = summary.owned.to_string();
        let maxed = summary.maxed.to_string();
        let owned_text = fl!("pets-owned", owned = summary.owned, total = summary.pets);
        let maxed_text = fl!("pets-maxed", count = summary.maxed);
        let line = div()
            .flex_none()
            .h(s.px(18.))
            .px(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgba(MUTED))
            .child(highlighted(owned_text, &[owned]))
            .child(div().text_color(rgba(FAINT)).child("\u{b7}"))
            .child(highlighted(maxed_text, &[maxed]))
            .child(div().flex_1())
            .child(fl!("pets-levels"))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .child(div().text_color(rgba(TEXT)).font_weight(FontWeight::SEMIBOLD).child(summary.levels.to_string()))
                    .child(div().text_size(s.px(11.)).text_color(rgba(FAINT)).child(format!("/{}", summary.levels_possible))),
            );
        let fraction = if summary.levels_possible == 0 { 0. } else { summary.levels as f32 / summary.levels_possible as f32 };
        let bar = div()
            .id("levels-bar")
            .role(Role::ProgressIndicator)
            .aria_label(fl!("pets-levels"))
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(f64::from(summary.levels_possible))
            .aria_numeric_value(f64::from(summary.levels))
            .flex_none()
            .h(s.px(4.))
            .mt(s.px(3.))
            .mx(s.px(4.))
            .mb(s.px(6.))
            .rounded(s.px(2.))
            .bg(rgba(GAUGE_EMPTY))
            .overflow_hidden()
            .child(div().h_full().w(relative(fraction)).rounded(s.px(2.)).bg(rgba(PROGRESS_FILL)));
        div().flex_none().flex().flex_col().child(line).child(bar)
    }

    /// While the menu filters or orders: what it does, how many pets are
    /// shown, and "Clear".
    fn filter_line(&mut self, choice: ListChoice, shown: usize, s: Scale, window: &Window, cx: &mut Context<Self>) -> Option<Div> {
        if !choice.filters() {
            return None;
        }
        let mut parts = Vec::new();
        match choice.show {
            Show::All => {}
            Show::NotOwned => parts.push(fl!("pets-show-not-owned")),
            Show::NotMaxed => parts.push(fl!("pets-show-not-maxed")),
        }
        if choice.order == Order::ClosestToNextLevel {
            parts.push(fl!("pets-closest-first"));
        }
        let count = fl!("pets-shown", count = shown);
        parts.push(count.clone());
        let text = parts.join(" \u{b7} ");
        let dim = HighlightStyle { color: Some(rgba(FAINT).into()), ..Default::default() };
        let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> =
            text.match_indices('\u{b7}').map(|(start, dot)| (start..start + dot.len(), dim)).collect();
        highlights.push((text.len() - count.len()..text.len(), dim));
        let focus = self.focus_of("clear-filter", window, cx);
        let clear = focusable(
            link("clear-filter", fl!("pets-clear"), s)
                .role(Role::Button)
                .aria_label(fl!("pets-clear-name"))
                .on_click(cx.listener(|this, _, _, cx| this.set_choice(ListChoice::default(), cx))),
            &focus,
            4.,
            s,
        );
        Some(
            div()
                .flex_none()
                .h(s.px(20.))
                .mt(s.px(-2.))
                .mb(s.px(4.))
                .px(s.px(4.))
                .flex()
                .items_center()
                .gap(s.px(6.))
                .text_size(s.px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(MUTED))
                .child(chrome::icon(Icon::ListFilter, 12., rgba(FAINT), s))
                .child(div().flex_1().min_w_0().truncate().child(StyledText::new(text).with_highlights(highlights)))
                .child(clear),
        )
    }

    fn footer(&self, s: Scale) -> Div {
        let mut footer = chrome::footer(s)
            .gap(s.px(5.))
            .child(chrome::icon(Icon::Info, 12., rgba(FAINT), s))
            .child(fl!("pets-souls-per-level"));
        for (index, souls) in SOULS_PER_LEVEL.iter().enumerate() {
            if index > 0 {
                footer = footer.child(div().text_color(rgba(FAINT)).child("\u{b7}"));
            }
            footer = footer.child(div().text_color(rgba(TEXT)).font_weight(FontWeight::SEMIBOLD).child(souls.to_string()));
        }
        footer.child(div().flex_1()).child(div().flex_none().text_color(rgba(FAINT)).child(fl!("pets-cap")))
    }

    /// The open menu under the filter button, right edges aligned, and the
    /// surface under it that closes it on a press anywhere else.
    fn menu_layer(&self, s: Scale, cx: &mut Context<Self>) -> Option<Vec<AnyElement>> {
        let lit = self.menu?;
        let bounds = self.menu_anchor.bounds()?;
        let choice = self.pets.read(cx).list_choice();
        let label = |text: String| {
            div().flex_none().h(s.px(22.)).px(s.px(8.)).flex().items_center().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(text)
        };
        let mut menu = chrome::popover(s)
            .id("pets-menu")
            .debug_selector(|| "pets-menu".to_string())
            .role(Role::Menu)
            .aria_label(fl!("pets-filter"))
            .top(bounds.bottom() + s.px(6.))
            .left(bounds.right() - s.px(MENU_WIDTH))
            .w(s.px(MENU_WIDTH))
            .font_weight(FontWeight::MEDIUM)
            .child(label(fl!("pets-show")));
        for (index, item) in MENU.into_iter().enumerate() {
            if index == 3 {
                menu = menu.child(div().h(px(1.)).mx(s.px(4.)).my(s.px(4.)).bg(rgba(DIVIDER))).child(label(fl!("pets-order")));
            }
            let current = item.is_current(choice);
            menu = menu.child(
                chrome::menu_item(("pets-menu-item", index), index == lit, s)
                    .gap(s.px(6.))
                    .debug_selector(move || format!("menu-{index}"))
                    .role(Role::MenuItemRadio)
                    .aria_label(item.label())
                    .aria_toggled(if current { Toggled::True } else { Toggled::False })
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(cx.listener(move |this, _, window, cx| this.pick(index, window, cx)))
                    .child(div().flex_none().w(s.px(12.)).flex().items_center().children(current.then(|| chrome::icon(Icon::Check, 12., rgba(TEXT), s))))
                    .child(div().when(current, |text| text.font_weight(FontWeight::SEMIBOLD)).child(item.label())),
            );
        }
        let scrim = chrome::scrim("pets-menu-scrim").on_any_mouse_down(cx.listener(|this, _, window, cx| this.close_menu(false, window, cx)));
        Some(vec![scrim.into_any_element(), menu.into_any_element()])
    }
}

impl Render for PetsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch_pointer(window, cx);
        let look = Look::of(&self.overlay);
        let s = look.s;
        self.drawn.clear();
        let source = self.source(cx);
        let header = self.header(s, window, cx);
        let card = chrome::card(WIDTH, look)
            .id("pets")
            .debug_selector(|| "pets".to_string())
            .role(Role::Dialog)
            .aria_label(fl!("pets-window-title"))
            .child(header)
            .child(self.source_line(&source, s));
        let card = if matches!(source, Source::NotRead) {
            let find = self.find_row(true, s, window, cx);
            let tabs = self.tabs(true, s, window, cx);
            card.child(find)
                .child(tabs)
                .child(chrome::empty(Some(fl!("pets-empty-title").into()), fl!("pets-empty-line"), s))
                .child(self.footer(s))
        } else {
            let find = self.find_row(false, s, window, cx);
            let tabs = self.tabs(false, s, window, cx);
            let choice = self.pets.read(cx).list_choice();
            let query = self.query(cx);
            let listing = {
                let pets = self.pets.read(cx);
                let empty = Default::default();
                let collection = pets.collection().unwrap_or(&empty);
                super::list::listing(collection, pets.pinned(), self.tab, &query, choice, |pet_id| fl!("pets-unknown", id = pet_id))
            };
            let summary = self.summary(listing.summary, s);
            let filter = self.filter_line(choice, listing.shown(), s, window, cx);
            let body = if listing.shown() > 0 {
                self.list_body(listing, s, cx).into_any_element()
            } else {
                self.no_match(&query, s, window, cx).into_any_element()
            };
            card.child(find).child(tabs).child(summary).children(filter).child(body).child(self.footer(s))
        };
        let menu = self.menu_layer(s, cx);
        let announcement = self.announcement.clone().map(|text| div().id("announcement").role(Role::Status).aria_label(text).absolute().top_0().left_0().size_0());
        chrome::root(look)
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(card)
            .children(menu.into_iter().flatten())
            .children(announcement)
    }
}

/// `text` with each of `parts`, in order, in `text` / 600.
fn highlighted(text: String, parts: &[String]) -> StyledText {
    let bright = HighlightStyle { color: Some(rgba(TEXT).into()), font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() };
    let dim = HighlightStyle { color: Some(rgba(FAINT).into()), ..Default::default() };
    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
    let mut from = 0;
    for part in parts.iter().filter(|part| !part.is_empty()) {
        if let Some(start) = text[from..].find(part.as_str()).map(|at| from + at) {
            highlights.push((start..start + part.len(), bright));
            from = start + part.len();
        }
    }
    highlights.extend(text.match_indices('\u{b7}').map(|(start, dot)| (start..start + dot.len(), dim)));
    highlights.sort_by_key(|(range, _)| range.start);
    StyledText::new(text).with_highlights(highlights)
}

/// Makes `element` the focus target of `focus`, with its ring: 2 px, 1 px
/// outside the control.
fn focusable(element: Stateful<Div>, focus: &Focus, radius: f32, s: Scale) -> Stateful<Div> {
    element
        .relative()
        .track_focus(&focus.handle)
        .when(focus.ring, |element| {
            element.child(
                div()
                    .debug_selector(|| "focus-ring".to_string())
                    .absolute()
                    .top(px(-3.))
                    .left(px(-3.))
                    .right(px(-3.))
                    .bottom(px(-3.))
                    .rounded(s.px(radius) + px(3.))
                    .border_2()
                    .border_color(rgba(FOCUS_RING)),
            )
        })
}

/// "Clear", "Search all families": `muted` / 600, underlined.
fn link(key: &'static str, text: impl Into<SharedString>, s: Scale) -> Stateful<Div> {
    div()
        .id(key)
        .debug_selector(move || key.to_string())
        .flex_none()
        .cursor_pointer()
        .text_color(rgba(MUTED))
        .font_weight(FontWeight::SEMIBOLD)
        .underline()
        .text_decoration_color(rgba(LINK_UNDERLINE))
        .hover(|link| link.text_color(rgba(TEXT)))
        .text_size(s.px(11.5))
        .child(text.into())
}

#[cfg(feature = "probes")]
impl PetsView {
    /// Lets the search take the keyboard inside GPUI only, and holds the
    /// pointer over the window, so a capture never asks Windows for the
    /// foreground.
    pub fn keep_keyboard_in_app(&mut self) {
        self.keyboard_in_app = true;
        self.pointer_held_over = true;
    }

    /// Puts the pointer in the row of `pet_id`, as its enter would; none
    /// takes it out.
    pub fn force_pointer_row(&mut self, pet_id: Option<u32>, cx: &mut Context<Self>) {
        self.pointer_row = pet_id;
        self.held_still = None;
        cx.notify();
    }

    pub fn show_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.select_tab(tab, cx);
    }

    pub fn show_menu(&mut self, cx: &mut Context<Self>) {
        self.open_menu(cx);
    }

    pub fn close_the_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_menu(false, window, cx);
    }

    /// The search field takes the keyboard as a click on it would; the
    /// capture then posts what it types.
    pub fn begin_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.start_search(false, window, cx);
    }

    /// Where the row of `pet_id` was last drawn, in window coordinates.
    pub fn row_bounds(&self, pet_id: u32) -> Option<gpui::Bounds<Pixels>> {
        self.row_marks.borrow().get(&pet_id).and_then(Anchor::bounds)
    }

    /// The window's view of the pointer moving: what a posted move did.
    pub fn held_still(&self) -> bool {
        self.held_still.is_some()
    }

    /// The list as the scrollbar check reads it.
    pub fn list_reading(&self) -> crate::capture_secondary::ListReading {
        let built = self.built.borrow();
        crate::capture_secondary::ListReading {
            viewport: self.rows.viewport(),
            max: f32::from(self.rows.max()),
            scrolled: f32::from(self.rows.scrolled()),
            first_built: self.drawn.first().is_some_and(|pet_id| built.contains(pet_id)),
            last_built: self.drawn.last().is_some_and(|pet_id| built.contains(pet_id)),
        }
    }
}

#[cfg(any(test, feature = "probes"))]
impl PetsView {
    pub fn is_typing(&self) -> bool {
        self.edit.is_editing()
    }

    pub fn focused_control(&self, window: &Window) -> Option<String> {
        self.controls
            .iter()
            .find(|(_, handle)| handle.is_focused(window))
            .map(|(key, _)| key.clone())
            .or_else(|| self.pins.iter().find(|(_, handle)| handle.is_focused(window)).map(|(pet_id, _)| format!("pin:{pet_id}")))
    }

    /// The pet whose row shows its pin because the pointer is in it.
    pub fn revealed_row(&self) -> Option<u32> {
        self.pointer_row.filter(|_| self.held_still.is_none())
    }
}

#[cfg(test)]
impl PetsView {
    pub fn menu_is_open(&self) -> bool {
        self.menu.is_some()
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }
}
