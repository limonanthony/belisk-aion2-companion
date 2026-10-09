//! The windows besides the meter and the toolbar: fight history, player
//! detail, the fight report, PvP, the to-dos, the resources, Settings, the
//! first-open prompt, the event reminder and What's new.
//!
//! Each kind has at most one window. Opening one that is already open brings
//! it forward, and the player detail window is pointed at the new player
//! instead, as the report window is at another fight; a newer reminder takes
//! the place of the one up. A window closes with its own ×, from the toolbar,
//! or when the app quits.
//! Like the meter's, the history, detail, report, PvP, to-do, resources,
//! prompt, reminder and What's new windows are frameless, topmost, out of the
//! taskbar and non-activating, and are exactly as large as their card. A tool
//! opens beside the meter while the meter is open, else under the toolbar.
//! The prompt keeps to the meter, and the reminder card keeps above the meter
//! (or the prompt on it) while the meter is open, and under the toolbar while
//! it is closed or steps aside between fights. What's new keeps under the
//! toolbar.
//! Settings is a normal window instead: it takes the keyboard focus, has a
//! taskbar button and opens in the middle of the screen. While it is the
//! active window it is topmost like the overlays, in front of the game; once
//! the user goes to another program it goes back among the normal windows.
//! The lock (Alt+CapsLock) applies to the meter, and to the reminder sitting
//! on it.

pub mod chrome;
mod monitor;
mod placement;
mod registry;
pub mod text_input;
pub mod typing;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyEntity, AnyWindowHandle, App, AppContext as _, Context, Entity, Global, Pixels, Render, Size, Window,
    WindowHandle,
};
use meter_core::report::fight_report::FightReport;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::changelog::WhatsNew;
use crate::detail::{DetailModel, DetailSource, DetailView};
use crate::history::{HistoryModel, HistoryView};
use crate::meter::{self, MeterModel, MeterView};
use crate::overlay::{self, Placement};
use crate::pvp::{PvpModel, PvpView};
use crate::reminders::{self, MeterLink, ReminderView, ShowReminder};
use crate::report::{build_off_thread, ReportModel, ReportRequest, ReportView};
use crate::settings::{self, DetailPlacement, Page, RuntimeModel, Section, SettingsView};
use crate::todos::{Contents, TodoView};
use crate::toolbar::{self, PromptView};
use crate::whats_new::{self, OpenChangelog, WhatsNewView};
pub(crate) use monitor::window_and_work_area;
use placement::{Anchor, Side, Stack};
pub use placement::Rect;
pub use registry::WindowKind;
use registry::{open_once, Opened, Registry};

const HISTORY_SIZE: (f32, f32) = (370.0, 420.0);
const DETAIL_SIZE: (f32, f32) = (520.0, 420.0);
const REPORT_SIZE: (f32, f32) = (520.0, 420.0);
const PVP_SIZE: (f32, f32) = (340.0, 420.0);
/// The tallest the to-do window gets at 100%; it fits its card from the
/// first frame on. The Resources window, the same view with fewer rows,
/// starts from it too.
const TODOS_SIZE: (f32, f32) = (370.0, 580.0);

#[derive(Clone)]
struct Open {
    window: AnyWindowHandle,
    /// None only for a window whose handle could not be read.
    hwnd: Option<HWND>,
    /// What the window shows, kept by the registry; none when the view is
    /// all there is, so closing the window releases it.
    model: Option<AnyEntity>,
}

struct Secondary {
    registry: Rc<RefCell<Registry<Open>>>,
    meter_hwnd: HWND,
    placement: Placement,
    changes: Entity<OpenWindows>,
}

impl Global for Secondary {}

/// Notifies whenever a window of this layer opens or closes, for the
/// toolbar's lit icons; [`is_open`] says which are open.
pub struct OpenWindows;

/// Sets the layer up once the meter's window exists: the windows are placed
/// next to it, the summary card's report button opens the report, and all of
/// them close when the app quits.
pub fn init(
    meter: &WindowHandle<MeterView>,
    meter_model: &Entity<MeterModel>,
    placement: Placement,
    cx: &mut App,
) -> Result<(), String> {
    let meter_hwnd = meter
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("cannot reach the meter window: {error:#}"))??;
    let meter_view = meter.entity(cx).map_err(|error| format!("cannot reach the meter view: {error:#}"))?;
    text_input::bind_keys(cx);
    let registry = Rc::new(RefCell::new(Registry::default()));
    let changes = cx.new(|_| OpenWindows);
    cx.set_global(Secondary { registry: registry.clone(), meter_hwnd, placement, changes });
    cx.subscribe(&reminders::global(cx), |_, request: &ShowReminder, cx| show_reminder(request, cx)).detach();
    let model = meter_model.clone();
    cx.subscribe(&meter_view, move |_, request: &ReportRequest, cx| open_report(&model, request, cx)).detach();
    cx.on_app_quit(move |cx| {
        let open = registry.borrow_mut().drain();
        for open in open {
            let _ = open.window.update(cx, |_, window, _| window.remove_window());
        }
        async {}
    })
    .detach();
    Ok(())
}

