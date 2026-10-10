use gpui::{
    point, px, AppContext as _, Bounds, KeyUpEvent, Keystroke, Modifiers, MouseButton, Pixels, Point, ScrollDelta,
    ScrollWheelEvent, TestAppContext, VisualTestContext,
};
use meter_core::capture::parser::pet_record::{PetLevel, PetProgress, PetSnapshot};
use meter_core::identity::KnownCharacter;
use meter_core::pets::{PetContent, PetReport};

use super::*;
use crate::secondary::text_input;
use crate::settings::{AppSettings, Settings};

const BLACK_DRAGON: u32 = 1001;
const KRALL_LABORER: u32 = 1002;
const FOSSA: u32 = 1004;
const SPARKIE: u32 = 1006;
const KALGOLEM: u32 = 1084;

struct Opened<'a> {
    view: Entity<PetsView>,
    pets: Entity<Pets>,
    cx: &'a mut VisualTestContext,
}

/// The game listing the collection at a login during this run.
fn login() -> PetReport {
    let levels = [(FOSSA, 2), (KALGOLEM, 3), (SPARKIE, 1)];
    let progress = [(FOSSA, 27), (SPARKIE, 24)];
    PetReport {
        character: KnownCharacter { name: "Alder".to_string(), server_id: 1002, class: "RANGER".to_string(), key: None },
        captured_at: crate::clock::now_ms() as f64 / 1000.0 + 1.0,
        content: PetContent::Snapshot(PetSnapshot {
            levels: levels.iter().map(|&(pet_id, level)| PetLevel { pet_id, level }).collect(),
            progress: progress.iter().map(|&(pet_id, progress)| PetProgress { pet_id, progress }).collect(),
            counters: Vec::new(),
        }),
    }
}

/// The real Pets window in GPUI's test platform, with no meter at all: as
/// opened from the toolbar while the meter is closed. `read`: the game
/// listed the collection at login.
fn open(cx: &mut TestAppContext, read: bool) -> Opened<'_> {
    let pets = cx.update(|cx| {
        text_input::bind_keys(cx);
        let store = cx.new(|_| Settings::in_memory(AppSettings::default()));
        settings::install(&store, cx);
        service::install(cx);
        let pets = service::global(cx);
        if read {
            pets.update(cx, |pets, cx| pets.apply(&[login()], cx));
        }
        pets
    });
    let (view, cx) = cx.add_window_view(|window, cx| PetsView::new(window, cx));
    Opened { view, pets, cx }
}

