//! The meter's overlay window: transparent, borderless, topmost, out of the
//! taskbar and Alt+Tab, never activated, click-through while locked, and
//! exactly as large as the card it shows.

use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;

use gpui::{
    canvas, point, px, size, App, AppContext as _, Bounds, DispatchPhase, Div, Entity, InteractiveElement,
    IntoElement, MouseButton, MouseMoveEvent, MouseUpEvent, Pixels, Point, Size, Styled as _, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE,
    DWMWA_NCRENDERING_POLICY, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWINDOWATTRIBUTE,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindow, GetWindowLongPtrW, GetWindowRect, SetLayeredWindowAttributes,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, GWL_EXSTYLE, GWL_STYLE, GW_HWNDNEXT,
    HWND_NOTOPMOST, HWND_TOPMOST,
    LWA_ALPHA, MA_NOACTIVATE, SPI_GETCLIENTAREAANIMATION, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WINDOW_STYLE, WM_MOUSEACTIVATE, WM_NCDESTROY, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
};

use crate::meter::{MeterModel, MeterView};

/// The first frame fits the window to the card; this only has to be close.
const INITIAL_WIDTH: f32 = 310.0;
const INITIAL_HEIGHT: f32 = 280.0;
const OVERLAY_MARGIN: f32 = 64.0;
const NO_CLICK_ACTIVATION_SUBCLASS: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Top-left of the primary monitor.
    Screen,
    /// Outside every monitor, for captures that must never show on screen.
    #[cfg(feature = "probes")]
    OffScreen,
}

/// The window every overlay is: a pop-up (topmost, tool window) with no title
/// bar, never focused or shown by GPUI itself, so the caller can strip the
/// frame before it appears.
pub fn popup_options(width: f32, height: f32) -> WindowOptions {
    let bounds = Bounds::new(point(px(OVERLAY_MARGIN), px(OVERLAY_MARGIN)), size(px(width), px(height)));
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: false,
        show: false,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    }
}

/// Strips the frame and makes the window non-activating: clicking it never
/// takes focus from the game.
pub fn make_overlay(hwnd: HWND) -> Result<(), String> {
    remove_frame(hwnd);
    update_ex_style(hwnd, |style| style | WS_EX_NOACTIVATE.0);
    unsafe { SetWindowSubclass(hwnd, Some(refuse_click_activation), NO_CLICK_ACTIVATION_SUBCLASS, 0) }
        .as_bool()
        .then_some(())
        .ok_or_else(|| "cannot keep the overlay from activating on a click: SetWindowSubclass failed".to_string())
}

/// GPUI answers `WM_MOUSEACTIVATE` with `MA_ACTIVATE` itself and never asks
/// `DefWindowProc`, the only place `WS_EX_NOACTIVATE` stops a click from
/// activating a window. The style is read per message because the typing
/// focus exception clears it while a field is edited. `MA_NOACTIVATE`, not
/// `MA_NOACTIVATEANDEAT`: the click still reaches GPUI.
unsafe extern "system" fn refuse_click_activation(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass: usize,
    _: usize,
) -> LRESULT {
    match message {
        WM_MOUSEACTIVATE if ex_style(hwnd) & WS_EX_NOACTIVATE.0 != 0 => LRESULT(MA_NOACTIVATE as isize),
        WM_NCDESTROY => unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(refuse_click_activation), subclass);
            DefSubclassProc(hwnd, message, wparam, lparam)
        },
        _ => unsafe { DefSubclassProc(hwnd, message, wparam, lparam) },
    }
}

/// Opens the meter's window, shown or not: a closed meter keeps its window,
/// hidden, so it opens again where it was left.
pub fn open(
    cx: &mut App,
    model: Entity<MeterModel>,
    placement: Placement,
    shown: bool,
) -> Result<WindowHandle<MeterView>, String> {
    let handle = cx
        .open_window(popup_options(INITIAL_WIDTH, INITIAL_HEIGHT), |_, cx| cx.new(|cx| MeterView::new(model, cx)))
        .map_err(|error| format!("cannot open the overlay window: {error:#}"))?;

    handle
        .update(cx, |_, window, _| -> Result<(), String> {
            let hwnd = hwnd(window)?;
            make_overlay(hwnd)?;
            let scale = window.scale_factor();
            let (width, height) = (INITIAL_WIDTH * scale, INITIAL_HEIGHT * scale);
            match placement {
                Placement::Screen => {
                    let margin = (OVERLAY_MARGIN * scale).round() as i32;
                    place(hwnd, margin, margin, width, height)?;
                }
                #[cfg(feature = "probes")]
                Placement::OffScreen => off_screen::place(hwnd, width, height)?,
            }
            if shown {
                reveal(hwnd, placement)?;
            }
            Ok(())
        })
        .map_err(|error| format!("cannot reach the overlay window: {error:#}"))??;
    Ok(handle)
}