/// The entity that notifies as windows of this layer open and close; none
/// before [`init`].
pub fn open_windows(cx: &App) -> Option<Entity<OpenWindows>> {
    cx.try_global::<Secondary>().map(|secondary| secondary.changes.clone())
}

pub fn is_open(kind: WindowKind, cx: &App) -> bool {
    cx.try_global::<Secondary>().is_some_and(|secondary| secondary.registry.borrow().get(kind).is_some())
}

/// Closes the window of `kind`, if one is open.
pub fn close(kind: WindowKind, cx: &mut App) {
    let Some(open) = cx.try_global::<Secondary>().and_then(|secondary| secondary.registry.borrow().get(kind).cloned()) else {
        return;
    };
    if let Err(error) = open.window.update(cx, |_, window, _| window.remove_window()) {
        crate::app_log::warn(format!("cannot close the {kind:?} window: {error:#}"));
    }
}

/// The native handle of the open window of `kind`. None while a window is
/// being opened: its first frame is drawn before the registry keeps it, and
/// a card placing itself then tries again at its next tick.
pub fn hwnd_of(kind: WindowKind, cx: &App) -> Option<HWND> {
    cx.try_global::<Secondary>()?.registry.borrow().get(kind)?.hwnd
}

/// The native handles of every open window of this layer.
pub fn open_hwnds(cx: &App) -> Vec<HWND> {
    cx.try_global::<Secondary>()
        .map(|secondary| secondary.registry.borrow().all().filter_map(|open| open.hwnd).collect())
        .unwrap_or_default()
}

fn changed(cx: &mut App) {
    if let Some(changes) = open_windows(cx) {
        changes.update(cx, |_, cx| cx.notify());
    }
}

/// The window a tool opens against, with where: beside the meter while it is
/// open, else under the toolbar.
fn tool_anchor(preferred: Anchor, cx: &App) -> (HWND, Anchor) {
    let meter_hwnd = cx.global::<Secondary>().meter_hwnd;
    if toolbar::meter_is_open(cx) {
        return (meter_hwnd, preferred);
    }
    match (toolbar::toolbar_hwnd(cx), preferred) {
        (Some(toolbar), Anchor::Beside(_)) => (toolbar, Anchor::Beside(Side::Below)),
        (Some(toolbar), anchor) => (toolbar, anchor),
        (None, anchor) => (meter_hwnd, anchor),
    }
}

/// A card stacked on another window and kept there as both move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    /// The first-open prompt, on the meter.
    Prompt,
    /// The reminder card: on the meter (or the prompt on it) while the meter
    /// is open and not stepped aside between fights, else under the toolbar.
    Reminder,
    /// What's new: under the toolbar, wherever the meter is.
    WhatsNew,
}

fn card_anchor(card: Card, cx: &App) -> Option<(HWND, Stack)> {
    let meter_hwnd = cx.try_global::<Secondary>()?.meter_hwnd;
    match card {
        Card::Prompt => Some((meter_hwnd, Stack::Above)),
        Card::Reminder if toolbar::meter_is_open(cx) && !toolbar::meter_is_aside(cx) => {
            Some((hwnd_of(WindowKind::Prompt, cx).unwrap_or(meter_hwnd), Stack::Above))
        }
        Card::Reminder | Card::WhatsNew => Some((toolbar::toolbar_hwnd(cx).unwrap_or(meter_hwnd), Stack::Below)),
    }
}

pub fn open_history(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let registry = registry(cx);
    let meter = meter_model.read(cx).meter().cloned();
    let place = tool_anchor(Anchor::Beside(Side::Below), cx);
    let opened = open_once(&registry, WindowKind::History, || {
        let model = cx.new(|cx| HistoryModel::new(meter, meter_model, cx));
        let meter_model = meter_model.clone();
        let view_model = model.clone();
        let (window, hwnd) = create_window(
            WindowKind::History,
            HISTORY_SIZE,
            place,
            move |window, cx| {
                let view = cx.new(|cx| HistoryView::new(view_model, meter_model.clone(), window, cx));
                cx.subscribe(&view, move |_, request: &ReportRequest, cx| open_report(&meter_model, request, cx)).detach();
                view
            },
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: Some(model.into()) })
    });
    finish(WindowKind::History, opened, cx);
}

