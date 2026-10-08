use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    px, AppContext as _, Entity, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext,
};
use meter_core::engine::meter::DpsMeter;
use meter_core::logger::AppLogger;

use super::view::{Dropdown, Slot};
use super::*;
use crate::bridge;
use crate::meter::OverlaySettings;
use crate::settings::{AppSettings, Settings};
use crate::todos;

struct Bar<'a> {
    view: Entity<ToolbarView>,
    hub: Entity<Hub>,
    requests: Rc<RefCell<Vec<ToolbarRequest>>>,
    cx: &'a mut VisualTestContext,
}

fn meter_core() -> Arc<DpsMeter> {
    let log = std::env::temp_dir().join(format!("toolbar-tests-{}.log", std::process::id()));
    let (sink, _events) = bridge::channel();
    Arc::new(DpsMeter::new(Arc::new(sink), Arc::new(AppLogger::new(log, Box::new(|_| {}))), None))
}

/// The real toolbar in GPUI's test platform, on in-memory settings, with a
/// hub whose meter has a real core.
fn open(cx: &mut TestAppContext, settings: AppSettings) -> Bar<'_> {
    let hub = cx.update(|cx| {
        let store = cx.new(|_| Settings::in_memory(settings));
        crate::settings::install(&store, cx);
        crate::reminders::install(&store, false, cx);
        todos::install(&store, false, cx);
        let model = cx.new(|_| MeterModel::new(OverlaySettings::default(), Some(meter_core())));
        let meter_window = cx
            .open_window(Default::default(), |_, cx| cx.new(|cx| MeterView::new(model.clone(), cx)))
            .expect("the meter's window");
        install_for_test(meter_window, model, cx)
    });
    let view_hub = hub.clone();
    let (view, cx) = cx.add_window_view(|window, cx| ToolbarView::new(view_hub, window, cx));
    let requests: Rc<RefCell<Vec<ToolbarRequest>>> = Rc::default();
    let heard = requests.clone();
    cx.update(|_, cx| {
        cx.subscribe(&view, move |_, request: &ToolbarRequest, _| heard.borrow_mut().push(*request)).detach();
    });
    cx.run_until_parked();
    Bar { view, hub, requests, cx }
}

impl Bar<'_> {
    fn centre(&mut self, selector: &'static str) -> gpui::Point<gpui::Pixels> {
        self.cx.debug_bounds(selector).unwrap_or_else(|| panic!("{selector} is not drawn")).center()
    }

    fn click(&mut self, selector: &'static str) {
        let at = self.centre(selector);
        self.cx.simulate_click(at, Modifiers::none());
        self.cx.run_until_parked();
    }

    fn is_drawn(&mut self, selector: &'static str) -> bool {
        self.cx.debug_bounds(selector).is_some()
    }

    fn view<R>(&mut self, read: impl FnOnce(&ToolbarView) -> R) -> R {
        let view = self.view.clone();
        self.cx.update(|_, cx| read(view.read(cx)))
    }

    fn requests(&self) -> Vec<ToolbarRequest> {
        self.requests.borrow().clone()
    }

    fn press(&mut self, key: &str) {
        let keystroke = Keystroke::parse(key).expect("a key name");
        self.cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
        self.cx.simulate_event(KeyUpEvent { keystroke });
        self.cx.run_until_parked();
    }

    fn fight_tracking(&mut self) -> bool {
        let hub = self.hub.clone();
        self.cx.update(|_, cx| hub.read(cx).meter_model().read(cx).meter().expect("a core").fight_tracking())
    }
}

#[gpui::test]
fn a_click_on_a_tool_asks_for_it_and_never_takes_the_keyboard(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.click("tool-history");
    assert_eq!(bar.requests(), vec![ToolbarRequest::Toggle(Tool::History)]);
    let focused = bar.cx.update(|window, cx| window.focused(cx));
    assert!(focused.is_none(), "the press gave the keyboard to the toolbar");

    let toolbar = bar.cx.debug_bounds("toolbar").expect("the bar");
    bar.cx.simulate_click(toolbar.origin + gpui::point(px(8.), px(10.)), Modifiers::none());
    let focused = bar.cx.update(|window, cx| window.focused(cx));
    assert!(focused.is_none(), "a press on the grip took the keyboard");
}

#[gpui::test]
fn a_tooltip_comes_500_ms_after_the_pointer_enters_and_moves_inside_change_nothing(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    let centre = bar.centre("tool-pvp");
    bar.cx.simulate_mouse_move(centre, None, Modifiers::none());
    bar.cx.executor().advance_clock(Duration::from_millis(300));
    bar.cx.simulate_mouse_move(centre + gpui::point(px(3.), px(2.)), None, Modifiers::none());
    bar.cx.executor().advance_clock(Duration::from_millis(250));
    bar.cx.run_until_parked();
    assert_eq!(bar.view(|view| view.tip()), Some(Slot::Tool(Tool::Pvp)), "the move inside restarted the wait");

    let away = bar.centre("tool-meter");
    bar.cx.simulate_mouse_move(away, None, Modifiers::none());
    bar.cx.run_until_parked();
    assert_eq!(bar.view(|view| view.tip()), None, "the tooltip stayed after the leave");
}

#[gpui::test]
fn the_bell_opens_its_dropdown_a_second_click_closes_it_and_pause_asks_for_the_switch(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.click("tool-reminders");
    assert_eq!(bar.view(|view| view.dropdown()), Some(Dropdown::Bell));
    assert!(bar.is_drawn("bell-dropdown"));

    bar.click("tool-reminders");
    assert_eq!(bar.view(|view| view.dropdown()), None);
    assert!(bar.requests().is_empty(), "the bell asks the app for nothing");

    bar.click("tool-reminders");
    bar.click("pause-reminders");
    assert_eq!(bar.requests(), vec![ToolbarRequest::PauseReminders(true)]);
    assert_eq!(bar.view(|view| view.dropdown()), None);
}

