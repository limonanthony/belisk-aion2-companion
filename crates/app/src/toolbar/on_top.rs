//! Keeps Belisk's overlays (the toolbar, the meter while shown, the tools and
//! cards that are up) at the top of the topmost band, in one fixed stack:
//! the toolbar at the bottom, then the meter, the tools, the reminder card,
//! and the soul tracker card on top, so no window of Belisk's ever covers a
//! card. Windows can leave a normal window above them: the full-screen game's
//! window was seen above Belisk's topmost windows, the toolbar then covered
//! by the game. Once a second, the check walks the z-order above each
//! overlay (a few windows, no hook, nothing read from another process but
//! its window's style) and, when a normal window of another program overlaps
//! one, puts the whole stack back on top, without activating any of it. It
//! never runs while a drag moves one of the overlays or while one of
//! Belisk's windows is the foreground (typing in a field, Settings), and
//! leaves topmost windows of other programs alone. Settings, a normal
//! window, is never in the stack.

use std::cmp::Reverse;
use std::ffi::c_void;
use std::time::Duration;

use gpui::{App, Entity};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetTopWindow, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsWindowVisible, GWL_EXSTYLE, GW_HWNDNEXT, GW_HWNDPREV, WS_EX_TOPMOST,
};

use super::Hub;
use crate::overlay;
use crate::secondary::{self, WindowKind};

const EVERY: Duration = Duration::from_secs(1);
/// A walk up the z-order stops here; the overlays sit near the top.
const MOST_ABOVE: usize = 64;
/// A walk down from the top of the z-order looking for the tools stops here.
const MOST_FROM_THE_TOP: usize = 4096;

/// Where an overlay sits in Belisk's stack, from the bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Toolbar,
    Meter,
    /// Every window of the secondary layer but the two cards and Settings:
    /// these keep the order they have among themselves, so the tool opened
    /// or brought forward last stays above the others.
    Tool,
    Reminder,
    PetCard,
}

impl Layer {
    /// None for Settings, a normal window outside the stack.
    fn of(kind: WindowKind) -> Option<Self> {
        match kind {
            WindowKind::Settings => None,
            WindowKind::Reminder => Some(Self::Reminder),
            WindowKind::PetCard => Some(Self::PetCard),
            WindowKind::History
            | WindowKind::Detail
            | WindowKind::Report
            | WindowKind::Pvp
            | WindowKind::Todos
            | WindowKind::Resources
            | WindowKind::Pets
            | WindowKind::Prompt
            | WindowKind::WhatsNew => Some(Self::Tool),
        }
    }
}

/// The overlays from the bottom of the stack to its top: by layer, and
/// within a layer as they are stacked now (`depth`: 0 at the top of the
/// z-order; a window it cannot place goes to the bottom of its layer).
pub fn stacking_order(overlays: &[(Layer, HWND)], depth: impl Fn(HWND) -> Option<usize>) -> Vec<HWND> {
    let mut order = overlays.to_vec();
    order.sort_by_key(|&(layer, hwnd)| (layer, Reverse(depth(hwnd).unwrap_or(usize::MAX))));
    order.into_iter().map(|(_, hwnd)| hwnd).collect()
}

/// A window above one of the overlays in the z-order, as the check sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Above {
    pub ours: bool,
    /// Visible and not cloaked.
    pub shown: bool,
    pub topmost: bool,
    pub rect: RECT,
}

/// The first window above an overlay at `ours` that covers it: on screen,
/// not Belisk's, overlapping it, and not topmost. A normal window above a
/// topmost one is out of place; a topmost window above it (another program's
/// overlay, a game that asked to be topmost) is in its own right and is not
/// fought.
pub fn covering(above: &[Above], ours: RECT) -> Option<usize> {
    above.iter().position(|window| window.shown && !window.ours && !window.topmost && overlaps(window.rect, ours))
}

fn overlaps(a: RECT, b: RECT) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// Runs the check once a second for as long as the hub lives.
pub fn start(hub: &Entity<Hub>, cx: &mut App) {
    let hub = hub.downgrade();
    cx.spawn(async move |cx| {
        let mut watch = Watch::default();
        loop {
            cx.background_executor().timer(EVERY).await;
            let checked = cx.update(|cx| hub.upgrade().map(|hub| watch.check(&hub, cx)));
            let Some(covered) = checked else {
                break;
            };
            if let Some(covered) = covered {
                watch.put_back_on_top(covered);
            }
        }
    })
    .detach();
}

/// What the check last saw, so each change is logged once.
#[derive(Default)]
struct Watch {
    restacked: Option<Restacked>,
    toolbar_hidden: bool,
}

/// An overlay found under a window of another program, and the overlays on
/// screen to put back on top.
struct Covered {
    under: String,
    overlays: Vec<(Layer, HWND)>,
}

/// An overlay was covered, and the stack was put back on top, or not.
#[derive(PartialEq)]
struct Restacked {
    /// The class of the window of another program that covered it.
    under: String,
    failure: Option<String>,
}

