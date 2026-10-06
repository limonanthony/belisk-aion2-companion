//! The windows besides the meter: fight history, player detail, PvP, the
//! to-dos, Settings and the event reminder.
//!
//! Each kind has at most one window. Opening one that is already open brings
//! it forward, and the player detail window is pointed at the new player
//! instead; a newer reminder takes the place of the one up. A window closes
//! with its own × or when the app quits.
//! Like the meter's, the history, detail, PvP, to-do and reminder windows are
//! frameless, topmost, out of the taskbar and non-activating, and are exactly
//! as large as their card; they are placed beside the meter the first time
//! they open, and the reminder keeps to the meter while it is up.
//! Settings is a normal window instead: it takes the keyboard focus, has a
//! taskbar button and opens in the middle of the screen. The lock
//! (Alt+CapsLock) applies to the meter, and to the reminder sitting on it.

pub mod chrome;
mod monitor;
mod placement;
mod registry;
pub mod text_input;
pub mod typing;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    AnyEntity, AnyWindowHandle, App, AppContext as _, Entity, Global, Pixels, Render, Size, Window, WindowHandle,
};
use windows::Win32::Foundation::HWND;

use crate::detail::{DetailModel, DetailSource, DetailView};
use crate::history::{HistoryModel, HistoryView};
use crate::meter::{self, MeterModel, MeterView};
use crate::overlay::{self, Placement};
use crate::pvp::{PvpModel, PvpView};
use crate::reminders::{self, MeterLink, ReminderView, ShowReminder};
use crate::settings::{self, DetailPlacement, RuntimeModel, Section, SettingsView};
use crate::todos::TodoView;
use placement::{Anchor, Side};
pub use placement::Rect;
pub use registry::WindowKind;
use registry::{Opened, Registry};

const HISTORY_SIZE: (f32, f32) = (370.0, 420.0);
const DETAIL_SIZE: (f32, f32) = (520.0, 420.0);
const PVP_SIZE: (f32, f32) = (340.0, 420.0);
/// The tallest the to-do window gets at 100%; it fits its card from the
/// first frame on.
const TODOS_SIZE: (f32, f32) = (370.0, 558.0);

#[derive(Clone)]
struct Open {
    window: AnyWindowHandle,
    /// What the window shows, kept by the registry; none when the view is
    /// all there is, so closing the window releases it.
    model: Option<AnyEntity>,
}

struct Secondary {
    registry: Rc<RefCell<Registry<Open>>>,
    meter_hwnd: HWND,
    placement: Placement,
}

impl Global for Secondary {}

/// Sets the layer up once the meter's window exists: the windows are placed
/// next to it, and all of them close when the app quits.
pub fn init(meter: &WindowHandle<MeterView>, placement: Placement, cx: &mut App) -> Result<(), String> {
    let meter_hwnd = meter
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("cannot reach the meter window: {error:#}"))??;
    text_input::bind_keys(cx);
    let registry = Rc::new(RefCell::new(Registry::default()));
    cx.set_global(Secondary { registry: registry.clone(), meter_hwnd, placement });
    cx.subscribe(&reminders::global(cx), |_, request: &ShowReminder, cx| show_reminder(request, cx)).detach();
    cx.on_app_quit(move |cx| {
        let open = registry.borrow_mut().drain();
        for open in open {
            let _ = open.window.update(cx, |_, window, _| window.remove_window());
        }
        async {}
    })
    .detach();
    Ok(())
}

