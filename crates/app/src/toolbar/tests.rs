use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    px, AppContext as _, Entity, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext,
};
use meter_core::engine::meter::DpsMeter;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;

use super::view::{Dropdown, Slot};
use super::*;
use crate::bridge;
use crate::meter::OverlaySettings;
use crate::settings::{AppSettings, Settings};
use crate::todos;
use crate::update::{Failure, ReleaseInfo, Snapshot, UpdateState};

struct Bar<'a> {
    view: Entity<ToolbarView>,
    hub: Entity<Hub>,
    requests: Rc<RefCell<Vec<ToolbarRequest>>>,
    /// What the core's own threads emit through: events reach the model
    /// through the bridge, as in the app.
    core_events: Arc<bridge::ChannelSink>,
    store: Entity<Settings>,
    /// What the update icon's clicks asked the updates for.
    update_commands: std::sync::mpsc::Receiver<crate::update::Command>,
    cx: &'a mut VisualTestContext,
}

/// The real toolbar in GPUI's test platform, on in-memory settings, with a
/// hub whose meter has a real core, not started, its events pumped into the
/// meter's model.
fn open(cx: &mut TestAppContext, settings: AppSettings) -> Bar<'_> {
    let checked = std::time::SystemTime::now();
    let up_to_date = Snapshot { state: UpdateState::UpToDate { checked_at: checked }, last_checked: Some(checked), check_again_at: checked };
    open_with_updates(cx, settings, up_to_date)
}