/// Opens the fight report `request` asks for, or puts it in the open report
/// window. A saved record's report and the meter's are built off the UI
/// thread; the window opens when the report is ready.
pub fn open_report(meter_model: &Entity<MeterModel>, request: &ReportRequest, cx: &mut App) {
    match request {
        ReportRequest::Saved(record) => {
            let record = record.clone();
            let meter_model = meter_model.clone();
            let record_id = record.id.clone();
            build_off_thread(
                move || FightReport::from_record(&record),
                move |report, cx| match report {
                    Some(report) => show_report(&meter_model, Arc::new(report), cx),
                    None => crate::app_log::warn(format!("record {record_id} is not a boss fight's, so it has no fight report")),
                },
                cx,
            );
        }
        ReportRequest::Held(report) => show_report(meter_model, report.clone(), cx),
        ReportRequest::OnMeter { target_id } => {
            let target_id = *target_id;
            let Some(meter) = meter_model.read(cx).meter().cloned() else {
                // Without a meter (dev captures) the report is the fight the
                // overlay shows, with nothing recorded beyond it.
                let report = meter_model
                    .read(cx)
                    .snapshot()
                    .and_then(|snapshot| FightReport::from_snapshot(snapshot, target_id, None));
                match report {
                    Some(report) => show_report(meter_model, Arc::new(report), cx),
                    None => crate::app_log::warn(format!("no fight report for target {target_id}: the overlay has no boss fight on it")),
                }
                return;
            };
            let meter_model = meter_model.clone();
            build_off_thread(
                move || meter.fight_report(target_id),
                move |report, cx| match report {
                    Some(report) => show_report(&meter_model, Arc::new(report), cx),
                    None => crate::app_log::warn(format!("no fight report for target {target_id}: the meter has no boss fight on it")),
                },
                cx,
            );
        }
    }
}

