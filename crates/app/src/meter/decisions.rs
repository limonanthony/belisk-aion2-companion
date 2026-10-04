//! Diagnostic lines for what the meter decided: which board, which boss, who
//! "you" are. They go to the core's log, but only after `enable` (the app's
//! `--debug-log` flag), and a state is written only when it changes, never per
//! snapshot. The switch is the app's own: the core's debug level also writes a
//! line for every hit and buff on the packet thread, and stays off. Lines may
//! hold player names: the log stays on the player's machine.

use std::collections::HashMap;
use std::sync::Arc;

use aether_core::logger::AppLogger;

#[derive(Default)]
pub struct DecisionLog {
    logger: Option<Arc<AppLogger>>,
    enabled: bool,
    last: HashMap<&'static str, String>,
}

impl DecisionLog {
    pub fn new(logger: Option<Arc<AppLogger>>) -> Self {
        Self { logger, enabled: false, last: HashMap::new() }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn enabled(&self) -> bool {
        self.enabled && self.logger.is_some()
    }

    /// Something happened. The text is built only when logging is on.
    pub fn event(&self, topic: &'static str, text: impl FnOnce() -> String) {
        if let Some(logger) = self.logger.as_ref().filter(|_| self.enabled) {
            logger.info(format!("[meter] {topic}: {}", text()));
        }
    }

    /// The state of `topic` as it is now; written only if it differs from the
    /// last one written.
    pub fn on_change(&mut self, topic: &'static str, state: impl FnOnce() -> String) {
        let Some(logger) = self.logger.as_ref().filter(|_| self.enabled) else {
            return;
        };
        let state = state();
        if self.last.get(topic) == Some(&state) {
            return;
        }
        logger.info(format!("[meter] {topic}: {state}"));
        self.last.insert(topic, state);
    }

    /// Forget what was written, so the next state is written again.
    pub fn forget(&mut self, topic: &'static str) {
        self.last.remove(topic);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    fn logger(name: &str) -> (Arc<AppLogger>, Arc<Mutex<Vec<String>>>) {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&lines);
        let path = std::env::temp_dir().join(format!("aether-decisions-{}-{name}.log", std::process::id()));
        let logger = Arc::new(AppLogger::new(
            path,
            Box::new(move |event| sink.lock().unwrap().push(event.message)),
        ));
        (logger, lines)
    }

    #[test]
    fn nothing_is_built_or_written_until_enabled() {
        let (logger, lines) = logger("off");
        let mut log = DecisionLog::new(Some(logger));

        log.on_change("board", || panic!("state built while off"));
        log.event("focus", || panic!("text built while off"));

        assert!(lines.lock().unwrap().is_empty());
        assert!(!log.enabled());
    }

    #[test]
    fn the_cores_per_packet_debug_level_is_not_needed_and_not_turned_on() {
        let (logger, lines) = logger("core");
        let mut log = DecisionLog::new(Some(Arc::clone(&logger)));
        log.enable();

        log.on_change("board", || "Session".to_string());

        assert_eq!(*lines.lock().unwrap(), vec!["[meter] board: Session"]);
        assert!(!logger.is_debug_enabled());
    }

    #[test]
    fn a_state_is_written_once_until_it_changes() {
        let (logger, lines) = logger("change");
        let mut log = DecisionLog::new(Some(logger));
        log.enable();

        log.on_change("board", || "Session".to_string());
        log.on_change("board", || "Session".to_string());
        log.on_change("board", || "Target".to_string());
        log.on_change("you", || "Session".to_string());

        assert_eq!(
            *lines.lock().unwrap(),
            vec!["[meter] board: Session", "[meter] board: Target", "[meter] you: Session"]
        );
    }

    #[test]
    fn a_meter_without_a_logger_logs_nothing() {
        let mut log = DecisionLog::default();
        log.enable();

        log.on_change("board", || panic!("state built without a logger"));
        log.event("focus", || panic!("text built without a logger"));
    }
}
