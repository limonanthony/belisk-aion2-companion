//! Live capture. The data and log folders keep the identifier of the Tauri app
//! this one replaced, so existing history, personal bests and recordings are
//! reused.

use std::path::PathBuf;
use std::sync::Arc;

use aether_core::config::DpsMeterConfig;
use aether_core::engine::meter::DpsMeter;
use aether_core::events::EventSink;
use aether_core::logger::AppLogger;

/// Bundle identifier of the app this one replaced; it names the data folders.
const IDENTIFIER: &str = "com.helveticxa.aether";

/// `%APPDATA%/<identifier>`, where the Tauri app kept its data.
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join(IDENTIFIER))
}

/// `%LOCALAPPDATA%/<identifier>/logs`, where the Tauri app kept its log.
fn log_file() -> PathBuf {
    match std::env::var_os("LOCALAPPDATA") {
        Some(dir) => PathBuf::from(dir).join(IDENTIFIER).join("logs").join("aether.log"),
        None => std::env::temp_dir().join("aether").join("aether.log"),
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

/// The defaults plus what the GPUI meter decided on: the "possible
/// boss" HP rule is always on, and mob collection follows the launch flag.
pub fn gpui_backend_config(collect_mobs: bool) -> DpsMeterConfig {
    DpsMeterConfig {
        show_possible_boss: true,
        collect_mobs,
        ..default_backend_config()
    }
}

/// Creates the meter and starts capture, Npcap first with WinDivert as the
/// fallback. The meter is returned even when capture fails so the caller can
/// report why.
pub fn start(
    events: Arc<dyn EventSink>,
    collect_mobs: bool,
) -> (Arc<DpsMeter>, Result<(), String>) {
    let logger = Arc::new(AppLogger::new(log_file(), Box::new(|_| {})));
    let meter = Arc::new(DpsMeter::new(events, logger, data_dir()));
    meter.apply_config(gpui_backend_config(collect_mobs));
    let started = meter.start_dps_meter();
    (meter, started)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_core::config::CaptureBackendPriority;

    #[test]
    fn backend_config_matches_the_defaults_of_the_previous_app() {
        let config = default_backend_config();
        assert_eq!(config.dps_snapshot_interval_ms, 200);
        assert_eq!(config.memory_snapshot_interval_ms, 2000);
        assert_eq!(config.max_packet_size_threshold, 8192);
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
    fn the_gpui_meter_shows_possible_bosses_and_collects_mobs_only_on_request() {
        assert!(gpui_backend_config(false).show_possible_boss);
        assert!(!gpui_backend_config(false).collect_mobs);
        assert!(gpui_backend_config(true).collect_mobs);
        assert!(!gpui_backend_config(false).output_debug_log, "the core's per-packet debug stays off");
        assert!(!default_backend_config().show_possible_boss && !default_backend_config().collect_mobs);
    }
}
