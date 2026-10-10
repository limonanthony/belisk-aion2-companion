//! The toolbar, the app's hub (docs/gpui-mockups/approved/toolbar.html,
//! values in tokens.md, "Hub"): a small horizontal overlay that opens and
//! closes every tool, with the Odyle Energy and Shugo keys counts and, while
//! an update is available, downloading or ready, the update icon (`update.rs`).
//! At launch only the toolbar opens, and the meter with it when Settings ›
//! Overlay › "Open the meter at launch" is on. Fights are tracked only while
//! the meter is open; closing the toolbar quits the app.
//!
//! The meter keeps its window while closed, hidden, so it opens again where
//! it was left. [`Hub`] holds what is open; the toolbar's view asks it for
//! every action through [`ToolbarRequest`].
//!
//! With Settings › Overlay › "Hide between fights" on, an open meter's window
//! also steps aside while the core says you are between fights
//! (`MeterEvent::DpsIdleHidden`), unless a fight summary is up or the
//! first-open prompt waits on it. It is open all the same: fights are
//! tracked. Opening it, or clicking its icon while it steps aside, shows it
//! for [`PEEK_SECS`] through `DpsMeter::peek_overlay`.

mod counts;
mod prompt;
#[cfg(test)]
mod tests;
mod update;
mod view;
mod window;

use gpui::{App, AppContext as _, Entity, Global, WindowHandle};
use windows::Win32::Foundation::HWND;

use crate::meter::{MeterModel, MeterView};
use crate::overlay::{self, Placement};
use crate::secondary::{self, WindowKind};
use crate::settings::{self, AppSettings, Section};
use crate::update as updates;
use update::UpdateClick;

pub use prompt::{prompt_size, PromptView};
pub use view::{Tool, ToolbarRequest, ToolbarView};
#[cfg(feature = "probes")]
pub use view::{Dropdown, Slot};

/// How long the meter stays up with no fight after it opens, or after its
/// icon is clicked while it steps aside.
pub const PEEK_SECS: u64 = 10;

/// What is open: the meter, the toolbar's window, and what Alt+Shift+CapsLock
/// hid.
pub struct Hub {
    meter_window: WindowHandle<MeterView>,
    /// None where windows have no native handle (tests).
    meter_hwnd: Option<HWND>,
    meter_model: Entity<MeterModel>,
    placement: Placement,
    meter_open: bool,
    /// Whether the meter's window is on screen, as last shown or hidden.
    meter_shown: bool,
    toolbar: Option<(WindowHandle<ToolbarView>, HWND)>,
    /// What Alt+Shift+CapsLock hid, shown again by its next press or by the
    /// tray's Show toolbar. The meter's window is not among it: it comes back
    /// only if it is still wanted then.
    hidden: Option<Hidden>,
}

/// The toolbar, and the kinds of the other windows: each is looked up again
/// when shown, so one closed meanwhile stays closed.
struct Hidden {
    toolbar: Option<HWND>,
    windows: Vec<WindowKind>,
}

impl Hub {
    pub fn meter_is_open(&self) -> bool {
        self.meter_open
    }

    /// Open, but stepped aside between fights. Never while the first-open
    /// prompt, which sits on the meter, waits for an answer.
    pub fn meter_is_aside(&self, cx: &App) -> bool {
        self.meter_open && self.meter_model.read(cx).steps_aside() && !secondary::is_open(WindowKind::Prompt, cx)
    }

    fn meter_wanted(&self, cx: &App) -> bool {
        self.meter_open && self.hidden.is_none() && !self.meter_is_aside(cx)
    }

    #[cfg(test)]
    pub fn meter_is_shown(&self) -> bool {
        self.meter_shown
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

    /// The core's fight rule, for the update icon's warning that a restart
    /// splits the fight.
    pub fn in_fight(&self, cx: &App) -> bool {
        self.meter_model.read(cx).meter().is_some_and(|meter| meter.in_fight())
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
        meter_model: meter_model.clone(),
        placement,
        meter_open,
        meter_shown: meter_open,
        toolbar: None,
        hidden: None,
    });
    cx.set_global(HubHandle(hub.clone()));
    follow_meter_state(&hub, &meter_model, cx);
    if meter_open {
        peek_meter(cx);
    }
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
        meter_model: meter_model.clone(),
        placement: Placement::Screen,
        meter_open: false,
        meter_shown: false,
        toolbar: None,
        hidden: None,
    });
    cx.set_global(HubHandle(hub.clone()));
    follow_meter_state(&hub, &meter_model, cx);
    hub
}

/// The meter's window follows the model (the core saying you are idle or
/// not, a fight summary going up or coming down) and the first-open prompt
/// opening or closing.
fn follow_meter_state(hub: &Entity<Hub>, meter_model: &Entity<MeterModel>, cx: &mut App) {
    let followed = hub.clone();
    cx.observe(meter_model, move |_, cx| sync_meter_window(&followed, cx)).detach();
    if let Some(open_windows) = secondary::open_windows(cx) {
        let followed = hub.clone();
        cx.observe(&open_windows, move |_, cx| sync_meter_window(&followed, cx)).detach();
    }
}

pub fn hub(cx: &App) -> Option<Entity<Hub>> {
    cx.try_global::<HubHandle>().map(|handle| handle.0.clone())
}

