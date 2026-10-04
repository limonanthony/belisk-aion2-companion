//! The windows besides the meter: fight history, player detail and PvP.
//!
//! Each kind has at most one window. Opening one that is already open raises
//! it, without activating it, and the player detail window is pointed at the
//! new player instead. A window closes with its own × or when the app quits.
//! Like the meter's, these windows are frameless, topmost, out of the taskbar
//! and non-activating, and are exactly as large as their card; they are placed
//! beside the meter the first time they open. The lock (Alt+CapsLock) applies
//! to the meter only, as in the Tauri app.

pub mod chrome;
mod monitor;
mod placement;
mod registry;
pub mod text_input;
pub mod typing;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyEntity, AnyWindowHandle, App, AppContext as _, Entity, Global, Render, Window, WindowHandle,
};
use windows::Win32::Foundation::HWND;

use crate::detail::{DetailModel, DetailSource, DetailView};
use crate::history::{HistoryModel, HistoryView};
use crate::meter::{MeterModel, MeterView};
use crate::overlay::{self, Placement};
use crate::pvp::{PvpModel, PvpView};
use placement::Side;
pub use registry::WindowKind;
use registry::{Opened, Registry};

const HISTORY_SIZE: (f32, f32) = (370.0, 420.0);
const DETAIL_SIZE: (f32, f32) = (520.0, 420.0);
const PVP_SIZE: (f32, f32) = (340.0, 420.0);

#[derive(Clone)]
struct Open {
    window: AnyWindowHandle,
    model: AnyEntity,
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
        let model = cx.new(|_| HistoryModel::new(meter));
        let meter_model = meter_model.clone();
        let view_model = model.clone();
        let window = create_window(
            WindowKind::History,
            HISTORY_SIZE,
            Side::Below,
            move |window, cx| cx.new(|cx| HistoryView::new(view_model, meter_model, window, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, model: model.into() })
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
            Side::Left,
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
        Ok(Open { window, model: model.into() })
    });
    finish(WindowKind::Pvp, opened, cx);
}

/// Opens the player detail window on `source`, or points the open one at it.
pub fn open_detail(meter_model: &Entity<MeterModel>, source: DetailSource, cx: &mut App) {
    let registry = registry(cx);
    let opened = registry.borrow_mut().open(WindowKind::Detail, || {
        let model = cx.new(|cx| DetailModel::new(meter_model.clone(), source.clone(), cx));
        let view_model = model.clone();
        let window = create_window(
            WindowKind::Detail,
            DETAIL_SIZE,
            Side::Right,
            move |_, cx| cx.new(|cx| DetailView::new(view_model, cx)),
            |_| {},
            cx,
        )?;
        Ok(Open { window, model: model.into() })
    });
    if opened == Ok(Opened::Existing) {
        let existing = registry.borrow().get(WindowKind::Detail).map(|open| open.model.clone());
        if let Some(model) = existing.and_then(|model| model.downcast::<DetailModel>().ok()) {
            model.update(cx, |model, cx| model.select(source, cx));
        }
    }
    finish(WindowKind::Detail, opened, cx);
}

/// The meter saved a fight: the history window, if open, lists it.
pub fn history_updated(cx: &mut App) {
    let Some(secondary) = cx.try_global::<Secondary>() else {
        return;
    };
    let open = secondary.registry.borrow().get(WindowKind::History).map(|open| open.model.clone());
    if let Some(model) = open.and_then(|model| model.downcast::<HistoryModel>().ok()) {
        model.update(cx, |model, cx| model.refresh(cx));
    }
}

/// The open window's handle, for the capture tooling.
#[cfg(feature = "probes")]
pub fn window_of(kind: WindowKind, cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<Secondary>()?.registry.borrow().get(kind).map(|open| open.window)
}

/// The open window's model, for the capture tooling.
#[cfg(feature = "probes")]
pub fn model_of<T: 'static>(kind: WindowKind, cx: &App) -> Option<Entity<T>> {
    let model = cx.try_global::<Secondary>()?.registry.borrow().get(kind)?.model.clone();
    model.downcast::<T>().ok()
}

fn registry(cx: &App) -> Rc<RefCell<Registry<Open>>> {
    cx.global::<Secondary>().registry.clone()
}

/// A window that was already open comes forward; a failed open is reported.
fn finish(kind: WindowKind, opened: Result<Opened, String>, cx: &mut App) {
    match opened {
        Ok(Opened::Created) => {}
        Ok(Opened::Existing) => {
            let open = registry(cx).borrow().get(kind).cloned();
            if let Some(open) = open {
                let _ = open.window.update(cx, |_, window, _| {
                    if let Ok(hwnd) = overlay::hwnd(window) {
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
    side: Side,
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
                    overlay::make_overlay(hwnd);
                    let scale = window.scale_factor();
                    let (device_width, device_height) = (width * scale, height * scale);
                    match placement {
                        Placement::Screen => {
                            let size = (device_width.ceil() as i32, device_height.ceil() as i32);
                            let (x, y) = monitor::window_and_work_area(meter_hwnd)
                                .map_or((64, 64), |(meter, work)| placement::beside(meter, size, work, side));
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
