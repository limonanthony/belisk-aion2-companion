//! The toolbar, the app's hub (docs/gpui-mockups/approved/toolbar.html,
//! values in tokens.md, "Hub"): a small horizontal overlay that opens and
//! closes every tool, with the Odyle Energy and Shugo keys counts.
//! At launch only the toolbar opens, and the meter with it when Settings ›
//! Overlay › "Open the meter at launch" is on. Fights are tracked only while
//! the meter is open; closing the toolbar quits the app.
//!
//! The meter keeps its window while closed, hidden, so it opens again where
//! it was left. [`Hub`] holds what is open; the toolbar's view asks it for
//! every action through [`ToolbarRequest`].

mod counts;
mod prompt;
#[cfg(test)]
mod tests;
mod view;
mod window;

use gpui::{App, AppContext as _, Entity, Global, WindowHandle};
use windows::Win32::Foundation::HWND;

use crate::meter::{MeterModel, MeterView};
use crate::overlay::{self, Placement};
use crate::secondary::{self, WindowKind};
use crate::settings::{self, AppSettings, Section};

pub use prompt::{prompt_size, PromptView};
pub use view::{Tool, ToolbarRequest, ToolbarView};
#[cfg(feature = "probes")]
pub use view::{Dropdown, Slot};

/// What is open: the meter, the toolbar's window, and what Alt+Shift+CapsLock
/// hid.
pub struct Hub {
    meter_window: WindowHandle<MeterView>,
    /// None where windows have no native handle (tests).
    meter_hwnd: Option<HWND>,
    meter_model: Entity<MeterModel>,
    placement: Placement,
    meter_open: bool,
    toolbar: Option<(WindowHandle<ToolbarView>, HWND)>,
    /// The windows Alt+Shift+CapsLock hid, shown again by its next press or
    /// by the tray's Show toolbar.
    hidden: Option<Vec<HWND>>,
}

impl Hub {
    pub fn meter_is_open(&self) -> bool {
        self.meter_open
    }

    #[cfg(test)]
    pub fn meter_model(&self) -> &Entity<MeterModel> {
        &self.meter_model
    }

    pub fn is_lit(&self, tool: Tool, cx: &App) -> bool {
        match tool.window() {
            Some(kind) => secondary::is_open(kind, cx),
            None => tool == Tool::Meter && self.meter_open,
        }
    }
}

struct HubHandle(Entity<Hub>);

impl Global for HubHandle {}

/// Makes the hub for the meter's window, already open (shown when
/// `meter_open`), and opens the toolbar.
pub fn install(
    meter_window: WindowHandle<MeterView>,
    meter_model: Entity<MeterModel>,
    placement: Placement,
    meter_open: bool,
    cx: &mut App,
) -> Result<(), String> {
    let meter_hwnd = meter_window
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("cannot reach the meter window: {error:#}"))??;
    let hub = cx.new(|_| Hub {
        meter_window,
        meter_hwnd: Some(meter_hwnd),
        meter_model,
        placement,
        meter_open,
        toolbar: None,
        hidden: None,
    });
    cx.set_global(HubHandle(hub.clone()));
    let saved = settings::global(cx).read(cx).get().windows.toolbar_at;
    let (toolbar, hwnd) = window::open(&hub, placement, saved, cx)?;
    let view = toolbar.entity(cx).map_err(|error| format!("cannot reach the toolbar's view: {error:#}"))?;
    cx.subscribe(&view, |_, request: &ToolbarRequest, cx| handle(*request, cx)).detach();
    hub.update(cx, |hub, _| hub.toolbar = Some((toolbar, hwnd)));
    Ok(())
}

/// A hub without windows, for the tests of what opening and closing the
/// meter does.
#[cfg(test)]
pub fn install_for_test(meter_window: WindowHandle<MeterView>, meter_model: Entity<MeterModel>, cx: &mut App) -> Entity<Hub> {
    let hub = cx.new(|_| Hub {
        meter_window,
        meter_hwnd: None,
        meter_model,
        placement: Placement::Screen,
        meter_open: false,
        toolbar: None,
        hidden: None,
    });
    cx.set_global(HubHandle(hub.clone()));
    hub
}

pub fn hub(cx: &App) -> Option<Entity<Hub>> {
    cx.try_global::<HubHandle>().map(|handle| handle.0.clone())
}

/// Without a hub (some dev runs) the meter is all there is, so it counts as
/// open.
pub fn meter_is_open(cx: &App) -> bool {
    hub(cx).is_none_or(|hub| hub.read(cx).meter_open)
}

pub fn toolbar_hwnd(cx: &App) -> Option<HWND> {
    hub(cx)?.read(cx).toolbar.map(|(_, hwnd)| hwnd)
}

#[cfg(feature = "probes")]
pub fn toolbar_window(cx: &App) -> Option<WindowHandle<ToolbarView>> {
    hub(cx)?.read(cx).toolbar.map(|(window, _)| window)
}

/// The first-open prompt is due when the meter opens from the toolbar: never
/// answered, and the meter does not already open at launch.
pub fn launch_prompt_due(settings: &AppSettings) -> bool {
    !settings.windows.launch_prompt_answered && !settings.windows.open_meter_at_launch
}

