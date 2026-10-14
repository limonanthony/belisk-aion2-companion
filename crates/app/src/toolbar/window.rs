//! The toolbar's window: an overlay like the meter's (topmost, out of the
//! taskbar and Alt+Tab, never activated) that is never click-through. It
//! opens where it was left, or at the top centre of the primary monitor, and
//! is exactly as large as the bar and the dropdown or tooltip it shows; one
//! that opens upward grows the window upward, so the bar stays still.

use gpui::{px, size, App, AppContext as _, Entity, Pixels, Size, Window, WindowHandle};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

use super::view::ToolbarView;
use super::Hub;
use crate::overlay::{self, Placement};
use crate::secondary::{self, Rect};
use crate::settings::{self, Spot};

/// The bar at 100% with no warning; the first frame fits the window to it.
const INITIAL_SIZE: (f32, f32) = (468.0, 34.0);
/// Between the top of the work area and a toolbar placed for the first time.
const TOP_MARGIN: f32 = 8.0;

pub fn open(hub: &Entity<Hub>, placement: Placement, cx: &mut App) -> Result<(WindowHandle<ToolbarView>, HWND), String> {
    let (width, height) = INITIAL_SIZE;
    let hub = hub.clone();
    let handle = cx
        .open_window(overlay::popup_options(width, height), move |window, cx| {
            cx.new(|cx| ToolbarView::new(hub, window, cx))
        })
        .map_err(|error| format!("cannot open the toolbar window: {error:#}"))?;
    let set_up = handle
        .update(cx, |_, window, cx| -> Result<HWND, String> {
            let hwnd = overlay::hwnd(window)?;
            overlay::make_overlay(hwnd)?;
            overlay::remember_drags(cx, hwnd, Spot::Toolbar);
            let scale = window.scale_factor();
            let (device_width, device_height) = (width * scale, height * scale);
            match placement {
                Placement::Screen => {
                    let size = (device_width.ceil() as i32, device_height.ceil() as i32);
                    let saved = settings::global(cx).read(cx).get().windows.remembered(Spot::Toolbar);
                    let margin = (TOP_MARGIN * scale).round() as i32;
                    overlay::place_after_update(cx, hwnd, device_width, device_height, move || {
                        saved
                            .and_then(|at| secondary::reopen_corner(at, size))
                            .or_else(|| top_centre(size.0, margin))
                            .unwrap_or((64, 64))
                    });
                    overlay::reveal_after_update(cx, hwnd, placement, "show the toolbar");
                }
                #[cfg(feature = "probes")]
                Placement::OffScreen => overlay::off_screen::show_after_update(cx, hwnd, device_width, device_height),
            }
            Ok(hwnd)
        })
        .map_err(|error| format!("cannot reach the toolbar window: {error:#}"))?;
    match set_up {
        Ok(hwnd) => Ok((handle, hwnd)),
        Err(error) => {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            Err(error)
        }
    }
}

fn top_centre(width: i32, margin: i32) -> Option<(i32, i32)> {
    let mut monitor = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let primary = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    if !unsafe { GetMonitorInfoW(primary, &mut monitor) }.as_bool() {
        return None;
    }
    let work = monitor.rcWork;
    Some(centred_at_top(Rect { left: work.left, top: work.top, right: work.right, bottom: work.bottom }, width, margin))
}

/// The top-left corner of a window `width` wide, centred at the top of `work`.
pub fn centred_at_top(work: Rect, width: i32, margin: i32) -> (i32, i32) {
    (work.left + (work.right - work.left - width) / 2, work.top + margin)
}

/// A dropdown or tooltip opens upward when the work area has less room under
/// the bar than it needs. Device pixels.
pub fn opens_upward(bar_bottom: i32, work_bottom: i32, needed: i32) -> bool {
    work_bottom - bar_bottom < needed
}

/// Sizes the window to `wanted` (logical), keeping the bar where it is on the
/// screen: when the bar moved `bar_shift` down inside the window (a dropdown
/// opened above it), the window moves up as much, and down again when it
/// closes. After the current frame, as `overlay::fit_client_size` does.
pub fn refit(window: &Window, wanted: Size<Pixels>, bar_shift: Pixels, cx: &mut App) {
    if bar_shift == px(0.) {
        let current = window.viewport_size();
        if (wanted.width - current.width).abs() >= px(0.5) || (wanted.height - current.height).abs() >= px(0.5) {
            overlay::fit_client_size(window, wanted, cx);
        }
        return;
    }
    let hwnd = match overlay::hwnd(window) {
        Ok(hwnd) => hwnd,
        Err(error) => {
            crate::app_log::warn(format!("cannot fit the toolbar: {error}"));
            return;
        }
    };
    let scale = window.scale_factor();
    let device = size((wanted.width.as_f32() * scale).ceil() as i32, (wanted.height.as_f32() * scale).ceil() as i32);
    let shift = (bar_shift.as_f32() * scale).round() as i32;
    cx.spawn(async move |_| {
        let mut rect = RECT::default();
        let moved = unsafe { GetWindowRect(hwnd, &mut rect) }.and_then(|()| unsafe {
            SetWindowPos(hwnd, None, rect.left, rect.top - shift, device.width, device.height, SWP_NOZORDER | SWP_NOACTIVATE)
        });
        if let Err(error) = moved {
            crate::app_log::warn(format!("cannot fit the toolbar: {error}"));
        }
    })
    .detach();
}

/// The bar's bottom edge and its monitor's work area bottom, in device pixels.
pub fn room(window: &Window) -> Option<(i32, i32)> {
    let hwnd = overlay::hwnd(window).ok()?;
    let (bar, work) = crate::secondary::window_and_work_area(hwnd)?;
    Some((bar.bottom, work.bottom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_toolbar_sits_at_the_top_centre() {
        let work = Rect { left: 0, top: 0, right: 1920, bottom: 1040 };
        assert_eq!(centred_at_top(work, 440, 8), (740, 8));
        let second = Rect { left: 1920, top: 0, right: 3840, bottom: 1080 };
        assert_eq!(centred_at_top(second, 440, 8), (2660, 8));
    }

    #[test]
    fn a_dropdown_opens_upward_only_without_room_under_the_bar() {
        assert!(!opens_upward(42, 1040, 230));
        assert!(opens_upward(1000, 1040, 230));
        assert!(!opens_upward(810, 1040, 230));
    }
}
