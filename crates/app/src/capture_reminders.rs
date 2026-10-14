//! Off-screen captures and checks of the reminder card, run by `--capture
//! --fixtures` after the Settings window's. The card opens on the meter, which
//! stays past every monitor; nothing asks Windows for the foreground, and the
//! only input is a click posted to the card itself. The run fails when the
//! card could take focus, overlaps the meter, or catches clicks while locked.
//! Each `reminder-*.png` shows the card and the meter where they sit, over a
//! plain backdrop.

use std::ffi::c_void;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{AsyncApp, Entity, WindowHandle};
use image::{imageops, Rgba, RgbaImage};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GetWindowRect, SetWindowPos, GUITHREADINFO, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};

use crate::capture::{self, grab_image, MK_LBUTTON, SETTLE};
use crate::capture_secondary::{pack, pause, post, show_scene, window_handle, window_hwnd};
use crate::clock::ClockTime;
use crate::fixtures;
use crate::meter::{Layout, MeterModel, MeterView};
use crate::overlay::{self, off_screen};
use crate::reminders::{CardKind, EventId, ReminderView, ShowReminder, GAP};
use crate::secondary::{self, Rect, WindowKind};
use crate::settings;

const TICK: Duration = Duration::from_millis(150);
/// Where the meter sits below the top of its work area while the card has
/// room above it.
const ROOM_ABOVE: i32 = 400;
const STACK_MARGIN: i32 = 16;
const BACKDROP: Rgba<u8> = Rgba([46, 52, 58, 255]);

type Result<T, E = String> = std::result::Result<T, E>;

