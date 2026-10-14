//! The meter's overlay window: transparent, borderless, topmost, out of the
//! taskbar and Alt+Tab, never activated, click-through while locked, and
//! exactly as large as the card it shows.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::fmt;
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
    ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindow, GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, PostMessageW, SendMessageW,
    SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, GWL_EXSTYLE,
    GWL_STYLE, GW_HWNDNEXT, HWND_NOTOPMOST, HWND_TOPMOST,
    LWA_ALPHA, MA_NOACTIVATE, SPI_GETCLIENTAREAANIMATION, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SHOW_WINDOW_CMD, SW_HIDE, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WINDOW_STYLE, WM_ACTIVATEAPP, WM_APP, WM_CANCELMODE, WM_CAPTURECHANGED, WM_EXITSIZEMOVE, WM_KILLFOCUS, WM_MOUSEACTIVATE, WM_MOVE,
    WM_NCDESTROY, WM_SHOWWINDOW, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
};

use crate::meter::{MeterModel, MeterView};
use crate::settings::{self, ScreenPoint, Spot};

/// The first frame fits the window to the card; this only has to be close.
const INITIAL_WIDTH: f32 = 310.0;
const INITIAL_HEIGHT: f32 = 280.0;
const OVERLAY_MARGIN: f32 = 64.0;
const NO_CLICK_ACTIVATION_SUBCLASS: usize = 1;
const OS_MOVE_SUBCLASS: usize = 2;

/// Posted to a window whose drag ended, so its last move and the report of
/// its place to GPUI happen from the message loop, outside any GPUI handler.
/// `wparam` 1: move to the screen point packed in `lparam` first.
const FINISH_DRAG: u32 = WM_APP + 0x100;

thread_local! {
    /// Windows run their messages on the UI thread, as GPUI does.
    static DRAGS: RefCell<Drags> = RefCell::new(Drags::default());
    /// What to tell of a window's place once the player leaves it there.
    static LEFT_LISTENERS: RefCell<HashMap<isize, Rc<dyn Fn(ScreenPoint)>>> = RefCell::new(HashMap::new());
    /// The windows to pull inside the work area after their first fit.
    static FIRST_FIT: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
}

/// The overlay window a drag is moving, and the move it still has to make.
#[derive(Default)]
struct Drags {
    dragged: Option<Dragged>,
    pending: Option<Move>,
}

struct Dragged {
    hwnd: isize,
    /// The `WindowDrag`'s grab: once it is let go, the drag is over even if
    /// nothing said so.
    grab: Rc<Cell<Option<Grab>>>,
}

/// A window and the screen point its top left goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Move {
    hwnd: isize,
    x: i32,
    y: i32,
}

/// A drag that ended: its window, and the move it had not made yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ending {
    hwnd: isize,
    target: Option<(i32, i32)>,
}

impl Drags {
    /// A pointer move asks for `target` while `grab` holds its window.
    /// Returns whether a deferred move must be scheduled (none waits yet),
    /// and the drag of another window this one replaces.
    fn ask(&mut self, target: Move, grab: &Rc<Cell<Option<Grab>>>) -> (bool, Option<Ending>) {
        let replaced = match &self.dragged {
            Some(dragged) if dragged.hwnd != target.hwnd => self.end(dragged.hwnd),
            _ => None,
        };
        self.dragged = Some(Dragged { hwnd: target.hwnd, grab: grab.clone() });
        (self.pending.replace(target).is_none(), replaced)
    }

    /// The move the deferred task makes: the latest one asked, if its window
    /// is still the one dragged.
    fn take_move(&mut self) -> Option<Move> {
        let pending = self.pending.take()?;
        self.dragged.as_ref().is_some_and(|dragged| dragged.hwnd == pending.hwnd && dragged.grab.get().is_some()).then_some(pending)
    }

    fn end(&mut self, hwnd: isize) -> Option<Ending> {
        let target = self.pending.take_if(|pending| pending.hwnd == hwnd).map(|pending| (pending.x, pending.y));
        self.dragged.take_if(|dragged| dragged.hwnd == hwnd)?;
        Some(Ending { hwnd, target })
    }

    /// Whether a `WM_MOVE` of `hwnd` is kept from GPUI. A drag whose grab was
    /// let go without an end is forgotten here, and the move goes through.
    fn keeps_move(&mut self, hwnd: isize) -> bool {
        let Some(dragged) = self.dragged.as_ref().filter(|dragged| dragged.hwnd == hwnd) else {
            return false;
        };
        if dragged.grab.get().is_some() {
            return true;
        }
        self.end(hwnd);
        false
    }
}

/// The messages that end a drag of the window they reach, whatever the
/// pointer does next: the mouse capture lost (the release, Alt+Tab, another
/// window taking it), the window hidden, the app or the keyboard focus lost.
fn ends_a_drag(message: u32, wparam: WPARAM) -> bool {
    match message {
        WM_CAPTURECHANGED | WM_CANCELMODE | WM_KILLFOCUS => true,
        WM_SHOWWINDOW | WM_ACTIVATEAPP => wparam.0 == 0,
        _ => false,
    }
}

fn pack(x: i32, y: i32) -> isize {
    ((y as u16 as isize) << 16) | x as u16 as isize
}