impl Opened<'_> {
    /// Key down then up, as a real press: buttons act on the release.
    fn press(&mut self, key: &str) {
        let keystroke = Keystroke::parse(key).expect("a key name");
        self.cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
        self.cx.simulate_event(KeyUpEvent { keystroke });
    }

    /// The key's press alone, as a window that never activates may get it.
    fn key_down(&mut self, key: &str) {
        let keystroke = Keystroke::parse(key).expect("a key name");
        self.cx.simulate_event(KeyDownEvent { keystroke, is_held: false, prefer_character_input: false });
    }

    fn key_up(&mut self, key: &str) {
        let keystroke = Keystroke::parse(key).expect("a key name");
        self.cx.simulate_event(KeyUpEvent { keystroke });
    }

    fn focused(&mut self) -> Option<String> {
        let view = self.view.clone();
        self.cx.update(|window, cx| view.read(cx).focused_control(window))
    }

    fn tab_to(&mut self, key: &str) {
        for _ in 0..40 {
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

    fn move_to(&mut self, at: Point<Pixels>) {
        self.cx.simulate_mouse_move(at, None, Modifiers::none());
    }

    fn view<R>(&mut self, read: impl FnOnce(&PetsView) -> R) -> R {
        let view = self.view.clone();
        self.cx.update(|_, cx| read(view.read(cx)))
    }

    fn pets<R>(&mut self, read: impl FnOnce(&Pets) -> R) -> R {
        let pets = self.pets.clone();
        self.cx.update(|_, cx| read(pets.read(cx)))
    }

    fn pin(&mut self, pet_id: u32) {
        let pets = self.pets.clone();
        self.cx.update(|_, cx| pets.update(cx, |pets, cx| pets.pin(pet_id, cx)));
    }

    fn query(&mut self) -> String {
        let view = self.view.clone();
        self.cx.update(|_, cx| view.read(cx).query(cx))
    }

    fn drawn(&mut self) -> Vec<u32> {
        self.view(|view| view.drawn.clone())
    }

    fn is_live(&mut self) -> bool {
        let view = self.view.clone();
        self.cx.update(|_, cx| matches!(view.read(cx).source(cx), Source::Live { .. }))
    }
}

#[gpui::test]
fn opened_with_the_meter_closed_it_lists_every_pet_read_at_login_in_game_order(cx: &mut TestAppContext) {
    let mut window = open(cx, true);

    let drawn = window.drawn();
    assert_eq!(drawn.len(), crate::game_data::pet_ids().len(), "every pet of the catalogue");
    assert_eq!(&drawn[..3], &[BLACK_DRAGON, KRALL_LABORER, 1003], "in the game's order");
    assert!(window.is_live(), "read at a login of this run");
    assert!(window.is_drawn("pet-1004") && window.is_drawn("pin-1004"));
}

#[gpui::test]
fn pinned_pets_sit_on_top_in_pin_order_and_stay_there_when_the_window_opens_again(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.pin(KALGOLEM);
    window.pin(FOSSA);
    window.cx.run_until_parked();

    assert_eq!(&window.drawn()[..3], &[KALGOLEM, FOSSA, BLACK_DRAGON]);
    assert!(window.bounds("pet-1084").top() < window.bounds("pet-1001").top());

    let reopened = window.cx.update(|window, cx| cx.new(|cx| PetsView::new(window, cx)));
    window.cx.run_until_parked();
    assert_eq!(window.cx.update(|_, cx| reopened.read(cx).pets.read(cx).pinned().to_vec()), vec![KALGOLEM, FOSSA]);
}

#[gpui::test]
fn tab_reaches_the_search_the_filter_the_open_tab_and_one_pin_and_the_arrows_move_inside_them(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.pin(FOSSA);
    window.cx.run_until_parked();
    window.tab_to("close");
    let fossa_pin = format!("pin:{FOSSA}");
    let expected = ["close", "search", "filter", "tab:0", fossa_pin.as_str(), "close"];
    let mut walked = vec![window.focused().unwrap_or_default()];
    for _ in 1..expected.len() {
        window.press("tab");
        walked.push(window.focused().unwrap_or_default());
    }
    assert_eq!(walked, expected, "one stop for the tabs and one for the pins, the first pinned pet's");

    window.tab_to("tab:0");
    window.press("right");
    assert_eq!(window.view(|view| view.tab()), Tab::Family(PetFamily::Cogni));
    assert_eq!(window.focused().as_deref(), Some("tab:1"), "the keyboard follows the tab");
    window.press("end");
    assert_eq!(window.view(|view| view.tab()), Tab::Family(PetFamily::Special));
    window.press("home");
    assert_eq!(window.view(|view| view.tab()), Tab::All);

    window.tab_to(&format!("pin:{FOSSA}"));
    window.press("down");
    assert_eq!(window.focused(), Some(format!("pin:{BLACK_DRAGON}")), "Down goes to the next row's pin");
    window.press("space");
    assert!(window.pets(|pets| pets.is_pinned(BLACK_DRAGON)), "Space pins");
    assert_eq!(window.focused(), Some(format!("pin:{BLACK_DRAGON}")), "the pin keeps the keyboard as its row goes up");
    window.press("enter");
    assert!(!window.pets(|pets| pets.is_pinned(BLACK_DRAGON)), "Enter unpins");
    window.press("up");
    window.press("up");
    assert_eq!(window.focused(), Some(format!("pin:{FOSSA}")));
}

#[gpui::test]
fn space_and_enter_pin_and_unpin_on_the_key_press_exactly_once(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.tab_to(&format!("pin:{BLACK_DRAGON}"));

    for key in ["space", "enter"] {
        window.key_down(key);
        assert!(window.pets(|pets| pets.is_pinned(BLACK_DRAGON)), "{key}'s press alone pins");
        window.key_up(key);
        assert!(window.pets(|pets| pets.is_pinned(BLACK_DRAGON)), "{key}'s release does not unpin it again");
        window.key_down(key);
        assert!(!window.pets(|pets| pets.is_pinned(BLACK_DRAGON)), "{key}'s press alone unpins");
        window.key_up(key);
        assert!(!window.pets(|pets| pets.is_pinned(BLACK_DRAGON)), "{key}'s release does not pin it again");
        assert_eq!(window.focused(), Some(format!("pin:{BLACK_DRAGON}")), "the pin keeps the keyboard");
    }
}

#[gpui::test]
fn a_pin_shows_while_the_pointer_is_in_its_row_and_a_pinned_pets_pin_always_shows(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    let row = window.bounds("pet-1006");

    window.move_to(row.center());
    assert_eq!(window.view(|view| view.revealed_row()), Some(SPARKIE));
    for fraction in [0.1, 0.5, 0.95] {
        window.move_to(point(row.left() + row.size.width * fraction, row.center().y));
        assert_eq!(window.view(|view| view.revealed_row()), Some(SPARKIE), "moves inside the row change nothing");
    }
    let header = window.bounds("pets").origin + point(px(60.), px(14.));
    window.move_to(header);
    assert_eq!(window.view(|view| view.revealed_row()), None, "the leave hides it");

    window.move_to(row.center());
    let pin = window.bounds("pin-1006");
    window.cx.simulate_click(pin.center(), Modifiers::none());
    assert!(window.pets(|pets| pets.is_pinned(SPARKIE)), "a click on the pin pins the pet");
    assert_eq!(window.view(|view| view.revealed_row()), None, "the row now under the pointer keeps its pin hidden");
    window.move_to(pin.center() - point(px(4.), px(0.)));
    assert!(window.view(|view| view.revealed_row()).is_some(), "until the pointer moves");
}

#[gpui::test]
fn the_search_takes_the_keyboard_reads_both_languages_and_escape_clears_then_gives_it_back(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.click("search");
    assert!(window.view(|view| view.is_typing()), "a click in the field takes the keyboard");

    window.cx.simulate_input("kalgo");
    let drawn = window.drawn();
    assert!(drawn.contains(&KALGOLEM) && !drawn.contains(&FOSSA), "{drawn:?}");
    assert!(window.is_drawn("clear-search"));

    window.press("escape");
    assert_eq!(window.query(), "", "the first Escape clears the search");
    assert!(window.view(|view| view.is_typing()), "and keeps the keyboard");
    window.cx.simulate_input("ténèbres");
    assert_eq!(window.drawn(), vec![1214], "a French name finds its pet too");
    window.press("escape");
    window.press("escape");
    assert!(!window.view(|view| view.is_typing()), "a second Escape gives the keyboard back");

    window.tab_to("search");
    window.press("enter");
    assert!(window.view(|view| view.is_typing()), "Enter on the field from the keyboard takes it too");
    window.cx.simulate_input("zzz");
    window.press("enter");
    assert!(!window.view(|view| view.is_typing()), "Enter gives the keyboard back");
    assert_eq!(window.focused().as_deref(), Some("search"), "to the field it came from");
    assert_eq!(window.query(), "zzz", "what was typed stays");
    assert!(window.drawn().is_empty());
}

#[gpui::test]
fn a_search_with_nothing_in_a_family_offers_every_family(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.click("search");
    window.cx.simulate_input("kalgo");
    window.press("enter");
    window.click("tab-2");
    assert_eq!(window.view(|view| view.tab()), Tab::Family(PetFamily::Fera));
    assert!(window.drawn().is_empty());

    window.click("search-all");
    assert_eq!(window.view(|view| view.tab()), Tab::All);
    assert!(window.drawn().contains(&KALGOLEM));
}

#[gpui::test]
fn the_show_and_order_menu_works_from_the_keyboard_filters_and_is_remembered(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.tab_to("filter");
    window.press("down");
    assert!(window.view(|view| view.menu_is_open()));
    window.press("down");
    window.press("down");
    window.press("enter");
    assert!(!window.view(|view| view.menu_is_open()), "a pick closes the menu");
    assert_eq!(window.focused().as_deref(), Some("filter"));
    assert_eq!(window.pets(|pets| pets.list_choice()), ListChoice { show: Show::NotMaxed, order: Order::Game });
    assert!(!window.drawn().contains(&KALGOLEM), "Lv 3 is left out");
    assert!(window.is_drawn("clear-filter"));

    window.press("down");
    window.press("end");
    window.press("enter");
    assert_eq!(window.pets(|pets| pets.list_choice()).order, Order::ClosestToNextLevel);
    assert_eq!(window.drawn()[0], SPARKIE, "24/25: one soul left");

    window.press("down");
    window.press("escape");
    assert!(!window.view(|view| view.menu_is_open()), "Escape closes it");

    let reopened = window.cx.update(|window, cx| cx.new(|cx| PetsView::new(window, cx)));
    let choice = window.cx.update(|_, cx| reopened.read(cx).pets.read(cx).list_choice());
    assert_eq!(choice, ListChoice { show: Show::NotMaxed, order: Order::ClosestToNextLevel }, "the choice stays with the collection");

    window.click("clear-filter");
    assert_eq!(window.pets(|pets| pets.list_choice()), ListChoice::default());
    assert!(!window.is_drawn("clear-filter"));
}

#[gpui::test]
fn before_any_read_the_window_says_how_to_have_the_pets_read_and_its_controls_are_inert(cx: &mut TestAppContext) {
    let mut window = open(cx, false);
    assert!(!window.is_live());
    assert!(window.drawn().is_empty());
    window.tab_to("close");
    window.press("tab");
    assert_eq!(window.focused().as_deref(), Some("close"), "no search, filter, tab or pin to reach");

    window.cx.update(|_, cx| crate::pets::record(cx, vec![login()]));
    window.cx.run_until_parked();
    assert_eq!(window.drawn().len(), crate::game_data::pet_ids().len(), "the login fills the open window");
}

/// The last pet of the catalogue, at the bottom of All.
const LAST_PET: u32 = 1299;

fn row_selector(pet_id: u32) -> &'static str {
    match pet_id {
        BLACK_DRAGON => "pet-1001",
        KRALL_LABORER => "pet-1002",
        LAST_PET => "pet-1299",
        _ => panic!("no selector for pet {pet_id}"),
    }
}

