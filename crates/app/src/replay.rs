use std::path::Path;
use std::sync::Arc;

use meter_core::engine::meter::DpsMeter;
use meter_core::events::EventSink;
use meter_core::logger::AppLogger;

/// Starts the meter without a capture driver and feeds it `recording` at
/// `speed` times the pace it was captured at.
pub fn start(
    events: Arc<dyn EventSink>,
    recording: &Path,
    speed: f64,
) -> Result<Arc<DpsMeter>, String> {
    let log_path = std::env::temp_dir().join("aether").join("gpui-app.log");
    let logger = Arc::new(AppLogger::new(log_path, Box::new(|_| {})));
    let meter = Arc::new(DpsMeter::new(events, logger, None));

    let mut config = meter.current_config();
    config.idle_reset_secs = 0;
    config.auto_record_unknown_server = false;
    config.hide_unknown_players = false;
    config.show_possible_boss = true;
    meter.apply_config(config);

    meter.start_for_replay()?;
    meter.replay_packet_recording_paced(recording.to_string_lossy().into_owned(), speed)?;
    Ok(meter)
}