pub async fn from_fixtures(
    meter: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    dir: &Path,
    cx: &mut AsyncApp,
) -> Result<()> {
    show_scene(model, fixtures::open_world(), cx);
    let meter_hwnd = meter
        .update(cx, |view, window, cx| {
            view.forced_pointer = None;
            cx.notify();
            overlay::hwnd(window)
        })
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    let home = window_rect(meter_hwnd)?;
    let (_, work) = cx.update(|cx| secondary::meter_and_work_area(cx)).ok_or("the meter's monitor is unknown")?;
    move_meter(meter_hwnd, home.left, work.top + ROOM_ABOVE)?;
    let foreground_before = foreground();

    show(vec![EventId::ShugoFestival], (16, 0), cx);
    let windows_before = cx.update(|cx| cx.windows().len());
    check_card("one event", meter_hwnd, foreground_before, cx).await?;
    shoot_stack(meter_hwnd, dir, "reminder-01-one", cx).await?;

    show(vec![EventId::ShugoFestival, EventId::RiftPortals], (2, 0), cx);
    let windows_after = cx.update(|cx| cx.windows().len());
    println!("single instance: a second reminder while one is up: {windows_before} windows before, {windows_after} after");
    if windows_after != windows_before {
        return Err("a newer reminder opened a second window instead of replacing the card".to_string());
    }
    check_card("two events", meter_hwnd, foreground_before, cx).await?;
    shoot_stack(meter_hwnd, dir, "reminder-02-two", cx).await?;

    show(vec![EventId::ShugoFestival, EventId::RiftPortals, EventId::ArenaOfTactics], (11, 0), cx);
    check_card("three events", meter_hwnd, foreground_before, cx).await?;
    shoot_stack(meter_hwnd, dir, "reminder-03-three", cx).await?;

    let card = window_handle::<ReminderView>(WindowKind::Reminder, cx)?;
    show(vec![EventId::DimensionalInvasion], (15, 30), cx);
    card.update(cx, |view, _, cx| view.force_time_left(0.35, cx)).map_err(gone)?;
    shoot_stack(meter_hwnd, dir, "reminder-04-drained", cx).await?;

    card.update(cx, |view, window, cx| view.force_pointer(Some(true), window, cx)).map_err(gone)?;
    let left_before = card.update(cx, |view, _, cx| view.time_left(cx)).map_err(gone)?;
    pause(cx, Duration::from_millis(600)).await;
    let left_after = card.update(cx, |view, _, cx| view.time_left(cx)).map_err(gone)?;
    println!("pointer over the card: {left_before:?} left, {left_after:?} after 600 ms");
    if left_after != left_before {
        return Err("the time left ran on under the pointer".to_string());
    }
    shoot_stack(meter_hwnd, dir, "reminder-05-pointer", cx).await?;
    card.update(cx, |view, window, cx| view.force_pointer(None, window, cx)).map_err(gone)?;

    set_meter_locked(meter, true, cx)?;
    pause(cx, TICK).await;
    check_click_through(true, cx)?;
    check_card("locked", meter_hwnd, foreground_before, cx).await?;
    shoot_stack(meter_hwnd, dir, "reminder-06-locked", cx).await?;
    set_meter_locked(meter, false, cx)?;
    pause(cx, TICK).await;
    check_click_through(false, cx)?;

    set_layout(Layout::Capsule, cx);
    show(vec![EventId::ShugoFestival, EventId::RiftPortals], (2, 0), cx);
    check_card("capsule", meter_hwnd, foreground_before, cx).await?;
    shoot_stack(meter_hwnd, dir, "reminder-07-capsule", cx).await?;
    set_layout(Layout::Full, cx);

    move_meter(meter_hwnd, home.left, work.top)?;
    show(vec![EventId::DimensionalInvasion], (15, 30), cx);
    let side = check_card("meter at the top of its screen", meter_hwnd, foreground_before, cx).await?;
    if side != Side::Below {
        return Err("with no room above the meter the card did not go below it".to_string());
    }
    shoot_stack(meter_hwnd, dir, "reminder-08-below", cx).await?;

    move_meter(meter_hwnd, home.left, work.top + ROOM_ABOVE)?;
    pause(cx, TICK * 2).await;
    let side = check_card("the meter moved down while the card is up", meter_hwnd, foreground_before, cx).await?;
    if side != Side::Above {
        return Err("the card did not follow the meter".to_string());
    }

    show_start_alert(vec![EventId::ShugoFestival, EventId::RiftPortals], (2, 0), cx);
    check_card("start alert", meter_hwnd, foreground_before, cx).await?;
    shoot_stack(meter_hwnd, dir, "reminder-09-start-alert", cx).await?;

    let card_hwnd = window_hwnd(WindowKind::Reminder, cx)?;
    let centre = {
        let rect = window_rect(card_hwnd)?;
        ((rect.right - rect.left) / 2, (rect.bottom - rect.top) / 2)
    };
    post(card_hwnd, WM_MOUSEMOVE, 0, pack(centre));
    post(card_hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(centre));
    post(card_hwnd, WM_LBUTTONUP, 0, pack(centre));
    pause(cx, Duration::from_millis(60)).await;
    check_never_focused(WindowKind::Reminder, "a click on the card", foreground_before, cx)?;
    pause(cx, Duration::from_millis(500)).await;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Reminder, cx).is_some());
    println!("a click on the card: still open {open}");
    if open {
        return Err("a click on the card did not dismiss it".to_string());
    }

    show(vec![EventId::ShugoFestival], (16, 0), cx);
    pause(cx, TICK).await;
    let card = window_handle::<ReminderView>(WindowKind::Reminder, cx)?;
    card.update(cx, |view, _, cx| view.force_time_left(0.01, cx)).map_err(gone)?;
    pause(cx, Duration::from_secs(1)).await;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Reminder, cx).is_some());
    println!("time-out: still open 1 s after the ring ran out: {open}");
    if open {
        return Err("the card stayed after its time ran out".to_string());
    }

    move_meter(meter_hwnd, home.left, home.top)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Above,
    Below,
}

/// A reminder starting a minute from now at `hour`:`minute` on the clock.
pub(crate) fn show(events: Vec<EventId>, time: (u8, u8), cx: &mut AsyncApp) {
    show_card(CardKind::Reminder, events, time, 60_000, cx);
}

/// A start alert for events that started just now, at `hour`:`minute` on the clock.
fn show_start_alert(events: Vec<EventId>, time: (u8, u8), cx: &mut AsyncApp) {
    show_card(CardKind::StartAlert, events, time, 0, cx);
}

