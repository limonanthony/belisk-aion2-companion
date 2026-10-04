//! The one place an overlay may take keyboard focus: a text field being
//! edited. Everywhere else the windows are non-activating, so clicking them
//! leaves the game focused.
//!
//! Only our own window is ever activated, and never through GPUI's
//! `activate_window`, which injects an Alt key press with `SendInput`. Giving
//! focus back names no window: the window is hidden for an instant, and Windows
//! activates whatever is next in the z-order (ShowWindow with `SW_HIDE`: "hides
//! the window and activates another window"). Our overlays are
//! `WS_EX_NOACTIVATE`, which Windows skips when it picks that window.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SetForegroundWindow, ShowWindow, SW_HIDE, WS_EX_NOACTIVATE,
};

use crate::overlay;

/// Where an edit is. The foreground request runs on a later turn than the
/// press on "+", so the edit can be stopped (Escape, the pointer leaving,
/// the window closing) while the request is in flight; whoever makes the
/// request asks [`Edit::began`] afterwards whether the edit is still wanted,
/// and gives focus back at once when it is not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Edit {
    #[default]
    Idle,
    /// "+" was pressed; the foreground request has not been answered.
    Starting,
    Editing,
}

impl Edit {
    /// "+" was pressed. Returns whether to ask Windows for the foreground.
    pub fn start(&mut self) -> bool {
        let idle = *self == Self::Idle;
        if idle {
            *self = Self::Starting;
        }
        idle
    }

    /// Whether the foreground request is still wanted.
    pub fn wants_focus(self) -> bool {
        self == Self::Starting
    }

    /// The foreground request was answered. Returns whether the edit goes
    /// on; when it was granted but the edit stopped meanwhile, the caller
    /// must give focus back ([`end`]).
    pub fn began(&mut self, granted: bool) -> bool {
        let wanted = *self == Self::Starting;
        if wanted {
            *self = if granted { Self::Editing } else { Self::Idle };
        }
        wanted && granted
    }

    /// Ends the edit, or the request in flight. Returns whether the window
    /// may hold focus, so [`end`] must run.
    pub fn stop(&mut self) -> bool {
        let editing = *self == Self::Editing;
        *self = Self::Idle;
        editing
    }

    pub fn is_editing(self) -> bool {
        self == Self::Editing
    }
}

/// Lets the window be activated and brings it to the foreground. Called while
/// handling a click on the window, which is what entitles this process to
/// `SetForegroundWindow`.
pub fn begin(hwnd: HWND) -> Result<(), String> {
    set_activatable(hwnd, true);
    unsafe { SetForegroundWindow(hwnd) }
        .as_bool()
        .then_some(())
        .ok_or_else(|| {
            set_activatable(hwnd, false);
            "Windows did not let the overlay take keyboard focus".to_string()
        })
}

/// Makes the window non-activating again and, if it still holds the
/// foreground, lets go of it.
pub fn end(hwnd: HWND) {
    set_activatable(hwnd, false);
    if is_foreground(hwnd) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        overlay::show_without_activating(hwnd);
    }
}

#[cfg(feature = "probes")]
pub fn is_activatable(hwnd: HWND) -> bool {
    overlay::ex_style(hwnd) & WS_EX_NOACTIVATE.0 == 0
}

pub fn set_activatable(hwnd: HWND, activatable: bool) {
    overlay::update_ex_style(hwnd, |style| {
        if activatable {
            style & !WS_EX_NOACTIVATE.0
        } else {
            style | WS_EX_NOACTIVATE.0
        }
    });
}

fn is_foreground(hwnd: HWND) -> bool {
    unsafe { GetForegroundWindow() == hwnd }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_granted_request_starts_the_edit_and_stopping_gives_focus_back() {
        let mut edit = Edit::default();
        assert!(edit.start());
        assert!(!edit.start(), "a second press asks nothing more");
        assert!(edit.wants_focus());
        assert!(edit.began(true));
        assert!(edit.is_editing());
        assert!(edit.stop(), "the window may hold focus");
        assert!(!edit.stop(), "stopping twice gives focus back once");
    }

    #[test]
    fn stopping_while_the_request_is_in_flight_drops_it() {
        let mut edit = Edit::default();
        edit.start();
        assert!(!edit.stop(), "nothing was granted yet");
        assert!(!edit.wants_focus());
        assert!(!edit.began(true), "granted too late: the caller gives focus back");
        assert_eq!(edit, Edit::Idle);
    }

    #[test]
    fn a_refused_request_leaves_nothing_to_undo() {
        let mut edit = Edit::default();
        edit.start();
        assert!(!edit.began(false));
        assert_eq!(edit, Edit::Idle);
        assert!(!edit.stop());
    }
}
