use std::time::Duration;

use meter_core::capture::parser::ticket_record::TicketRecord;
use meter_core::identity::KnownCharacter;
use meter_core::tickets::{TicketContent, TicketReport};
use gpui::{point, AppContext as _, Bounds, KeyUpEvent, Keystroke, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::*;
use crate::clock::now_ms;
use crate::secondary::text_input;
use crate::settings::{AppSettings, Settings};
use crate::todos::{self, NewOwnTodo, OwnKind};

struct Opened<'a> {
    view: Entity<TodoView>,
    todos: Entity<Todos>,
    character: CharacterId,
    cx: &'a mut VisualTestContext,
}

fn known(name: &str, class: &str) -> KnownCharacter {
    KnownCharacter { name: name.to_string(), server_id: 1001, class: class.to_string(), key: None }
}

/// The real to-do window in GPUI's test platform, on in-memory settings that
/// remember two invented characters.
fn open(cx: &mut TestAppContext) -> Opened<'_> {
    open_listing(Contents::Todos, cx)
}

fn open_listing(contents: Contents, cx: &mut TestAppContext) -> Opened<'_> {
    let (todos, meter) = cx.update(|cx| {
        text_input::bind_keys(cx);
        let store = cx.new(|_| {
            let mut all = AppSettings::default();
            all.known_characters = vec![known("Alder", "RANGER"), known("Birch", "CLERIC")];
            Settings::in_memory(all)
        });
        settings::install(&store, cx);
        todos::install(&store, false, cx);
        let overlay = store.read(cx).get().overlay.clone();
        (global(cx), cx.new(|_| MeterModel::new(overlay, None)))
    });
    let (view, cx) = cx.add_window_view(|window, cx| TodoView::new(contents, meter, window, cx));
    let character = cx.update(|_, cx| todos.read(cx).selected()).expect("a remembered character");
    Opened { view, todos, character, cx }
}

impl Opened<'_> {
    /// Key down then up, as a real press: buttons act on the release.
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
        for _ in 0..120 {
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

    fn is_drawn(&mut self, selector: &'static str) -> bool {
        self.cx.debug_bounds(selector).is_some()
    }

    fn click(&mut self, selector: &'static str) {
        let centre = self.bounds(selector).center();
        self.cx.simulate_click(centre, Modifiers::none());
    }

    /// A press on the window's title, where no control is.
    fn click_title(&mut self) {
        let card = self.bounds("todos");
        self.cx.simulate_click(card.origin + point(px(60.), px(14.)), Modifiers::none());
    }

    fn move_to(&mut self, at: Point<Pixels>) {
        self.cx.simulate_mouse_move(at, None, Modifiers::none());
    }

    fn update<R>(&mut self, change: impl FnOnce(&mut Todos, &mut Context<Todos>) -> R) -> R {
        let todos = self.todos.clone();
        self.cx.update(|_, cx| todos.update(cx, change))
    }

    fn row(&mut self, id: RowId) -> super::super::RowView {
        let (todos, character) = (self.todos.clone(), self.character);
        let list = self.cx.update(|_, cx| todos.read(cx).list_of(character)).expect("a list");
        list.sections
            .into_iter()
            .flat_map(|section| section.rows)
            .flat_map(|row| match row.kind.clone() {
                RowKind::Group(group) => std::iter::once(row).chain(group.children).collect::<Vec<_>>(),
                _ => vec![row],
            })
            .find(|row| row.id == id)
            .expect("the row is in the list")
    }

    fn count(&mut self, id: RowId) -> u32 {
        match self.row(id).kind {
            RowKind::Count { count, .. } => count,
            kind => panic!("not a count: {kind:?}"),
        }
    }

    fn amount(&mut self, row: &'static str) -> Option<u32> {
        match self.row(RowId::Builtin(row)).kind {
            RowKind::Amount(amount) => amount.tracked.then_some(amount.value),
            kind => panic!("not an amount: {kind:?}"),
        }
    }

    fn add_own(&mut self, name: &str, kind: OwnKind) -> u32 {
        let character = self.character;
        let request = NewOwnTodo { name: name.to_string(), kind, cadence: Cadence::Daily };
        self.update(|todos, cx| todos.add_own(character, request, cx)).expect("added")
    }

    fn view<R>(&mut self, read: impl FnOnce(&TodoView) -> R) -> R {
        let view = self.view.clone();
        self.cx.update(|_, cx| read(view.read(cx)))
    }

    fn ring_shown(&mut self) -> bool {
        self.is_drawn("focus-ring")
    }

    /// The game lists Alder's tickets at login: (ticket, regular pool, extra pool).
    fn login(&mut self, tickets: &[(u32, u32, u32)]) {
        let records = tickets
            .iter()
            .map(|&(id, count, charge)| TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge })
            .collect();
        let report = TicketReport { character: known("Alder", "RANGER"), captured_at: now_ms() as f64 / 1000.0, content: TicketContent::Snapshot(records) };
        self.update(|todos, cx| todos.apply_tickets(&[report], cx));
    }

    fn source(&mut self, row: &'static str) -> Source {
        self.row(RowId::Builtin(row)).source
    }
}

