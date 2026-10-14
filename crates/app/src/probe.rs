//! `--overlay-probe`: exercises the overlay window requirements against the
//! real Win32 window and prints one line per check. It touches only our own
//! window; the only other window it looks at is the foreground one, by handle.
//! It sends no mouse input to anything.

use std::fmt::Display;
use std::mem::size_of;
use std::time::Duration;

use gpui::{App, AsyncApp, WindowHandle};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::HiDpi::{
    GetAwarenessFromDpiAwarenessContext, GetDpiForMonitor, GetDpiForWindow,
    GetWindowDpiAwarenessContext, DPI_AWARENESS_PER_MONITOR_AWARE, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetForegroundWindow, GetWindow, GetWindowRect,
    IsWindowVisible, PostMessageW, SendMessageW, SetWindowPos, WindowFromPoint, GW_OWNER,
    HTCLIENT, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCHITTEST,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT,
};

use crate::meter::MeterView;
use crate::overlay;

/// The window is exactly the 310 px card.
const CLIENT_WIDTH: f32 = 310.0;
const SETTLE: Duration = Duration::from_millis(1500);
const PHASE_HOLD: Duration = Duration::from_millis(1800);
const MESSAGE_SETTLE: Duration = Duration::from_millis(80);
/// Lives in `Win32_UI_Controls`, a feature this crate does not otherwise need.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// Client coordinates at 100% content scale: header, then the gauge line (the
/// tool row under the pointer), then the rows.
const TITLE_POINT: (f32, f32) = (60.0, 20.0);
/// Reset, capsule, lock: the lock button is third with a meter, second without.
const LOCK_BUTTON_WITH_RESET: (f32, f32) = (70.0, 42.0);
const LOCK_BUTTON_WITHOUT_RESET: (f32, f32) = (46.0, 42.0);
const ROW_POINT: (f32, f32) = (130.0, 64.0);

pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn run(window: WindowHandle<MeterView>, foreground_before_open: HWND, has_reset: bool, cx: &mut App) {
    cx.spawn(async move |cx| {
        probe(window, foreground_before_open, has_reset, cx).await;
        println!("DONE");
        cx.update(|cx| cx.quit());
    })
    .detach();
}

fn report(id: &str, ok: bool, detail: impl Display) {
    println!("{id}\t{}\t{detail}", if ok { "PASS" } else { "FAIL" });
}

fn info(id: &str, detail: impl Display) {
    println!("{id}\tINFO\t{detail}");
}

async fn pause(cx: &AsyncApp, duration: Duration) {
    cx.background_executor().timer(duration).await;
}

