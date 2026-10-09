//! The to-do window (docs/gpui-mockups/approved/todos.html, values in
//! tokens.md, "To-do window"): one character's to-dos in two sections, Today
//! and This week, with the character picker above and the reset hour below.
//! What it shows comes from the `Todos` entity, and every action goes back to
//! it with the character the window showed. The Resources window
//! (resources.html, tokens.md "Resources window") is the same view listing
//! the resources instead, so both windows follow the same character.
//!
//! A line under the picker says where the values come from; a value the game
//! gave has no arrows, a click on it types over it, and its tooltip says when
//! the game said it.
//!
//! The window never activates, like the other overlays, except while a text
//! field takes the keyboard (an amount being typed, the name of a new to-do),
//! as the PvP name field does; see `secondary::typing`. Enter, Escape, a click
//! outside the field or the pointer leaving the window gives the keyboard
//! back to the game.
//!
//! A row's check and × appear when the pointer enters the row and go when it
//! leaves; moves inside the row change nothing.

mod form;
mod rows;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    div, prelude::*, px, relative, rgb, rgba, AnyElement, App, ClickEvent, Context, Div, ElementId, Entity, Focusable as _,
    FocusHandle, FontWeight, HighlightStyle, Hsla, KeyDownEvent, MouseMoveEvent, Pixels, Point, Render, Role, ScrollHandle,
    SharedString, Stateful, StyledText, Subscription, Task, Window,
};
use windows::Win32::Foundation::HWND;

use super::copy;
use super::{global, Cadence, CharacterId, RemovedOwnTodo, RowId, RowKind, Section, Source, SourceLine, TodoList, Todos};
use crate::assets::Icon;
use crate::i18n::{self, fl};
use crate::meter::{MeterModel, OverlaySettings};
use crate::overlay;
use crate::secondary::chrome::{self, Anchor, Look, Scale, FAINT, GOOD, HOT, MUTED, SOFT, TEXT, WARN};
use crate::secondary::text_input::{TextInput, TextInputEvent};
use crate::secondary::typing::{self, Edit};
use crate::secondary;
use crate::settings::{self, reveal_marker, Reveal, Section as SettingsSection};

const WIDTH: f32 = 370.0;
/// The list scrolls past this height, so the to-do window is at most 580 px
/// tall at 100% (612 while a note shows).
const LIST_MAX: f32 = 452.0;
const NOTE_FOR: Duration = Duration::from_secs(4);
/// How long the pointer may be away before typing ends.
const LEAVE_GRACE: Duration = Duration::from_millis(400);
/// The amount field takes this many digits; the largest cap is 840.
const AMOUNT_DIGITS: usize = 4;

const FULL_FILL: u32 = 0xf2c66d1f;
const PRESS: u32 = 0xfff0e133;
const CHIP_WARN: u32 = 0xf2c66d21;
const NOTE_GOOD: u32 = 0x86e3a81f;
const FOCUS_RING: u32 = 0xf3ebe1e6;
const DIVIDER: u32 = 0xfff0e12e;
const LINK_UNDERLINE: u32 = 0xfff6ec40;
const FIELD_BORDER: u32 = 0xfff6ec8c;
const FIGURE_RIM: u32 = 0xfff0e11f;
const WARN_SOFT: u32 = 0xf2c66db3;
const FORM_PLATE: u32 = 0xfff0e10d;
const SEGMENT_TRACK: u32 = 0x0000004d;
const TRACK: u32 = crate::style::TRACK;
const HIDDEN_OPACITY: f32 = 0.42;
const DISABLED_OPACITY: f32 = 0.45;

/// What the window lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contents {
    /// The to-do window: Today, This week and your own to-dos.
    Todos,
    /// The Resources window: every resource, nothing to hide or add.
    Resources,
}

/// What the field types: an amount, or how much of a row the game fills is
/// done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typing {
    Amount,
    Progress,
}

/// A figure being typed, for the character and row the window showed.
struct AmountEdit {
    character: CharacterId,
    row: &'static str,
    typing: Typing,
    /// What the field opened with; Enter on it unchanged keeps the amount's
    /// refill timing.
    opened_with: Option<u32>,
    from_keyboard: bool,
}

/// The "+ Add a to-do" form, for the character it was opened on.
struct AddForm {
    character: CharacterId,
    counted: bool,
    count: u32,
    cadence: Cadence,
}

enum Note {
    Removed(RemovedOwnTodo),
    Added { name: String, section: Section },
    /// The game listed the shown character's tickets at login; `replaced`:
    /// values typed there were dropped.
    Login { replaced: bool },
}

/// A row in the order the window draws it, for moving the keyboard on when
/// one leaves.
struct Drawn {
    id: RowId,
    focus_key: String,
}

/// A control's keyboard focus as the view sees it this frame.
#[derive(Clone)]
struct Focus {
    handle: FocusHandle,
    /// Focused, and the last input came from the keyboard.
    ring: bool,
    reveal: Option<Reveal>,
}