impl Watch {
    /// The windows on screen are checked even while Alt+Shift+CapsLock has the
    /// rest hidden: a card shown then is shown on purpose and must stay on top.
    fn check(&mut self, hub: &Entity<Hub>, cx: &mut App) -> Option<Covered> {
        let (toolbar, hidden_on_purpose) = {
            let hub = hub.read(cx);
            (hub.toolbar.map(|(_, hwnd)| hwnd), hub.hidden.is_some())
        };
        if hidden_on_purpose {
            self.toolbar_hidden = false;
        } else if let Some(toolbar) = toolbar {
            let hidden = !is_visible(toolbar);
            if hidden != self.toolbar_hidden {
                self.toolbar_hidden = hidden;
                if hidden {
                    crate::app_log::warn(format!("the toolbar's window is hidden, though Alt+Shift+CapsLock did not hide it: {}", describe(toolbar)));
                } else {
                    crate::app_log::info("the toolbar's window is shown again");
                }
            }
        }
        if overlay::is_dragging() || foreground_is_ours() {
            return None;
        }
        let overlays = overlays(cx);
        let under = overlays.iter().find_map(|&(_, hwnd)| {
            let above = windows_above(hwnd);
            let seen: Vec<Above> = above.iter().map(|(_, window)| *window).collect();
            covering(&seen, rect_of(hwnd)).map(|index| class_name(above[index].0))
        });
        match under {
            Some(under) => Some(Covered { under, overlays }),
            None => {
                self.restacked = None;
                None
            }
        }
    }

    /// Runs outside the update, so no window call runs inside the App borrow.
    fn put_back_on_top(&mut self, covered: Covered) {
        let restacked = Restacked { under: covered.under, failure: stack(&covered.overlays).err() };
        if self.restacked.as_ref() != Some(&restacked) {
            let under = &restacked.under;
            match &restacked.failure {
                None => crate::app_log::info(format!(
                    "an overlay was under a window of class {under}; Belisk's overlays put back on top in their order, without activating"
                )),
                Some(error) => crate::app_log::warn(format!(
                    "an overlay was under a window of class {under}; putting Belisk's overlays back on top failed: {error}"
                )),
            }
        }
        self.restacked = Some(restacked);
    }
}

/// Puts Belisk's overlays on screen back in their stack at the top of the
/// topmost band, without activating any, after one was shown or raised on
/// its own (which puts it above all the others). It runs after the current
/// update, so after the windows that update showed ([`overlay::after_update`])
/// and with them counted among the overlays on screen.
pub fn restack(cx: &App) {
    cx.spawn(async move |cx| {
        let on_screen = cx.update(|cx| overlays(cx));
        if let Err(error) = stack(&on_screen) {
            crate::app_log::warn(format!("cannot put Belisk's overlays back on top in their order: {error}"));
        }
    })
    .detach();
}

/// Belisk's overlays on screen, with their layer.
fn overlays(cx: &App) -> Vec<(Layer, HWND)> {
    let mut overlays = Vec::new();
    if let Some(hub) = super::hub(cx) {
        let hub = hub.read(cx);
        overlays.extend(hub.toolbar.map(|(_, hwnd)| (Layer::Toolbar, hwnd)));
        overlays.extend(hub.meter_hwnd.filter(|_| hub.meter_shown).map(|hwnd| (Layer::Meter, hwnd)));
    }
    overlays.extend(secondary::windows_on_screen(cx).into_iter().filter_map(|(kind, hwnd)| Some((Layer::of(kind)?, hwnd))));
    overlays.retain(|&(_, hwnd)| is_visible(hwnd));
    overlays
}

fn stack(overlays: &[(Layer, HWND)]) -> Result<(), String> {
    let tools: Vec<HWND> = overlays.iter().filter(|(layer, _)| *layer == Layer::Tool).map(|(_, hwnd)| *hwnd).collect();
    let depths = if tools.len() > 1 { depths_from_the_top(&tools) } else { Vec::new() };
    let depth = |hwnd: HWND| tools.iter().position(|tool| *tool == hwnd).and_then(|index| depths.get(index).copied().flatten());
    overlay::stack(&stacking_order(overlays, depth))
}

/// Where each of `windows` is in the z-order, 0 at the top; None for one the
/// walk did not reach.
fn depths_from_the_top(windows: &[HWND]) -> Vec<Option<usize>> {
    let mut depths = vec![None; windows.len()];
    let mut next = unsafe { GetTopWindow(None) };
    let mut depth = 0;
    while let Ok(window) = next {
        if depth == MOST_FROM_THE_TOP {
            break;
        }
        if let Some(index) = windows.iter().position(|ours| *ours == window) {
            depths[index] = Some(depth);
            if depths.iter().all(Option::is_some) {
                break;
            }
        }
        depth += 1;
        next = unsafe { GetWindow(window, GW_HWNDNEXT) };
    }
    depths
}

/// The toolbar's (or any overlay's) state for a log line: shown, topmost,
/// and the first window of another program that covers it.
pub fn describe(hwnd: HWND) -> String {
    let above = windows_above(hwnd);
    let seen: Vec<Above> = above.iter().map(|(_, window)| *window).collect();
    let cover = covering(&seen, rect_of(hwnd)).map_or_else(|| "nothing".to_string(), |index| class_name(above[index].0));
    format!("visible {}, topmost {}, {} windows above it, covered by {cover}", is_visible(hwnd), is_topmost(hwnd), above.len())
}