#[gpui::test]
fn space_and_enter_toggle_a_tick_row_and_delete_hides_a_row_and_moves_on(cx: &mut TestAppContext) {
    let mut window = open(cx);
    let own = window.add_own("Mine", OwnKind::Tick);
    let key = format!("row:own-{own}");
    window.tab_to(&key);
    assert!(window.ring_shown(), "the ring around the row");
    assert!(window.is_drawn("check-own-1") || window.view(|view| view.revealed_row().is_none()), "the check shows with the keyboard on the row");

    window.press("space");
    assert!(window.row(RowId::Own(own)).is_done());
    assert_eq!(window.view(|view| view.announcement.clone()).as_deref(), Some("Mine: done"));
    window.press("enter");
    assert!(!window.row(RowId::Own(own)).is_done());
    assert_eq!(window.focused().as_deref(), Some(key.as_str()), "the row keeps the keyboard");

    window.tab_to("fig:duty");
    window.press("delete");
    assert!(window.row(RowId::Builtin("duty")).hidden, "Delete on the figure hides the row");
    assert_eq!(window.focused().as_deref(), Some(key.as_str()), "the keyboard goes to the next row");
    assert_eq!(window.view(|view| view.announcement.clone()).as_deref(), Some("Duty Missions hidden"));

    window.press("delete");
    assert!(window.row(RowId::Builtin("duty")).hidden);
    assert!(window.view(|view| matches!(view.note, Some(Note::Removed(_)))), "your own is removed with an Undo note");
    window.tab_to("undo");
    window.press("enter");
    assert!(!window.row(RowId::Own(own)).hidden, "Undo puts it back");
    assert_eq!(window.focused().as_deref(), Some(key.as_str()), "and the keyboard on it");
}

#[gpui::test]
fn arrows_and_keys_move_a_count_within_its_limits(cx: &mut TestAppContext) {
    let mut window = open(cx);
    let duty = RowId::Builtin("duty");
    window.tab_to("fig:duty");
    window.press("up");
    window.press("up");
    assert_eq!(window.count(duty), 2);
    window.press("down");
    assert_eq!(window.count(duty), 1);
    window.press("space");
    window.press("enter");
    assert_eq!(window.count(duty), 3, "Space and Enter add one");
    for _ in 0..4 {
        window.press("up");
    }
    assert_eq!(window.count(duty), 5, "never past 5");
    assert!(window.row(duty).is_done());
    assert_eq!(window.view(|view| view.announcement.clone()).as_deref(), Some("Duty Missions: done"));

    window.press("shift-tab");
    assert_eq!(window.focused().as_deref(), Some("less:duty"), "the one-less arrow before the figure");
    window.press("enter");
    assert_eq!(window.count(duty), 4);
    window.press("tab");
    window.press("tab");
    assert_eq!(window.focused().as_deref(), Some("more:duty"));
    window.press("space");
    assert_eq!(window.count(duty), 5);
    assert_eq!(window.focused().as_deref(), Some("fig:duty"), "the arrow went dim at the limit: the figure keeps the keyboard");
    window.press("tab");
    assert_ne!(window.focused().as_deref(), Some("more:duty"), "a dimmed arrow is out of the tab order");
}

