//! Off-screen captures and checks of the toolbar (docs/gpui-mockups/approved/
//! toolbar.html), the first-open prompt and the reminder card under the
//! toolbar, run by `--capture --fixtures` after the reminder card's. The
//! toolbar, the meter and every card stay past every monitor; the only input
//! is mouse messages posted to the toolbar itself, and nothing asks Windows
//! for the foreground. The values are invented (`fixtures::MIRELLE_TICKETS`
//! with the changes each state needs).

use std::path::Path;
use std::time::Duration;

use gpui::{AsyncApp, WindowHandle};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowPos, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
};

use crate::capture::{self, MK_LBUTTON, SETTLE};
use crate::capture_reminders::{self, shoot_together};
use crate::capture_secondary::{close, pack, pause, post, window_hwnd};
use crate::capture_todos::{identify, known, set_membership, set_reset_ahead};
use crate::clock::now_ms;
use crate::fixtures;
use crate::overlay::{self, off_screen};
use crate::reminders::EventId;
use crate::secondary::{self, WindowKind};
use crate::settings;
use crate::todos;
use crate::toolbar::{self, Dropdown, Slot, Tool, ToolbarView};

type Result<T, E = String> = std::result::Result<T, E>;

const TICK: Duration = Duration::from_millis(150);
/// How far above the bottom of its work area the toolbar goes to be low.
const LOW: i32 = 40;

pub(crate) fn toolbar_window(cx: &mut AsyncApp) -> Result<(WindowHandle<ToolbarView>, HWND)> {
    let window = cx.update(|cx| toolbar::toolbar_window(cx)).ok_or("the toolbar is not open")?;
    let hwnd = window
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("the toolbar window is gone: {error:#}"))??;
    Ok((window, hwnd))
}

pub(crate) fn view(cx: &mut AsyncApp, change: impl FnOnce(&mut ToolbarView, &mut gpui::Context<ToolbarView>)) -> Result<()> {
    let (window, _) = toolbar_window(cx)?;
    window.update(cx, |view, _, cx| change(view, cx)).map_err(|error| format!("the toolbar window is gone: {error:#}"))
}

pub(crate) async fn shot(dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    pause(cx, SETTLE).await;
    let (_, hwnd) = toolbar_window(cx)?;
    off_screen::ensure_off_screen(hwnd)?;
    capture::shoot_hwnd(hwnd, dir, name, cx).await
}

/// A click on `slot`, posted to the toolbar's own window where the control
/// was last drawn.
pub(crate) async fn click(slot: Slot, cx: &mut AsyncApp) -> Result<()> {
    let (window, hwnd) = toolbar_window(cx)?;
    let (centre, scale) = window
        .update(cx, |view, window, _| (view.centre_of(slot), window.scale_factor()))
        .map_err(|error| format!("the toolbar window is gone: {error:#}"))?;
    let centre = centre.ok_or_else(|| format!("{slot:?} is not on the toolbar"))?;
    let at = ((centre.x.as_f32() * scale).round() as i32, (centre.y.as_f32() * scale).round() as i32);
    post(hwnd, WM_MOUSEMOVE, 0, pack(at));
    post(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(at));
    post(hwnd, WM_LBUTTONUP, 0, pack(at));
    pause(cx, TICK * 2).await;
    Ok(())
}

pub(crate) fn login(tickets: &[(u32, u32, u32)], cx: &mut AsyncApp) {
    let report = fixtures::ticket_login(&known("Mirelle"), now_ms(), tickets);
    cx.update(|cx| todos::global(cx).update(cx, |todos, cx| todos.apply_tickets(&[report], cx)));
}

/// Mirelle's tickets with `changes` (ticket, regular pool, extra pool).
pub(crate) fn tickets(changes: &[(u32, u32, u32)]) -> Vec<(u32, u32, u32)> {
    fixtures::MIRELLE_TICKETS
        .iter()
        .map(|&(id, count, charge)| changes.iter().find(|change| change.0 == id).copied().unwrap_or((id, count, charge)))
        .collect()
}

fn window_rect(hwnd: HWND) -> Result<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|error| format!("GetWindowRect failed: {error}"))?;
    Ok(rect)
}

fn move_toolbar(hwnd: HWND, x: i32, y: i32) -> Result<()> {
    unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) }
        .map_err(|error| format!("cannot move the toolbar: {error}"))?;
    off_screen::ensure_off_screen(hwnd)
}

fn check_toolbar_never_activates(label: &str, cx: &mut AsyncApp) -> Result<()> {
    let (_, hwnd) = toolbar_window(cx)?;
    let style = overlay::ex_style(hwnd);
    println!(
        "{label}: toolbar non-activating {}, click-through {}",
        style & WS_EX_NOACTIVATE.0 != 0,
        style & WS_EX_TRANSPARENT.0 != 0
    );
    if style & WS_EX_TRANSPARENT.0 != 0 {
        return Err(format!("{label}: the toolbar is click-through"));
    }
    capture::check_click_activation(hwnd, label, MA_NOACTIVATE)
}