fn unpack(lparam: LPARAM) -> (i32, i32) {
    (lparam.0 as u16 as i16 as i32, (lparam.0 >> 16) as u16 as i16 as i32)
}

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
/// takes focus from the game. The style bits and the subclass send Windows'
/// move and size messages nowhere, so a failure here still fails the open;
/// [`place_after_update`] makes Windows apply them.
pub fn make_overlay(hwnd: HWND) -> Result<(), String> {
    remove_frame(hwnd);
    set_ex_style(hwnd, |style| style | WS_EX_NOACTIVATE.0);
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
///
/// GPUI answers every `WM_MOVE` by redrawing the whole window, as it cannot
/// reuse a view drawn before a move. While a drag moves the window the moves
/// are kept from it (the window's composition moves with it unredrawn), and
/// it hears the final position once the drag ends ([`end_drag`]).
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
        WM_MOVE if DRAGS.with_borrow_mut(|drags| drags.keeps_move(hwnd.0 as isize)) => LRESULT(0),
        FINISH_DRAG => {
            finish_drag(hwnd, wparam, lparam);
            LRESULT(0)
        }
        WM_NCDESTROY => unsafe {
            DRAGS.with_borrow_mut(|drags| drags.end(hwnd.0 as isize));
            LEFT_LISTENERS.with_borrow_mut(|listeners| listeners.remove(&(hwnd.0 as isize)));
            take_first_fit(hwnd);
            let _ = RemoveWindowSubclass(hwnd, Some(refuse_click_activation), subclass);
            DefSubclassProc(hwnd, message, wparam, lparam)
        },
        _ => {
            if ends_a_drag(message, wparam) {
                end_drag(hwnd);
            }
            unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
        }
    }
}

/// Whether a drag is moving one of the overlay windows now.
pub fn is_dragging() -> bool {
    DRAGS.with_borrow(|drags| drags.dragged.is_some())
}

/// Ends the drag of `hwnd`, if it is dragged: its moves reach GPUI again at
/// once, and [`FINISH_DRAG`] makes the move still owed and tells GPUI where
/// the window is.
fn end_drag(hwnd: HWND) {
    if let Some(ending) = DRAGS.with_borrow_mut(|drags| drags.end(hwnd.0 as isize)) {
        post_finish(ending);
    }
}

fn post_finish(ending: Ending) {
    let (moves, (x, y)) = (ending.target.is_some(), ending.target.unwrap_or_default());
    let hwnd = HWND(ending.hwnd as *mut c_void);
    if let Err(error) = unsafe { PostMessageW(Some(hwnd), FINISH_DRAG, WPARAM(moves as usize), LPARAM(pack(x, y))) } {
        crate::app_log::warn(format!("cannot finish the overlay's drag: {error}"));
    }
}

/// The move a drag still owed, then GPUI told where the window ended up, as
/// Windows tells it of a move (`WM_MOVE` with the client area's top left on
/// the screen). Nothing if a new drag of the window has begun since.
fn finish_drag(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) {
    if DRAGS.with_borrow(|drags| drags.dragged.as_ref().is_some_and(|dragged| dragged.hwnd == hwnd.0 as isize)) {
        return;
    }
    if wparam.0 == 1 {
        let (x, y) = unpack(lparam);
        if let Err(error) = unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) } {
            crate::app_log::warn(format!("cannot move the overlay: {error}"));
        }
    }
    let mut origin = POINT::default();
    if unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        unsafe {
            let _ = SendMessageW(hwnd, WM_MOVE, Some(WPARAM(0)), Some(LPARAM(pack(origin.x, origin.y))));
        }
    }
    report_left_at(hwnd);
}

/// Saves `spot` as the place of the overlay window `hwnd` each time a drag of
/// it ends: once the last move is made, so the place is the one the window
/// ended up at, wherever the pointer was released.
pub fn remember_drags(cx: &App, hwnd: HWND, spot: Spot) {
    listen_for_left(cx, hwnd, spot);
}

/// [`remember_drags`] for a window with its own title bar, whose moves Windows
/// runs: its place is saved when the move ends.
pub fn remember_os_moves(cx: &App, hwnd: HWND, spot: Spot) -> Result<(), String> {
    listen_for_left(cx, hwnd, spot);
    unsafe { SetWindowSubclass(hwnd, Some(report_os_move), OS_MOVE_SUBCLASS, 0) }
        .as_bool()
        .then_some(())
        .ok_or_else(|| "cannot remember where the window is left: SetWindowSubclass failed".to_string())
}

fn listen_for_left(cx: &App, hwnd: HWND, spot: Spot) {
    let outside = cx.to_async();
    let listener = move |at: ScreenPoint| {
        outside.spawn(async move |cx| cx.update(|cx| settings::leave_window_at(spot, at, cx))).detach();
    };
    LEFT_LISTENERS.with_borrow_mut(|listeners| listeners.insert(hwnd.0 as isize, Rc::new(listener)));
}