fn show_card(kind: CardKind, events: Vec<EventId>, (hour, minute): (u8, u8), starts_in_ms: i64, cx: &mut AsyncApp) {
    let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as i64);
    let request = ShowReminder { events, start_ms: now_ms + starts_in_ms, starts_at: ClockTime { hour, minute }, kind, is_test: false };
    cx.update(|cx| secondary::show_reminder(&request, cx));
}

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

/// The overlay window of `kind` is non-activating, and neither the
/// foreground window nor the active or focused window of the UI thread,
/// which owns it.
pub fn check_never_focused(kind: WindowKind, label: &str, foreground_before: HWND, cx: &mut AsyncApp) -> Result<()> {
    let card = window_hwnd(kind, cx)?;
    let style = overlay::ex_style(card);
    let foreground = foreground();
    let mut thread = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
    unsafe { GetGUIThreadInfo(GetCurrentThreadId(), &mut thread) }
        .map_err(|error| format!("GetGUIThreadInfo failed: {error}"))?;
    println!(
        "{label}: topmost {}, tool window {}, non-activating {}; the {kind:?} window is the foreground window {}, the active window {}, the focus window {}; foreground unchanged {}",
        style & WS_EX_TOPMOST.0 != 0,
        style & WS_EX_TOOLWINDOW.0 != 0,
        style & WS_EX_NOACTIVATE.0 != 0,
        foreground == card,
        thread.hwndActive == card,
        thread.hwndFocus == card,
        foreground == foreground_before,
    );
    if style & WS_EX_NOACTIVATE.0 == 0 {
        return Err(format!("{label}: the {kind:?} window is activatable"));
    }
    if foreground == card || thread.hwndActive == card || thread.hwndFocus == card {
        return Err(format!("{label}: the {kind:?} window took the focus"));
    }
    if foreground != foreground_before {
        return Err(format!(
            "{label}: the foreground window changed since the check began ({:?} before, {:?} now); if it was not you switching windows during the run, the {kind:?} window moved it",
            foreground_before.0, foreground.0
        ));
    }
    capture::check_click_activation(card, &format!("{label}: a click on the {kind:?} window"), MA_NOACTIVATE)
}

/// Focus, then where the card sits: on the meter's left edge, the gap above
/// or below it, never over it.
async fn check_card(label: &str, meter_hwnd: HWND, foreground_before: HWND, cx: &mut AsyncApp) -> Result<Side> {
    pause(cx, TICK * 2).await;
    check_never_focused(WindowKind::Reminder, label, foreground_before, cx)?;
    let card = window_handle::<ReminderView>(WindowKind::Reminder, cx)?;
    let (card_hwnd, dpi) = card.update(cx, |_, window, _| overlay::hwnd(window).map(|hwnd| (hwnd, window.scale_factor()))).map_err(gone)??;
    let content_scale = cx.update(|cx| settings::global(cx).read(cx).get().overlay.scale());
    let gap = (GAP * content_scale * dpi).round() as i32;
    let (meter, card) = (window_rect(meter_hwnd)?, window_rect(card_hwnd)?);
    let overlaps = card.left < meter.right && meter.left < card.right && card.top < meter.bottom && meter.top < card.bottom;
    let side = if card.bottom <= meter.top { Side::Above } else { Side::Below };
    let measured = match side {
        Side::Above => meter.top - card.bottom,
        Side::Below => card.top - meter.bottom,
    };
    println!(
        "{label}: card {}x{} {side:?} the meter ({}x{}), gap {measured} px (expected {gap}), left edges {} and {}, overlap {overlaps}",
        card.right - card.left,
        card.bottom - card.top,
        meter.right - meter.left,
        meter.bottom - meter.top,
        card.left,
        meter.left,
    );
    if overlaps {
        return Err(format!("{label}: the reminder overlaps the meter"));
    }
    if measured != gap || card.left != meter.left {
        return Err(format!("{label}: the reminder is not {gap} px from the meter on its left edge"));
    }
    Ok(side)
}