impl Opened<'_> {
    fn row_is_built(&mut self, pet_id: u32) -> bool {
        let view = self.view.clone();
        self.cx.update(|window, cx| {
            view.update(cx, |_, cx| cx.notify());
            let _ = window.draw(cx);
        });
        self.cx.debug_bounds(row_selector(pet_id)).is_some()
    }

    fn scroll_to_bottom(&mut self) {
        let view = self.view.clone();
        self.cx.update(|window, cx| {
            view.read(cx).rows.scroll_to(px(100_000.));
            view.update(cx, |_, cx| cx.notify());
            let _ = window.draw(cx);
        });
        self.cx.run_until_parked();
    }
}

#[gpui::test]
fn only_the_rows_in_and_near_the_view_are_built_and_scrolling_builds_the_others(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    let last = LAST_PET;
    assert!(window.row_is_built(BLACK_DRAGON), "the top of the list is in view");
    assert!(!window.row_is_built(last), "a row far below the view is not built");
    assert_eq!(window.drawn().len(), crate::game_data::pet_ids().len(), "every pet is still listed for the keyboard");

    window.scroll_to_bottom();
    assert!(window.row_is_built(last), "scrolled to the bottom, the last row is built");
    assert!(!window.row_is_built(KRALL_LABORER), "and the top ones are not");
}