pub fn open_history(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let registry = registry(cx);
    let meter = meter_model.read(cx).meter().cloned();
    let opened = registry.borrow_mut().open(WindowKind::History, || {
        let model = cx.new(|cx| HistoryModel::new(meter, cx));
        let meter_model = meter_model.clone();
        let view_model = model.clone();
        let window = create_window(
            WindowKind::History,
            HISTORY_SIZE,
            Anchor::Beside(Side::Below),
            move |window, cx| cx.new(|cx| HistoryView::new(view_model, meter_model, window, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, model: Some(model.into()) })
    });
    finish(WindowKind::History, opened, cx);
}

pub fn open_pvp(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let registry = registry(cx);
    let opened = registry.borrow_mut().open(WindowKind::Pvp, || {
        let meter = meter_model.read(cx).meter().cloned();
        let model = cx.new(|cx| PvpModel::new(meter_model.clone(), cx));
        let view_model = model.clone();
        let pvp_mode_to_reset = meter.clone();
        let window = create_window(
            WindowKind::Pvp,
            PVP_SIZE,
            Anchor::Beside(Side::Left),
            move |window, cx| cx.new(|cx| PvpView::new(view_model, window, cx)),
            move |_| {
                if let Some(meter) = &pvp_mode_to_reset {
                    meter.set_pvp_mode(false);
                }
            },
            cx,
        )?;
        if let Some(meter) = &meter {
            meter.set_pvp_mode(true);
        }
        Ok(Open { window, model: Some(model.into()) })
    });
    finish(WindowKind::Pvp, opened, cx);
}

pub fn open_todos(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let registry = registry(cx);
    let opened = registry.borrow_mut().open(WindowKind::Todos, || {
        let meter_model = meter_model.clone();
        let window = create_window(
            WindowKind::Todos,
            TODOS_SIZE,
            Anchor::Beside(Side::Right),
            move |window, cx| cx.new(|cx| TodoView::new(meter_model, window, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, model: None })
    });
    finish(WindowKind::Todos, opened, cx);
}

/// Opens the player detail window on `source`, or points the open one at it.
pub fn open_detail(meter_model: &Entity<MeterModel>, source: DetailSource, cx: &mut App) {
    let registry = registry(cx);
    let anchor = match settings::global(cx).read(cx).get().windows.detail_opens {
        DetailPlacement::Beside => Anchor::Beside(Side::Right),
        DetailPlacement::Centred => Anchor::Centre,
    };
    let opened = registry.borrow_mut().open(WindowKind::Detail, || {
        let model = cx.new(|cx| DetailModel::new(meter_model.clone(), source.clone(), cx));
        let view_model = model.clone();
        let window = create_window(
            WindowKind::Detail,
            DETAIL_SIZE,
            anchor,
            move |_, cx| cx.new(|cx| DetailView::new(view_model, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, model: Some(model.into()) })
    });
    if opened == Ok(Opened::Existing) {
        let existing = registry.borrow().get(WindowKind::Detail).and_then(|open| open.model.clone());
        if let Some(model) = existing.and_then(|model| model.downcast::<DetailModel>().ok()) {
            model.update(cx, |model, cx| model.select(source, cx));
        }
    }
    finish(WindowKind::Detail, opened, cx);
}

/// Opens Settings, or brings the open window to the front. It happens once
/// the current event is over: the window reads the meter's lock, and the
/// meter's window is still busy with the menu click that asked for Settings.
pub fn open_settings(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let meter_model = meter_model.clone();
    cx.defer(move |cx| open_settings_now(&meter_model, None, cx));
}

/// [`open_settings`] on `section`, also when the window was already open.
pub fn open_settings_at(meter_model: &Entity<MeterModel>, section: Section, cx: &mut App) {
    let meter_model = meter_model.clone();
    cx.defer(move |cx| open_settings_now(&meter_model, Some(section), cx));
}

fn open_settings_now(meter_model: &Entity<MeterModel>, section: Option<Section>, cx: &mut App) {
    let registry = registry(cx);
    let placement = cx.global::<Secondary>().placement;
    let opened = registry.borrow_mut().open(WindowKind::Settings, || {
        let runtime = cx.new(|cx| RuntimeModel::new(meter_model.clone(), cx));
        let meter_window = meter::window(cx);
        let view_runtime = runtime.clone();
        let handle = settings::open_window(
            placement,
            move |window, cx| cx.new(|cx| SettingsView::new(view_runtime, Some(meter_window), window, cx)),
            cx,
        )?;
        let view = handle.entity(cx).map_err(|error| format!("cannot reach the Settings view: {error:#}"))?;
        let forget = registry.clone();
        cx.observe_release(&view, move |_, _| {
            forget.borrow_mut().remove(WindowKind::Settings);
        })
        .detach();
        Ok(Open { window: handle.into(), model: Some(runtime.into()) })
    });
    if let (Some(section), Ok(_)) = (section, &opened) {
        let window = registry.borrow().get(WindowKind::Settings).and_then(|open| open.window.downcast::<SettingsView>());
        if let Some(window) = window {
            let _ = window.update(cx, |view, _, cx| view.show_section(section, cx));
        }
    }
    finish(WindowKind::Settings, opened, cx);
}

/// Shows a reminder on the meter, or puts it in place of the one up.
pub fn show_reminder(request: &ShowReminder, cx: &mut App) {
    let registry = registry(cx);
    let open = registry.borrow().get(WindowKind::Reminder).and_then(|open| open.window.downcast::<ReminderView>());
    if let Some(window) = open {
        if window.update(cx, |view, _, cx| view.show(request, cx)).is_ok() {
            return;
        }
        registry.borrow_mut().remove(WindowKind::Reminder);
    }
    let (meter_hwnd, placement) = {
        let secondary = cx.global::<Secondary>();
        (secondary.meter_hwnd, secondary.placement)
    };
    let overlay = settings::global(cx).read(cx).get().overlay.clone();
    let expected = reminders::expected_size(request, &overlay);
    let gap = reminders::GAP * overlay.scale();
    let meter_window = meter::window(cx);
    let request = request.clone();
    let opened: Result<Opened, String> = registry.borrow_mut().open(WindowKind::Reminder, || {
        let on_meter = Rc::new(OnMeter { meter_hwnd, placement, placed: Cell::new(None) });
        let handle = create_window(
            WindowKind::Reminder,
            (expected.width.as_f32(), expected.height.as_f32()),
            Anchor::Stacked { gap },
            move |window, cx| {
                let link = MeterLink { window: meter_window, placement: on_meter };
                cx.new(|cx| ReminderView::new(&request, Some(link), window, cx))
            },
            |_| {},
            cx,
        )?;
        Ok(Open { window: handle, model: None })
    });
    if let Err(error) = opened {
        eprintln!("cannot open the Reminder window: {error}");
    }
}

/// Keeps a window on the meter's left edge, a gap above it, or below it when
/// its monitor has no room above, as the meter moves and the window's card
/// changes size. Off screen (dev captures) the meter is past every monitor,
/// and the window with it.
pub struct OnMeter {
    meter_hwnd: HWND,
    placement: Placement,
    /// Where the window was last put, in device pixels.
    placed: Cell<Option<Rect>>,
}

impl OnMeter {
    /// `card` and `gap` are logical pixels at the window's scale.
    pub fn place(&self, card: Size<Pixels>, gap: f32, window: &Window, cx: &mut App) {
        let Some((meter, work)) = monitor::window_and_work_area(self.meter_hwnd) else {
            return;
        };
        let scale = window.scale_factor();
        let size = ((card.width.as_f32() * scale).ceil() as i32, (card.height.as_f32() * scale).ceil() as i32);
        let (x, y) = Anchor::Stacked { gap }.corner(meter, size, work, scale);
        let target = Rect { left: x, top: y, right: x + size.0, bottom: y + size.1 };
        if self.placed.replace(Some(target)) == Some(target) {
            return;
        }
        let hwnd = match overlay::hwnd(window) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                eprintln!("cannot place the reminder: {error}");
                return;
            }
        };
        let placement = self.placement;
        // After the current frame: SetWindowPos sends WM_SIZE back into GPUI.
        cx.spawn(async move |_| {
            if let Err(error) = overlay::place(hwnd, x, y, size.0 as f32, size.1 as f32) {
                eprintln!("{error}");
            }
            match placement {
                Placement::Screen => {}
                #[cfg(feature = "probes")]
                Placement::OffScreen => {
                    if let Err(error) = overlay::off_screen::ensure_off_screen(hwnd) {
                        eprintln!("the reminder: {error}");
                    }
                }
            }
        })
        .detach();
    }
}

/// The meter saved a fight: the history window, if open, lists it.
pub fn history_updated(cx: &mut App) {
    let Some(secondary) = cx.try_global::<Secondary>() else {
        return;
    };
    let open = secondary.registry.borrow().get(WindowKind::History).and_then(|open| open.model.clone());
    if let Some(model) = open.and_then(|model| model.downcast::<HistoryModel>().ok()) {
        model.update(cx, |model, cx| model.refresh(cx));
    }
}

/// The meter's window and the work area of its monitor, in device pixels,
/// for the capture tooling.
#[cfg(feature = "probes")]
pub fn meter_and_work_area(cx: &App) -> Option<(Rect, Rect)> {
    monitor::window_and_work_area(cx.try_global::<Secondary>()?.meter_hwnd)
}

/// The open window's handle, for the capture tooling.
#[cfg(feature = "probes")]
pub fn window_of(kind: WindowKind, cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<Secondary>()?.registry.borrow().get(kind).map(|open| open.window)
}

/// The open window's model, for the capture tooling.
#[cfg(feature = "probes")]
pub fn model_of<T: 'static>(kind: WindowKind, cx: &App) -> Option<Entity<T>> {
    let model = cx.try_global::<Secondary>()?.registry.borrow().get(kind)?.model.clone()?;
    model.downcast::<T>().ok()
}