/// Locked, the card lets every click through, as the meter does, and shows
/// no ×; unlocked, it takes clicks again.
fn check_click_through(locked: bool, cx: &mut AsyncApp) -> Result<()> {
    let card = window_hwnd(WindowKind::Reminder, cx)?;
    let style = overlay::ex_style(card);
    let click_through = style & WS_EX_TRANSPARENT.0 != 0 && style & WS_EX_LAYERED.0 != 0;
    let view = window_handle::<ReminderView>(WindowKind::Reminder, cx)?;
    let without_dismiss = view.update(cx, |view, _, _| view.is_locked()).map_err(gone)?;
    println!("meter locked {locked}: the card is click-through {click_through}, drawn without × {without_dismiss}");
    if click_through != locked || without_dismiss != locked {
        return Err(format!("the reminder does not follow the meter's lock ({locked})"));
    }
    Ok(())
}

fn set_meter_locked(meter: WindowHandle<MeterView>, locked: bool, cx: &mut AsyncApp) -> Result<()> {
    meter.update(cx, |view, window, cx| view.set_locked(locked, window, cx)).map_err(|error| format!("overlay window gone: {error:#}"))
}

/// On the in-memory store of capture runs, so the meter and the card both
/// follow, as with the switch in Settings.
fn set_layout(layout: Layout, cx: &mut AsyncApp) {
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.overlay.layout = layout)));
}

fn move_meter(meter: HWND, x: i32, y: i32) -> Result<()> {
    unsafe { SetWindowPos(meter, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) }
        .map_err(|error| format!("cannot move the meter: {error}"))?;
    off_screen::ensure_off_screen(meter)
}

fn window_rect(hwnd: HWND) -> Result<Rect> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|error| format!("GetWindowRect failed: {error}"))?;
    Ok(Rect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom })
}

/// The meter and the card, each as PrintWindow draws it, placed as they sit
/// on the (virtual) screen.
async fn shoot_stack(meter_hwnd: HWND, dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    pause(cx, SETTLE).await;
    let card_hwnd = window_hwnd(WindowKind::Reminder, cx)?;
    shoot_together(&[meter_hwnd, card_hwnd], dir, name, cx).await
}

/// Windows each as PrintWindow draws it, placed as they sit on the (virtual)
/// screen, over a plain backdrop.
pub(crate) async fn shoot_together(windows: &[HWND], dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    let raw: Vec<isize> = windows.iter().map(|hwnd| hwnd.0 as isize).collect();
    let count = raw.len();
    let path = dir.join(format!("{name}.png"));
    let target = path.clone();
    let (width, height) = cx
        .background_executor()
        .spawn(async move {
            let stack = raw
                .into_iter()
                .map(|raw| {
                    let hwnd = HWND(raw as *mut c_void);
                    Ok::<_, String>((window_rect(hwnd)?, grab_image(hwnd)?))
                })
                .collect::<Result<Vec<_>>>()?;
            let left = stack.iter().map(|(rect, _)| rect.left).min().unwrap_or(0) - STACK_MARGIN;
            let top = stack.iter().map(|(rect, _)| rect.top).min().unwrap_or(0) - STACK_MARGIN;
            let right = stack.iter().map(|(rect, _)| rect.right).max().unwrap_or(0) + STACK_MARGIN;
            let bottom = stack.iter().map(|(rect, _)| rect.bottom).max().unwrap_or(0) + STACK_MARGIN;
            let mut canvas = RgbaImage::from_pixel((right - left) as u32, (bottom - top) as u32, BACKDROP);
            for (rect, image) in &stack {
                imageops::overlay(&mut canvas, image, i64::from(rect.left - left), i64::from(rect.top - top));
            }
            canvas.save(&target).map_err(|error| format!("cannot write {}: {error}", target.display()))?;
            Ok::<_, String>(canvas.dimensions())
        })
        .await?;
    println!("captured {} ({width}x{height}, {count} windows)", path.display());
    Ok(())
}

fn gone(error: impl std::fmt::Display) -> String {
    format!("the reminder window is gone: {error:#}")
}