impl Opened<'_> {
    /// The list's viewport, its furthest scroll, and the scroll now (all
    /// positive, down the list), after a frame drawn with them, so the thumb
    /// that frame draws is the one the pointer meets.
    fn list_scroll(&mut self) -> (Bounds<Pixels>, Pixels, Pixels) {
        let view = self.view.clone();
        self.cx.update(|window, cx| {
            view.update(cx, |_, cx| cx.notify());
            let _ = window.draw(cx);
        });
        self.view(|view| (view.rows.viewport(), view.rows.max(), view.rows.scrolled()))
    }

    /// The thumb's length and how far it runs, as drawn.
    fn thumb_run(&mut self) -> (Pixels, Pixels) {
        let (viewport, max, _) = self.list_scroll();
        let length = viewport.size.height * (viewport.size.height / (viewport.size.height + max));
        (length, viewport.size.height - px(4.) - length)
    }

    /// The middle of the thumb, in its gutter on the list's right.
    fn thumb_middle(&mut self) -> Point<Pixels> {
        let (viewport, max, scrolled) = self.list_scroll();
        let (length, travel) = self.thumb_run();
        point(viewport.right() - px(4.), viewport.top() + px(2.) + travel * (scrolled / max) + length / 2.)
    }

    /// Presses at `from`, moves the pointer through `through` with the button
    /// held, and releases at the last of them.
    fn drag_pointer(&mut self, from: Point<Pixels>, through: &[Point<Pixels>]) {
        self.cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        for at in through {
            self.cx.simulate_mouse_move(*at, Some(MouseButton::Left), Modifiers::none());
        }
        let release = through.last().copied().unwrap_or(from);
        self.cx.simulate_mouse_up(release, MouseButton::Left, Modifiers::none());
        self.cx.run_until_parked();
    }
}