pub struct TodoView {
    contents: Contents,
    todos: Entity<Todos>,
    meter_model: Entity<MeterModel>,
    overlay: OverlaySettings,
    /// The window's handle, kept so the keyboard can be given back even when
    /// the window is going away.
    hwnd: Option<HWND>,
    edit: Edit,
    name_input: Entity<TextInput>,
    amount_input: Entity<TextInput>,
    amount_group: FocusHandle,
    form_group: FocusHandle,
    amount_edit: Option<AmountEdit>,
    adding: Option<AddForm>,
    listing_hidden: HashSet<Section>,
    /// The row the pointer is in, from its enter and leave only.
    pointer_row: Option<RowId>,
    /// After a row left under the pointer, the row now there keeps its check
    /// and × hidden until the pointer moves from here.
    held_still: Option<Point<Pixels>>,
    /// The picker is open with this option lit.
    picker: Option<usize>,
    picker_anchor: Anchor,
    /// Where the open amount field and add form sit, so a press outside
    /// them takes their focus away.
    amount_mark: Anchor,
    form_mark: Anchor,
    note: Option<Note>,
    note_timer: Option<Task<()>>,
    announcement: Option<SharedString>,
    shown_character: Option<CharacterId>,
    /// The shown character and its last login the window knows of: a newer
    /// login while it is shown brings the login note.
    login_seen: (Option<CharacterId>, Option<i64>),
    /// The last list drawn had a value the player typed.
    typed_shown: bool,
    drawn: Vec<Drawn>,
    controls: HashMap<String, FocusHandle>,
    root_focus: FocusHandle,
    list_scroll: ScrollHandle,
    list_size: Anchor,
    reveal: Rc<Cell<bool>>,
    drag: overlay::WindowDrag,
    leave_timer: Option<Task<()>>,
    /// Tests and dev captures give a field the keyboard inside GPUI only:
    /// neither may ask Windows for the foreground.
    #[cfg(any(test, feature = "probes"))]
    keyboard_in_app: bool,
    /// A capture's window is off screen, so no pointer is ever over it.
    #[cfg(any(test, feature = "probes"))]
    pointer_held_over: bool,
    /// Where each row was drawn, for captures that post pointer messages to
    /// rows.
    #[cfg(feature = "probes")]
    row_marks: std::cell::RefCell<HashMap<String, Anchor>>,
    _subscriptions: Vec<Subscription>,
}

