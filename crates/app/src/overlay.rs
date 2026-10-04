//! The meter's overlay window: transparent, borderless, topmost, out of the
//! taskbar and Alt+Tab, never activated, click-through while locked, and
//! exactly as large as the card it shows.

use std::ffi::c_void;

use gpui::{
    point, px, size, App, AppContext as _, Bounds, Entity, Pixels, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{COLORREF, HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE,
    DWMWA_NCRENDERING_POLICY, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWINDOWATTRIBUTE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowLongPtrW, GetWindowRect, SetLayeredWindowAttributes,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, GWL_EXSTYLE, GWL_STYLE,
    LWA_ALPHA, SPI_GETCLIENTAREAANIMATION, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WINDOW_STYLE, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
};

use crate::meter::{MeterModel, MeterView};

/// The first frame fits the window to the card; this only has to be close.
const INITIAL_WIDTH: f32 = 310.0;
const INITIAL_HEIGHT: f32 = 280.0;
const OVERLAY_MARGIN: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Top-left of the primary monitor.
    Screen,
    /// Outside every monitor, for captures that must never show on screen.
    #[cfg(feature = "probes")]
    OffScreen,
}

pub fn open(
    cx: &mut App,
    model: Entity<MeterModel>,
    placement: Placement,
) -> Result<WindowHandle<MeterView>, String> {
    let bounds = Bounds::new(
        point(px(OVERLAY_MARGIN), px(OVERLAY_MARGIN)),
        size(px(INITIAL_WIDTH), px(INITIAL_HEIGHT)),
    );
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                // Shown below, once the frame is gone, so it never flashes.
                show: false,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                window_background: WindowBackgroundAppearance::Transparent,
                ..Default::default()
            },
            |_, cx| cx.new(|cx| MeterView::new(model, cx)),
        )
        .map_err(|error| format!("cannot open the overlay window: {error:#}"))?;

    handle
        .update(cx, |_, window, _| -> Result<(), String> {
            let hwnd = hwnd(window)?;
            remove_frame(hwnd);
            update_ex_style(hwnd, |style| style | WS_EX_NOACTIVATE.0);
            let scale = window.scale_factor();
            let (width, height) = (INITIAL_WIDTH * scale, INITIAL_HEIGHT * scale);
            match placement {
                Placement::Screen => {
                    let margin = (OVERLAY_MARGIN * scale).round() as i32;
                    place(hwnd, margin, margin, width, height)?;
                    unsafe {
                        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    }
                    Ok(())
                }
                #[cfg(feature = "probes")]
                Placement::OffScreen => off_screen::show(hwnd, width, height),
            }
        })
        .map_err(|error| format!("cannot reach the overlay window: {error:#}"))??;
    Ok(handle)
}

/// GPUI creates its pop-up with an overlapped style, so Windows gives it a
/// frame: 8 px at the sides and bottom that answer clicks, and on Windows 11 a
/// 1 px light border with rounded corners drawn by DWM around the whole
/// window. A plain `WS_POPUP` has no frame, so the window is exactly its
/// client area. The DWM attributes are belt and braces for the border and
/// corners; Windows 10 does not know them, and their failure changes nothing.
fn remove_frame(hwnd: HWND) {
    let framed = WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;
    unsafe {
        let style = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
        SetWindowLongPtrW(hwnd, GWL_STYLE, ((style & !framed) | WS_POPUP).0 as isize);
    }
    set_dwm_attribute(hwnd, DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED.0);
    set_dwm_attribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND.0);
    set_dwm_attribute(hwnd, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE);
}

fn set_dwm_attribute<T: Copy>(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, value: T) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            attribute,
            &value as *const T as *const c_void,
            std::mem::size_of::<T>() as u32,
        );
    }
}

/// Moves and sizes the window, in device pixels, without activating it.
fn place(hwnd: HWND, x: i32, y: i32, width: f32, height: f32) -> Result<(), String> {
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width.ceil() as i32,
            height.ceil() as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|error| format!("cannot place the overlay: {error}"))
    }
}

/// The native handle behind a GPUI window, through `raw-window-handle`.
pub fn hwnd(window: &Window) -> Result<HWND, String> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| format!("window handle unavailable: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(win32) => Ok(HWND(win32.hwnd.get() as *mut c_void)),
        other => Err(format!("not a Win32 window handle: {other:?}")),
    }
}

/// Locked means click-through: every mouse event goes to whatever is behind.
/// Windows only skips a window in hit testing when it is both transparent and
/// layered.
pub fn set_click_through(window: &Window, click_through: bool) -> Result<(), String> {
    let hwnd = hwnd(window)?;
    update_ex_style(hwnd, |style| {
        if click_through {
            style | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0
        } else {
            style & !WS_EX_TRANSPARENT.0
        }
    });
    unsafe {
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)
            .map_err(|error| format!("SetLayeredWindowAttributes failed: {error}"))
    }
}