fn open_with_updates(cx: &mut TestAppContext, settings: AppSettings, updates: Snapshot) -> Bar<'_> {
    let (sink, events) = bridge::channel();
    let core_events = Arc::new(sink);
    let log = std::env::temp_dir().join(format!("toolbar-tests-{}.log", std::process::id()));
    let core = Arc::new(DpsMeter::new(core_events.clone(), Arc::new(AppLogger::new(log, Box::new(|_| {}))), None));
    let (hub, store, update_commands) = cx.update(|cx| {
        let store = cx.new(|_| Settings::in_memory(settings));
        crate::settings::install(&store, cx);
        crate::reminders::install(&store, false, cx);
        todos::install(&store, false, cx);
        let update_commands = crate::update::install_recording_fixture(&store, updates, cx);
        let model = cx.new(|_| MeterModel::new(OverlaySettings::default(), Some(core)));
        bridge::spawn_pump(model.clone(), events, false, cx).detach();
        let meter_window = cx
            .open_window(Default::default(), |_, cx| cx.new(|cx| MeterView::new(model.clone(), cx)))
            .expect("the meter's window");
        (install_for_test(meter_window, model, cx), store, update_commands)
    });
    let view_hub = hub.clone();
    let (view, cx) = cx.add_window_view(|window, cx| ToolbarView::new(view_hub, window, cx));
    let requests: Rc<RefCell<Vec<ToolbarRequest>>> = Rc::default();
    let heard = requests.clone();
    cx.update(|_, cx| {
        cx.subscribe(&view, move |_, request: &ToolbarRequest, _| heard.borrow_mut().push(*request)).detach();
    });
    cx.run_until_parked();
    Bar { view, hub, requests, core_events, store, update_commands, cx }
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

    /// The core's snapshot loop saying whether you are between fights.
    fn core_says_idle(&mut self, idle: bool) {
        self.core_events.emit(MeterEvent::DpsIdleHidden(idle));
        self.cx.run_until_parked();
    }

    fn meter_shown(&mut self) -> bool {
        let hub = self.hub.clone();
        self.cx.update(|_, cx| hub.read(cx).meter_is_shown())
    }

    fn open_meter(&mut self) {
        self.cx.update(|_, cx| open_meter(cx));
        self.cx.run_until_parked();
    }

    fn click_meter_icon(&mut self) {
        self.cx.update(|_, cx| handle(ToolbarRequest::Toggle(Tool::Meter), cx));
        self.cx.run_until_parked();
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

#[gpui::test]
fn an_open_meter_steps_aside_while_the_core_says_idle_and_keeps_tracking_fights(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.open_meter();
    assert!(bar.meter_shown());

    bar.core_says_idle(true);
    assert!(!bar.meter_shown(), "between fights the window steps aside");
    assert!(bar.cx.update(|_, cx| meter_is_open(cx) && meter_is_aside(cx)));
    assert!(bar.fight_tracking(), "hidden is not closed: fights are still tracked");

    bar.core_says_idle(false);
    assert!(bar.meter_shown(), "your first hit brings it back");
}

#[gpui::test]
fn a_fight_summary_keeps_the_meter_on_screen_until_it_comes_down(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.open_meter();
    for event in crate::meter::a_kill() {
        bar.core_events.emit(event);
    }
    bar.core_says_idle(true);
    assert!(bar.meter_shown(), "the kill's summary is up to be read");

    let hub = bar.hub.clone();
    bar.cx.update(|_, cx| {
        let model = hub.read(cx).meter_model().clone();
        model.update(cx, |model, cx| model.dismiss_summary(cx));
    });
    bar.cx.run_until_parked();
    assert!(!bar.meter_shown(), "with the summary gone it steps aside");
}

#[gpui::test]
fn opening_the_meter_shows_it_for_a_moment_even_between_fights(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.core_says_idle(true);
    assert!(!bar.meter_shown());

    bar.open_meter();

    assert!(bar.meter_shown(), "the core's peek brought it up");
    assert!(!bar.cx.update(|_, cx| meter_is_aside(cx)));
}

#[gpui::test]
fn the_meter_icon_shows_a_meter_that_steps_aside_and_closes_one_on_screen(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.open_meter();
    bar.core_says_idle(true);

    bar.click_meter_icon();
    assert!(bar.cx.update(|_, cx| meter_is_open(cx)), "the click peeked instead of closing");
    assert!(bar.meter_shown());
    assert!(bar.fight_tracking());

    bar.click_meter_icon();
    assert!(!bar.cx.update(|_, cx| meter_is_open(cx)), "on screen, the icon closes it as before");
    assert!(!bar.meter_shown());
}

#[gpui::test]
fn showing_everything_again_leaves_a_meter_that_steps_aside_hidden(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    bar.open_meter();
    bar.cx.update(|_, cx| toggle_everything(cx));
    assert!(!bar.meter_shown(), "Alt+Shift+CapsLock hides the meter with the rest");

    bar.core_says_idle(false);
    assert!(!bar.meter_shown(), "a hit while everything is hidden shows nothing");
    bar.core_says_idle(true);
    bar.cx.update(|_, cx| toggle_everything(cx));
    assert!(!bar.meter_shown(), "between fights it stays aside");

    bar.core_says_idle(false);
    assert!(bar.meter_shown());
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

#[gpui::test]
fn the_prompt_redraws_for_a_new_look_not_for_other_settings(cx: &mut TestAppContext) {
    let (store, view, cx) = open_prompt(cx);
    let redraws = Rc::new(std::cell::Cell::new(0));
    let counter = redraws.clone();
    cx.update(|_, cx| cx.observe(&view, move |_, _| counter.set(counter.get() + 1)).detach());

    store.update(cx, |settings, cx| settings.update(cx, |all| all.windows.toolbar_at = Some(crate::settings::ScreenPoint { x: 5, y: 5 })));
    cx.run_until_parked();
    assert_eq!(redraws.get(), 0, "the toolbar moved");
    store.update(cx, |settings, cx| settings.update(cx, |all| all.overlay.background_alpha = 40));
    cx.run_until_parked();
    assert_eq!(redraws.get(), 1, "the card's look changed");
}

const NEXT: crate::changelog::Version = crate::changelog::Version::new(9, 0, 0);

fn updates_at(state: UpdateState) -> Snapshot {
    let now = std::time::SystemTime::now();
    Snapshot { state, last_checked: Some(now), check_again_at: now }
}

fn available(last_failure: Option<Failure>) -> UpdateState {
    let release = ReleaseInfo { version: NEXT, size: 4_200_000, notes_url: "https://github.com/o/r".into(), published: None };
    UpdateState::Available { release, last_failure }
}

impl Bar<'_> {
    fn show_updates(&mut self, state: UpdateState) {
        let store = self.store.clone();
        self.cx.update(|_, cx| crate::update::install_fixture(&store, updates_at(state), cx));
        self.cx.run_until_parked();
    }

    fn commands(&self) -> Vec<String> {
        self.update_commands.try_iter().map(|command| format!("{command:?}")).collect()
    }

    fn rest_on(&mut self, selector: &'static str) -> Option<(gpui::SharedString, Option<gpui::SharedString>)> {
        let centre = self.centre(selector);
        self.cx.simulate_mouse_move(centre, None, Modifiers::none());
        self.cx.executor().advance_clock(Duration::from_millis(500));
        self.cx.run_until_parked();
        let view = self.view.clone();
        self.cx.update(|_, cx| view.read(cx).tip_lines(cx))
    }
}

#[gpui::test]
fn the_update_icon_shows_only_while_there_is_something_to_do_or_know(cx: &mut TestAppContext) {
    let mut bar = open(cx, AppSettings::default());
    let now = std::time::SystemTime::now();
    for (state, shown) in [
        (UpdateState::UpToDate { checked_at: now }, false),
        (UpdateState::Checking, false),
        (UpdateState::Unreachable { next_try: now }, false),
        (UpdateState::RateLimited { until: now }, false),
        (UpdateState::Failed { failure: Failure::Check("odd".into()) }, false),
        (UpdateState::Disabled, false),
        (available(None), true),
        (available(Some(Failure::SignatureMismatch)), true),
        (UpdateState::Downloading { version: NEXT, downloaded: 1, total: 2, paused_for_fight: false }, true),
        (UpdateState::Downloading { version: NEXT, downloaded: 1, total: 2, paused_for_fight: true }, true),
        (UpdateState::Verifying { version: NEXT }, true),
        (UpdateState::Ready { version: NEXT }, true),
    ] {
        let label = format!("{state:?}");
        bar.show_updates(state);
        assert_eq!(bar.is_drawn("update"), shown, "{label}");
    }

    let (shugo, update, fold) = (bar.centre("count-shugo"), bar.centre("update"), bar.centre("fold"));
    assert!(shugo.x < update.x && update.x < fold.x, "first in the end group, before fold");
    bar.click("fold");
    assert!(bar.is_drawn("update"), "the folded handle keeps it");
    assert!(bar.centre("unfold").x < bar.centre("update").x);
}

#[gpui::test]
fn a_click_on_the_update_icon_downloads_opens_the_details_or_restarts_and_never_takes_the_keyboard(cx: &mut TestAppContext) {
    let mut bar = open_with_updates(cx, AppSettings::default(), updates_at(available(None)));
    bar.click("update");
    assert_eq!(bar.requests(), vec![ToolbarRequest::Update(UpdateClick::StartDownload)]);
    let focused = bar.cx.update(|window, cx| window.focused(cx));
    assert!(focused.is_none(), "the press gave the keyboard to the toolbar");
    bar.cx.update(|_, cx| handle(ToolbarRequest::Update(UpdateClick::StartDownload), cx));
    assert_eq!(bar.commands(), vec!["StartDownload"], "the download starts at once, no confirmation");

    bar.requests.borrow_mut().clear();
    for state in [
        UpdateState::Downloading { version: NEXT, downloaded: 1, total: 2, paused_for_fight: false },
        UpdateState::Downloading { version: NEXT, downloaded: 1, total: 2, paused_for_fight: true },
        UpdateState::Verifying { version: NEXT },
    ] {
        bar.show_updates(state);
        bar.click("update");
    }
    assert_eq!(bar.requests(), vec![ToolbarRequest::Update(UpdateClick::ShowDetails); 3], "Settings › About has the details");

    bar.requests.borrow_mut().clear();
    bar.show_updates(UpdateState::Ready { version: NEXT });
    bar.click("update");
    assert_eq!(bar.requests(), vec![ToolbarRequest::Update(UpdateClick::Restart)]);
    bar.cx.update(|_, cx| handle(ToolbarRequest::Update(UpdateClick::Restart), cx));
    let restart_tried = bar.cx.update(|_, cx| crate::update::global(cx).read(cx).restart_failure().is_some());
    assert!(restart_tried, "the restart was asked for at once (it cannot start a staged exe in a test)");
}

#[gpui::test]
fn the_update_tooltip_comes_after_500_ms_and_warns_that_restarting_splits_a_fight(cx: &mut TestAppContext) {
    let mut bar = open_with_updates(cx, AppSettings::default(), updates_at(available(None)));
    let (name, line) = bar.rest_on("update").expect("the tooltip is up");
    assert_eq!(name.as_ref(), "Update available: 9.0.0");
    assert_eq!(line.as_deref(), Some("Click to download it. It installs only when you restart."));

    bar.show_updates(UpdateState::Downloading { version: NEXT, downloaded: 62, total: 100, paused_for_fight: false });
    let (name, _) = bar.rest_on("update").expect("the tooltip is up");
    assert_eq!(name.as_ref(), "Downloading 9.0.0 · 62%");

    bar.show_updates(UpdateState::Ready { version: NEXT });
    let view = bar.view.clone();
    bar.cx.update(|_, cx| view.update(cx, |view, cx| view.force_fight(Some(false), cx)));
    let (_, line) = bar.rest_on("update").expect("the tooltip is up");
    assert_eq!(line.as_deref(), Some("Closes and opens again in the new version. Nothing is lost."));
    bar.cx.update(|_, cx| view.update(cx, |view, cx| view.force_fight(Some(true), cx)));
    let (name, line) = bar.rest_on("update").expect("the tooltip is up");
    assert_eq!(name.as_ref(), "Restart to update to 9.0.0");
    assert_eq!(line.as_deref(), Some("You are fighting: restarting now splits this fight in History."));
}

#[gpui::test]
fn when_the_update_icon_goes_the_keyboard_moves_to_fold(cx: &mut TestAppContext) {
    let mut bar = open_with_updates(cx, AppSettings::default(), updates_at(UpdateState::Ready { version: NEXT }));
    let view = bar.view.clone();
    bar.cx.update(|window, cx| window.focus(&view.read(cx).focus_handle(), cx));
    bar.press("end");
    bar.press("left");
    bar.press("left");
    bar.press("enter");
    assert_eq!(bar.requests(), vec![ToolbarRequest::Update(UpdateClick::Restart)], "Left reaches it like the other tools");

    bar.press("left");
    bar.press("right");
    bar.show_updates(UpdateState::UpToDate { checked_at: std::time::SystemTime::now() });
    bar.press("enter");
    assert!(bar.view(|view| view.is_folded()), "the keyboard went to fold, the next control");
}