/// Without a hub (some dev runs) the meter is all there is, so it counts as
/// open.
pub fn meter_is_open(cx: &App) -> bool {
    hub(cx).is_none_or(|hub| hub.read(cx).meter_open)
}

/// Whether the open meter steps aside between fights; a reminder card then
/// sits under the toolbar, as while the meter is closed.
pub fn meter_is_aside(cx: &App) -> bool {
    hub(cx).is_some_and(|hub| hub.read(cx).meter_is_aside(cx))
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
                Tool::Pets => secondary::open_pets(cx),
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
        ToolbarRequest::Update(UpdateClick::StartDownload) => updates::global(cx).read(cx).start_download(),
        ToolbarRequest::Update(UpdateClick::ShowDetails) => secondary::open_settings_at(&model, Section::About, cx),
        ToolbarRequest::Update(UpdateClick::Restart) => updates::global(cx).update(cx, |updates, cx| updates.restart(cx)),
        ToolbarRequest::Quit => cx.quit(),
    }
}

/// The meter's icon: shows a meter that steps aside between fights for a
/// moment, closes an open meter, or opens it and, the first time, asks
/// whether it should open at launch.
fn toggle_meter_from_toolbar(cx: &mut App) {
    if meter_is_aside(cx) {
        peek_meter(cx);
        return;
    }
    if meter_is_open(cx) {
        close_meter(cx);
        return;
    }
    open_meter(cx);
    if launch_prompt_due(settings::global(cx).read(cx).get()) {
        secondary::open_prompt(cx);
    }
}

/// Keeps the meter up for [`PEEK_SECS`] even with no fight. The core answers
/// with `DpsIdleHidden(false)`, which brings the window back.
fn peek_meter(cx: &App) {
    let Some(hub) = hub(cx) else {
        return;
    };
    if let Some(meter) = hub.read(cx).meter_model.read(cx).meter() {
        meter.peek_overlay(PEEK_SECS);
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
        Some(hub.meter_model.clone())
    });
    let Some(model) = changed else {
        return;
    };
    // The peek comes first, so the snapshot loop's first tick with fights
    // tracked already sees it.
    if open {
        peek_meter(cx);
    }
    if let Some(meter) = model.read(cx).meter() {
        meter.set_fight_tracking(open);
    }
    sync_meter_window(&hub, cx);
    if !open {
        secondary::close(WindowKind::Prompt, cx);
    }
}

/// Shows or hides the meter's window as the hub and the model now want it,
/// without taking the focus; nothing when it already is.
fn sync_meter_window(hub: &Entity<Hub>, cx: &mut App) {
    let change = hub.update(cx, |hub, cx| {
        let wanted = hub.meter_wanted(cx);
        if hub.meter_shown == wanted {
            return None;
        }
        hub.meter_shown = wanted;
        cx.notify();
        Some((wanted, hub.meter_hwnd, hub.placement))
    });
    let Some((shown, Some(hwnd), placement)) = change else {
        return;
    };
    if shown {
        match overlay::reveal(hwnd, placement) {
            Ok(()) => overlay::raise_without_activating(hwnd),
            Err(error) => crate::app_log::warn(format!("cannot show the meter: {error}")),
        }
    } else {
        overlay::hide(hwnd);
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
        show_hidden(&hidden, placement, cx);
        sync_meter_window(&hub, cx);
        return;
    }
    let toolbar = hub.read(cx).toolbar.map(|(_, hwnd)| hwnd);
    if let Some(hwnd) = toolbar {
        overlay::hide(hwnd);
    }
    let windows = secondary::hide_all(cx);
    hub.update(cx, |hub, _| hub.hidden = Some(Hidden { toolbar, windows }));
    sync_meter_window(&hub, cx);
}

fn show_hidden(hidden: &Hidden, placement: Placement, cx: &App) {
    if let Some(hwnd) = hidden.toolbar {
        if let Err(error) = overlay::reveal(hwnd, placement) {
            crate::app_log::warn(format!("cannot show the toolbar again: {error}"));
        }
    }
    secondary::show_again(&hidden.windows, cx);
}

/// The tray's Show toolbar, and a second launch: brings back what
/// Alt+Shift+CapsLock hid, unfolds the toolbar and brings it, and the meter
/// when it is on screen, in front. Neither takes the focus.
pub fn show_toolbar(cx: &mut App) {
    let Some(hub) = hub(cx) else {
        return;
    };
    let (hidden, placement, toolbar) = hub.update(cx, |hub, _| (hub.hidden.take(), hub.placement, hub.toolbar));
    if let Some(hidden) = hidden {
        show_hidden(&hidden, placement, cx);
    }
    sync_meter_window(&hub, cx);
    let shown_meter = {
        let hub = hub.read(cx);
        hub.meter_hwnd.filter(|_| hub.meter_shown)
    };
    let mut forward = Vec::new();
    if let Some((window, hwnd)) = toolbar {
        let _ = window.update(cx, |view, _, cx| view.unfold(cx));
        forward.push(hwnd);
    }
    forward.extend(shown_meter);
    match placement {
        Placement::Screen => forward.into_iter().for_each(overlay::bring_forward),
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