pub async fn from_fixtures(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let foreground_before = capture_reminders::foreground();
    identify("Mirelle", cx);
    set_membership(true, cx);
    login(&tickets(&[]), cx);
    cx.update(toolbar::close_meter);
    view(cx, |view, cx| view.force_tip(None, cx))?;
    shot(dir, "toolbar-01-launch", cx).await?;
    check_toolbar_never_activates("a click on the toolbar", cx)?;

    view(cx, |view, cx| view.force_tip(Some(Slot::Tool(Tool::Meter)), cx))?;
    shot(dir, "toolbar-02-meter-tip", cx).await?;

    view(cx, |view, cx| view.force_tip(None, cx))?;
    click(Slot::Tool(Tool::Todos), cx).await?;
    let todos_open = cx.update(|cx| secondary::is_open(WindowKind::Todos, cx));
    println!("entry point: a click on To-dos on the toolbar: to-do window open {todos_open}");
    if !todos_open {
        return Err("the to-do window did not open from the toolbar".to_string());
    }
    cx.update(toolbar::open_meter);
    shot(dir, "toolbar-03-meter-and-todos-lit", cx).await?;
    view(cx, |view, cx| view.force_tip(None, cx))?;
    let meter = cx.update(|cx| crate::meter::window(cx));
    let meter_hwnd = meter
        .update(cx, |view, window, cx| {
            view.forced_pointer = Some(true);
            view.force_close_tip(cx);
            overlay::hwnd(window)
        })
        .map_err(|error| format!("the meter window is gone: {error:#}"))??;
    pause(cx, SETTLE).await;
    capture::shoot_hwnd(meter_hwnd, dir, "toolbar-04b-meter-close-tip", cx).await?;
    meter
        .update(cx, |view, _, cx| {
            view.forced_pointer = None;
            cx.notify();
        })
        .map_err(|error| format!("the meter window is gone: {error:#}"))?;
    view(cx, |view, cx| view.force_tip(Some(Slot::Tool(Tool::Meter)), cx))?;
    shot(dir, "toolbar-04-meter-open-tip", cx).await?;
    view(cx, |view, cx| view.force_tip(None, cx))?;
    click(Slot::Tool(Tool::Todos), cx).await?;
    let todos_open = cx.update(|cx| secondary::is_open(WindowKind::Todos, cx));
    println!("a second click on To-dos: to-do window open {todos_open}");
    if todos_open {
        return Err("a second click on the lit To-dos did not close it".to_string());
    }

    for (label, tool, kind) in [("Fight history", Tool::History, WindowKind::History), ("PvP meter", Tool::Pvp, WindowKind::Pvp)] {
        click(Slot::Tool(tool), cx).await?;
        let open = cx.update(|cx| secondary::is_open(kind, cx));
        println!("entry point: a click on {label} on the toolbar: {kind:?} window open {open}");
        if !open {
            return Err(format!("the {kind:?} window did not open from the toolbar"));
        }
        close(kind, cx).await?;
    }

    view(cx, |view, cx| view.force_tip(Some(Slot::Odyle), cx))?;
    shot(dir, "toolbar-05-odyle-tip", cx).await?;

    set_reset_ahead(5, cx);
    login(&tickets(&[(1, 21, 3)]), cx);
    view(cx, |view, cx| view.force_tip(Some(Slot::Shugo), cx))?;
    shot(dir, "toolbar-06a-shugo-full-calm", cx).await?;
    set_reset_ahead(1, cx);
    pause(cx, TICK).await;
    shot(dir, "toolbar-06-shugo-full", cx).await?;

    login(&tickets(&[(8, 7, 0), (4, 14, 0)]), cx);
    view(cx, |view, cx| view.force_tip(Some(Slot::Warn), cx))?;
    shot(dir, "toolbar-07-warning", cx).await?;
    view(cx, |view, cx| view.force_dropdown(Some(Dropdown::Full), cx))?;
    shot(dir, "toolbar-08-full-dropdown", cx).await?;

    view(cx, |view, cx| view.force_dropdown(Some(Dropdown::Bell), cx))?;
    shot(dir, "toolbar-09-bell-dropdown", cx).await?;
    view(cx, |view, cx| view.force_dropdown(None, cx))?;

    set_reminders_paused(true, cx);
    view(cx, |view, cx| view.force_tip(Some(Slot::Tool(Tool::Reminders)), cx))?;
    shot(dir, "toolbar-10-reminders-paused", cx).await?;
    set_reminders_paused(false, cx);

    view(cx, |view, cx| view.force_tip(Some(Slot::Quit), cx))?;
    shot(dir, "toolbar-11-quit-tip", cx).await?;

    login(&tickets(&[(1, 21, 3), (8, 7, 0)]), cx);
    view(cx, |view, cx| {
        view.force_tip(None, cx);
        view.set_folded(true, cx);
    })?;
    shot(dir, "toolbar-12-folded-warning", cx).await?;
    view(cx, |view, cx| view.set_folded(false, cx))?;

    let (_, hwnd) = toolbar_window(cx)?;
    let home = window_rect(hwnd)?;
    let (_, work) = secondary::window_and_work_area(hwnd).ok_or("the toolbar's monitor is unknown")?;
    move_toolbar(hwnd, home.left, work.bottom - LOW)?;
    view(cx, |view, cx| {
        view.force_up(Some(true));
        view.force_dropdown(Some(Dropdown::Full), cx);
    })?;
    shot(dir, "toolbar-13-low-dropdown-up", cx).await?;
    view(cx, |view, cx| {
        view.force_dropdown(None, cx);
        view.force_up(None);
    })?;
    pause(cx, SETTLE).await;
    move_toolbar(hwnd, home.left, home.top)?;

    first_open_prompt(dir, cx).await?;
    reminder_under_toolbar(dir, cx).await?;
    check_toolbar_never_activates("the toolbar after its run", cx)?;
    let foreground_after = capture_reminders::foreground();
    println!("toolbar run: foreground unchanged {}", foreground_after == foreground_before);
    Ok(())
}