#[gpui::test]
fn an_amount_typed_is_kept_by_enter_and_a_click_outside_and_dropped_by_escape(cx: &mut TestAppContext) {
    let mut window = open(cx);
    window.tab_to("fig:shugo");
    window.press("enter");
    assert!(window.view(|view| view.amount_field_is_open() && view.is_typing()), "the field opens and takes the keyboard");
    window.cx.simulate_input("24");
    window.press("enter");
    assert_eq!(window.amount("shugo"), Some(24));
    assert!(!window.view(|view| view.is_typing()), "Enter gives the keyboard back");
    assert_eq!(window.focused().as_deref(), Some("fig:shugo"), "to the figure it was opened from");

    window.press("space");
    window.cx.simulate_input("7");
    window.press("escape");
    assert_eq!(window.amount("shugo"), Some(24), "Escape keeps nothing");
    assert!(!window.view(|view| view.amount_field_is_open() || view.is_typing()));
    assert_eq!(window.focused().as_deref(), Some("fig:shugo"));

    window.press("enter");
    window.cx.simulate_input("9");
    window.press("enter");
    assert_eq!(window.amount("shugo"), Some(9), "what is typed replaces the figure the field opened on");

    window.click("fig-invasion");
    assert!(window.view(|view| view.is_typing()));
    let view = window.view.clone();
    let focused = window.cx.update(|w, cx| view.read(cx).amount_input.read(cx).focus_handle(cx).is_focused(w));
    window.cx.simulate_input("3x");
    let typed = window.cx.update(|_, cx| view.read(cx).amount_input.read(cx).text().to_string());
    assert_eq!((focused, typed.as_str()), (true, "3"), "the field has the keyboard and takes digits only");
    window.click_title();
    assert_eq!(window.amount("invasion"), Some(3), "a click outside keeps the digits typed");
    assert!(!window.view(|view| view.amount_field_is_open() || view.is_typing()), "and gives the keyboard back");

    window.click("fig-invasion");
    window.click("amount-clear");
    assert_eq!(window.amount("invasion"), None, "the clear button stops tracking");
}

#[gpui::test]
fn the_add_form_takes_the_keyboard_adds_on_enter_and_closes_on_escape(cx: &mut TestAppContext) {
    let mut window = open(cx);
    window.tab_to("add-line");
    window.press("enter");
    assert!(window.view(|view| view.form_is_open() && view.is_typing()));
    window.cx.simulate_input("Chores");
    window.press("enter");
    let added = window.row(RowId::Own(1));
    assert_eq!((added.label.as_str(), added.yours), ("Chores", true));
    assert!(!window.view(|view| view.form_is_open() || view.is_typing()), "Enter adds and gives the keyboard back");
    assert_eq!(window.focused().as_deref(), Some("add-line"));
    assert!(window.view(|view| matches!(view.note, Some(Note::Added { .. }))), "with the Added note");

    window.press("enter");
    window.press("escape");
    assert!(!window.view(|view| view.form_is_open() || view.is_typing()));
    assert_eq!(window.focused().as_deref(), Some("add-line"));

    window.press("enter");
    window.click_title();
    assert!(!window.view(|view| view.form_is_open()), "a click outside closes a form with no name");

    window.tab_to("add-line");
    window.press("enter");
    window.cx.simulate_input("Kept");
    window.click_title();
    assert!(window.view(|view| view.form_is_open()), "a form with a name stays");
    assert!(!window.view(|view| view.is_typing()), "but gives the keyboard back");
}

#[gpui::test]
fn a_rows_controls_come_on_entering_and_go_on_leaving_and_never_flash_inside(cx: &mut TestAppContext) {
    let mut window = open(cx);
    let duty = RowId::Builtin("duty");
    let row = window.bounds("row-duty");
    window.move_to(row.center());
    assert_eq!(window.view(|view| view.revealed_row()), Some(duty));
    let x = window.bounds("x-duty");
    let spots = [
        point(row.left() + px(2.), row.center().y),
        x.center(),
        point(row.left() + row.size.width * 0.6, row.top() + px(2.)),
        point(row.right() - px(1.), row.bottom() - px(2.)),
        row.center(),
    ];
    for at in spots {
        window.move_to(at);
        assert_eq!(window.view(|view| view.revealed_row()), Some(duty), "at {at:?} inside the row");
        assert!(window.is_drawn("x-duty"));
    }
    window.move_to(point(row.center().x, row.bottom() + px(40.)));
    assert_ne!(window.view(|view| view.revealed_row()), Some(duty), "gone when the pointer leaves");
    assert!(!window.is_drawn("x-duty"));
    assert!(!window.ring_shown(), "no keyboard, no ring");
}