unsafe extern "system" fn report_os_move(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass: usize,
    _: usize,
) -> LRESULT {
    match message {
        WM_EXITSIZEMOVE => report_left_at(hwnd),
        WM_NCDESTROY => unsafe {
            LEFT_LISTENERS.with_borrow_mut(|listeners| listeners.remove(&(hwnd.0 as isize)));
            let _ = RemoveWindowSubclass(hwnd, Some(report_os_move), subclass);
        },
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// Tells the window's listener where the window is, unless it is minimized.
fn report_left_at(hwnd: HWND) {
    let listener = LEFT_LISTENERS.with_borrow(|listeners| listeners.get(&(hwnd.0 as isize)).cloned());
    let Some(listener) = listener else {
        return;
    };
    let mut rect = RECT::default();
    if unsafe { IsIconic(hwnd) }.as_bool() || unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return;
    }
    listener(ScreenPoint { x: rect.left, y: rect.top });
}

/// Moves the window to the corner `corner` gives for the window's size in
/// device pixels, if it gives one, then shows it as `show` says, in one go
/// once the update that asked has returned: a window opened hidden is never
/// seen at the place it was created at.
pub fn move_then_show_after_update(
    cx: &App,
    hwnd: HWND,
    show: SHOW_WINDOW_CMD,
    corner: impl FnOnce((i32, i32)) -> Option<(i32, i32)> + 'static,
) {
    after_update(cx, move || {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
            if let Some((x, y)) = corner((rect.right - rect.left, rect.bottom - rect.top)) {
                if let Err(error) = unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) } {
                    crate::app_log::warn(format!("cannot move the window: {error}"));
                }
            }
        }
        unsafe {
            let _ = ShowWindow(hwnd, show);
        }
    });
}

/// Windows opened at a place the player left are pulled inside the work area
/// once more after their first fit to the card, which can have grown them
/// past the edge ([`fit_client_size`]); a window that is not armed is never
/// moved by a fit, so a card growing during a fight never slides the meter.
pub(crate) fn pull_after_first_fit(hwnd: HWND) {
    FIRST_FIT.with_borrow_mut(|armed| armed.insert(hwnd.0 as isize));
}

fn take_first_fit(hwnd: HWND) -> bool {
    FIRST_FIT.with_borrow_mut(|armed| armed.remove(&(hwnd.0 as isize)))
}

/// A frame needed no fit: the window kept the size it was placed with, so
/// nothing is left to pull.
pub fn first_fit_not_needed(window: &Window) {
    if let Ok(hwnd) = hwnd(window) {
        take_first_fit(hwnd);
    }
}

fn pull_inside(hwnd: HWND) {
    let Some((x, y)) = crate::secondary::corner_inside_work_area(hwnd) else {
        return;
    };
    if let Err(error) = unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) } {
        crate::app_log::warn(format!("cannot pull the window inside the work area: {error}"));
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
        .update(cx, |_, window, cx| -> Result<(), String> {
            let hwnd = hwnd(window)?;
            make_overlay(hwnd)?;
            let scale = window.scale_factor();
            let (width, height) = (INITIAL_WIDTH * scale, INITIAL_HEIGHT * scale);
            remember_drags(cx, hwnd, Spot::Meter);
            match placement {
                Placement::Screen => {
                    let margin = (OVERLAY_MARGIN * scale).round() as i32;
                    let saved = settings::global(cx).read(cx).get().windows.remembered(Spot::Meter);
                    let size = (width.ceil() as i32, height.ceil() as i32);
                    place_after_update(cx, hwnd, width, height, move || {
                        match saved.and_then(|at| crate::secondary::reopen_corner(at, size)) {
                            Some(corner) => {
                                pull_after_first_fit(hwnd);
                                corner
                            }
                            None => (margin, margin),
                        }
                    });
                }
                #[cfg(feature = "probes")]
                Placement::OffScreen => off_screen::place_after_update(cx, hwnd, width, height),
            }
            if shown {
                reveal_after_update(cx, hwnd, placement, "show the meter");
            }
            Ok(())
        })
        .map_err(|error| format!("cannot reach the overlay window: {error:#}"))??;
    Ok(handle)
}

/// Shows the window without activating it. `ShowWindow` answers whether the
/// window was visible before, not whether it worked, so the result is read
/// back.
pub fn show_without_activating(hwnd: HWND) -> Result<(), String> {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    if unsafe { IsWindowVisible(hwnd) }.as_bool() {
        Ok(())
    } else {
        Err("the window is still hidden after ShowWindow".to_string())
    }
}

/// Shows a window `placement` put in place, at the top of the topmost band,
/// without activating it: a hidden window keeps its place in the z-order,
/// and Windows may have put Belisk's topmost windows behind a full-screen
/// game meanwhile (seen with a hidden soul tracker card under the game), so
/// showing alone can leave it under the game, unseen. Off screen it is
/// checked to be past every monitor before and after.
pub fn reveal(hwnd: HWND, placement: Placement) -> Result<(), String> {
    match placement {
        Placement::Screen => {
            show_without_activating(hwnd)?;
            raise_without_activating(hwnd)
        }
        #[cfg(feature = "probes")]
        Placement::OffScreen => {
            off_screen::ensure_off_screen(hwnd)?;
            show_without_activating(hwnd)?;
            off_screen::ensure_off_screen(hwnd)
        }
    }
}