#[gpui::test]
fn a_press_elsewhere_closes_a_dropdown_and_reaches_nothing(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.click("tool-reminders");
    bar.click("tool-history");
    assert_eq!(bar.view(|view| view.dropdown()), None);
    assert!(bar.requests().is_empty(), "the press under the dropdown's cover opened History");
}

#[gpui::test]
fn fold_leaves_a_handle_and_unfold_brings_the_bar_back(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.click("fold");
    assert!(bar.view(|view| view.is_folded()));
    assert!(!bar.is_drawn("tool-meter") && bar.is_drawn("unfold"));
    bar.click("unfold");
    assert!(!bar.view(|view| view.is_folded()));
    assert!(bar.is_drawn("tool-meter"));
}

#[gpui::test]
fn quit_asks_to_end_the_app_at_once(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.click("quit");
    assert_eq!(bar.requests(), vec![ToolbarRequest::Quit]);
}

#[gpui::test]
fn a_count_asks_for_the_resources_window(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.click("count-shugo");
    assert_eq!(bar.requests(), vec![ToolbarRequest::ToggleResources]);
}

#[gpui::test]
fn with_the_keyboard_given_right_moves_along_the_bar_and_enter_presses(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    let view = bar.view.clone();
    bar.cx.update(|window, cx| window.focus(&view.read(cx).focus_handle(), cx));
    bar.press("right");
    bar.press("right");
    bar.press("enter");
    assert_eq!(bar.requests(), vec![ToolbarRequest::Toggle(Tool::Pvp)]);
    bar.press("left");
    bar.press("space");
    assert_eq!(bar.requests().last(), Some(&ToolbarRequest::Toggle(Tool::Meter)));
}

#[gpui::test]
fn the_meter_icon_lights_while_the_meter_is_open_and_fights_are_tracked_only_then(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.cx.update(|_, cx| {
        if let Some(hub) = hub(cx) {
            let model = hub.read(cx).meter_model().clone();
            if let Some(meter) = model.read(cx).meter() {
                meter.set_fight_tracking(false);
            }
        }
    });
    assert!(!bar.fight_tracking());

    bar.cx.update(|_, cx| handle(ToolbarRequest::Toggle(Tool::Meter), cx));
    bar.cx.run_until_parked();
    assert!(bar.cx.update(|_, cx| meter_is_open(cx)));
    assert!(bar.fight_tracking(), "opening the meter did not start tracking fights");
    let hub = bar.hub.clone();
    let lit = bar.cx.update(|_, cx| hub.read(cx).is_lit(Tool::Meter, cx));
    assert!(lit);

    bar.cx.update(|_, cx| close_meter(cx));
    bar.cx.run_until_parked();
    assert!(!bar.cx.update(|_, cx| meter_is_open(cx)));
    assert!(!bar.fight_tracking(), "closing the meter left fights tracked");
}

#[test]
fn the_launch_prompt_is_due_until_answered_and_never_when_the_meter_opens_at_launch() {
    let mut settings = AppSettings::default();
    assert!(launch_prompt_due(&settings), "a first open asks");
    settings.windows.open_meter_at_launch = true;
    assert!(!launch_prompt_due(&settings), "the meter already opens at launch");
    settings.windows.open_meter_at_launch = false;
    settings.windows.launch_prompt_answered = true;
    assert!(!launch_prompt_due(&settings), "Not now is final");
}

fn open_prompt(cx: &mut TestAppContext) -> (Entity<Settings>, Entity<PromptView>, &mut VisualTestContext) {
    let store = cx.update(|cx| {
        let store = cx.new(|_| Settings::in_memory(AppSettings::default()));
        crate::settings::install(&store, cx);
        store
    });
    let (view, cx) = cx.add_window_view(|_, cx| PromptView::unplaced(cx));
    (store, view, cx)
}

/// (answered, opens at launch), read without the prompt's window, which Not
/// now closes.
fn answered(store: &Entity<Settings>, cx: &mut VisualTestContext) -> (bool, bool) {
    store.read_with(cx, |settings, _| {
        let windows = &settings.get().windows;
        (windows.launch_prompt_answered, windows.open_meter_at_launch)
    })
}

#[gpui::test]
fn yes_turns_the_setting_on_says_so_and_is_final(cx: &mut TestAppContext) {
    let (store, view, cx) = open_prompt(cx);
    let yes = cx.debug_bounds("prompt-yes").expect("the Yes button").center();
    cx.simulate_click(yes, Modifiers::none());
    assert_eq!(answered(&store, cx), (true, true));
    assert!(cx.update(|_, cx| view.read(cx).is_done()));
    assert!(cx.debug_bounds("prompt-done").is_some(), "no note after Yes");
    assert!(!store.read_with(cx, |settings, _| launch_prompt_due(settings.get())));
}

#[gpui::test]
fn not_now_answers_without_turning_the_setting_on(cx: &mut TestAppContext) {
    let (store, _, cx) = open_prompt(cx);
    let no = cx.debug_bounds("prompt-no").expect("the Not now button").center();
    cx.simulate_click(no, Modifiers::none());
    assert_eq!(answered(&store, cx), (true, false));
    assert!(!store.read_with(cx, |settings, _| launch_prompt_due(settings.get())));
}