#[gpui::test]
fn after_a_hide_by_pointer_the_row_now_there_waits_for_the_pointer_to_move(cx: &mut TestAppContext) {
    let mut window = open(cx);
    let odyle = window.bounds("row-odyle");
    window.move_to(odyle.center());
    let x = window.bounds("x-odyle").center();
    window.move_to(x);
    window.cx.simulate_click(x, Modifiers::none());
    assert!(window.row(RowId::Builtin("odyle")).hidden);
    assert_eq!(window.view(|view| view.revealed_row()), None, "the row that slid under the pointer shows nothing yet");
    assert!(!window.is_drawn("x-shugo"));

    window.cx.simulate_mouse_move(point(x.x - px(3.), x.y), None::<MouseButton>, Modifiers::none());
    assert_eq!(window.view(|view| view.revealed_row()), Some(RowId::Builtin("shugo")), "the pointer moved: its controls come");
}

#[gpui::test]
fn sections_and_groups_fold_and_show_lists_hidden_rows_with_put_back(cx: &mut TestAppContext) {
    let mut window = open(cx);
    window.tab_to("section:today");
    window.press("space");
    assert!(!window.is_drawn("row-duty"), "Today folded");
    window.press("enter");
    assert!(window.is_drawn("row-duty"));

    let character = window.character;
    window.update(|todos, cx| todos.hide(character, "trial", cx));
    window.tab_to("show:week");
    window.press("enter");
    assert!(window.is_drawn("put-trial"), "the hidden row is listed with its put-back button");
    window.tab_to("put:trial");
    window.press("enter");
    assert!(!window.row(RowId::Builtin("trial")).hidden);
    assert_eq!(window.focused().as_deref(), Some("fig:trial"), "the keyboard follows the row back");
    assert_eq!(window.view(|view| view.announcement.clone()).as_deref(), Some("Ascension Trial put back"));
}

#[gpui::test]
fn no_remembered_character_shows_the_empty_state(cx: &mut TestAppContext) {
    let meter = cx.update(|cx| {
        let store = cx.new(|_| Settings::in_memory(AppSettings::default()));
        settings::install(&store, cx);
        todos::install(&store, false, cx);
        let overlay = store.read(cx).get().overlay.clone();
        cx.new(|_| MeterModel::new(overlay, None))
    });
    let (_, cx) = cx.add_window_view(|window, cx| TodoView::new(Contents::Todos, meter, window, cx));
    assert!(cx.debug_bounds("todos").is_some());
    assert!(cx.debug_bounds("character").is_none(), "no selector without a character");
    assert!(cx.debug_bounds("change").is_some(), "the footer still says when the day resets");
}

#[gpui::test]
fn a_game_value_has_no_arrows_a_click_types_over_it_and_its_clear_button_goes_back(cx: &mut TestAppContext) {
    let mut window = open(cx);
    window.login(&[(1, 7, 3), (101, 9, 7)]);
    assert!(window.is_drawn("fig-shugo"));
    assert!(!window.is_drawn("more-shugo") && !window.is_drawn("less-shugo"), "no arrows on the game's value");

    window.tab_to("fig:shugo");
    window.press("enter");
    assert!(window.view(|view| view.amount_field_is_open() && view.is_typing()), "Enter types over it");
    window.cx.simulate_input("9");
    window.press("enter");
    assert_eq!(window.amount("shugo"), Some(9), "above what the game said is kept");
    assert_eq!(window.source("shugo"), Source::Typed);
    assert_eq!(window.focused().as_deref(), Some("fig:shugo"));
    assert!(!window.is_drawn("more-shugo"), "a value typed over the game's has no arrows either");

    window.press("enter");
    window.click("amount-clear");
    assert_eq!(window.amount("shugo"), Some(7), "the field's × goes back to the game's value");
    assert!(matches!(window.source("shugo"), Source::GameEarlier { .. }));
    assert!(!window.view(|view| view.amount_field_is_open() || view.is_typing()));

    let fissure = RowId::Builtin("fissure");
    assert!(!window.is_drawn("more-fissure"), "this week's game rows read without arrows");
    window.tab_to("fig:fissure");
    window.press("up");
    assert_eq!(window.count(fissure), 5, "done is 14 minus the 9 left; the arrow keys change nothing");
    window.press("enter");
    assert!(window.view(|view| view.amount_field_is_open() && view.is_typing()), "Enter types over a weekly game row too");
    window.cx.simulate_input("8");
    window.press("enter");
    assert_eq!((window.count(fissure), window.source("fissure")), (8, Source::Typed));
    window.press("enter");
    window.click("amount-clear");
    assert_eq!(window.count(fissure), 5, "the × goes back to the game's count");

    window.login(&[(90_000_002, 0, 0)]);
    window.update(|todos, cx| todos.set_group_open("sanct", true, cx));
    window.tab_to("row:forgeboss");
    window.press("enter");
    let forgeboss = RowId::Builtin("forgeboss");
    assert!(!window.row(forgeboss).is_done(), "a game-done tick is typed over as not done");
    assert_eq!(window.source("forgeboss"), Source::Typed);
    window.press("enter");
    assert!(window.row(forgeboss).is_done(), "and pressed again it goes back to the game's");
}