fn registry(cx: &App) -> Rc<RefCell<Registry<Open>>> {
    cx.global::<Secondary>().registry.clone()
}

/// A window that was already open comes forward; a failed open is reported.
/// Settings is brought to the foreground like any normal window; the overlays
/// never are.
fn finish(kind: WindowKind, opened: Result<Opened, String>, cx: &mut App) {
    match opened {
        Ok(Opened::Created) => {}
        Ok(Opened::Existing) => {
            let open = registry(cx).borrow().get(kind).cloned();
            let foreground = kind == WindowKind::Settings && cx.global::<Secondary>().placement == Placement::Screen;
            if let Some(open) = open {
                let _ = open.window.update(cx, |_, window, _| {
                    let Ok(hwnd) = overlay::hwnd(window) else { return };
                    if foreground {
                        settings::bring_forward(hwnd);
                    } else {
                        overlay::raise_without_activating(hwnd);
                    }
                });
            }
        }
        Err(error) => eprintln!("cannot open the {kind:?} window: {error}"),
    }
}

/// Opens a transparent pop-up like the meter's, places it, shows it without
/// activating it, and forgets it in the registry when its view is released
/// (however the window ends).
fn create_window<V: Render + 'static>(
    kind: WindowKind,
    (width, height): (f32, f32),
    anchor: Anchor,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
    on_close: impl FnOnce(&mut App) + 'static,
    cx: &mut App,
) -> Result<AnyWindowHandle, String> {
    let (placement, meter_hwnd, registry) = {
        let secondary = cx.global::<Secondary>();
        (secondary.placement, secondary.meter_hwnd, secondary.registry.clone())
    };
    let handle = cx
        .open_window(overlay::popup_options(width, height), build)
        .map_err(|error| format!("cannot open the window: {error:#}"))?;
    let set_up = handle
        .entity(cx)
        .map_err(|error| format!("cannot reach the window's view: {error:#}"))
        .and_then(|view| {
            handle
                .update(cx, |_, window, _| -> Result<(), String> {
                    let hwnd = overlay::hwnd(window)?;
                    overlay::make_overlay(hwnd)?;
                    let scale = window.scale_factor();
                    let (device_width, device_height) = (width * scale, height * scale);
                    match placement {
                        Placement::Screen => {
                            let size = (device_width.ceil() as i32, device_height.ceil() as i32);
                            let (x, y) = monitor::window_and_work_area(meter_hwnd)
                                .map_or((64, 64), |(meter, work)| anchor.corner(meter, size, work, scale));
                            overlay::place(hwnd, x, y, device_width, device_height)?;
                            overlay::show_without_activating(hwnd);
                            Ok(())
                        }
                        #[cfg(feature = "probes")]
                        Placement::OffScreen => overlay::off_screen::show(hwnd, device_width, device_height),
                    }
                })
                .map_err(|error| format!("cannot reach the window: {error:#}"))??;
            Ok(view)
        });
    // A window that could not be set up is closed, so the registry (which
    // does not keep it) and the screen agree, and the next open makes one.
    let view = match set_up {
        Ok(view) => view,
        Err(error) => {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            return Err(error);
        }
    };
    cx.observe_release(&view, move |_, cx| {
        registry.borrow_mut().remove(kind);
        on_close(cx);
    })
    .detach();
    Ok(handle.into())
}