fn set_reminders_paused(paused: bool, cx: &mut AsyncApp) {
    cx.update(|cx| {
        settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.reminders.enabled = !paused))
    });
}

/// The meter opened from the toolbar asks once; closing the meter without
/// answering takes the card away.
async fn first_open_prompt(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    cx.update(|cx| {
        toolbar::close_meter(cx);
        settings::global(cx).update(cx, |settings, cx| {
            settings.update(cx, |all| {
                all.windows.launch_prompt_answered = false;
                all.windows.open_meter_at_launch = false;
            })
        });
    });
    click(Slot::Tool(Tool::Meter), cx).await?;
    let (meter_open, prompt_open) = cx.update(|cx| (toolbar::meter_is_open(cx), secondary::is_open(WindowKind::Prompt, cx)));
    println!("first open from the toolbar: meter open {meter_open}, prompt open {prompt_open}");
    if !meter_open || !prompt_open {
        return Err("the first open from the toolbar did not show the meter with the prompt".to_string());
    }
    let meter_hwnd = cx.update(|cx| crate::meter::window(cx)).update(cx, |_, window, _| overlay::hwnd(window));
    let meter_hwnd = meter_hwnd.map_err(|error| format!("the meter window is gone: {error:#}"))??;
    pause(cx, SETTLE).await;
    let prompt_hwnd = window_hwnd(WindowKind::Prompt, cx)?;
    shoot_together(&[meter_hwnd, prompt_hwnd], dir, "toolbar-14-first-open-prompt", cx).await?;
    capture::check_click_activation(prompt_hwnd, "a click on the prompt", MA_NOACTIVATE)?;

    click(Slot::Tool(Tool::Meter), cx).await?;
    let prompt_open = cx.update(|cx| secondary::is_open(WindowKind::Prompt, cx));
    println!("meter closed without an answer: prompt open {prompt_open}");
    if prompt_open {
        return Err("the prompt stayed after the meter closed".to_string());
    }
    Ok(())
}

/// While the meter is closed the reminder card sits 6 px under the toolbar,
/// left edges aligned.
async fn reminder_under_toolbar(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    cx.update(toolbar::close_meter);
    capture_reminders::show(vec![EventId::ShugoFestival], (22, 0), cx);
    pause(cx, SETTLE).await;
    let (_, toolbar_hwnd) = toolbar_window(cx)?;
    let card_hwnd = window_hwnd(WindowKind::Reminder, cx)?;
    let (bar, card) = (window_rect(toolbar_hwnd)?, window_rect(card_hwnd)?);
    println!(
        "reminder under the toolbar: left edges {} and {}, gap {} px",
        bar.left,
        card.left,
        card.top - bar.bottom
    );
    if card.left != bar.left || card.top <= bar.bottom {
        return Err("the reminder card is not under the toolbar while the meter is closed".to_string());
    }
    shoot_together(&[toolbar_hwnd, card_hwnd], dir, "toolbar-15-reminder-under-toolbar", cx).await?;
    close(WindowKind::Reminder, cx).await?;
    cx.update(toolbar::open_meter);
    Ok(())
}