#[gpui::test]
fn a_login_while_the_window_is_open_shows_its_note_for_four_seconds(cx: &mut TestAppContext) {
    let mut window = open(cx);
    let character = window.character;
    window.update(|todos, cx| todos.set_amount(character, "shugo", Some(9), cx));
    window.move_to(point(px(1.), px(1.)));
    window.cx.run_until_parked();
    assert!(window.view(|view| view.typed_shown), "the window drew the typed 9");
    assert!(window.view(|view| view.note.is_none()));

    window.login(&[(1, 7, 3)]);
    assert!(window.view(|view| matches!(view.note, Some(Note::Login { replaced: true }))), "the login replaced the typed 9");
    assert!(window.is_drawn("note"));
    assert_eq!(window.amount("shugo"), Some(7));

    window.cx.executor().advance_clock(Duration::from_secs(5));
    window.cx.run_until_parked();
    assert!(window.view(|view| view.note.is_none()), "gone after 4 s");
    assert!(window.is_drawn("source-line"));
}

#[gpui::test]
fn the_resources_window_lists_the_recharges_alone_and_its_rows_hide_nothing(cx: &mut TestAppContext) {
    let mut window = open_listing(Contents::Resources, cx);
    assert!(window.is_drawn("resources"));
    assert!(window.is_drawn("row-odyle") && window.is_drawn("row-conquest") && window.is_drawn("row-fafnium"));
    assert!(!window.is_drawn("row-duty") && !window.is_drawn("row-fissure"), "to-dos stay in the to-do window");
    assert!(!window.is_drawn("section-recharges") && !window.is_drawn("add-line") && !window.is_drawn("change"));

    let row = window.bounds("row-odyle");
    for at in [row.center(), point(row.left() + px(2.), row.center().y), point(row.right() - px(2.), row.top() + px(2.))] {
        window.move_to(at);
        assert_eq!(window.view(|view| view.revealed_row()), Some(RowId::Builtin("odyle")), "at {at:?} inside the row");
        assert!(!window.is_drawn("x-odyle"), "nothing to hide in the Resources window");
    }
    window.move_to(point(row.center().x, row.top() - px(30.)));
    assert_eq!(window.view(|view| view.revealed_row()), None, "gone when the pointer leaves");

    window.tab_to("fig:odyle");
    assert!(window.ring_shown());
    window.press("delete");
    assert!(!window.row(RowId::Builtin("odyle")).hidden, "Delete hides nothing here");
    window.press("enter");
    assert!(window.view(|view| view.amount_field_is_open() && view.is_typing()));
    window.cx.simulate_input("300");
    window.press("enter");
    assert_eq!(window.amount("odyle"), Some(300), "the to-dos' own data");
    assert!(!window.view(|view| view.is_typing()), "the keyboard goes back");
    assert_eq!(window.focused().as_deref(), Some("fig:odyle"));

    window.tab_to("row:conquest");
    window.press("enter");
    assert!(window.is_drawn("row-c-krao"), "the Conquest dungeons open in place");
}
