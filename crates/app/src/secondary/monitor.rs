use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

use super::placement::Rect;

/// The window's rectangle and the work area (the monitor minus the taskbar)
/// of the monitor it is on, in device pixels.
pub fn window_and_work_area(hwnd: HWND) -> Option<(Rect, Rect)> {
    let mut window = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut window).ok()? };
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let found = unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    found.then(|| (rect(window), rect(info.rcWork)))
}

/// The work area of the monitor under the screen point, if there is one.
pub fn work_area_at((x, y): (i32, i32)) -> Option<Rect> {
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let found = unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONULL);
        !monitor.is_invalid() && GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    found.then(|| rect(info.rcWork))
}

fn rect(rect: RECT) -> Rect {
    Rect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom }
}
