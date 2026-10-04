use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
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

fn rect(rect: RECT) -> Rect {
    Rect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom }
}