fn windows_above(hwnd: HWND) -> Vec<(HWND, Above)> {
    let me = unsafe { GetCurrentProcessId() };
    let mut above = Vec::new();
    let mut next = unsafe { GetWindow(hwnd, GW_HWNDPREV) };
    while let Ok(window) = next {
        if window.is_invalid() || above.len() == MOST_ABOVE {
            break;
        }
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(window, Some(&mut process)) };
        above.push((window, Above { ours: process == me, shown: is_visible(window) && !is_cloaked(window), topmost: is_topmost(window), rect: rect_of(window) }));
        next = unsafe { GetWindow(window, GW_HWNDPREV) };
    }
    above
}

fn foreground_is_ours() -> bool {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        return false;
    }
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut process)) };
    process == unsafe { GetCurrentProcessId() }
}

fn is_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd) }.as_bool()
}

fn is_topmost(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    style & WS_EX_TOPMOST.0 != 0
}

fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    let read = unsafe { DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, std::mem::size_of::<u32>() as u32) };
    read.is_ok() && cloaked != 0
}

fn rect_of(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect);
    }
    rect
}

fn class_name(hwnd: HWND) -> String {
    let mut name = [0u16; 128];
    let length = unsafe { GetClassNameW(hwnd, &mut name) };
    String::from_utf16_lossy(&name[..length.max(0) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT { left, top, right, bottom }
    }

    const TOOLBAR: RECT = RECT { left: 81, top: 56, right: 525, bottom: 92 };

    fn window(ours: bool, shown: bool, topmost: bool, rect: RECT) -> Above {
        Above { ours, shown, topmost, rect }
    }

    #[test]
    fn a_normal_window_of_another_program_over_the_toolbar_covers_it() {
        let game = window(false, true, false, rect(0, 0, 2560, 1440));
        assert_eq!(covering(&[game], TOOLBAR), Some(0));
    }

    #[test]
    fn topmost_hidden_cloaked_far_or_own_windows_above_do_not() {
        let above = [
            window(false, true, true, rect(0, 0, 2560, 1440)),
            window(false, false, false, rect(0, 0, 2560, 1440)),
            window(false, true, false, rect(1000, 1000, 1200, 1100)),
            window(true, true, false, rect(0, 0, 400, 400)),
        ];
        assert_eq!(covering(&above, TOOLBAR), None, "another program's topmost overlay, a hidden or cloaked window, one elsewhere, one of ours");
    }

    #[test]
    fn the_first_covering_window_is_named() {
        let above = [window(false, true, true, rect(0, 0, 10, 10)), window(false, true, false, rect(0, 0, 2560, 1440)), window(false, true, false, TOOLBAR)];
        assert_eq!(covering(&above, TOOLBAR), Some(1));
    }

    fn hwnd(value: usize) -> HWND {
        HWND(value as *mut c_void)
    }

    #[test]
    fn the_cards_go_over_every_other_overlay_whatever_order_the_overlays_come_in() {
        let (toolbar, meter, history, reminder, card) = (hwnd(1), hwnd(2), hwnd(3), hwnd(4), hwnd(5));
        let expected = vec![toolbar, meter, history, reminder, card];
        let found_in = [
            vec![(Layer::PetCard, card), (Layer::Tool, history), (Layer::Toolbar, toolbar), (Layer::Reminder, reminder), (Layer::Meter, meter)],
            vec![(Layer::Reminder, reminder), (Layer::Meter, meter), (Layer::PetCard, card), (Layer::Toolbar, toolbar), (Layer::Tool, history)],
            vec![(Layer::Toolbar, toolbar), (Layer::Meter, meter), (Layer::Tool, history), (Layer::Reminder, reminder), (Layer::PetCard, card)],
        ];
        for overlays in found_in {
            assert_eq!(stacking_order(&overlays, |_| None), expected, "from {overlays:?}");
        }
    }

    #[test]
    fn the_tools_keep_their_order_and_one_the_walk_did_not_find_goes_under_them() {
        let (history, pets, detail, lost) = (hwnd(1), hwnd(2), hwnd(3), hwnd(4));
        let depth = |window: HWND| match window {
            window if window == pets => Some(2),
            window if window == detail => Some(5),
            window if window == history => Some(9),
            _ => None,
        };
        let overlays = [(Layer::Tool, pets), (Layer::Tool, lost), (Layer::Tool, history), (Layer::Tool, detail)];
        assert_eq!(stacking_order(&overlays, depth), vec![lost, history, detail, pets], "the highest now is the highest after");
    }

    #[test]
    fn settings_is_never_in_the_stack() {
        assert_eq!(Layer::of(WindowKind::Settings), None);
        assert_eq!(Layer::of(WindowKind::PetCard), Some(Layer::PetCard));
        assert_eq!(Layer::of(WindowKind::WhatsNew), Some(Layer::Tool), "What's new and the prompt keep under the cards");
    }
}
