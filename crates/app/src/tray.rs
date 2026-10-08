//! The notification-area icon: it brings the toolbar back after the meter and
//! every tool were closed, and quits the app. A left click shows the toolbar;
//! the right-click menu has "Show toolbar" (the default item, in bold) and
//! "Quit". No balloon or toast notification is ever raised.
//!
//! Plain Win32 through the `windows` crate the app already depends on: the
//! icon is owned by one hidden window on the UI thread, whose messages GPUI's
//! loop dispatches, like the global shortcuts'. A tray crate would add a menu
//! toolkit, and none of the ones considered can mark the default menu item.

use std::mem::size_of;

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::StreamExt;
use gpui::App;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
    NIN_SELECT, NOTIFYICONDATAW, NOTIFYICON_VERSION_4, NOTIFY_ICON_MESSAGE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, GetSystemMetrics,
    GetWindowLongPtrW, LoadImageW, PostMessageW, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    SetMenuDefaultItem, SetWindowLongPtrW, TrackPopupMenu, GWLP_USERDATA, HICON, IMAGE_ICON, LR_DEFAULTCOLOR, MF_STRING,
    SM_CXSMICON, SM_CYSMICON, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CONTEXTMENU, WM_NCDESTROY, WM_NULL,
    WNDCLASSW, WS_OVERLAPPED,
};

use crate::i18n::fl;

/// What the player asked of the app through the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowToolbar,
    Quit,
}

const CLASS_NAME: PCWSTR = w!("AION2DpsMeterTray");
const NOTIFICATION: u32 = WM_APP + 1;
const ICON_ID: u32 = 1;
/// `NIN_SELECT` with the key flag: Enter or Space on the focused icon.
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
/// The icon the build embeds in the exe (`windows/icon.ico`), resource 1.
const EXE_ICON_RESOURCE: usize = 1;
const SHOW_TOOLBAR_COMMAND: usize = 1;
const QUIT_COMMAND: usize = 2;
const TOOLTIP_CAPACITY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gesture {
    Select,
    OpenMenu,
}

/// With `NOTIFYICON_VERSION_4` the low word of `lparam` says what happened:
/// a left click or Enter/Space on the focused icon selects it, the right
/// button or the menu key asks for the context menu.
fn gesture(lparam: LPARAM) -> Option<Gesture> {
    match (lparam.0 & 0xFFFF) as u32 {
        NIN_SELECT | NIN_KEYSELECT => Some(Gesture::Select),
        WM_CONTEXTMENU => Some(Gesture::OpenMenu),
        _ => None,
    }
}

fn menu_action(command: usize) -> Option<TrayAction> {
    match command {
        SHOW_TOOLBAR_COMMAND => Some(TrayAction::ShowToolbar),
        QUIT_COMMAND => Some(TrayAction::Quit),
        _ => None,
    }
}

/// Where the menu opens, from the `wparam` of a version 4 notification: the
/// icon's anchor point, packed as two signed 16-bit coordinates.
fn anchor(wparam: WPARAM) -> POINT {
    POINT { x: i32::from((wparam.0 & 0xFFFF) as u16 as i16), y: i32::from(((wparam.0 >> 16) & 0xFFFF) as u16 as i16) }
}

struct WindowState {
    actions: UnboundedSender<TrayAction>,
    taskbar_created: u32,
    icon: HICON,
    tooltip: String,
}

/// The hidden window that owns the notification-area icon. Dropping it takes
/// the icon away.
struct TrayWindow {
    hwnd: HWND,
}

impl TrayWindow {
    /// A top-level window that is never shown: message-only windows do not
    /// receive the broadcast that says Explorer restarted.
    fn create(actions: UnboundedSender<TrayAction>, icon: HICON, tooltip: String) -> Result<Self, String> {
        let instance = module_instance()?;
        let class = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance, lpszClassName: CLASS_NAME, ..Default::default() };
        if unsafe { RegisterClassW(&class) } == 0 && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS {
            return Err(format!("cannot register the tray window class: {}", windows::core::Error::from_win32()));
        }
        let state = Box::new(WindowState {
            actions,
            taskbar_created: unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
            icon,
            tooltip,
        });
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                CLASS_NAME,
                w!(""),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .map_err(|error| format!("cannot create the tray window: {error}"))?;
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize) };
        Ok(Self { hwnd })
    }

    fn add_icon(&self) -> Result<(), String> {
        let state = unsafe { &*window_state(self.hwnd) };
        add_icon(self.hwnd, state)
    }
}