/// Sizes the client area to `content` (logical pixels), keeping the window's
/// top-left corner. GPUI's own `Window::resize` calls `SetWindowPos` without
/// `SWP_NOACTIVATE` and `SWP_NOZORDER`, which could activate the overlay over
/// the game, so this does it directly. It runs after the current frame:
/// `SetWindowPos` sends `WM_SIZE` back into GPUI synchronously.
pub fn fit_client_size(window: &Window, content: Size<Pixels>, cx: &mut App) {
    let hwnd = match hwnd(window) {
        Ok(hwnd) => hwnd,
        Err(error) => {
            eprintln!("cannot fit the overlay to its card: {error}");
            return;
        }
    };
    let scale = window.scale_factor();
    let width = (content.width.as_f32() * scale).ceil() as i32;
    let height = (content.height.as_f32() * scale).ceil() as i32;
    cx.spawn(async move |_| {
        if let Err(error) = set_client_size(hwnd, width, height) {
            eprintln!("cannot fit the overlay to its card: {error}");
        }
    })
    .detach();
}

fn set_client_size(hwnd: HWND, width: i32, height: i32) -> Result<(), String> {
    let mut outer = RECT::default();
    let mut client = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer).map_err(|error| format!("GetWindowRect failed: {error}"))?;
        GetClientRect(hwnd, &mut client).map_err(|error| format!("GetClientRect failed: {error}"))?;
        let frame_width = (outer.right - outer.left) - (client.right - client.left);
        let frame_height = (outer.bottom - outer.top) - (client.bottom - client.top);
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            width + frame_width,
            height + frame_height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|error| format!("SetWindowPos failed: {error}"))
    }
}

pub fn ex_style(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 }
}

/// Windows' "Animation effects" setting (Settings > Accessibility > Visual
/// effects). GPUI does not read it, so bars and fades would move regardless.
pub fn system_animates() -> bool {
    let mut enabled = windows::core::BOOL(1);
    let queried = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut enabled as *mut _ as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    queried.is_err() || enabled.as_bool()
}

fn update_ex_style(hwnd: HWND, change: impl FnOnce(u32) -> u32) {
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, change(ex_style(hwnd)) as isize);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// Showing the overlay where no monitor can display it. GPUI is asked for a
/// hidden window (it would move bounds outside every monitor back onto the
/// primary one), which is then moved past the left edge of the virtual screen
/// and only then shown, without activation.
#[cfg(feature = "probes")]
pub mod off_screen {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_ENABLED,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWINDOWATTRIBUTE,
    };
    use windows::Win32::Graphics::Gdi::{MonitorFromRect, MONITOR_DEFAULTTONULL};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, ShowWindow,
        GWL_STYLE, SM_CXVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE,
        SW_SHOWNOACTIVATE,
    };

    /// Room to the left of the virtual screen, well beyond any card.
    const CLEARANCE: i32 = 4_000;

    /// `width` and `height` are device pixels.
    pub fn show(hwnd: HWND, width: f32, height: f32) -> Result<(), String> {
        let (left, top) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
        super::place(hwnd, left - CLEARANCE, top, width, height)?;
        ensure_off_screen(hwnd)?;
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        ensure_off_screen(hwnd)
    }

    /// The window's frame as Windows and DWM report it, for the captures' log.
    pub fn frame_report(hwnd: HWND) -> String {
        let mut outer = RECT::default();
        let mut client = RECT::default();
        unsafe {
            let _ = GetWindowRect(hwnd, &mut outer);
            let _ = GetClientRect(hwnd, &mut client);
        }
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
        let read = |attribute: DWMWINDOWATTRIBUTE| {
            let mut value = 0u32;
            unsafe {
                DwmGetWindowAttribute(hwnd, attribute, &mut value as *mut u32 as *mut c_void, 4)
                    .map(|()| format!("{value:#x}"))
                    .unwrap_or_else(|error| format!("unreadable ({})", error.message()))
            }
        };
        format!(
            "style {style:#010x}; window {}x{}, client {}x{}; DWM non-client rendering {}, corner preference {}, border colour {}",
            outer.right - outer.left,
            outer.bottom - outer.top,
            client.right,
            client.bottom,
            read(DWMWA_NCRENDERING_ENABLED),
            read(DWMWA_WINDOW_CORNER_PREFERENCE),
            read(DWMWA_BORDER_COLOR),
        )
    }

    /// Hides the window and fails if any part of it is on a monitor.
    pub fn ensure_off_screen(hwnd: HWND) -> Result<(), String> {
        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect).map_err(|error| format!("GetWindowRect failed: {error}"))?;
            let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            if !MonitorFromRect(&rect, MONITOR_DEFAULTTONULL).is_invalid() || virtual_width <= 0 {
                let _ = ShowWindow(hwnd, SW_HIDE);
                return Err(format!(
                    "the overlay reached a monitor at ({}, {})-({}, {}); hidden",
                    rect.left, rect.top, rect.right, rect.bottom
                ));
            }
        }
        Ok(())
    }
}