/// What the toolbar asked for.
pub fn handle(request: ToolbarRequest, cx: &mut App) {
    let Some(hub) = hub(cx) else {
        return;
    };
    let model = hub.read(cx).meter_model.clone();
    match request {
        ToolbarRequest::Toggle(Tool::Meter) => toggle_meter_from_toolbar(cx),
        ToolbarRequest::Toggle(Tool::Reminders) => {}
        ToolbarRequest::Toggle(tool) => {
            let Some(kind) = tool.window() else { return };
            if secondary::is_open(kind, cx) {
                secondary::close(kind, cx);
                return;
            }
            match tool {
                Tool::Pvp => secondary::open_pvp(&model, cx),
                Tool::Todos => secondary::open_todos(&model, cx),
                Tool::History => secondary::open_history(&model, cx),
                Tool::Settings => secondary::open_settings(&model, cx),
                Tool::Meter | Tool::Reminders => {}
            }
        }
        ToolbarRequest::ToggleResources => {
            if secondary::is_open(WindowKind::Resources, cx) {
                secondary::close(WindowKind::Resources, cx);
            } else {
                secondary::open_resources(&model, cx);
            }
        }
        ToolbarRequest::OpenResources => secondary::open_resources(&model, cx),
        ToolbarRequest::PauseReminders(paused) => {
            settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.reminders.enabled = !paused));
        }
        ToolbarRequest::ReminderSettings => secondary::open_settings_at(&model, Section::Reminders, cx),
        ToolbarRequest::Moved(at) => {
            settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.windows.toolbar_at = Some(at)));
        }
        ToolbarRequest::Quit => cx.quit(),
    }
}

/// The meter's icon: closes an open meter, or opens it and, the first time,
/// asks whether it should open at launch.
fn toggle_meter_from_toolbar(cx: &mut App) {
    if meter_is_open(cx) {
        close_meter(cx);
        return;
    }
    open_meter(cx);
    if launch_prompt_due(settings::global(cx).read(cx).get()) {
        secondary::open_prompt(cx);
    }
}

pub fn open_meter(cx: &mut App) {
    set_meter_open(true, cx);
}

/// The meter's ×: the toolbar stays. A prompt left unanswered goes with it,
/// to ask again next time.
pub fn close_meter(cx: &mut App) {
    set_meter_open(false, cx);
}

fn set_meter_open(open: bool, cx: &mut App) {
    let Some(hub) = hub(cx) else {
        return;
    };
    let changed = hub.update(cx, |hub, cx| {
        if hub.meter_open == open {
            return None;
        }
        hub.meter_open = open;
        cx.notify();
        Some((hub.meter_hwnd, hub.placement, hub.meter_model.clone()))
    });
    let Some((hwnd, placement, model)) = changed else {
        return;
    };
    if let Some(meter) = model.read(cx).meter() {
        meter.set_fight_tracking(open);
    }
    if let Some(hwnd) = hwnd {
        if open {
            match overlay::reveal(hwnd, placement) {
                Ok(()) => overlay::raise_without_activating(hwnd),
                Err(error) => eprintln!("cannot show the meter: {error}"),
            }
        } else {
            overlay::hide(hwnd);
        }
    }
    if !open {
        secondary::close(WindowKind::Prompt, cx);
    }
}

/// Alt+Shift+CapsLock: hides the toolbar and every open tool, or shows what
/// it hid.
pub fn toggle_everything(cx: &mut App) {
    let Some(hub) = hub(cx) else {
        return;
    };
    let (hidden, placement) = hub.update(cx, |hub, _| (hub.hidden.take(), hub.placement));
    if let Some(hidden) = hidden {
        show_hidden(&hidden, placement);
        return;
    }
    let mut shown: Vec<HWND> = {
        let hub = hub.read(cx);
        hub.toolbar.map(|(_, hwnd)| hwnd).into_iter().chain(hub.meter_hwnd.filter(|_| hub.meter_open)).collect()
    };
    shown.extend(secondary::open_hwnds(cx));
    for &hwnd in &shown {
        overlay::hide(hwnd);
    }
    hub.update(cx, |hub, _| hub.hidden = Some(shown));
}

fn show_hidden(hidden: &[HWND], placement: Placement) {
    for &hwnd in hidden {
        if let Err(error) = overlay::reveal(hwnd, placement) {
            eprintln!("cannot show a window again: {error}");
        }
    }
}

/// The tray's Show toolbar, and a second launch: brings back what
/// Alt+Shift+CapsLock hid, unfolds the toolbar and brings it in front.
pub fn show_toolbar(cx: &mut App) {
    let Some(hub) = hub(cx) else {
        return;
    };
    let (hidden, placement, toolbar) = hub.update(cx, |hub, _| (hub.hidden.take(), hub.placement, hub.toolbar));
    if let Some(hidden) = hidden {
        show_hidden(&hidden, placement);
    }
    let Some((window, hwnd)) = toolbar else {
        return;
    };
    let _ = window.update(cx, |view, _, cx| view.unfold(cx));
    match placement {
        Placement::Screen => overlay::bring_forward(hwnd),
        #[cfg(feature = "probes")]
        Placement::OffScreen => {}
    }
}

/// The lock shortcut acts on the meter's window only while the meter is
/// open.
pub fn meter_window_if_open(cx: &App) -> Option<WindowHandle<MeterView>> {
    let hub = hub(cx)?;
    let hub = hub.read(cx);
    hub.meter_open.then_some(hub.meter_window)
}