async fn probe(window: WindowHandle<MeterView>, foreground_before_open: HWND, has_reset: bool, cx: &mut AsyncApp) {
    let lock_button = if has_reset { LOCK_BUTTON_WITH_RESET } else { LOCK_BUTTON_WITHOUT_RESET };
    pause(cx, SETTLE).await;

    let Ok(Ok((hwnd, scale))) = window.update(cx, |_, window, _| {
        overlay::hwnd(window).map(|hwnd| (hwnd, window.scale_factor()))
    }) else {
        report("5e", false, "no Win32 handle from the GPUI window");
        return;
    };
    report("5e", true, format!("raw-window-handle gave HWND {:#x}", hwnd.0 as usize));

    let physical = |(x, y): (f32, f32)| ((x * scale).round() as i32, (y * scale).round() as i32);
    let screen_point = |logical: (f32, f32)| {
        let (x, y) = physical(logical);
        let mut point = POINT { x, y };
        unsafe {
            let _ = ClientToScreen(hwnd, &mut point);
        }
        point
    };
    let foreground_unchanged = || unsafe { GetForegroundWindow() } == foreground_before_open;

    // 5a / 5b: styles and activation.
    info(
        "5b.foreground",
        format!(
            "foreground window before the overlay opened: HWND {:#x}, not ours: {}",
            foreground_before_open.0 as usize,
            !foreground_before_open.0.is_null() && foreground_before_open != hwnd
        ),
    );
    let style = overlay::ex_style(hwnd);
    let has = |flag: u32| style & flag != 0;
    report(
        "5a",
        has(WS_EX_TOOLWINDOW.0)
            && has(WS_EX_TOPMOST.0)
            && !has(WS_EX_APPWINDOW.0)
            && unsafe { GetWindow(hwnd, GW_OWNER) }.is_err()
            && unsafe { IsWindowVisible(hwnd) }.as_bool(),
        format!("exstyle {style:#010x}: TOOLWINDOW, TOPMOST, no APPWINDOW, no owner, visible"),
    );
    report(
        "5b.open",
        has(WS_EX_NOACTIVATE.0) && foreground_unchanged(),
        "WS_EX_NOACTIVATE set; foreground window unchanged since before the overlay opened",
    );
    let active = window
        .update(cx, |_, window, _| window.is_window_active())
        .unwrap_or(true);
    report("5b.inactive", !active, "GPUI reports the overlay window as not active");

    // 5f: DPI.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let mut client = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    let client_width = client.right - client.left;
    let per_monitor_aware = unsafe {
        GetAwarenessFromDpiAwarenessContext(GetWindowDpiAwarenessContext(hwnd))
            == DPI_AWARENESS_PER_MONITOR_AWARE
    };
    report(
        "5f.window",
        per_monitor_aware
            && (scale * 96.0 - dpi as f32).abs() < 0.5
            && (client_width as f32 - CLIENT_WIDTH * scale).abs() <= 1.0,
        format!(
            "per-monitor aware: {per_monitor_aware}; window dpi {dpi}, GPUI scale {scale}, client width {client_width}px for {CLIENT_WIDTH} logical"
        ),
    );
    let monitors = monitor_dpis();
    info(
        "5f.monitors",
        monitors
            .iter()
            .map(|monitor| format!("{}x{}@{}dpi", monitor.width, monitor.height, monitor.dpi))
            .collect::<Vec<_>>()
            .join(", "),
    );
    if let Some(other) = monitors.iter().find(|monitor| monitor.dpi != dpi) {
        check_other_monitor(window, hwnd, other, cx).await;
    } else {
        info("5f.cross", "only one DPI in use on this machine; cross-monitor check not possible");
    }

    // 5d: hover while unfocused; the whole card, header included, is client area.
    let hover_at = |point| {
        let (x, y) = physical(point);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_MOUSEMOVE, WPARAM(0), lparam(x, y));
        }
    };
    hover_at(ROW_POINT);
    pause(cx, MESSAGE_SETTLE).await;
    let hovered = window.update(cx, |view, _, _| view.pointer_over).unwrap_or(false);
    report(
        "5d.hover",
        hovered && foreground_unchanged(),
        "a mouse move over the card showed the controls while another window had focus",
    );
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_MOUSELEAVE, WPARAM(0), LPARAM(0));
    }
    pause(cx, MESSAGE_SETTLE).await;
    let hovered = window.update(cx, |view, _, _| view.pointer_over).unwrap_or(true);
    report("5d.unhover", !hovered, "the pointer leaving the window put the gauge back");

    for (id, point, expected, what) in [
        ("5d.hit.title", TITLE_POINT, HTCLIENT, "header title area is client area"),
        ("5d.hit.button", lock_button, HTCLIENT, "tool row button is client area"),
        ("5d.hit.row", ROW_POINT, HTCLIENT, "row area is client area"),
    ] {
        hover_at(point);
        pause(cx, MESSAGE_SETTLE).await;
        let screen = screen_point(point);
        let result = unsafe {
            SendMessageW(hwnd, WM_NCHITTEST, Some(WPARAM(0)), Some(lparam(screen.x, screen.y)))
        };
        report(id, result.0 == expected as isize, format!("{what} (WM_NCHITTEST = {})", result.0));
    }

    let outside = {
        let mut point = screen_point((0.0, ROW_POINT.1));
        point.x -= 4;
        point
    };
    let result = unsafe {
        SendMessageW(hwnd, WM_NCHITTEST, Some(WPARAM(0)), Some(lparam(outside.x, outside.y)))
    };
    info(
        "5a.margin",
        format!("WM_NCHITTEST 4px left of the card is {} (window rect: {})", result.0, window_rect_text(hwnd)),
    );

    // 5c: lock toggles click-through at runtime.
    let centre = screen_point(ROW_POINT);
    let top_at_centre = || unsafe { WindowFromPoint(centre) };

    println!("PHASE unlocked {}", window_rect_text(hwnd));
    pause(cx, PHASE_HOLD).await;
    report(
        "5c.unlocked",
        top_at_centre() == hwnd,
        "WindowFromPoint over the overlay returns the overlay",
    );

    let locked = window.update(cx, |_, window, cx| overlay::set_click_through(window, cx, true));
    pause(cx, MESSAGE_SETTLE).await;
    let style = overlay::ex_style(hwnd);
    println!("PHASE locked {}", window_rect_text(hwnd));
    pause(cx, PHASE_HOLD).await;
    report(
        "5c.lock",
        matches!(locked, Ok(Ok(())))
            && style & WS_EX_TRANSPARENT.0 != 0
            && style & WS_EX_LAYERED.0 != 0
            && top_at_centre() != hwnd
            && foreground_unchanged(),
        format!(
            "exstyle {style:#010x} has LAYERED|TRANSPARENT; WindowFromPoint over the overlay skips it; foreground unchanged"
        ),
    );

    let unlocked = window.update(cx, |_, window, cx| overlay::set_click_through(window, cx, false));
    pause(cx, MESSAGE_SETTLE).await;
    let style = overlay::ex_style(hwnd);
    report(
        "5c.unlock",
        matches!(unlocked, Ok(Ok(())))
            && style & WS_EX_TRANSPARENT.0 == 0
            && top_at_centre() == hwnd
            && foreground_unchanged(),
        format!("exstyle {style:#010x} without TRANSPARENT; the overlay is hit again; foreground unchanged"),
    );

    hover_at(lock_button);
    pause(cx, MESSAGE_SETTLE).await;
    let (button_x, button_y) = physical(lock_button);
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_LBUTTONDOWN, WPARAM(1), lparam(button_x, button_y));
        let _ = PostMessageW(Some(hwnd), WM_LBUTTONUP, WPARAM(0), lparam(button_x, button_y));
    }
    pause(cx, Duration::from_millis(300)).await;
    let locked_by_button = overlay::ex_style(hwnd) & WS_EX_TRANSPARENT.0 != 0;
    let _ = window.update(cx, |view, window, cx| view.toggle_lock(window, cx));
    pause(cx, MESSAGE_SETTLE).await;
    let unlocked_by_shortcut_path = overlay::ex_style(hwnd) & WS_EX_TRANSPARENT.0 == 0;
    report(
        "5c.button",
        locked_by_button && unlocked_by_shortcut_path && foreground_unchanged(),
        "the lock button locked the window; the Alt+CapsLock toggle unlocked it",
    );
}