pub fn hide(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// Runs `calls` once the update that asked for them has returned, in the
/// order they were asked. Windows sends its messages for a window call
/// synchronously: `WM_SHOWWINDOW` for a show (GPUI draws the window inside
/// it), `WM_MOVE` and `WM_SIZE` for a move, a resize or a frame change. GPUI
/// answers them through the App (`bounds_changed`, the frame), which fails
/// with "RefCell already borrowed" while an update holds it, and the report
/// or the frame is lost. Every call that shows, moves, resizes or reframes a
/// GPUI window from inside an update goes through here.
pub fn after_update(cx: &App, calls: impl FnOnce() + 'static) {
    cx.spawn(async move |_| calls()).detach();
}

/// [`reveal`] after the current update; a failure is logged as "cannot
/// {what}".
pub fn reveal_after_update(cx: &App, hwnd: HWND, placement: Placement, what: impl fmt::Display + 'static) {
    after_update(cx, move || {
        if let Err(error) = reveal(hwnd, placement) {
            crate::app_log::warn(format!("cannot {what}: {error}"));
        }
    });
}

/// Puts the window above every non-topmost window and above the other
/// topmost ones, without activating it.
pub fn raise_without_activating(hwnd: HWND) -> Result<(), String> {
    unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) }
        .map_err(|error| format!("cannot put the window on top: {error}"))
}

/// Puts the windows of `order` at the top of the topmost band, the last one
/// on top, without activating any: each is raised over the ones before it.
/// One that cannot be raised is reported, and the others still go up.
pub fn stack(order: &[HWND]) -> Result<(), String> {
    let failed: Vec<String> = order
        .iter()
        .filter_map(|&hwnd| raise_without_activating(hwnd).err().map(|error| format!("{hwnd:?}: {error}")))
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("; "))
    }
}

/// Takes the window out of the topmost band, without activating it, and puts
/// it behind `went_to`, the window the user went to. Leaving the band puts it
/// above every normal window, the one just activated included when that one
/// rose first, so it then swaps places with `went_to` only when `went_to` is
/// right below: a topmost window is not, nor is the desktop, which stays at
/// the bottom, and the window must not sink under either.
pub fn lower_behind(hwnd: HWND, went_to: Option<HWND>) -> Result<(), String> {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    unsafe { SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags) }
        .map_err(|error| format!("cannot take the window out of the topmost band: {error}"))?;
    let next = unsafe { GetWindow(hwnd, GW_HWNDNEXT) };
    if let Some(went_to) = went_to.filter(|&went_to| next.is_ok_and(|next| next == went_to)) {
        unsafe { SetWindowPos(hwnd, Some(went_to), 0, 0, 0, 0, flags) }
            .map_err(|error| format!("cannot put the window behind the one the user went to: {error}"))?;
    }
    Ok(())
}