impl Drop for TrayWindow {
    fn drop(&mut self) {
        let data = notify_icon_data(self.hwnd);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn module_instance() -> Result<HINSTANCE, String> {
    unsafe { GetModuleHandleW(None) }
        .map(|module| HINSTANCE(module.0))
        .map_err(|error| format!("cannot find the program's module: {error}"))
}

fn window_state(hwnd: HWND) -> *const WindowState {
    unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState }
}

fn notify_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: ICON_ID, ..Default::default() }
}

fn add_icon(hwnd: HWND, state: &WindowState) -> Result<(), String> {
    let mut data = notify_icon_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = NOTIFICATION;
    data.hIcon = state.icon;
    for (slot, unit) in data.szTip.iter_mut().zip(state.tooltip.encode_utf16().take(TOOLTIP_CAPACITY - 1)) {
        *slot = unit;
    }
    notify(NIM_ADD, &data, "add the icon")?;
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    notify(NIM_SETVERSION, &data, "select the icon's notification version")
}

fn notify(message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW, what: &str) -> Result<(), String> {
    if unsafe { Shell_NotifyIconW(message, data) }.as_bool() {
        Ok(())
    } else {
        Err(format!("the notification area refused to {what}"))
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let state = window_state(hwnd);
    if state.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let state = unsafe { &*state };
    match message {
        NOTIFICATION => {
            match gesture(lparam) {
                Some(Gesture::Select) => {
                    let _ = state.actions.unbounded_send(TrayAction::ShowToolbar);
                }
                Some(Gesture::OpenMenu) => {
                    if let Some(action) = pick_from_menu(hwnd, anchor(wparam)) {
                        let _ = state.actions.unbounded_send(action);
                    }
                }
                None => {}
            }
            LRESULT(0)
        }
        message if message == state.taskbar_created => {
            if let Err(error) = add_icon(hwnd, state) {
                eprintln!("cannot bring the tray icon back after Explorer restarted: {error}");
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            drop(unsafe { Box::from_raw(state as *const WindowState as *mut WindowState) });
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// Shows the context menu at `at` and waits for the player's choice. The
/// window comes to the foreground first, or the menu would stay open after a
/// click elsewhere; the null message afterwards is the documented way to make
/// it close at once.
fn pick_from_menu(hwnd: HWND, at: POINT) -> Option<TrayAction> {
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    let show = HSTRING::from(fl!("tray-show-toolbar"));
    let quit = HSTRING::from(fl!("tray-quit"));
    let chosen = unsafe {
        let built = AppendMenuW(menu, MF_STRING, SHOW_TOOLBAR_COMMAND, &show).is_ok()
            && AppendMenuW(menu, MF_STRING, QUIT_COMMAND, &quit).is_ok()
            && SetMenuDefaultItem(menu, SHOW_TOOLBAR_COMMAND as u32, 0).is_ok();
        let chosen = if built {
            let _ = SetForegroundWindow(hwnd);
            TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, at.x, at.y, None, hwnd, None).0 as usize
        } else {
            0
        };
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        chosen
    };
    menu_action(chosen)
}

fn load_exe_icon() -> Result<HICON, String> {
    let instance = module_instance()?;
    let (width, height) = unsafe { (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)) };
    unsafe {
        LoadImageW(
            Some(instance),
            PCWSTR(EXE_ICON_RESOURCE as *const u16),
            IMAGE_ICON,
            width,
            height,
            LR_DEFAULTCOLOR,
        )
    }
    .map(|handle| HICON(handle.0))
    .map_err(|error| format!("cannot load the program's icon: {error}"))
}

/// Puts the icon in the notification area and hands every choice the player
/// makes through it to `on_action`, on the UI thread. The icon goes away when
/// the app quits; without that it would linger until the pointer crosses it,
/// because quitting ends the process.
pub fn install(cx: &mut App, on_action: impl Fn(TrayAction, &mut App) + 'static) -> Result<(), String> {
    let icon = load_exe_icon()?;
    let (actions, mut requested) = unbounded();
    let tray = TrayWindow::create(actions, icon, fl!("tray-tooltip"))?;
    tray.add_icon()?;

    let tray = std::rc::Rc::new(std::cell::RefCell::new(Some(tray)));
    let on_quit = std::rc::Rc::clone(&tray);
    cx.on_app_quit(move |_| {
        on_quit.borrow_mut().take();
        unsafe {
            let _ = DestroyIcon(icon);
        }
        async {}
    })
    .detach();

    cx.spawn(async move |cx| {
        let _tray = tray;
        while let Some(action) = requested.next().await {
            cx.update(|cx| on_action(action, cx));
        }
    })
    .detach();
    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc::UnboundedReceiver;
    use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

    use super::*;

    fn window_without_icon() -> (TrayWindow, UnboundedReceiver<TrayAction>) {
        let (actions, requested) = unbounded();
        let window = TrayWindow::create(actions, HICON::default(), "test".to_owned()).expect("the hidden window opens");
        (window, requested)
    }

    fn notify_window(window: &TrayWindow, event: u32) {
        unsafe { SendMessageW(window.hwnd, NOTIFICATION, Some(WPARAM(0)), Some(LPARAM(event as isize))) };
    }

    #[test]
    fn a_left_click_or_the_keyboard_selects_the_icon() {
        assert_eq!(gesture(LPARAM(NIN_SELECT as isize)), Some(Gesture::Select));
        assert_eq!(gesture(LPARAM(NIN_KEYSELECT as isize)), Some(Gesture::Select));
    }

    #[test]
    fn the_right_button_or_the_menu_key_opens_the_menu() {
        assert_eq!(gesture(LPARAM(WM_CONTEXTMENU as isize)), Some(Gesture::OpenMenu));
    }

    #[test]
    fn the_icon_id_in_the_high_word_does_not_change_the_gesture() {
        assert_eq!(gesture(LPARAM(((ICON_ID as isize) << 16) | NIN_SELECT as isize)), Some(Gesture::Select));
    }

    #[test]
    fn other_notifications_are_ignored() {
        assert_eq!(gesture(LPARAM(0x0200)), None);
    }

    #[test]
    fn each_menu_command_is_its_action() {
        assert_eq!(menu_action(SHOW_TOOLBAR_COMMAND), Some(TrayAction::ShowToolbar));
        assert_eq!(menu_action(QUIT_COMMAND), Some(TrayAction::Quit));
        assert_eq!(menu_action(0), None);
    }

    #[test]
    fn the_menu_anchor_reads_signed_coordinates() {
        let packed = ((-20i16 as u16 as usize) << 16) | 1500;
        let point = anchor(WPARAM(packed));
        assert_eq!((point.x, point.y), (1500, -20));
    }

    #[test]
    fn the_tooltip_is_the_product_name() {
        assert_eq!(fl!("tray-tooltip"), "AION 2 DPS Meter");
    }

    #[test]
    fn a_left_click_on_the_hidden_window_asks_to_show_the_toolbar() {
        let (window, mut requested) = window_without_icon();

        notify_window(&window, NIN_SELECT);

        assert_eq!(requested.try_recv().ok(), Some(TrayAction::ShowToolbar));
        assert!(requested.try_recv().is_err());
    }

    #[test]
    fn a_second_window_can_be_opened_while_the_first_lives() {
        let (_first, _) = window_without_icon();
        let (second, mut requested) = window_without_icon();

        notify_window(&second, NIN_KEYSELECT);

        assert_eq!(requested.try_recv().ok(), Some(TrayAction::ShowToolbar));
    }
}