impl TodoView {
    pub fn new(contents: Contents, meter_model: Entity<MeterModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let todos = global(cx);
        let store = settings::global(cx);
        let overlay = store.read(cx).get().overlay.clone();
        let name_input = cx.new(|cx| TextInput::new(fl!("todos-window-name"), cx).with_max_chars(super::MAX_OWN_NAME_CHARS));
        let amount_input = cx.new(|cx| TextInput::new("", cx).with_max_chars(AMOUNT_DIGITS).digits_only().right_aligned());
        let amount_group = cx.focus_handle();
        let form_group = cx.focus_handle();
        let root_focus = cx.focus_handle();
        let shown_character = todos.read(cx).selected();
        let login_seen = (shown_character, shown_character.and_then(|id| todos.read(cx).logged_in_ms(id)));
        let locale = i18n::locale(cx);
        let subscriptions = vec![
            cx.observe(&locale, |this, _, cx| {
                this.name_input.update(cx, |input, cx| input.set_placeholder(fl!("todos-window-name"), cx))
            }),
            cx.observe_in(&todos, window, |this, todos, window, cx| {
                let selected = todos.read(cx).selected();
                this.follow_character(selected, window, cx);
                let logged_in_ms = selected.and_then(|id| todos.read(cx).logged_in_ms(id));
                this.note_login(selected, logged_in_ms, cx);
                cx.notify();
            }),
            cx.observe(&store, |this, store, cx| {
                let overlay = &store.read(cx).get().overlay;
                if *overlay != this.overlay {
                    this.overlay = overlay.clone();
                    cx.notify();
                }
            }),
            cx.observe(&name_input, |_, _, cx| cx.notify()),
            cx.observe(&amount_input, |_, _, cx| cx.notify()),
            cx.subscribe_in(&name_input, window, |this, _, event: &TextInputEvent, window, cx| match event {
                TextInputEvent::Submitted(_) => this.submit_form(window, cx),
                TextInputEvent::Cancelled => this.close_form(true, window, cx),
            }),
            cx.subscribe_in(&amount_input, window, |this, _, event: &TextInputEvent, window, cx| match event {
                TextInputEvent::Submitted(_) => this.close_amount(true, true, window, cx),
                TextInputEvent::Cancelled => this.close_amount(false, true, window, cx),
            }),
            cx.on_focus_out(&amount_group, window, |this, _, window, cx| this.close_amount(true, false, window, cx)),
            cx.on_focus_out(&form_group, window, |this, _, window, cx| this.left_form(window, cx)),
            // A focused control that goes away (a row hidden, a field
            // closed) leaves the focus with the window, so Tab keeps working.
            cx.on_focus_lost(window, |this, window, cx| window.focus(&this.root_focus, cx)),
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.stop_typing(window, cx);
                }
            }),
        ];
        let hwnd = overlay::hwnd(window).map_err(|error| eprintln!("the to-do window cannot take the keyboard: {error}")).ok();
        // Closing the window or quitting mid-edit keeps the amount typed and
        // gives the keyboard back.
        cx.on_release(|this, cx| this.finish_typing(cx)).detach();
        cx.on_app_quit(|this, cx| {
            this.finish_typing(cx);
            this.todos.update(cx, |todos, _| todos.flush());
            async {}
        })
        .detach();
        window.focus(&root_focus, cx);
        Self {
            contents,
            todos,
            meter_model,
            overlay,
            #[cfg(any(test, feature = "probes"))]
            keyboard_in_app: hwnd.is_none(),
            hwnd,
            edit: Edit::Idle,
            name_input,
            amount_input,
            amount_group,
            form_group,
            amount_edit: None,
            adding: None,
            listing_hidden: HashSet::new(),
            pointer_row: None,
            held_still: None,
            picker: None,
            picker_anchor: Anchor::default(),
            amount_mark: Anchor::default(),
            form_mark: Anchor::default(),
            note: None,
            note_timer: None,
            announcement: None,
            shown_character,
            login_seen,
            typed_shown: false,
            drawn: Vec::new(),
            controls: HashMap::new(),
            root_focus,
            list_scroll: ScrollHandle::new(),
            list_size: Anchor::default(),
            reveal: Rc::new(Cell::new(false)),
            drag: overlay::WindowDrag::default(),
            leave_timer: None,
            #[cfg(any(test, feature = "probes"))]
            pointer_held_over: false,
            #[cfg(feature = "probes")]
            row_marks: std::cell::RefCell::default(),
            _subscriptions: subscriptions,
        }
    }

    /// Another character's list: what was open for the last one closes, an
    /// amount being typed is kept for the character it was typed for.
    fn follow_character(&mut self, selected: Option<CharacterId>, window: &mut Window, cx: &mut Context<Self>) {
        if selected == self.shown_character {
            return;
        }
        self.shown_character = selected;
        self.close_amount(true, false, window, cx);
        self.close_form(false, window, cx);
        self.listing_hidden.clear();
        self.pointer_row = None;
        self.picker = None;
    }

    /// The game listed the shown character's tickets again since the window
    /// last looked: what it filled, and whether values typed were replaced.
    fn note_login(&mut self, shown: Option<CharacterId>, logged_in_ms: Option<i64>, cx: &mut Context<Self>) {
        let (seen_character, seen_login) = std::mem::replace(&mut self.login_seen, (shown, logged_in_ms));
        if shown.is_some() && seen_character == shown && logged_in_ms > seen_login {
            self.show_note(Note::Login { replaced: self.typed_shown }, cx);
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

    /// `in_list`: the control scrolls with the list, which brings it into
    /// view when the keyboard moves the focus to it.
    fn focus_of(&mut self, key: &str, in_list: bool, window: &Window, cx: &mut App) -> Focus {
        let handle = self.handle(key, cx);
        let focused = handle.is_focused(window);
        Focus {
            ring: focused && window.last_input_was_keyboard(),
            reveal: (focused && in_list).then(|| Reveal { scroll: self.list_scroll.clone(), pending: self.reveal.clone() }),
            handle,
        }
    }

    fn focus_control(&mut self, key: &str, window: &mut Window, cx: &mut App) {
        let handle = self.handle(key, cx);
        window.focus(&handle, cx);
    }

    fn is_focused(&self, key: &str, window: &Window) -> bool {
        self.controls.get(key).is_some_and(|handle| handle.is_focused(window))
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        match event.keystroke.key.as_str() {
            "tab" if !modifiers.control && !modifiers.alt && !modifiers.platform => {
                cx.stop_propagation();
                self.picker = None;
                if modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                self.reveal.set(true);
                cx.notify();
            }
            "escape" if self.picker.is_some() => {
                cx.stop_propagation();
                self.close_picker(true, window, cx);
            }
            _ => {}
        }
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

    /// Gives the open field the keyboard. Windows answers a foreground
    /// request only soon after a click on our window, so this is called from
    /// the click or key that opened the field; the window calls run on the
    /// next turn, not inside GPUI's event handling. If the field closed
    /// before the request was answered, the keyboard is given back at once.
    fn take_keyboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.edit.is_editing() {
            self.focus_field(window, cx);
            return;
        }
        if !self.edit.start() {
            return;
        }
        if self.keyboard_in_app() {
            self.edit.began(true);
            self.focus_field(window, cx);
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
                eprintln!("{error}");
            }
            let going_on = this
                .update_in(cx, |this, window, cx| {
                    let going_on = this.edit.began(taken.is_ok());
                    if going_on {
                        this.focus_field(window, cx);
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

    fn focus_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = if self.amount_edit.is_some() {
            &self.amount_input
        } else if self.adding.is_some() {
            &self.name_input
        } else {
            return;
        };
        let handle = input.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        // A field opened by this event is not drawn yet; the press that
        // opened it may still move the focus, so it is given again once the
        // field is on screen.
        cx.on_next_frame(window, move |this, window, cx| {
            if this.edit.is_editing() && (this.amount_edit.is_some() || this.adding.is_some()) {
                window.focus(&handle, cx);
            }
        });
    }

    /// Back to non-activating once no field wants the keyboard. Nothing is
    /// given focus: Windows picks the window under ours.
    fn release_keyboard_if_unused(&mut self, window: &Window, cx: &mut Context<Self>) {
        let wanted = self.amount_edit.is_some() || (self.adding.is_some() && self.form_group.contains_focused(window, cx));
        if wanted || !self.edit.stop() {
            return;
        }
        self.leave_timer = None;
        if let Some(hwnd) = self.hwnd {
            cx.spawn(async move |_, _| typing::end(hwnd)).detach();
        }
    }

    /// The pointer left the window, or another window was activated: what
    /// was typed is kept and the keyboard goes back.
    fn stop_typing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_amount(true, false, window, cx);
        if self.adding.is_some() && self.form_group.contains_focused(window, cx) {
            window.focus(&self.root_focus, cx);
            self.left_form(window, cx);
        }
        self.release_keyboard_if_unused(window, cx);
    }

    /// The window is closing or the app quitting: no later turn to wait for.
    fn finish_typing(&mut self, cx: &mut App) {
        if let Some(edit) = self.amount_edit.take() {
            let typed = self.amount_input.read(cx).text().to_string();
            commit_amount(&self.todos, &edit, &typed, cx);
        }
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
                    this.stop_typing(window, cx);
                }
            });
        }));
    }

    // ---- Amounts ----

    fn open_amount(&mut self, row: &'static str, typing: Typing, tracked: Option<u32>, from_keyboard: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(character) = self.shown_character else {
            return;
        };
        self.close_amount(true, false, window, cx);
        if self.adding.is_some() && self.form_is_blank(cx) {
            self.close_form(false, window, cx);
        }
        self.amount_edit = Some(AmountEdit { character, row, typing, opened_with: tracked, from_keyboard });
        self.amount_input.update(cx, |input, cx| match tracked {
            Some(value) => input.set_text_selected(&value.to_string(), cx),
            None => input.clear(cx),
        });
        self.take_keyboard(window, cx);
        cx.notify();
    }

    /// Ends the amount being typed: `keep` saves what the field holds (an
    /// empty field stops tracking); `refocus` puts the keyboard focus back on
    /// the figure when the field was opened from the keyboard.
    fn close_amount(&mut self, keep: bool, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.amount_edit.take() else {
            return;
        };
        if keep {
            let typed = self.amount_input.read(cx).text().to_string();
            commit_amount(&self.todos, &edit, &typed, cx);
        }
        self.amount_input.update(cx, |input, cx| input.clear(cx));
        if refocus && edit.from_keyboard {
            self.focus_control(&format!("fig:{}", edit.row), window, cx);
        }
        self.release_keyboard_if_unused(window, cx);
        cx.notify();
    }

    /// The field's ×: what the player typed goes, so the row stops being
    /// tracked, or shows the game's value again where the game gave one.
    fn clear_amount(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.amount_edit.as_ref() else {
            return;
        };
        let (character, row, typing) = (edit.character, edit.row, edit.typing);
        set_typed(&self.todos, character, row, typing, None, cx);
        self.close_amount(false, true, window, cx);
    }

    // ---- Adding ----

    fn open_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(character) = self.shown_character else {
            return;
        };
        self.close_amount(true, false, window, cx);
        if self.adding.is_none() {
            self.adding = Some(AddForm { character, counted: false, count: 3, cadence: Cadence::Daily });
            self.name_input.update(cx, |input, cx| input.clear(cx));
        }
        self.take_keyboard(window, cx);
        cx.notify();
    }

    fn form_is_blank(&self, cx: &App) -> bool {
        self.name_input.read(cx).text().trim().is_empty()
    }

    fn close_form(&mut self, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding.take().is_none() {
            return;
        }
        self.name_input.update(cx, |input, cx| input.clear(cx));
        if refocus {
            self.focus_control("add-line", window, cx);
        }
        self.release_keyboard_if_unused(window, cx);
        cx.notify();
    }

    /// The focus left the form: the keyboard goes back, and a form with no
    /// name closes.
    fn left_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding.is_none() {
            return;
        }
        if self.form_is_blank(cx) {
            self.close_form(false, window, cx);
        } else {
            self.release_keyboard_if_unused(window, cx);
            cx.notify();
        }
    }

    fn submit_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.adding.as_ref() else {
            return;
        };
        let name = self.name_input.read(cx).text().trim().to_string();
        if name.is_empty() {
            return;
        }
        let request = super::NewOwnTodo {
            name: name.clone(),
            kind: if form.counted { super::OwnKind::Count(form.count) } else { super::OwnKind::Tick },
            cadence: form.cadence,
        };
        let (character, section) = (form.character, form.cadence.section());
        match self.todos.update(cx, |todos, cx| todos.add_own(character, request, cx)) {
            Ok(_) => {
                let shown: String = name.chars().take(super::MAX_OWN_NAME_CHARS).collect();
                self.show_note(Note::Added { name: shown, section }, cx);
            }
            Err(error) => eprintln!("the to-do \"{name}\" was not added: {error:?}"),
        }
        self.close_form(true, window, cx);
    }

    // ---- Rows ----

    fn pointer_on_row(&mut self, id: RowId, hovered: bool, cx: &mut Context<Self>) {
        let now = if hovered {
            Some(id)
        } else if self.pointer_row == Some(id) {
            None
        } else {
            return;
        };
        if self.pointer_row != now {
            self.pointer_row = now;
            cx.notify();
        }
    }

    /// A press outside the open field takes the focus from it, as in a web
    /// page: an amount typed is kept, a form with no name closes, and the
    /// keyboard goes back. A press on another control has moved the focus
    /// already.
    fn on_mouse_down(&mut self, event: &gpui::MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let outside = |anchor: &Anchor| !anchor.bounds().is_some_and(|bounds| bounds.contains(&event.position));
        if self.amount_edit.is_some() && outside(&self.amount_mark) {
            if self.amount_group.contains_focused(window, cx) {
                window.focus(&self.root_focus, cx);
            }
            self.close_amount(true, false, window, cx);
        }
        if self.adding.is_some() && outside(&self.form_mark) {
            if self.form_group.contains_focused(window, cx) {
                window.focus(&self.root_focus, cx);
            }
            self.left_form(window, cx);
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.held_still.is_some_and(|at| at != event.position) {
            self.held_still = None;
            cx.notify();
        }
    }

    /// The row the keyboard goes to when `id` leaves the list: the next one,
    /// else the one before.
    fn neighbour_of(&self, id: RowId) -> Option<String> {
        let at = self.drawn.iter().position(|drawn| drawn.id == id)?;
        self.drawn.get(at + 1).or_else(|| at.checked_sub(1).and_then(|before| self.drawn.get(before))).map(|drawn| drawn.focus_key.clone())
    }

    /// The row's ×: a built-in hides, your own is removed with an Undo note.
    fn take_away(&mut self, character: CharacterId, id: RowId, label: String, by_keyboard: bool, window: &mut Window, cx: &mut Context<Self>) {
        let next = by_keyboard.then(|| self.neighbour_of(id)).flatten();
        let gone = match id {
            RowId::Builtin(builtin) => {
                let hidden = self.todos.update(cx, |todos, cx| todos.hide(character, builtin, cx));
                if hidden {
                    self.announce(fl!("todos-window-hidden-announce", label = label.as_str()));
                }
                hidden
            }
            RowId::Own(own) => match self.todos.update(cx, |todos, cx| todos.remove_own(character, own, cx)) {
                Some(removed) => {
                    self.announce(fl!("todos-window-removed-announce", name = removed.todo.name.as_str()));
                    self.show_note(Note::Removed(removed), cx);
                    true
                }
                None => false,
            },
        };
        if !gone {
            return;
        }
        if by_keyboard {
            match next {
                Some(key) => self.focus_control(&key, window, cx),
                None => window.focus(&self.root_focus, cx),
            }
        } else {
            self.pointer_row = None;
            self.held_still = Some(window.mouse_position());
        }
        cx.notify();
    }

    fn put_back(&mut self, character: CharacterId, row: &'static str, label: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.todos.update(cx, |todos, cx| todos.show(character, row, cx)) {
            self.announce(fl!("todos-window-put-back-announce", label = label.as_str()));
            let key = self.primary_key_of(RowId::Builtin(row), cx);
            self.focus_control(&key, window, cx);
        }
    }

    /// The control a row is reached by: the row itself for ticks, the figure
    /// for counts and amounts.
    fn primary_key_of(&self, id: RowId, cx: &App) -> String {
        let kind = self.todos.read(cx).list().and_then(|list| {
            list.sections.into_iter().flat_map(|section| section.rows).find(|row| row.id == id).map(|row| row.kind)
        });
        match kind {
            Some(RowKind::Count { .. } | RowKind::Amount(_)) => format!("fig:{}", row_key(id)),
            _ => format!("row:{}", row_key(id)),
        }
    }

    fn announce(&mut self, text: String) {
        self.announcement = Some(text.into());
    }

    fn announce_progress(&mut self, label: String, change: Option<super::ProgressChange>) {
        if let Some(change) = change.filter(|change| change.was_done != change.is_done) {
            let label = label.as_str();
            self.announce(if change.is_done {
                fl!("todos-window-announce-done", label = label)
            } else {
                fl!("todos-window-announce-not-done", label = label)
            });
        }
    }

    fn show_note(&mut self, note: Note, cx: &mut Context<Self>) {
        self.note = Some(note);
        self.note_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTE_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.note = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn undo_removal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Note::Removed(removed)) = self.note.take() else {
            return;
        };
        self.note_timer = None;
        let id = RowId::Own(removed.todo.id);
        let name = removed.todo.name.clone();
        if self.todos.update(cx, |todos, cx| todos.restore_own(removed, cx)) {
            self.announce(fl!("todos-window-put-back-announce", label = name.as_str()));
            let key = self.primary_key_of(id, cx);
            self.focus_control(&key, window, cx);
        }
        cx.notify();
    }

    // ---- The character picker ----

    fn open_picker(&mut self, cx: &mut Context<Self>) {
        let characters = self.todos.read(cx).characters();
        let current = characters.iter().position(|character| Some(character.id) == self.shown_character);
        self.picker = Some(current.unwrap_or(0));
        cx.notify();
    }

    fn close_picker(&mut self, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.picker = None;
        if refocus {
            self.focus_control("character", window, cx);
        }
        cx.notify();
    }

    fn pick(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let characters = self.todos.read(cx).characters();
        if let Some(character) = characters.get(index) {
            let (id, playing) = (character.id, character.playing);
            self.todos.update(cx, |todos, cx| if playing { todos.follow_the_game(cx) } else { todos.select(id, cx) });
        }
        self.close_picker(true, window, cx);
    }

    fn on_picker_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let count = self.todos.read(cx).characters().len();
        if count == 0 {
            return;
        }
        let last = count - 1;
        match (self.picker, event.keystroke.key.as_str()) {
            (None, "down" | "up" | "enter" | "space") => self.open_picker(cx),
            (Some(lit), "down") => self.picker = Some((lit + 1).min(last)),
            (Some(lit), "up") => self.picker = Some(lit.saturating_sub(1)),
            (Some(_), "home") => self.picker = Some(0),
            (Some(_), "end") => self.picker = Some(last),
            (Some(lit), "enter" | "space") => self.pick(lit, window, cx),
            (Some(_), "escape") => self.close_picker(true, window, cx),
            _ => return,
        }
        cx.stop_propagation();
        window.prevent_default();
        cx.notify();
    }

    // ---- Drawing ----

    fn header(&mut self, list: Option<&TodoList>, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let title = match self.contents {
            Contents::Todos => {
                let left = list.map(|list| SharedString::from(list.left_text.clone()));
                chrome::title(Icon::ListChecks, fl!("todos-window-title"), left, s)
            }
            Contents::Resources => {
                let full_chip = list.and_then(|list| list.resources.full_text.clone()).map(|text| warn_chip(text, s));
                chrome::title(Icon::Package, fl!("resources-window-title"), None, s).children(full_chip)
            }
        };
        let close_focus = self.focus_of("close", false, window, cx);
        let close = focusable(
            chrome::tool_button("close", Icon::Close, false, s)
                .role(Role::Button)
                .aria_label(fl!("common-close"))
                .on_click(|_, window, _| window.remove_window()),
            &close_focus,
            7.,
            s,
        );
        let handle = div().flex_1().min_w_0().h_full().flex().items_center().gap(s.px(8.));
        div()
            .flex_none()
            .h(s.px(28.))
            .pl(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(8.))
            .child(self.drag.handle(handle).child(title))
            .child(close)
            .child(self.drag.listener())
    }

    fn selector(&mut self, list: &TodoList, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let character = &list.character;
        let open = self.picker.is_some();
        let name = self.overlay.display_name(&character.name);
        let focus = self.focus_of("character", false, window, cx);
        let element = chrome::input_box(open, s)
            .h(s.px(28.))
            .pr(s.px(7.))
            .id("character")
            .debug_selector(|| "character".to_string())
            .role(Role::ComboBox)
            .aria_label(fl!("common-character"))
            .aria_value(SharedString::from(name.clone()))
            .aria_expanded(open)
            .mt(s.px(2.))
            .mb(s.px(6.))
            .cursor_pointer()
            .child(self.picker_anchor.marker())
            .children(character.class().map(|class| chrome::class_icon(class, 16., s)))
            .child(div().flex_none().max_w(s.px(150.)).truncate().text_size(s.px(13.)).font_weight(FontWeight::SEMIBOLD).child(name))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(s.px(11.5))
                    .text_color(rgba(MUTED))
                    .child(character.class_and_server()),
            )
            .when(character.playing, |element| {
                element.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(s.px(6.))
                        .text_size(s.px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(GOOD))
                        .child(div().size(s.px(6.)).rounded_full().bg(rgb(GOOD)))
                        .child(fl!("todos-window-playing")),
                )
            })
            .child(chrome::icon(Icon::ChevronDown, 13., rgba(MUTED), s))
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                if matches!(event, ClickEvent::Keyboard(_)) {
                    return;
                }
                if this.picker.is_some() {
                    this.close_picker(true, window, cx);
                } else {
                    this.open_picker(cx);
                }
            }))
            .on_key_down(cx.listener(Self::on_picker_key));
        focusable(element, &focus, 7., s)
    }

    /// The open picker over the window, and the surface under it that closes
    /// it on a press anywhere else.
    fn picker_layer(&self, s: Scale, cx: &mut Context<Self>) -> Option<Vec<AnyElement>> {
        let lit = self.picker?;
        let bounds = self.picker_anchor.bounds()?;
        let characters = self.todos.read(cx).characters();
        let options = characters.iter().enumerate().map(|(index, character)| {
            let current = Some(character.id) == self.shown_character;
            chrome::menu_item(("character-option", index), index == lit, s)
                .debug_selector(move || format!("character-{index}"))
                .h(s.px(30.))
                .role(Role::ListBoxOption)
                .aria_label(SharedString::from(self.overlay.display_name(&character.name)))
                .aria_selected(current)
                .on_click(cx.listener(move |this, _, window, cx| this.pick(index, window, cx)))
                .children(character.class().map(|class| chrome::class_icon(class, 16., s)))
                .child(
                    div()
                        .flex_none()
                        .max_w(s.px(130.))
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(self.overlay.display_name(&character.name)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(s.px(11.5))
                        .text_color(rgba(MUTED))
                        .child(character.class_and_server()),
                )
                .child(div().flex_none().text_size(s.px(11.)).text_color(rgba(FAINT)).child(character.status.clone()))
        });
        let menu = chrome::popover(s)
            .id("character-picker")
            .debug_selector(|| "character-picker".to_string())
            .role(Role::ListBox)
            .aria_label(fl!("common-character"))
            .top(bounds.bottom() + s.px(4.))
            .left(bounds.left())
            .w(bounds.size.width)
            .children(options)
            .child(div().h(px(1.)).mx(s.px(4.)).my(s.px(4.)).bg(rgba(DIVIDER)))
            .child(
                div()
                    .px(s.px(8.))
                    .pb(s.px(4.))
                    .whitespace_normal()
                    .line_height(relative(1.35))
                    .text_size(s.px(11.))
                    .text_color(rgba(MUTED))
                    .child(match self.contents {
                        Contents::Todos => fl!("todos-window-picker-note"),
                        Contents::Resources => fl!("resources-window-picker-note"),
                    }),
            );
        let scrim = chrome::scrim("picker-scrim").on_any_mouse_down(cx.listener(|this, _, window, cx| this.close_picker(false, window, cx)));
        Some(vec![scrim.into_any_element(), menu.into_any_element()])
    }

    fn note_line(&mut self, list: &TodoList, s: Scale, window: &Window, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let (glyph, good, text, bold) = match &self.note {
            Some(Note::Removed(removed)) => {
                let name = removed.todo.name.clone();
                (Icon::Trash, false, copy::removed_note(&name), Some(name))
            }
            Some(Note::Added { name, section }) => (Icon::Check, true, copy::added_note(name, &section.title()), Some(name.clone())),
            Some(Note::Login { replaced }) => {
                let text = if *replaced { fl!("todos-window-login-note-replaced") } else { fl!("todos-window-login-note") };
                (Icon::Check, true, text, None)
            }
            None if self.contents == Contents::Todos && list.left == 0 => {
                let text = copy::everything_done(&self.overlay.display_name(&list.character.name));
                (Icon::Check, true, text.clone(), Some(text))
            }
            None => return None,
        };
        let highlights: Vec<_> = bold
            .and_then(|bold| text.find(&bold).map(|start| start..start + bold.len()))
            .map(|range| (range, HighlightStyle { font_weight: Some(FontWeight::BOLD), ..Default::default() }))
            .into_iter()
            .collect();
        let colour: gpui::Hsla = if good { rgb(GOOD).into() } else { rgba(MUTED).into() };
        let undo = matches!(self.note, Some(Note::Removed(_))).then(|| {
            let focus = self.focus_of("undo", false, window, cx);
            focusable(
                link("undo", fl!("todos-window-undo"), s)
                    .role(Role::Button)
                    .aria_label(fl!("todos-window-undo"))
                    .on_click(cx.listener(|this, _, window, cx| this.undo_removal(window, cx))),
                &focus,
                4.,
                s,
            )
        });
        Some(
            div()
                .id("note")
                .debug_selector(|| "note".to_string())
                .role(Role::Status)
                .aria_label(SharedString::from(text.clone()))
                .flex_none()
                .mb(s.px(6.))
                .flex()
                .items_start()
                .gap(s.px(7.))
                .py(s.px(7.))
                .px(s.px(9.))
                .rounded(s.px(8.))
                .bg(rgba(if good { NOTE_GOOD } else { SOFT }))
                .text_color(colour)
                .text_size(s.px(11.5))
                .line_height(relative(1.35))
                .whitespace_normal()
                .child(chrome::icon(glyph, 13., colour, s).mt(s.px(1.)))
                .child(div().flex_1().min_w_0().child(StyledText::new(text).with_highlights(highlights)))
                .children(undo),
        )
    }

    /// "From the game, live · logged in at 19:02": where the values come from,
    /// times and names brighter, cut off with an ellipsis.
    fn source_line(&self, list: &TodoList, s: Scale) -> Stateful<Div> {
        let name = self.overlay.display_name(&list.character.name);
        let source_line = match self.contents {
            Contents::Todos => &list.source_line,
            Contents::Resources => &list.resources.source_line,
        };
        let (text, strong): (String, Vec<String>) = match source_line {
            SourceLine::Live { logged_in_at: Some(time) } => (fl!("todos-source-live", time = time.as_str()), vec![time.clone()]),
            SourceLine::Live { logged_in_at: None } => (fl!("todos-source-live-now"), Vec::new()),
            SourceLine::Earlier { at } => {
                let text = match self.contents {
                    Contents::Todos => fl!("todos-source-earlier-to-dos", stamp = at.as_str()),
                    Contents::Resources => fl!("todos-source-earlier", stamp = at.as_str()),
                };
                (text, vec![at.clone()])
            }
            SourceLine::Shared { from } => {
                let from = self.overlay.display_name(from);
                (fl!("todos-source-shared", name = name.as_str(), from = from.as_str()), vec![name, from])
            }
            SourceLine::NoneYet => (fl!("todos-source-none", name = name.as_str()), vec![name]),
        };
        let bright = HighlightStyle { color: Some(rgba(TEXT).into()), font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() };
        let dim = HighlightStyle { color: Some(rgba(FAINT).into()), ..Default::default() };
        let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
        let mut from = 0;
        for part in strong.iter().filter(|part| !part.is_empty()) {
            if let Some(start) = text[from..].find(part.as_str()).map(|at| from + at) {
                highlights.push((start..start + part.len(), bright));
                from = start + part.len();
            }
        }
        highlights.extend(text.match_indices('\u{b7}').map(|(start, dot)| (start..start + dot.len(), dim)));
        highlights.sort_by_key(|(range, _)| range.start);
        div()
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
            .child(div().flex_1().min_w_0().truncate().child(StyledText::new(text).with_highlights(highlights)))
    }

    fn footer(&mut self, reset_time: String, weekly: String, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let footer = chrome::footer(s)
            .gap(s.px(5.))
            .child(chrome::icon(Icon::Clock, 12., rgba(FAINT), s))
            .child(fl!("todos-window-daily-reset"))
            .child(div().text_color(rgba(TEXT)).font_weight(FontWeight::SEMIBOLD).child(reset_time));
        if self.contents == Contents::Resources {
            let caps = if self.todos.read(cx).membership() {
                fl!("resources-window-membership-caps")
            } else {
                fl!("resources-window-plain-caps")
            };
            return footer.child(div().flex_1()).child(div().flex_none().text_color(rgba(FAINT)).child(caps));
        }
        let focus = self.focus_of("change", false, window, cx);
        let change = focusable(
            link("change", fl!("todos-window-change"), s).role(Role::Link).aria_label(fl!("todos-window-change-name")).on_click(cx.listener(|this, _, _, cx| {
                secondary::open_settings_at(&this.meter_model, SettingsSection::Todos, cx);
            })),
            &focus,
            4.,
            s,
        );
        footer
            .child(div().text_color(rgba(FAINT)).child("\u{b7}"))
            .child(change)
            .child(div().flex_1())
            .child(div().flex_none().text_color(rgba(FAINT)).child(weekly))
    }
}

