//! The app's global shortcuts. Alt+CapsLock locks the meter, the only way back
//! from a click-through overlay; Alt+Shift+CapsLock shows or hides the toolbar
//! and every open tool. Neither needs a window: the caller says what an action
//! does when it arrives.

use futures::channel::mpsc::unbounded;
use futures::StreamExt;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use gpui::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleLock,
    ToggleToolbar,
}

impl HotkeyAction {
    fn hotkey(self) -> HotKey {
        match self {
            Self::ToggleLock => HotKey::new(Some(Modifiers::ALT), Code::CapsLock),
            Self::ToggleToolbar => HotKey::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::CapsLock),
        }
    }

    fn shortcut_name(self) -> &'static str {
        match self {
            Self::ToggleLock => "Alt+CapsLock",
            Self::ToggleToolbar => "Alt+Shift+CapsLock",
        }
    }
}

/// Which of the asked shortcuts the system accepted. A shortcut another
/// program already holds fails alone; the others stay registered.
pub struct Registration {
    registered: Vec<HotkeyAction>,
    pub failures: Vec<String>,
}

impl Registration {
    pub fn has(&self, action: HotkeyAction) -> bool {
        self.registered.contains(&action)
    }
}

/// Registers the shortcuts of `actions` for as long as the app runs. The
/// manager's hidden window receives `WM_HOTKEY` through GPUI's message loop on
/// this thread; the handler only queues, and `on_action` runs on the UI thread.
pub fn register(
    actions: &[HotkeyAction],
    cx: &mut App,
    on_action: impl Fn(HotkeyAction, &mut App) + 'static,
) -> Result<Registration, String> {
    let manager = GlobalHotKeyManager::new().map_err(|error| format!("cannot set up global shortcuts: {error}"))?;
    let mut registration = Registration { registered: Vec::new(), failures: Vec::new() };
    let mut ids = Vec::new();
    for &action in actions {
        let hotkey = action.hotkey();
        match manager.register(hotkey) {
            Ok(()) => {
                registration.registered.push(action);
                ids.push((hotkey.id(), action));
            }
            Err(error) => registration.failures.push(format!(
                "cannot register {} (is another copy of the meter running?): {error}",
                action.shortcut_name()
            )),
        }
    }

    let (presses, mut pressed) = unbounded();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state != HotKeyState::Pressed {
            return;
        }
        if let Some(&(_, action)) = ids.iter().find(|(id, _)| *id == event.id) {
            let _ = presses.unbounded_send(action);
        }
    }));

    cx.spawn(async move |cx| {
        let _manager = manager;
        while let Some(action) = pressed.next().await {
            cx.update(|cx| on_action(action, cx));
        }
    })
    .detach();
    Ok(registration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_its_own_shortcut() {
        assert_ne!(HotkeyAction::ToggleLock.hotkey().id(), HotkeyAction::ToggleToolbar.hotkey().id());
    }

    #[test]
    fn the_lock_shortcut_stays_alt_capslock() {
        assert_eq!(HotkeyAction::ToggleLock.hotkey(), HotKey::new(Some(Modifiers::ALT), Code::CapsLock));
    }

    #[test]
    fn the_toolbar_shortcut_is_alt_shift_capslock() {
        assert_eq!(
            HotkeyAction::ToggleToolbar.hotkey(),
            HotKey::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::CapsLock)
        );
    }
}
