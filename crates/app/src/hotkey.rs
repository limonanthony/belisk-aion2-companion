//! Alt+CapsLock, the app's default lock shortcut: the only way back from a
//! click-through overlay until the main window exists. Phase 5 generalises
//! shortcuts.

use futures::channel::mpsc::unbounded;
use futures::StreamExt;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use gpui::{App, WindowHandle};

use crate::meter::MeterView;

/// Registers the shortcut for as long as the app runs. The manager's hidden
/// window receives `WM_HOTKEY` through GPUI's message loop on this thread; the
/// handler only queues, and the toggle runs on the UI thread.
pub fn register_lock_toggle(window: WindowHandle<MeterView>, cx: &mut App) -> Result<(), String> {
    let manager = GlobalHotKeyManager::new()
        .map_err(|error| format!("cannot set up global shortcuts: {error}"))?;
    let lock = HotKey::new(Some(Modifiers::ALT), Code::CapsLock);
    manager.register(lock).map_err(|error| {
        format!("cannot register Alt+CapsLock (is another copy of the meter running?): {error}")
    })?;

    let (presses, mut pressed) = unbounded();
    let id = lock.id();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id == id && event.state == HotKeyState::Pressed {
            let _ = presses.unbounded_send(());
        }
    }));

    cx.spawn(async move |cx| {
        let _manager = manager;
        while pressed.next().await.is_some() {
            let toggled = window.update(cx, |view, window, cx| view.toggle_lock(window, cx));
            if toggled.is_err() {
                break;
            }
        }
    })
    .detach();
    Ok(())
}