impl Render for TodoView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch_pointer(window, cx);
        let look = Look::of(&self.overlay);
        let s = look.s;
        let list = self.todos.read(cx).list();
        self.drawn.clear();
        self.typed_shown = list.as_ref().is_some_and(|list| self.has_typed_value(list));

        let (selector, title) = match self.contents {
            Contents::Todos => ("todos", fl!("todos-window-title")),
            Contents::Resources => ("resources", fl!("resources-window-title")),
        };
        let header = self.header(list.as_ref(), s, window, cx);
        let card = chrome::card(WIDTH, look)
            .id(selector)
            .debug_selector(move || selector.to_string())
            .role(Role::Dialog)
            .aria_label(title)
            .child(header);
        let card = match &list {
            Some(list) => {
                let selector = self.selector(list, s, window, cx);
                let source = self.source_line(list, s);
                let note = self.note_line(list, s, window, cx);
                let body = self.list_body(list, s, window, cx);
                let footer = self.footer(list.footer.reset_time.clone(), list.footer.weekly.clone(), s, window, cx);
                card.child(selector).child(source).children(note).child(body).child(footer)
            }
            None => {
                let reset_time = self.todos.read(cx).reset_clock().to_string();
                let footer = self.footer(reset_time, copy::weekly_on_wednesday(), s, window, cx);
                let line = match self.contents {
                    Contents::Todos => fl!("todos-window-no-character-line"),
                    Contents::Resources => fl!("resources-window-no-character-line"),
                };
                card.child(chrome::empty(Some(fl!("todos-window-no-character-title").into()), line, s)).child(footer)
            }
        };
        let picker = self.picker_layer(s, cx);
        let announcement = self.announcement.clone().map(|text| {
            div().id("announcement").role(Role::Status).aria_label(text).absolute().top_0().left_0().size_0()
        });
        chrome::root(look)
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(Self::on_mouse_down))
            .child(card)
            .children(picker.into_iter().flatten())
            .children(announcement)
    }
}