struct Monitor {
    dpi: u32,
    width: i32,
    height: i32,
    origin: (i32, i32),
}

fn monitor_dpis() -> Vec<Monitor> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let monitors = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        unsafe {
            if GetMonitorInfoW(monitor, &mut info).as_bool()
                && GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok()
            {
                monitors.push(Monitor {
                    dpi: dpi_x,
                    width: info.rcMonitor.right - info.rcMonitor.left,
                    height: info.rcMonitor.bottom - info.rcMonitor.top,
                    origin: (info.rcMonitor.left, info.rcMonitor.top),
                });
            }
        }
        true.into()
    }

    let mut monitors: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut monitors as *mut _ as isize));
    }
    monitors
}

async fn check_other_monitor(
    window: WindowHandle<MeterView>,
    hwnd: HWND,
    other: &Monitor,
    cx: &mut AsyncApp,
) {
    let home = window_origin(hwnd);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            other.origin.0 + 100,
            other.origin.1 + 100,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    pause(cx, SETTLE).await;
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let scale = window.update(cx, |_, window, _| window.scale_factor()).unwrap_or(0.0);
    let mut client = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    let client_width = client.right - client.left;
    report(
        "5f.cross",
        dpi == other.dpi
            && (scale * 96.0 - dpi as f32).abs() < 0.5
            && (client_width as f32 - CLIENT_WIDTH * scale).abs() <= 1.0,
        format!(
            "moved to a {}dpi monitor: window dpi {dpi}, GPUI scale {scale}, client width {client_width}px",
            other.dpi
        ),
    );
    unsafe {
        let _ = SetWindowPos(hwnd, None, home.0, home.1, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
    pause(cx, SETTLE).await;
}

fn lparam(x: i32, y: i32) -> LPARAM {
    LPARAM((((y as u16) as isize) << 16) | (x as u16) as isize)
}

fn window_origin(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect);
    }
    (rect.left, rect.top)
}

pub fn window_rect_text(hwnd: HWND) -> String {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect);
    }
    format!("{} {} {} {}", rect.left, rect.top, rect.right, rect.bottom)
}