pub fn show_without_activating(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

/// Shows a window `placement` put in place, without activating it; off
/// screen it is checked to be past every monitor before and after.
pub fn reveal(hwnd: HWND, placement: Placement) -> Result<(), String> {
    match placement {
        Placement::Screen => {
            show_without_activating(hwnd);
            Ok(())
        }
        #[cfg(feature = "probes")]
        Placement::OffScreen => {
            off_screen::ensure_off_screen(hwnd)?;
            show_without_activating(hwnd);
            off_screen::ensure_off_screen(hwnd)
        }
    }
}

pub fn hide(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// Puts the window above every non-topmost window and above the other
/// topmost ones, without activating it.
pub fn raise_without_activating(hwnd: HWND) {
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Takes the window out of the topmost band, without activating it, and puts
/// it behind `went_to`, the window the user went to. Leaving the band puts it
/// above every normal window, the one just activated included when that one
/// rose first, so it then swaps places with `went_to` only when `went_to` is
/// right below: a topmost window is not, nor is the desktop, which stays at
/// the bottom, and the window must not sink under either.
pub fn lower_behind(hwnd: HWND, went_to: Option<HWND>) {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
        let next = GetWindow(hwnd, GW_HWNDNEXT);
        if let Some(went_to) = went_to.filter(|&went_to| next.is_ok_and(|next| next == went_to)) {
            let _ = SetWindowPos(hwnd, Some(went_to), 0, 0, 0, 0, flags);
        }
    }
}

/// Makes the overlay visible, on top, and on a monitor, for a later launch
/// asking the running meter to show itself. It does not take focus: the
/// overlay is non-activating by design, and pulling focus out of the game
/// would be worse than a meter that stays where it was.
pub fn bring_forward(hwnd: HWND) {
    show_without_activating(hwnd);
    if unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL) }.is_invalid() {
        move_onto_primary_monitor(hwnd);
    }
    raise_without_activating(hwnd);
}

fn move_onto_primary_monitor(hwnd: HWND) {
    let mut monitor = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let primary = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        if !GetMonitorInfoW(primary, &mut monitor).as_bool() {
            return;
        }
        let margin = OVERLAY_MARGIN as i32;
        let _ = SetWindowPos(
            hwnd,
            None,
            monitor.rcWork.left + margin,
            monitor.rcWork.top + margin,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

/// Sizes the window to the content whose children's bounds were just laid
/// out, when it differs from the window; see [`fit_client_size`].
pub fn fit_to_children(children: &[Bounds<Pixels>], window: &Window, cx: &mut App) {
    let wanted = children.iter().fold(size(px(0.), px(0.)), |wanted, child| {
        size(
            wanted.width.max(child.origin.x + child.size.width),
            wanted.height.max(child.origin.y + child.size.height),
        )
    });
    let current = window.viewport_size();
    if (wanted.width - current.width).abs() >= px(0.5) || (wanted.height - current.height).abs() >= px(0.5) {
        fit_client_size(window, wanted, cx);
    }
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
pub(crate) fn place(hwnd: HWND, x: i32, y: i32, width: f32, height: f32) -> Result<(), String> {
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

/// Where a drag took hold of the window: the pointer's offset from the
/// window's top left in device pixels, taken once at the press. Kept in
/// device pixels so a move onto a monitor with another DPI (which changes the
/// window's scale) does not make the window jump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Grab {
    x: i32,
    y: i32,
}

impl Grab {
    /// `position` is the press, in the window's logical pixels.
    fn at(window: &Window, position: Point<Pixels>) -> Self {
        Self::at_scale(position, window.scale_factor())
    }

    fn at_scale(position: Point<Pixels>, scale: f32) -> Self {
        let (x, y) = device(position, scale);
        Self { x, y }
    }

    /// How far the window must move, in device pixels, for the grabbed point
    /// to be under the pointer at `position` (logical, at `scale`).
    fn offset(self, position: Point<Pixels>, scale: f32) -> (i32, i32) {
        let (x, y) = device(position, scale);
        (x - self.x, y - self.y)
    }
}

fn device(position: Point<Pixels>, scale: f32) -> (i32, i32) {
    ((position.x.as_f32() * scale).round() as i32, (position.y.as_f32() * scale).round() as i32)
}

/// Moves the window so the grabbed point is under the pointer at `position`
/// (logical, at the window's current scale), after the current event as in
/// [`fit_client_size`]. The target is computed now, from the position the
/// event's coordinates are relative to, so moves that arrive before the
/// window has caught up do not add up.
fn drag_to(window: &Window, grab: Grab, position: Point<Pixels>, cx: &mut App) {
    let (dx, dy) = grab.offset(position, window.scale_factor());
    if dx == 0 && dy == 0 {
        return;
    }
    let target = hwnd(window).and_then(|hwnd| {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|error| format!("GetWindowRect failed: {error}"))?;
        Ok((hwnd, rect.left + dx, rect.top + dy))
    });
    let (hwnd, x, y) = match target {
        Ok(target) => target,
        Err(error) => {
            eprintln!("cannot move the overlay: {error}");
            return;
        }
    };
    cx.spawn(async move |_| {
        if let Err(error) = unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) } {
            eprintln!("cannot move the overlay: {error}");
        }
    })
    .detach();
}