impl TodoView {
    /// A row of this window shows a value the player typed.
    fn has_typed_value(&self, list: &TodoList) -> bool {
        let typed = |row: &super::RowView| row.source == Source::Typed;
        match self.contents {
            Contents::Todos => list.sections.iter().flat_map(|section| &section.rows).any(typed),
            Contents::Resources => list.resources.rows.iter().any(typed),
        }
    }
}

/// Saves what the amount field holds, unless it still holds what it opened
/// with: re-typing the same figure would restart the refill count.
fn commit_amount(todos: &Entity<Todos>, edit: &AmountEdit, typed: &str, cx: &mut App) {
    let typed = typed.trim();
    let value = if typed.is_empty() {
        None
    } else {
        match typed.parse::<u32>() {
            Ok(value) => Some(value),
            Err(error) => {
                eprintln!("the amount \"{typed}\" for {} was not kept: {error}", edit.row);
                return;
            }
        }
    };
    if value == edit.opened_with {
        return;
    }
    set_typed(todos, edit.character, edit.row, edit.typing, value, cx);
}

fn set_typed(todos: &Entity<Todos>, character: CharacterId, row: &'static str, typing: Typing, value: Option<u32>, cx: &mut App) {
    todos.update(cx, |todos, cx| match typing {
        Typing::Amount => todos.set_amount(character, row, value, cx),
        Typing::Progress => todos.set_progress(character, row, value, cx),
    });
}