/// Makes the overlay visible, on top, and on a monitor, for a later launch
/// asking the running meter to show itself. It does not take focus: the
/// overlay is non-activating by design, and pulling focus out of the game
/// would be worse than a meter that stays where it was.
pub fn bring_forward(hwnd: HWND) -> Result<(), String> {
    show_without_activating(hwnd)?;
    if unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL) }.is_invalid() {
        move_onto_primary_monitor(hwnd);
    }
    raise_without_activating(hwnd)
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
    } else {
        first_fit_not_needed(window);
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

/// Applies a frame [`make_overlay`] removed, then [`place`]s the window, once
/// the update that opened it has returned. `corner` is asked then, not
/// before, so a window anchored to another one opened in the same update
/// sees where that one was put. A window's first place is asked before any
/// other, so a later one of it (a card's `Stacking::place`) is not undone.
pub fn place_after_update(cx: &App, hwnd: HWND, width: f32, height: f32, corner: impl FnOnce() -> (i32, i32) + 'static) {
    after_update(cx, move || {
        refresh_frame(hwnd);
        let (x, y) = corner();
        if let Err(error) = place(hwnd, x, y, width, height) {
            crate::app_log::warn(error);
        }
    });
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
/// layered. The frame change that makes Windows read the style again is asked
/// through [`after_update`]: the lock is pressed inside an update.
pub fn set_click_through(window: &Window, cx: &App, click_through: bool) -> Result<(), String> {
    let hwnd = hwnd(window)?;
    set_ex_style(hwnd, |style| {
        if click_through {
            style | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0
        } else {
            style & !WS_EX_TRANSPARENT.0
        }
    });
    after_update(cx, move || refresh_frame(hwnd));
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
            crate::app_log::warn(format!("cannot fit the overlay to its card: {error}"));
            return;
        }
    };
    let scale = window.scale_factor();
    let width = (content.width.as_f32() * scale).ceil() as i32;
    let height = (content.height.as_f32() * scale).ceil() as i32;
    cx.spawn(async move |_| {
        if let Err(error) = set_client_size(hwnd, width, height) {
            crate::app_log::warn(format!("cannot fit the overlay to its card: {error}"));
        }
        if take_first_fit(hwnd) {
            pull_inside(hwnd);
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
/// window has caught up do not add up; only the latest target of those is
/// applied, and only while the window is still the one dragged.
fn drag_to(window: &Window, grab: &Rc<Cell<Option<Grab>>>, from: Grab, position: Point<Pixels>, cx: &mut App) {
    let (dx, dy) = from.offset(position, window.scale_factor());
    if dx == 0 && dy == 0 {
        return;
    }
    let target = hwnd(window).and_then(|hwnd| {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|error| format!("GetWindowRect failed: {error}"))?;
        Ok(Move { hwnd: hwnd.0 as isize, x: rect.left + dx, y: rect.top + dy })
    });
    let target = match target {
        Ok(target) => target,
        Err(error) => {
            crate::app_log::warn(format!("cannot move the overlay: {error}"));
            return;
        }
    };
    let (schedule, replaced) = DRAGS.with_borrow_mut(|drags| drags.ask(target, grab));
    if let Some(replaced) = replaced {
        post_finish(replaced);
    }
    if !schedule {
        return;
    }
    cx.spawn(async move |_| {
        let Some(next) = DRAGS.with_borrow_mut(Drags::take_move) else {
            return;
        };
        let hwnd = HWND(next.hwnd as *mut c_void);
        if let Err(error) = unsafe { SetWindowPos(hwnd, None, next.x, next.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) } {
            crate::app_log::warn(format!("cannot move the overlay: {error}"));
        }
    })
    .detach();
}

/// Ends the drag of the window at once; the move it still owes follows
/// ([`end_drag`]).
fn release_drag(window: &Window) {
    if let Ok(hwnd) = hwnd(window) {
        end_drag(hwnd);
    }
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
    /// that drags is taken, so no element focuses on it.
    pub fn handle_around_controls<E: InteractiveElement>(&self, area: E) -> E {
        let grab = self.0.clone();
        area.on_mouse_down(MouseButton::Left, move |event, window, _| {
            if window.default_prevented() {
                return;
            }
            grab.set(Some(Grab::at(window, event.position)));
            window.prevent_default();
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
                        drag_to(window, &moved, from, event.position, cx);
                    } else {
                        moved.set(None);
                        release_drag(window);
                    }
                });
                let released = grab.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, _| {
                    if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                        released.set(None);
                        release_drag(window);
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

/// Changes the extended style and has Windows read it again.
pub(crate) fn update_ex_style(hwnd: HWND, change: impl FnOnce(u32) -> u32) {
    set_ex_style(hwnd, change);
    refresh_frame(hwnd);
}

pub(crate) fn set_ex_style(hwnd: HWND, change: impl FnOnce(u32) -> u32) {
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, change(ex_style(hwnd)) as isize);
    }
}

/// `SWP_FRAMECHANGED` without a move or a resize: Windows recomputes the
/// frame and, when the client area changed with it, tells the window of a
/// move and a resize.
pub(crate) fn refresh_frame(hwnd: HWND) {
    unsafe {
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

    use gpui::App;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_ENABLED,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWINDOWATTRIBUTE,
    };
    use windows::Win32::Graphics::Gdi::{MonitorFromRect, MONITOR_DEFAULTTONULL};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, ShowWindow,
        GWL_STYLE, SM_CXVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE,
    };

    /// Room to the left of the virtual screen, well beyond any card.
    const CLEARANCE: i32 = 4_000;

    /// [`super::place_after_update`] past the left edge of the virtual
    /// screen, then checked to be off every monitor, without showing the
    /// window. `width` and `height` are device pixels.
    pub fn place_after_update(cx: &App, hwnd: HWND, width: f32, height: f32) {
        super::place_after_update(cx, hwnd, width, height, || {
            let (left, top) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
            (left - CLEARANCE, top)
        });
        super::after_update(cx, move || {
            if let Err(error) = ensure_off_screen(hwnd) {
                crate::app_log::warn(error);
            }
        });
    }

    /// [`place_after_update`], then shows the window as [`super::reveal`]
    /// does off screen. `width` and `height` are device pixels.
    pub fn show_after_update(cx: &App, hwnd: HWND, width: f32, height: f32) {
        place_after_update(cx, hwnd, width, height);
        super::reveal_after_update(cx, hwnd, super::Placement::OffScreen, "show the window off screen");
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
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gpui::{point, px};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        WM_ACTIVATEAPP, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KILLFOCUS, WM_MOUSEMOVE, WM_MOVE, WM_SHOWWINDOW,
    };

    use super::{
        after_update, end_drag, ends_a_drag, make_overlay, move_then_show_after_update, pack, place_after_update, pull_after_first_fit,
        pull_inside, raise_without_activating, remember_drags, remember_os_moves, reveal, show_without_activating, stack, take_first_fit,
        unpack, Drags, Ending, Grab, Move, Placement, DRAGS,
    };
    use crate::settings::{self, AppSettings, ScreenPoint, Settings, Spot};
    use gpui::AppContext as _;

    /// A hidden topmost tool window of this test's thread, never shown or
    /// activated: the z-order holds hidden windows too, so a stack can be
    /// read back without anything appearing on screen.
    struct Hidden(windows::Win32::Foundation::HWND);

    impl Hidden {
        fn new() -> Self {
            use windows::core::w;
            use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP};
            let hwnd = unsafe {
                CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST, w!("STATIC"), None, WS_POPUP, -32000, -32000, 1, 1, None, None, None, None)
            };
            Self(hwnd.expect("a hidden test window"))
        }

        fn destroyed() -> windows::Win32::Foundation::HWND {
            Self::new().0
        }
    }

    impl Drop for Hidden {
        fn drop(&mut self) {
            let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0) };
        }
    }

    fn is_above(upper: &Hidden, lower: &Hidden) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindow, GW_HWNDNEXT};
        let mut next = unsafe { GetWindow(upper.0, GW_HWNDNEXT) };
        for _ in 0..4096 {
            match next {
                Ok(window) if window == lower.0 => return true,
                Ok(window) => next = unsafe { GetWindow(window, GW_HWNDNEXT) },
                Err(_) => return false,
            }
        }
        false
    }

    #[test]
    fn a_stack_puts_the_windows_in_its_order_whatever_their_order_before() {
        let (toolbar, tool, card) = (Hidden::new(), Hidden::new(), Hidden::new());
        stack(&[card.0, tool.0, toolbar.0]).expect("the first stack");
        assert!(is_above(&toolbar, &tool) && is_above(&tool, &card), "the toolbar was raised last");

        stack(&[toolbar.0, tool.0, card.0]).expect("the second stack");
        assert!(is_above(&card, &tool), "the card is on top of the tool");
        assert!(is_above(&tool, &toolbar), "and the tool on top of the toolbar");
    }

    #[gpui::test]
    fn win32_calls_asked_inside_an_update_run_in_order_after_it_returns(cx: &mut gpui::TestAppContext) {
        let order = Rc::new(RefCell::new(Vec::new()));
        cx.update(|cx| {
            for step in ["first", "second"] {
                let (order, outside) = (order.clone(), cx.to_async());
                after_update(cx, move || {
                    // Taking the App again would panic were an update still holding it.
                    outside.update(|_| ());
                    order.borrow_mut().push(step);
                });
            }
            assert!(order.borrow().is_empty(), "nothing runs inside the update that asked");
        });
        cx.run_until_parked();
        assert_eq!(*order.borrow(), ["first", "second"]);
    }

    #[gpui::test]
    fn a_window_placed_after_an_update_is_anchored_to_where_the_other_one_asked_in_it_was_put(cx: &mut gpui::TestAppContext) {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

        let (base, anchored) = (Hidden::new(), Hidden::new());
        let (base_hwnd, anchored_hwnd) = (base.0, anchored.0);
        let corner_of = move |hwnd| {
            let mut rect = RECT::default();
            unsafe { GetWindowRect(hwnd, &mut rect) }.expect("a window rect");
            (rect.left, rect.top)
        };
        let before = corner_of(base_hwnd);
        cx.update(|cx| {
            place_after_update(cx, base_hwnd, 40.0, 20.0, || (100, 120));
            place_after_update(cx, anchored_hwnd, 40.0, 20.0, move || {
                let (left, top) = corner_of(base_hwnd);
                (left + 10, top + 10)
            });
            assert_eq!(corner_of(base_hwnd), before, "nothing is placed inside the update that asked");
        });
        cx.run_until_parked();
        assert_eq!(corner_of(base_hwnd), (100, 120));
        assert_eq!(corner_of(anchored_hwnd), (110, 130), "the anchor was read once the first window was in place");
    }

    #[test]
    fn a_window_that_cannot_be_raised_is_reported_and_the_others_still_go_up() {
        let (below, above) = (Hidden::new(), Hidden::new());
        let gone = Hidden::destroyed();
        stack(&[above.0, below.0]).expect("the first stack");

        let failed = stack(&[below.0, gone, above.0]).expect_err("a destroyed window cannot be raised");
        assert!(failed.contains(&format!("{gone:?}")), "the failure names the window: {failed}");
        assert!(is_above(&above, &below), "the windows that could be raised were");
        assert!(raise_without_activating(gone).is_err());
    }

    #[test]
    fn a_window_that_cannot_be_shown_is_not_reported_as_revealed() {
        let gone = Hidden::destroyed();
        assert!(show_without_activating(gone).is_err(), "ShowWindow's answer is not taken for success");
        assert!(reveal(gone, Placement::Screen).is_err());
    }

    const A: isize = 0x1000;
    const B: isize = 0x2000;

    fn held() -> Rc<Cell<Option<Grab>>> {
        Rc::new(Cell::new(Some(Grab { x: 10, y: 10 })))
    }

    fn to(hwnd: isize, x: i32, y: i32) -> Move {
        Move { hwnd, x, y }
    }

    fn corner_of(hwnd: windows::Win32::Foundation::HWND) -> (i32, i32) {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect) }.expect("a window rect");
        (rect.left, rect.top)
    }

    /// Handles the messages posted to `hwnd`, as the message loop does.
    fn dispatch_posted(hwnd: windows::Win32::Foundation::HWND) {
        use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, MSG, PM_REMOVE};
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE) }.as_bool() {
            unsafe { DispatchMessageW(&message) };
        }
    }

    fn spot_of_window(store: &gpui::Entity<Settings>, spot: Spot, cx: &mut gpui::TestAppContext) -> Option<ScreenPoint> {
        store.read_with(cx, |settings, _| settings.get().windows.remembered(spot))
    }

    fn in_memory_settings(cx: &mut gpui::TestAppContext) -> gpui::Entity<Settings> {
        cx.update(|cx| {
            let store = cx.new(|_| Settings::in_memory(AppSettings::default()));
            settings::install(&store, cx);
            store
        })
    }

    #[gpui::test]
    fn a_drag_released_away_from_its_window_is_remembered_at_the_place_the_window_ended_up(cx: &mut gpui::TestAppContext) {
        let store = in_memory_settings(cx);
        let toolbar = Hidden::new();
        make_overlay(toolbar.0).expect("an overlay window");
        cx.update(|cx| remember_drags(cx, toolbar.0, Spot::Toolbar));

        // The pointer outran the window: the last move is still owed when the
        // button is released, far from the window, where no element of it hears it.
        DRAGS.with_borrow_mut(|drags| drags.ask(to(toolbar.0.0 as isize, 300, 200), &held()));
        end_drag(toolbar.0);
        assert_eq!(spot_of_window(&store, Spot::Toolbar, cx), None, "nothing is saved before the drag's last move is made");
        dispatch_posted(toolbar.0);
        cx.run_until_parked();

        assert_eq!(corner_of(toolbar.0), (300, 200));
        assert_eq!(spot_of_window(&store, Spot::Toolbar, cx), Some(ScreenPoint { x: 300, y: 200 }));
    }

    #[gpui::test]
    fn a_press_that_never_moved_the_window_remembers_nothing(cx: &mut gpui::TestAppContext) {
        let store = in_memory_settings(cx);
        let pets = Hidden::new();
        make_overlay(pets.0).expect("an overlay window");
        cx.update(|cx| remember_drags(cx, pets.0, Spot::Pets));

        end_drag(pets.0);
        dispatch_posted(pets.0);
        cx.run_until_parked();

        assert_eq!(spot_of_window(&store, Spot::Pets, cx), None);
    }

    #[gpui::test]
    fn each_window_remembers_its_own_spot(cx: &mut gpui::TestAppContext) {
        let store = in_memory_settings(cx);
        let (history, todos) = (Hidden::new(), Hidden::new());
        for (window, spot) in [(&history, Spot::History), (&todos, Spot::Todos)] {
            make_overlay(window.0).expect("an overlay window");
            cx.update(|cx| remember_drags(cx, window.0, spot));
        }

        for (window, x) in [(&history, 100), (&todos, 700)] {
            DRAGS.with_borrow_mut(|drags| drags.ask(to(window.0.0 as isize, x, 50), &held()));
            end_drag(window.0);
            dispatch_posted(window.0);
        }
        cx.run_until_parked();

        assert_eq!(spot_of_window(&store, Spot::History, cx), Some(ScreenPoint { x: 100, y: 50 }));
        assert_eq!(spot_of_window(&store, Spot::Todos, cx), Some(ScreenPoint { x: 700, y: 50 }));
        assert_eq!(spot_of_window(&store, Spot::Pets, cx), None);
    }

    #[gpui::test]
    fn a_window_with_a_title_bar_is_remembered_when_windows_ends_its_move(cx: &mut gpui::TestAppContext) {
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WM_EXITSIZEMOVE};

        let store = in_memory_settings(cx);
        let window = Hidden::new();
        cx.update(|cx| remember_os_moves(cx, window.0, Spot::Settings)).expect("the move is watched");
        unsafe { SetWindowPos(window.0, None, 420, 130, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) }.expect("moved");
        cx.run_until_parked();
        assert_eq!(spot_of_window(&store, Spot::Settings, cx), None, "a move in progress saves nothing");

        unsafe { SendMessageW(window.0, WM_EXITSIZEMOVE, None, None) };
        cx.run_until_parked();

        assert_eq!(spot_of_window(&store, Spot::Settings, cx), Some(ScreenPoint { x: 420, y: 130 }));
    }

    thread_local! {
        static SHOWN_AT: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
    }

    /// Notes where the window was when Windows showed it.
    unsafe extern "system" fn note_where_shown(
        hwnd: windows::Win32::Foundation::HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _: usize,
        _: usize,
    ) -> windows::Win32::Foundation::LRESULT {
        if message == WM_SHOWWINDOW && wparam.0 == 1 {
            SHOWN_AT.set(Some(corner_of(hwnd)));
        }
        unsafe { windows::Win32::UI::Shell::DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    #[gpui::test]
    fn a_hidden_window_is_moved_after_the_update_and_only_then_shown(cx: &mut gpui::TestAppContext) {
        use windows::Win32::UI::WindowsAndMessaging::{IsWindowVisible, SW_SHOWNOACTIVATE};

        let window = Hidden::new();
        let hwnd = window.0;
        assert!(unsafe { windows::Win32::UI::Shell::SetWindowSubclass(hwnd, Some(note_where_shown), 77, 0) }.as_bool());
        SHOWN_AT.set(None);
        cx.update(|cx| {
            move_then_show_after_update(cx, hwnd, SW_SHOWNOACTIVATE, |size| {
                assert_eq!(size, (1, 1));
                Some((-30000, 400))
            });
            assert!(!unsafe { IsWindowVisible(hwnd) }.as_bool(), "nothing is shown inside the update that asked");
        });
        cx.run_until_parked();
        assert_eq!(SHOWN_AT.get(), Some((-30000, 400)), "Windows showed the window where it had been moved to");
        assert!(unsafe { IsWindowVisible(hwnd) }.as_bool());
    }

    #[gpui::test]
    fn a_window_that_gets_no_corner_is_still_shown(cx: &mut gpui::TestAppContext) {
        use windows::Win32::UI::WindowsAndMessaging::{IsWindowVisible, SW_SHOWNOACTIVATE};

        let window = Hidden::new();
        let before = corner_of(window.0);
        cx.update(|cx| move_then_show_after_update(cx, window.0, SW_SHOWNOACTIVATE, |_| None));
        cx.run_until_parked();
        assert_eq!(corner_of(window.0), before, "the spot was on no monitor: the window stays where it opened");
        assert!(unsafe { IsWindowVisible(window.0) }.as_bool(), "and it is shown, not left hidden for good");
    }

    #[test]
    fn a_window_sticking_out_after_its_first_fit_is_pulled_inside_once() {
        let (window, other) = (Hidden::new(), Hidden::new());
        pull_after_first_fit(window.0);
        assert!(!take_first_fit(other.0), "another window is not armed");
        assert!(take_first_fit(window.0), "the first fit pulls");
        assert!(!take_first_fit(window.0), "a later fit, in the middle of a fight, never does");

        let outside = (-32000, -32000);
        assert_eq!(corner_of(window.0), outside);
        pull_inside(window.0);
        let pulled = corner_of(window.0);
        let on_a_monitor = crate::secondary::reopen_corner(ScreenPoint { x: pulled.0, y: pulled.1 }, (1, 1)).is_some();
        assert!(on_a_monitor, "the window was pulled onto a monitor's work area: {pulled:?}");
    }

    #[test]
    fn a_drag_keeps_the_moves_from_gpui_and_makes_only_the_latest_move_asked() {
        let (mut drags, grab) = (Drags::default(), held());
        assert!(!drags.keeps_move(A), "no drag, no move kept");
        assert_eq!(drags.ask(to(A, 5, 5), &grab), (true, None), "the first move asked is scheduled");
        assert_eq!(drags.ask(to(A, 9, 7), &grab), (false, None), "one already waits: this one replaces its target");
        assert!(drags.keeps_move(A));
        assert!(!drags.keeps_move(B), "another window's moves still reach GPUI");
        assert_eq!(drags.take_move(), Some(to(A, 9, 7)));
        assert_eq!(drags.take_move(), None);
        assert_eq!(drags.ask(to(A, 12, 7), &grab), (true, None), "after the move is made, the next is scheduled again");
    }

    #[test]
    fn the_end_gives_back_the_move_owed_and_the_moves_reach_gpui_again() {
        let (mut drags, grab) = (Drags::default(), held());
        drags.ask(to(A, 5, 5), &grab);
        assert_eq!(drags.end(A), Some(Ending { hwnd: A, target: Some((5, 5)) }));
        assert!(!drags.keeps_move(A));
        assert_eq!(drags.take_move(), None, "the move scheduled before the end does nothing");
        assert_eq!(drags.end(A), None, "a second end (the release after the capture loss) does nothing");

        drags.ask(to(A, 6, 6), &grab);
        assert_eq!(drags.take_move(), Some(to(A, 6, 6)));
        assert_eq!(drags.end(A), Some(Ending { hwnd: A, target: None }), "every move made: only the report is owed");
    }

    #[test]
    fn a_grab_let_go_without_an_end_no_longer_keeps_the_moves() {
        let (mut drags, grab) = (Drags::default(), held());
        drags.ask(to(A, 5, 5), &grab);
        grab.set(None);
        assert_eq!(drags.take_move(), None, "the move scheduled is not made");
        drags.ask(to(A, 5, 5), &held());
        let stopped = held();
        drags.ask(to(A, 8, 8), &stopped);
        stopped.set(None);
        assert!(!drags.keeps_move(A), "the move goes to GPUI");
        assert_eq!(drags.end(A), None, "and the drag is forgotten");
    }

    #[test]
    fn a_move_scheduled_for_one_window_never_moves_another() {
        let (mut drags, grab) = (Drags::default(), held());
        drags.ask(to(A, 5, 5), &grab);
        let (scheduled, replaced) = drags.ask(to(B, 300, 40), &held());
        assert_eq!(replaced, Some(Ending { hwnd: A, target: Some((5, 5)) }), "A's drag ends with its own move");
        assert!(scheduled, "B's move gets its own task");
        assert!(!drags.keeps_move(A));
        assert!(drags.keeps_move(B));
        assert_eq!(drags.take_move(), Some(to(B, 300, 40)), "the task queued for A makes B's move, on B");
    }

    #[test]
    fn capture_loss_hiding_and_focus_loss_end_a_drag() {
        assert!(ends_a_drag(WM_CAPTURECHANGED, WPARAM(0)));
        assert!(ends_a_drag(WM_CANCELMODE, WPARAM(0)));
        assert!(ends_a_drag(WM_KILLFOCUS, WPARAM(0)));
        assert!(ends_a_drag(WM_SHOWWINDOW, WPARAM(0)), "hidden");
        assert!(!ends_a_drag(WM_SHOWWINDOW, WPARAM(1)), "shown");
        assert!(ends_a_drag(WM_ACTIVATEAPP, WPARAM(0)), "the app deactivated");
        assert!(!ends_a_drag(WM_ACTIVATEAPP, WPARAM(1)));
        assert!(!ends_a_drag(WM_MOVE, WPARAM(0)));
        assert!(!ends_a_drag(WM_MOUSEMOVE, WPARAM(1)));
    }

    #[test]
    fn screen_points_left_of_and_above_the_primary_monitor_survive_the_message() {
        for (x, y) in [(0, 0), (1920, 1080), (-1920, -200), (-1, 32767), (-32768, 5)] {
            assert_eq!(unpack(LPARAM(pack(x, y))), (x, y));
        }
    }

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