/// Pressing a handle and moving drags the window. Not GPUI's
/// `WindowControlArea::Drag`: that makes the area non-client, and Windows
/// reports the pointer crossing between client and non-client area as a
/// leave, which clears GPUI's hover flag while the pointer stays on the
/// window. The grab lives from the press to the release, or to a move
/// without the left button.
#[derive(Clone, Default)]
pub struct WindowDrag(Rc<Cell<Option<Grab>>>);

impl WindowDrag {
    pub fn handle(&self, handle: Div) -> Div {
        let grab = self.0.clone();
        handle.on_mouse_down(MouseButton::Left, move |event, window, _| {
            grab.set(Some(Grab::at(window, event.position)));
        })
    }

    /// [`handle`](Self::handle) for an area holding controls: a press a
    /// control already took (`prevent_default`) does not drag, and a press
    /// that drags is taken, so no element focuses on it. `grabbed` hears of
    /// each drag that starts.
    pub fn handle_around_controls<E: InteractiveElement>(
        &self,
        area: E,
        grabbed: impl Fn(&mut Window, &mut App) + 'static,
    ) -> E {
        let grab = self.0.clone();
        area.on_mouse_down(MouseButton::Left, move |event, window, cx| {
            if window.default_prevented() {
                return;
            }
            grab.set(Some(Grab::at(window, event.position)));
            window.prevent_default();
            grabbed(window, cx);
        })
    }

    pub fn stop(&self) {
        self.0.set(None);
    }

    /// Drawn once per window: the window follows the pointer wherever it is
    /// while the button is held, not only while it is over a handle.
    pub fn listener(&self) -> impl IntoElement {
        let grab = self.0.clone();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moved = grab.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let Some(from) = moved.get() else {
                        return;
                    };
                    if event.pressed_button == Some(MouseButton::Left) {
                        drag_to(window, from, event.position, cx);
                    } else {
                        moved.set(None);
                    }
                });
                let released = grab.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, _| {
                    if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                        released.set(None);
                    }
                });
            },
        )
        .absolute()
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

pub(crate) fn update_ex_style(hwnd: HWND, change: impl FnOnce(u32) -> u32) {
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

    /// Moves the window past the left edge of the virtual screen, without
    /// showing it. `width` and `height` are device pixels.
    pub fn place(hwnd: HWND, width: f32, height: f32) -> Result<(), String> {
        let (left, top) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
        super::place(hwnd, left - CLEARANCE, top, width, height)?;
        ensure_off_screen(hwnd)
    }

    /// [`place`], then shows the window. `width` and `height` are device pixels.
    pub fn show(hwnd: HWND, width: f32, height: f32) -> Result<(), String> {
        place(hwnd, width, height)?;
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

#[cfg(test)]
mod tests {
    use gpui::{point, px};

    use super::Grab;

    #[test]
    fn a_drag_follows_the_pointer_in_device_pixels() {
        let grab = Grab::at_scale(point(px(60.), px(20.)), 1.0);
        assert_eq!(grab.offset(point(px(30.), px(32.)), 1.0), (-30, 12));
        assert_eq!(grab.offset(point(px(60.), px(20.)), 1.0), (0, 0));
    }

    #[test]
    fn a_new_dpi_mid_drag_does_not_move_the_window() {
        let grab = Grab::at_scale(point(px(60.), px(20.)), 1.0);
        // The window reached a 150% monitor: the same device point now reads
        // as 40 x 13.3 logical pixels.
        assert_eq!(grab.offset(point(px(40.), px(20. / 1.5)), 1.5), (0, 0));
        assert_eq!(grab.offset(point(px(50.), px(20. / 1.5)), 1.5), (15, 0));
    }
}