#[gpui::test]
fn the_thumb_keeps_following_the_pointer_over_the_rows_and_outside_the_list_and_builds_the_rows_it_brings(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    let (viewport, max, scrolled) = window.list_scroll();
    assert!(max > px(0.), "the whole collection overflows the list");
    assert_eq!(scrolled, px(0.));
    let (_, travel) = window.thumb_run();
    let over_rows = viewport.center().x;

    let middle = window.thumb_middle();
    window.cx.simulate_mouse_down(middle, MouseButton::Left, Modifiers::none());
    window.cx.simulate_mouse_move(point(over_rows, middle.y + px(40.)), Some(MouseButton::Left), Modifiers::none());
    let expected = max * (px(40.) / travel);
    assert!((window.list_scroll().2 - expected).abs() < px(0.5), "over the rows, the list follows the thumb in proportion");
    window.cx.simulate_mouse_move(point(over_rows, viewport.bottom() + px(300.)), Some(MouseButton::Left), Modifiers::none());
    assert_eq!(window.list_scroll().2, max, "below the list, the list reaches its end");
    window.cx.simulate_mouse_up(point(over_rows, viewport.bottom() + px(300.)), MouseButton::Left, Modifiers::none());
    assert!(window.row_is_built(LAST_PET), "and the last row is built");
    assert!(!window.row_is_built(KRALL_LABORER), "and the top ones are not");

    let middle = window.thumb_middle();
    let above = point(over_rows, viewport.top() - px(300.));
    window.drag_pointer(middle, &[point(over_rows, middle.y), above]);
    assert_eq!(window.list_scroll().2, px(0.), "above the list, the list is back at its start");
    assert!(window.row_is_built(KRALL_LABORER));
    assert!(!window.row_is_built(LAST_PET));

    let released_at = window.list_scroll().2;
    window.cx.simulate_mouse_move(point(over_rows, viewport.bottom()), Some(MouseButton::Left), Modifiers::none());
    assert_eq!(window.list_scroll().2, released_at, "once released, the thumb no longer follows the pointer");
}

#[gpui::test]
fn a_press_beside_the_thumb_brings_it_there_and_the_wheel_still_scrolls(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    let (viewport, max, _) = window.list_scroll();
    let gutter_bottom = point(viewport.right() - px(4.), viewport.bottom() - px(1.));
    window.cx.simulate_mouse_down(gutter_bottom, MouseButton::Left, Modifiers::none());
    window.cx.simulate_mouse_up(gutter_bottom, MouseButton::Left, Modifiers::none());
    window.cx.run_until_parked();
    assert_eq!(window.list_scroll().2, max, "a press at the gutter's bottom brings the thumb there");
    assert!(window.row_is_built(LAST_PET));

    window.cx.simulate_event(ScrollWheelEvent {
        position: viewport.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(120.))),
        ..Default::default()
    });
    window.cx.run_until_parked();
    assert!((window.list_scroll().2 - (max - px(120.))).abs() < px(0.5), "the wheel scrolls the list up");
}

#[gpui::test]
fn a_focused_pin_scrolled_out_of_view_keeps_the_keyboard_and_the_arrows_still_move(cx: &mut TestAppContext) {
    let mut window = open(cx, true);
    window.tab_to(&format!("pin:{BLACK_DRAGON}"));
    window.scroll_to_bottom();
    assert!(window.row_is_built(BLACK_DRAGON), "the row whose pin has the keyboard stays built");
    assert_eq!(window.focused(), Some(format!("pin:{BLACK_DRAGON}")));

    window.press("down");
    assert_eq!(window.focused(), Some(format!("pin:{KRALL_LABORER}")), "Down reaches the next pin, even out of view");
    window.press("space");
    assert!(window.pets(|pets| pets.is_pinned(KRALL_LABORER)));

    window.press("end");
    assert_eq!(window.focused(), Some(format!("pin:{}", LAST_PET)), "End reaches the last pin");
    assert!(window.row_is_built(LAST_PET));
}
