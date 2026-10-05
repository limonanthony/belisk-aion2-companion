//! Carries a settings change to what it affects, at once: the overlay model
//! (every window reads its look from there), the decision log, and the core's
//! running config. Runs on the UI thread; nothing here touches the meter's own
//! threads except `apply_config`, which only swaps a lock-protected struct.

use std::sync::Arc;
use std::time::Duration;

use aether_core::config::DpsMeterConfig;
use aether_core::engine::meter::DpsMeter;
use gpui::{App, Entity, Task};

use super::model::{AppSettings, Settings};
use crate::meter::MeterModel;

/// The core logs every config it applies, so a run of changes (a slider drag)
/// is applied once, after it settles.
const CORE_APPLY_AFTER: Duration = Duration::from_millis(150);

/// Starts applying changes of `settings`. What the settings were when this is
/// called is taken as already applied, except the decision log, which is
/// switched on here.
pub fn connect(settings: &Entity<Settings>, model: &Entity<MeterModel>, meter: Option<Arc<DpsMeter>>, cx: &mut App) {
    let mut applied = settings.read(cx).get().clone();
    model.update(cx, |model, _| model.set_decision_log(applied.capture.decision_log));

    let model = model.clone();
    let mut core_apply: Option<Task<()>> = None;
    cx.observe(settings, move |settings, cx| {
        let next = settings.read(cx).get().clone();
        if next.overlay != applied.overlay {
            model.update(cx, |model, cx| {
                model.set_overlay_settings(next.overlay.clone());
                cx.notify();
            });
        }
        if next.capture.decision_log != applied.capture.decision_log {
            model.update(cx, |model, _| model.set_decision_log(next.capture.decision_log));
        }
        if let Some(meter) = meter.clone().filter(|meter| core_update(&applied, &next, meter.current_config()).is_some()) {
            let apply = cx.spawn(async move |cx| {
                cx.background_executor().timer(CORE_APPLY_AFTER).await;
                cx.update(|cx| {
                    let latest = settings.read(cx).get().clone();
                    meter.apply_config(latest.backend_config(meter.current_config()));
                });
            });
            drop(core_apply.replace(apply));
        }
        applied = next;
    })
    .detach();
}

/// The config the core should run with after `before` became `after`, built
/// on the `running` one so what the settings do not own (the PvP switch, the
/// intervals) survives. `None` when no option the core reads changed. The
/// capture backend priority is read only when the meter starts, so changing
/// it alone is not a change for the running core.
fn core_update(before: &AppSettings, after: &AppSettings, running: DpsMeterConfig) -> Option<DpsMeterConfig> {
    let changed = before.behaviour != after.behaviour
        || before.overlay.players_shown != after.overlay.players_shown
        || before.capture.record_unfamiliar_servers != after.capture.record_unfamiliar_servers
        || before.capture.collect_mob_data != after.capture.collect_mob_data;
    changed.then(|| after.backend_config(running))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::DetailPlacement;
    use aether_core::config::CaptureBackendPriority;

    fn running() -> DpsMeterConfig {
        DpsMeterConfig { pvp_mode_on: true, dps_snapshot_interval_ms: 200, ..DpsMeterConfig::default() }
    }

    #[test]
    fn a_counting_change_reaches_the_core_and_keeps_what_settings_do_not_own() {
        let before = AppSettings::default();
        let mut after = before.clone();
        after.behaviour.training_dummy_only = true;
        after.overlay.players_shown = 12;

        let config = core_update(&before, &after, running()).expect("the core is told");

        assert!(config.my_muzhuang_only);
        assert_eq!(config.max_player_count, 12);
        assert!(config.pvp_mode_on);
        assert_eq!(config.dps_snapshot_interval_ms, 200);
    }

    #[test]
    fn look_log_placement_and_backend_priority_changes_leave_the_core_alone() {
        let before = AppSettings::default();
        let mut after = before.clone();
        after.overlay.background_alpha = 10;
        after.capture.decision_log = true;
        after.windows.detail_opens = DetailPlacement::Centred;
        after.capture.backend_priority = CaptureBackendPriority::WinDivertFirst;

        assert!(core_update(&before, &after, running()).is_none());
    }
}