/// Shows `report` in the report window, opening it if needed.
pub fn show_report(meter_model: &Entity<MeterModel>, report: Arc<FightReport>, cx: &mut App) {
    let registry = registry(cx);
    let place = tool_anchor(Anchor::Beside(Side::Right), cx);
    let opened = open_once(&registry, WindowKind::Report, || {
        let model = cx.new(|cx| ReportModel::new(meter_model.clone(), report.clone(), cx));
        let view_model = model.clone();
        let (window, hwnd) = create_window(
            WindowKind::Report,
            REPORT_SIZE,
            place,
            move |window, cx| cx.new(|cx| ReportView::new(view_model, window, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: Some(model.into()) })
    });
    if opened == Ok(Opened::Existing) {
        let existing = registry.borrow().get(WindowKind::Report).and_then(|open| open.model.clone());
        if let Some(model) = existing.and_then(|model| model.downcast::<ReportModel>().ok()) {
            model.update(cx, |model, cx| model.show(report, cx));
        }
    }
    finish(WindowKind::Report, opened, cx);
}

pub fn open_pvp(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let registry = registry(cx);
    let place = tool_anchor(Anchor::Beside(Side::Left), cx);
    let opened = open_once(&registry, WindowKind::Pvp, || {
        let meter = meter_model.read(cx).meter().cloned();
        let model = cx.new(|cx| PvpModel::new(meter_model.clone(), cx));
        let view_model = model.clone();
        let pvp_mode_to_reset = meter.clone();
        let (window, hwnd) = create_window(
            WindowKind::Pvp,
            PVP_SIZE,
            place,
            move |window, cx| cx.new(|cx| PvpView::new(view_model, window, cx)),
            move |_| {
                if let Some(meter) = &pvp_mode_to_reset {
                    meter.set_pvp_mode(false);
                }
            },
            cx,
        )?;
        if let Some(meter) = &meter {
            meter.set_pvp_mode(true);
        }
        Ok(Open { window, hwnd: Some(hwnd), model: Some(model.into()) })
    });
    finish(WindowKind::Pvp, opened, cx);
}

pub fn open_todos(meter_model: &Entity<MeterModel>, cx: &mut App) {
    open_todo_view(WindowKind::Todos, Contents::Todos, Side::Right, meter_model, cx);
}

/// The Resources window: every resource the to-dos track, opened below the
/// meter so it and the to-do window can be open side by side.
pub fn open_resources(meter_model: &Entity<MeterModel>, cx: &mut App) {
    open_todo_view(WindowKind::Resources, Contents::Resources, Side::Below, meter_model, cx);
}

fn open_todo_view(kind: WindowKind, contents: Contents, side: Side, meter_model: &Entity<MeterModel>, cx: &mut App) {
    let registry = registry(cx);
    let place = tool_anchor(Anchor::Beside(side), cx);
    let opened = open_once(&registry, kind, || {
        let meter_model = meter_model.clone();
        let (window, hwnd) = create_window(
            kind,
            TODOS_SIZE,
            place,
            move |window, cx| cx.new(|cx| TodoView::new(contents, meter_model, window, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: None })
    });
    finish(kind, opened, cx);
}

/// Opens the player detail window on `source`, or points the open one at it.
pub fn open_detail(meter_model: &Entity<MeterModel>, source: DetailSource, cx: &mut App) {
    let registry = registry(cx);
    let anchor = match settings::global(cx).read(cx).get().windows.detail_opens {
        DetailPlacement::Beside => Anchor::Beside(Side::Right),
        DetailPlacement::Centred => Anchor::Centre,
    };
    let place = tool_anchor(anchor, cx);
    let opened = open_once(&registry, WindowKind::Detail, || {
        let model = cx.new(|cx| DetailModel::new(meter_model.clone(), source.clone(), cx));
        let view_model = model.clone();
        let (window, hwnd) = create_window(
            WindowKind::Detail,
            DETAIL_SIZE,
            place,
            move |_, cx| cx.new(|cx| DetailView::new(view_model, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: Some(model.into()) })
    });
    if opened == Ok(Opened::Existing) {
        let existing = registry.borrow().get(WindowKind::Detail).and_then(|open| open.model.clone());
        if let Some(model) = existing.and_then(|model| model.downcast::<DetailModel>().ok()) {
            model.update(cx, |model, cx| model.select(source, cx));
        }
    }
    finish(WindowKind::Detail, opened, cx);
}

/// Opens Settings, or brings the open window to the front. It happens once
/// the current event is over: the window reads the meter's lock, and the
/// meter's window is still busy with the menu click that asked for Settings.
pub fn open_settings(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let meter_model = meter_model.clone();
    cx.defer(move |cx| open_settings_now(&meter_model, None, cx));
}

/// [`open_settings`] on `section`, also when the window was already open.
pub fn open_settings_at(meter_model: &Entity<MeterModel>, section: Section, cx: &mut App) {
    let meter_model = meter_model.clone();
    cx.defer(move |cx| open_settings_now(&meter_model, Some(Page::Section(section)), cx));
}

/// [`open_settings`] on About › Changelog, also when the window was already
/// open.
pub fn open_changelog(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let meter_model = meter_model.clone();
    cx.defer(move |cx| open_settings_now(&meter_model, Some(Page::Changelog), cx));
}

fn open_settings_now(meter_model: &Entity<MeterModel>, page: Option<Page>, cx: &mut App) {
    let registry = registry(cx);
    let placement = cx.global::<Secondary>().placement;
    let opened = open_once(&registry, WindowKind::Settings, || {
        let runtime = cx.new(|cx| RuntimeModel::new(meter_model.clone(), cx));
        let meter_window = meter::window(cx);
        let view_runtime = runtime.clone();
        let handle = settings::open_window(
            placement,
            move |window, cx| cx.new(|cx| SettingsView::new(view_runtime, Some(meter_window), window, cx)),
            cx,
        )?;
        let view = handle.entity(cx).map_err(|error| format!("cannot reach the Settings view: {error:#}"))?;
        if placement == Placement::Screen {
            handle
                .update(cx, |_, window, cx| follow_activation(window, cx))
                .map_err(|error| format!("cannot reach the Settings window: {error:#}"))?;
        }
        let forget = registry.clone();
        cx.observe_release(&view, move |_, cx| {
            forget.borrow_mut().remove(WindowKind::Settings);
            changed(cx);
        })
        .detach();
        let hwnd = handle.update(cx, |_, window, _| overlay::hwnd(window).ok()).ok().flatten();
        Ok(Open { window: handle.into(), hwnd, model: Some(runtime.into()) })
    });
    if let (Some(page), Ok(_)) = (page, &opened) {
        let window = registry.borrow().get(WindowKind::Settings).and_then(|open| open.window.downcast::<SettingsView>());
        if let Some(window) = window {
            let _ = window.update(cx, |view, window, cx| view.show(page, window, cx));
        }
    }
    finish(WindowKind::Settings, opened, cx);
}

/// Settings is in front of everything, the game included, while it is the
/// active window, and goes back among the normal windows once the user goes
/// to another program.
fn follow_activation(window: &mut Window, cx: &mut Context<SettingsView>) {
    cx.observe_window_activation(window, |_, window, _| {
        let Ok(hwnd) = overlay::hwnd(window) else { return };
        match settings_band(window.is_window_active(), foreground()) {
            SettingsBand::Topmost => overlay::raise_without_activating(hwnd),
            SettingsBand::Normal(went_to) => overlay::lower_behind(hwnd, went_to),
            SettingsBand::Unchanged => {}
        }
    })
    .detach();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Foreground {
    Ours,
    Other(HWND),
    Nobody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsBand {
    Topmost,
    /// Among the normal windows, behind the window the user went to.
    Normal(Option<HWND>),
    Unchanged,
}

/// Another window of ours holding the keyboard is a to-do or PvP field being
/// typed in: the user is still in the app, so Settings stays in front.
fn settings_band(active: bool, foreground: Foreground) -> SettingsBand {
    match (active, foreground) {
        (true, _) => SettingsBand::Topmost,
        (false, Foreground::Ours) => SettingsBand::Unchanged,
        (false, Foreground::Other(window)) => SettingsBand::Normal(Some(window)),
        (false, Foreground::Nobody) => SettingsBand::Normal(None),
    }
}

fn foreground() -> Foreground {
    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() {
        return Foreground::Nobody;
    }
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(window, Some(&mut process)) };
    if process == std::process::id() {
        Foreground::Ours
    } else {
        Foreground::Other(window)
    }
}

/// Shows a reminder on the meter, or puts it in place of the one up.
pub fn show_reminder(request: &ShowReminder, cx: &mut App) {
    let registry = registry(cx);
    let open = registry.borrow().get(WindowKind::Reminder).and_then(|open| open.window.downcast::<ReminderView>());
    if let Some(window) = open {
        if window.update(cx, |view, _, cx| view.show(request, cx)).is_ok() {
            log_card(request, "put on the card window already open", cx);
            return;
        }
        registry.borrow_mut().remove(WindowKind::Reminder);
        log_card(request, "the open card's window was gone, opening a new one", cx);
    }
    let placement = cx.global::<Secondary>().placement;
    let overlay = settings::global(cx).read(cx).get().overlay.clone();
    let expected = reminders::expected_size(request, &overlay);
    let gap = reminders::GAP * overlay.scale();
    let place = card_place(Card::Reminder, gap, cx);
    let meter_window = meter::window(cx);
    let for_window = request.clone();
    let opened: Result<Opened, String> = open_once(&registry, WindowKind::Reminder, || {
        let stacking = Rc::new(Stacking { card: Card::Reminder, placement, placed: Cell::new(None) });
        let (window, hwnd) = create_window(
            WindowKind::Reminder,
            (expected.width.as_f32(), expected.height.as_f32()),
            place,
            move |window, cx| {
                let link = MeterLink { window: meter_window, placement: stacking };
                cx.new(|cx| ReminderView::new(&for_window, Some(link), window, cx))
            },
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: None })
    });
    match opened {
        Ok(Opened::Created) => log_card(request, "opened a card window", cx),
        Ok(Opened::Existing) => log_card(request, "a card window was already registered, nothing opened", cx),
        Err(error) => {
            crate::app_log::error(format!("cannot open the Reminder window: {error}"));
            log_card(request, &format!("cannot open the card window: {error}"), cx);
        }
    }
}

fn log_card(request: &ShowReminder, outcome: &str, cx: &App) {
    reminders::global(cx).read(cx).log_card(|| format!("{:?} at {}: {outcome}", request.events, request.starts_at));
}

/// Opens the first-open prompt on the meter; nothing when it is up already.
pub fn open_prompt(cx: &mut App) {
    let Some(secondary) = cx.try_global::<Secondary>() else {
        return;
    };
    let (registry, placement) = (secondary.registry.clone(), secondary.placement);
    let overlay = settings::global(cx).read(cx).get().overlay.clone();
    let expected = toolbar::prompt_size(&overlay);
    let place = card_place(Card::Prompt, reminders::GAP * overlay.scale(), cx);
    let opened = open_once(&registry, WindowKind::Prompt, || {
        let stacking = Rc::new(Stacking { card: Card::Prompt, placement, placed: Cell::new(None) });
        let (window, hwnd) = create_window(
            WindowKind::Prompt,
            (expected.width.as_f32(), expected.height.as_f32()),
            place,
            move |window, cx| cx.new(|cx| PromptView::new(stacking, window, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: None })
    });
    finish(WindowKind::Prompt, opened, cx);
}

/// Opens What's new under the toolbar; nothing when it is up already. Its
/// "Full changelog" opens Settings › About › Changelog.
pub fn open_whats_new(meter_model: &Entity<MeterModel>, whats_new: WhatsNew, cx: &mut App) {
    let Some(secondary) = cx.try_global::<Secondary>() else {
        return;
    };
    let (registry, placement) = (secondary.registry.clone(), secondary.placement);
    let scale = settings::global(cx).read(cx).get().overlay.scale();
    let place = card_place(Card::WhatsNew, reminders::GAP * scale, cx);
    let (width, height) = whats_new::EXPECTED_SIZE;
    let opened = open_once(&registry, WindowKind::WhatsNew, || {
        let stacking = Rc::new(Stacking { card: Card::WhatsNew, placement, placed: Cell::new(None) });
        let meter_model = meter_model.clone();
        let (window, hwnd) = create_window(
            WindowKind::WhatsNew,
            (width * scale, height * scale),
            place,
            move |window, cx| {
                let view = cx.new(|cx| WhatsNewView::new(whats_new, stacking, window, cx));
                cx.subscribe(&view, move |_, _: &OpenChangelog, cx| open_changelog(&meter_model, cx)).detach();
                view
            },
            |_| {},
            cx,
        )?;
        Ok(Open { window, hwnd: Some(hwnd), model: None })
    });
    finish(WindowKind::WhatsNew, opened, cx);
}

/// Where a card's window first goes: as [`Stacking`] keeps it.
fn card_place(card: Card, gap: f32, cx: &mut App) -> (HWND, Anchor) {
    let meter_hwnd = cx.global::<Secondary>().meter_hwnd;
    let (against, preferred) = card_anchor(card, cx).unwrap_or((meter_hwnd, Stack::Above));
    (against, Anchor::Stacked { gap, preferred })
}

/// Keeps a card's window `gap` from the window it is stacked on, left edges
/// aligned, on the preferred side or the other one when its monitor has no
/// room there, as that window moves and the card changes size. Off screen
/// (dev captures) the meter and the toolbar are past every monitor, and the
/// card with them.
pub struct Stacking {
    card: Card,
    placement: Placement,
    /// Where the window was last put, in device pixels.
    placed: Cell<Option<Rect>>,
}

impl Stacking {
    /// `card` and `gap` are logical pixels at the window's scale.
    pub fn place(&self, card: Size<Pixels>, gap: f32, window: &Window, cx: &mut App) {
        let Some((against, preferred)) = card_anchor(self.card, cx) else {
            return;
        };
        let Some((anchor, work)) = monitor::window_and_work_area(against) else {
            return;
        };
        let scale = window.scale_factor();
        let size = ((card.width.as_f32() * scale).ceil() as i32, (card.height.as_f32() * scale).ceil() as i32);
        let (x, y) = Anchor::Stacked { gap, preferred }.corner(anchor, size, work, scale);
        let target = Rect { left: x, top: y, right: x + size.0, bottom: y + size.1 };
        if self.placed.replace(Some(target)) == Some(target) {
            return;
        }
        let hwnd = match overlay::hwnd(window) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                crate::app_log::warn(format!("cannot place the {:?} card: {error}", self.card));
                return;
            }
        };
        let placement = self.placement;
        #[cfg(feature = "probes")]
        let card_kind = self.card;
        // After the current frame: SetWindowPos sends WM_SIZE back into GPUI.
        cx.spawn(async move |_| {
            if let Err(error) = overlay::place(hwnd, x, y, size.0 as f32, size.1 as f32) {
                crate::app_log::warn(error.to_string());
            }
            match placement {
                Placement::Screen => {}
                #[cfg(feature = "probes")]
                Placement::OffScreen => {
                    if let Err(error) = overlay::off_screen::ensure_off_screen(hwnd) {
                        crate::app_log::warn(format!("the {card_kind:?} card: {error}"));
                    }
                }
            }
        })
        .detach();
    }
}

/// The meter saved a fight: the history window, if open, lists it.
pub fn history_updated(cx: &mut App) {
    let Some(secondary) = cx.try_global::<Secondary>() else {
        return;
    };
    let open = secondary.registry.borrow().get(WindowKind::History).and_then(|open| open.model.clone());
    if let Some(model) = open.and_then(|model| model.downcast::<HistoryModel>().ok()) {
        model.update(cx, |model, cx| model.refresh(cx));
    }
}

/// The meter's window and the work area of its monitor, in device pixels,
/// for the capture tooling.
#[cfg(feature = "probes")]
pub fn meter_and_work_area(cx: &App) -> Option<(Rect, Rect)> {
    monitor::window_and_work_area(cx.try_global::<Secondary>()?.meter_hwnd)
}

/// The open window's handle, for the capture tooling.
#[cfg(feature = "probes")]
pub fn window_of(kind: WindowKind, cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<Secondary>()?.registry.borrow().get(kind).map(|open| open.window)
}

/// The open window's model, for the capture tooling.
#[cfg(feature = "probes")]
pub fn model_of<T: 'static>(kind: WindowKind, cx: &App) -> Option<Entity<T>> {
    let model = cx.try_global::<Secondary>()?.registry.borrow().get(kind)?.model.clone()?;
    model.downcast::<T>().ok()
}

fn registry(cx: &App) -> Rc<RefCell<Registry<Open>>> {
    cx.global::<Secondary>().registry.clone()
}

/// Brings the window forward as [`forward`] decides; a failed open is
/// reported.
fn finish(kind: WindowKind, opened: Result<Opened, String>, cx: &mut App) {
    let opened = match opened {
        Ok(opened) => opened,
        Err(error) => {
            crate::app_log::error(format!("cannot open the {kind:?} window: {error}"));
            return;
        }
    };
    if opened == Opened::Created {
        changed(cx);
    }
    let action = forward(kind, opened, cx.global::<Secondary>().placement);
    if action == Forward::Nothing {
        return;
    }
    let Some(open) = registry(cx).borrow().get(kind).cloned() else {
        return;
    };
    let _ = open.window.update(cx, |_, window, _| {
        let Ok(hwnd) = overlay::hwnd(window) else { return };
        match action {
            Forward::Foreground => settings::bring_forward(hwnd),
            Forward::Raise => overlay::raise_without_activating(hwnd),
            Forward::Nothing => {}
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Forward {
    Nothing,
    Raise,
    Foreground,
}

/// Settings on screen is brought to the foreground also when it was just
/// created: GPUI's own activation leaves it under every topmost window, which
/// a game can be. The overlays are never given focus.
fn forward(kind: WindowKind, opened: Opened, placement: Placement) -> Forward {
    match (kind, placement, opened) {
        (WindowKind::Settings, Placement::Screen, _) => Forward::Foreground,
        (_, _, Opened::Created) => Forward::Nothing,
        (_, _, Opened::Existing) => Forward::Raise,
    }
}

/// Opens a transparent pop-up like the meter's, places it against the window
/// `against` as `anchor` says, shows it without activating it, and forgets it
/// in the registry when its view is released (however the window ends).
fn create_window<V: Render + 'static>(
    kind: WindowKind,
    (width, height): (f32, f32),
    (against, anchor): (HWND, Anchor),
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
    on_close: impl FnOnce(&mut App) + 'static,
    cx: &mut App,
) -> Result<(AnyWindowHandle, HWND), String> {
    let (placement, registry) = {
        let secondary = cx.global::<Secondary>();
        (secondary.placement, secondary.registry.clone())
    };
    let handle = cx
        .open_window(overlay::popup_options(width, height), build)
        .map_err(|error| format!("cannot open the window: {error:#}"))?;
    let set_up = handle
        .entity(cx)
        .map_err(|error| format!("cannot reach the window's view: {error:#}"))
        .and_then(|view| {
            handle
                .update(cx, |_, window, _| -> Result<HWND, String> {
                    let hwnd = overlay::hwnd(window)?;
                    overlay::make_overlay(hwnd)?;
                    let scale = window.scale_factor();
                    let (device_width, device_height) = (width * scale, height * scale);
                    match placement {
                        Placement::Screen => {
                            let size = (device_width.ceil() as i32, device_height.ceil() as i32);
                            let (x, y) = monitor::window_and_work_area(against)
                                .map_or((64, 64), |(against, work)| anchor.corner(against, size, work, scale));
                            overlay::place(hwnd, x, y, device_width, device_height)?;
                            overlay::show_without_activating(hwnd);
                        }
                        #[cfg(feature = "probes")]
                        Placement::OffScreen => overlay::off_screen::show(hwnd, device_width, device_height)?,
                    }
                    Ok(hwnd)
                })
                .map_err(|error| format!("cannot reach the window: {error:#}"))?
                .map(|hwnd| (view, hwnd))
        });
    // A window that could not be set up is closed, so the registry (which
    // does not keep it) and the screen agree, and the next open makes one.
    let (view, hwnd) = match set_up {
        Ok(set_up) => set_up,
        Err(error) => {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            return Err(error);
        }
    };
    cx.observe_release(&view, move |_, cx| {
        registry.borrow_mut().remove(kind);
        on_close(cx);
        changed(cx);
    })
    .detach();
    Ok((handle.into(), hwnd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_comes_to_the_foreground_when_opened_and_when_opened_again() {
        assert_eq!(forward(WindowKind::Settings, Opened::Created, Placement::Screen), Forward::Foreground);
        assert_eq!(forward(WindowKind::Settings, Opened::Existing, Placement::Screen), Forward::Foreground);
    }

    #[test]
    fn an_overlay_opened_again_is_raised_without_focus() {
        let overlays = [
            WindowKind::History,
            WindowKind::Detail,
            WindowKind::Report,
            WindowKind::Pvp,
            WindowKind::Todos,
            WindowKind::Resources,
            WindowKind::WhatsNew,
        ];
        for kind in overlays {
            assert_eq!(forward(kind, Opened::Created, Placement::Screen), Forward::Nothing, "{kind:?}");
            assert_eq!(forward(kind, Opened::Existing, Placement::Screen), Forward::Raise, "{kind:?}");
        }
    }

    #[test]
    fn active_settings_is_in_front_of_everything() {
        for foreground in [Foreground::Ours, Foreground::Other(game()), Foreground::Nobody] {
            assert_eq!(settings_band(true, foreground), SettingsBand::Topmost, "{foreground:?}");
        }
    }

    #[test]
    fn settings_goes_behind_the_program_the_user_went_to() {
        assert_eq!(settings_band(false, Foreground::Other(game())), SettingsBand::Normal(Some(game())));
        assert_eq!(settings_band(false, Foreground::Nobody), SettingsBand::Normal(None));
    }

    #[test]
    fn settings_stays_in_front_while_another_window_of_ours_takes_the_keyboard() {
        assert_eq!(settings_band(false, Foreground::Ours), SettingsBand::Unchanged);
    }

    fn game() -> HWND {
        HWND(0x1234 as *mut _)
    }

    const METER: HWND = HWND(0x5678 as *mut _);

    /// The meter's model and the hub, and this layer with a made-up handle
    /// for the meter, since the test platform's windows have none.
    fn meter_scene(cx: &mut gpui::TestAppContext) -> (Entity<MeterModel>, Rc<RefCell<Registry<Open>>>) {
        use gpui::AppContext as _;

        let registry: Rc<RefCell<Registry<Open>>> = Rc::default();
        let model = cx.update(|cx| {
            let store = cx.new(|_| settings::Settings::in_memory(settings::AppSettings::default()));
            settings::install(&store, cx);
            let model = cx.new(|_| MeterModel::new(crate::meter::OverlaySettings::default(), None));
            let changes = cx.new(|_| OpenWindows);
            cx.set_global(Secondary { registry: registry.clone(), meter_hwnd: METER, placement: Placement::Screen, changes });
            let meter_window = cx
                .open_window(Default::default(), |_, cx| cx.new(|cx| MeterView::new(model.clone(), cx)))
                .expect("the meter's window");
            crate::meter::install_window(meter_window, cx);
            toolbar::install_for_test(meter_window, model.clone(), cx);
            toolbar::open_meter(cx);
            model
        });
        (model, registry)
    }

    /// The core's word, folded in as the bridge does.
    fn core_says_idle(model: &Entity<MeterModel>, idle: bool, cx: &mut gpui::TestAppContext) {
        model.update(cx, |model, cx| {
            if model.apply(meter_core::events::MeterEvent::DpsIdleHidden(idle), std::time::Instant::now()) {
                cx.notify();
            }
        });
        cx.run_until_parked();
    }

    fn meter_shown(cx: &mut gpui::TestAppContext) -> bool {
        cx.update(|cx| toolbar::hub(cx).expect("the hub").read(cx).meter_is_shown())
    }

    #[gpui::test]
    fn the_reminder_card_sits_under_the_toolbar_while_the_meter_steps_aside(cx: &mut gpui::TestAppContext) {
        let (model, _registry) = meter_scene(cx);
        let reminder = |cx: &mut gpui::TestAppContext| cx.update(|cx| card_anchor(Card::Reminder, cx));
        assert_eq!(reminder(cx), Some((METER, Stack::Above)), "on the meter");

        core_says_idle(&model, true, cx);
        let (_, side) = reminder(cx).expect("a place for the card");
        assert_eq!(side, Stack::Below, "under the toolbar, as while the meter is closed");

        core_says_idle(&model, false, cx);
        assert_eq!(reminder(cx), Some((METER, Stack::Above)), "back on the meter at your first hit");
    }

    #[gpui::test]
    fn whats_new_sits_under_the_toolbar_even_with_the_meter_open(cx: &mut gpui::TestAppContext) {
        let _ = meter_scene(cx);
        let (anchor, under_toolbar) =
            cx.update(|cx| (card_anchor(Card::WhatsNew, cx), toolbar::toolbar_hwnd(cx).unwrap_or(METER)));
        assert_eq!(anchor, Some((under_toolbar, Stack::Below)), "with the meter open, still under the toolbar");
    }

    #[gpui::test]
    fn a_reminder_card_opens_while_the_meter_steps_aside_between_fights(cx: &mut gpui::TestAppContext) {
        let (model, registry) = meter_scene(cx);
        core_says_idle(&model, true, cx);
        assert!(cx.update(|cx| toolbar::meter_is_aside(cx)), "the meter is open and stepped aside");

        // What `show_reminder` does, on a test window: the real one needs a
        // native handle. The card's first frame places it, which reads
        // whether the meter is aside, which asks the registry.
        let opened: Result<Opened, String> = cx.update(|cx| {
            let meter_window = meter::window(cx);
            let stacking = Rc::new(Stacking { card: Card::Reminder, placement: Placement::Screen, placed: Cell::new(None) });
            let request = ShowReminder {
                events: vec![reminders::EventId::ShugoFestival],
                start_ms: crate::clock::now_ms() + 60_000,
                starts_at: crate::clock::ClockTime { hour: 22, minute: 30 },
                is_test: false,
            };
            open_once(&registry, WindowKind::Reminder, || {
                let window = cx
                    .open_window(Default::default(), |window, cx| {
                        let link = MeterLink { window: meter_window, placement: stacking };
                        cx.new(|cx| ReminderView::new(&request, Some(link), window, cx))
                    })
                    .map_err(|error| format!("{error:#}"))?;
                Ok(Open { window: window.into(), hwnd: None, model: None })
            })
        });
        cx.run_until_parked();

        assert_eq!(opened, Ok(Opened::Created));
        assert!(cx.update(|cx| is_open(WindowKind::Reminder, cx)));
    }

    #[gpui::test]
    fn the_first_open_prompt_keeps_the_meter_on_screen_until_it_is_answered(cx: &mut gpui::TestAppContext) {
        use gpui::Modifiers;

        let (model, registry) = meter_scene(cx);
        // What `open_prompt` does, but on a test window: the real one needs a
        // native handle.
        let (prompt, vcx) = cx.add_window_view(|_, cx| PromptView::unplaced(cx));
        vcx.update(|window, cx| {
            let window = window.window_handle();
            let opened = open_once(&registry, WindowKind::Prompt, || Ok::<_, String>(Open { window, hwnd: None, model: None }));
            assert_eq!(opened, Ok(Opened::Created));
            let registry = registry.clone();
            cx.observe_release(&prompt, move |_, cx| {
                registry.borrow_mut().remove(WindowKind::Prompt);
                changed(cx);
            })
            .detach();
            changed(cx);
        });
        drop(prompt);
        let cx = &mut **vcx;

        core_says_idle(&model, true, cx);
        assert!(meter_shown(cx), "the opening peek is over, but the prompt waits on the meter for an answer");
        assert!(cx.update(|cx| is_open(WindowKind::Prompt, cx) && !toolbar::meter_is_aside(cx)));

        let not_now = vcx.debug_bounds("prompt-no").expect("the Not now button").center();
        vcx.simulate_click(not_now, Modifiers::none());
        vcx.run_until_parked();
        let cx = &mut **vcx;
        assert!(!cx.update(|cx| is_open(WindowKind::Prompt, cx)), "Not now closes the prompt");
        assert!(!meter_shown(cx), "answered, the meter steps aside between fights again");
    }
}