/// The stable part of a row's control keys: the catalogue id, or "own-<n>".
fn row_key(id: RowId) -> String {
    match id {
        RowId::Builtin(id) => id.to_string(),
        RowId::Own(id) => format!("own-{id}"),
    }
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
        .when_some(focus.reveal.clone(), |element, reveal| element.child(reveal_marker(reveal)))
}

/// "Show", "Hide again", "Change", "Undo": `muted` / 600, underlined.
fn link(key: &str, text: impl Into<SharedString>, s: Scale) -> Stateful<Div> {
    let selector = key.to_string();
    div()
        .id(element_id("link", key))
        .debug_selector(move || selector.clone())
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

/// The `chip-warn` "N full", 18 px tall.
fn warn_chip(text: impl Into<SharedString>, s: Scale) -> Div {
    div()
        .debug_selector(|| "full-chip".to_string())
        .flex_none()
        .h(s.px(18.))
        .px(s.px(7.))
        .flex()
        .items_center()
        .gap(s.px(4.))
        .rounded_full()
        .bg(rgba(CHIP_WARN))
        .text_color(rgb(WARN))
        .text_size(s.px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(chrome::icon(Icon::TriangleAlert, 11., rgb(WARN), s))
        .child(text.into())
}

fn element_id(prefix: &str, key: &str) -> ElementId {
    ElementId::Name(SharedString::from(format!("{prefix}-{key}")))
}

#[cfg(feature = "probes")]
impl TodoView {
    /// Lets a field take the keyboard inside GPUI only, and holds the pointer
    /// over the window, so a capture never asks Windows for the foreground.
    pub fn keep_keyboard_in_app(&mut self) {
        self.keyboard_in_app = true;
        self.pointer_held_over = true;
    }

    /// Puts the pointer in `row`, as its enter would; none takes it out.
    pub fn force_pointer_row(&mut self, row: Option<RowId>, cx: &mut Context<Self>) {
        self.pointer_row = row;
        self.held_still = None;
        cx.notify();
    }

    pub fn list_hidden(&mut self, place: Section, cx: &mut Context<Self>) {
        self.listing_hidden.insert(place);
        cx.notify();
    }

    pub fn open_amount_field(&mut self, row: &'static str, typing: Typing, tracked: Option<u32>, window: &mut Window, cx: &mut Context<Self>) {
        self.open_amount(row, typing, tracked, false, window, cx);
    }

    pub fn open_add_form(&mut self, counted: bool, cadence: Cadence, window: &mut Window, cx: &mut Context<Self>) {
        self.open_form(window, cx);
        if let Some(form) = self.adding.as_mut() {
            form.counted = counted;
            form.cadence = cadence;
        }
    }

    pub fn show_picker(&mut self, cx: &mut Context<Self>) {
        self.open_picker(cx);
    }

    pub fn close_character_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_picker(false, window, cx);
    }

    /// The note a removal by the × shows, for a removal made on the model.
    pub fn show_removed_note(&mut self, removed: Option<RemovedOwnTodo>, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(removed) = removed {
            self.show_note(Note::Removed(removed), cx);
        }
    }

}

#[cfg(any(test, feature = "probes"))]
impl TodoView {
    pub fn is_typing(&self) -> bool {
        self.edit.is_editing()
    }

    pub fn amount_field_is_open(&self) -> bool {
        self.amount_edit.is_some()
    }

    pub fn form_is_open(&self) -> bool {
        self.adding.is_some()
    }

    pub fn focused_control(&self, window: &Window) -> Option<String> {
        self.controls.iter().find(|(_, handle)| handle.is_focused(window)).map(|(key, _)| key.clone())
    }

    /// The row whose check and × show, if any.
    pub fn revealed_row(&self) -> Option<RowId> {
        self.pointer_row.filter(|_| self.held_still.is_none())
    }
}

#[cfg(feature = "probes")]
impl TodoView {
    /// Where the row with `key` ("duty", "own-1") was last drawn, in window
    /// coordinates.
    pub fn row_bounds(&self, key: &str) -> Option<gpui::Bounds<Pixels>> {
        self.row_marks.borrow().get(key).and_then(Anchor::bounds)
    }

    /// The window's view of the pointer moving: what a posted move did.
    pub fn held_still(&self) -> bool {
        self.held_still.is_some()
    }
}
