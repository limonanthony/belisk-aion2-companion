//! Live capture. The data and log folders keep the identifier of the Tauri app
//! this one replaced, so existing history, personal bests and recordings are
//! reused.

use std::path::PathBuf;
use std::sync::Arc;

use meter_core::config::DpsMeterConfig;
use meter_core::engine::meter::DpsMeter;
use meter_core::events::EventSink;
use meter_core::logger::AppLogger;

use crate::settings::AppSettings;

/// Bundle identifier of the app this one replaced; it names the data folders.
const IDENTIFIER: &str = "com.helveticxa.aether";

/// `%APPDATA%/<identifier>`, where the Tauri app kept its data and where
/// `settings.json` lives.
pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join(IDENTIFIER))
}

/// `%LOCALAPPDATA%/<identifier>/logs`, where the Tauri app kept its log.
pub fn log_dir() -> PathBuf {
    match std::env::var_os("LOCALAPPDATA") {
        Some(dir) => PathBuf::from(dir).join(IDENTIFIER).join("logs"),
        None => std::env::temp_dir().join("aether"),
    }
}

/// The backend settings Aether applied on a fresh install, as of `tauri-final`.
/// Only two differ from the core's own defaults.
pub fn default_backend_config() -> DpsMeterConfig {
    DpsMeterConfig {
        dps_snapshot_interval_ms: 200,
        unknown_packet_stall_resync_delay_ms: 50,
        ..DpsMeterConfig::default()
    }
}

/// The core's config for these settings, on top of what the previous app
/// applied on a fresh install.
pub fn backend_config(settings: &AppSettings) -> DpsMeterConfig {
    settings.backend_config(default_backend_config())
}

/// Creates the meter and starts capture with the backend priority and the
/// other options of `settings`; Npcap first by default, with WinDivert as the
/// fallback. The meter is returned even when capture fails so the caller can
/// report why. `track_fights` is whether the meter window is open: it is set
/// before capture starts, so a closed meter never counts a fight.
pub fn start(events: Arc<dyn EventSink>, settings: &AppSettings, track_fights: bool) -> (Arc<DpsMeter>, Result<(), String>) {
    let logger = Arc::new(AppLogger::new(log_dir().join("aether.log"), Box::new(|_| {})));
    let meter = Arc::new(DpsMeter::new(events, logger, data_dir()));
    meter.apply_config(backend_config(settings));
    meter.set_known_characters(settings.known_characters.clone());
    meter.set_fight_tracking(track_fights);
    let started = meter.start_dps_meter();
    (meter, started)
}

#[cfg(test)]
mod tests {
    use super::*;
    use meter_core::config::CaptureBackendPriority;

    #[test]
    fn backend_config_matches_the_defaults_of_the_previous_app() {
        let config = default_backend_config();
        assert_eq!(config.dps_snapshot_interval_ms, 200);
        assert_eq!(config.memory_snapshot_interval_ms, 2000);
        assert_eq!(config.stall_resync_delay_ms, 1000);
        assert_eq!(config.full_processor_stall_resync_delay_ms, 200);
        assert_eq!(config.unknown_packet_stall_resync_delay_ms, 50);
        assert!(!config.boss_only && !config.pvp_mode_on && !config.show_possible_boss);
        assert!(!config.my_muzhuang_only && !config.hide_unknown_players);
        assert_eq!(config.max_player_count, 10);
        assert_eq!(config.capture_backend_priority, CaptureBackendPriority::NpcapFirst);
        assert!(config.hide_when_idle && config.auto_record_unknown_server);
        assert_eq!(config.idle_reset_secs, 300);
    }

    #[test]
    fn the_settings_decide_what_the_meter_starts_with() {
        let defaults = backend_config(&AppSettings::default());
        assert!(defaults.show_possible_boss);
        assert!(!defaults.collect_mobs);
        assert!(!defaults.output_debug_log, "the core's per-packet debug stays off");
        assert_eq!(defaults.dps_snapshot_interval_ms, 200);

        let mut settings = AppSettings::default();
        settings.capture.collect_mob_data = true;
        settings.capture.backend_priority = CaptureBackendPriority::WinDivertFirst;
        let config = backend_config(&settings);
        assert!(config.collect_mobs);
        assert_eq!(config.capture_backend_priority, CaptureBackendPriority::WinDivertFirst);
        assert!(!default_backend_config().show_possible_boss && !default_backend_config().collect_mobs);
    }
}
