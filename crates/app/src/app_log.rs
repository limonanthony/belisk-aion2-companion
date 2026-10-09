//! Where the app's own diagnostics go. The shipped exe has no console, so
//! `eprintln!` reaches nobody: failures are written to the log file the core
//! writes to (`belisk.log`), through the one logger `run` installs and hands
//! to the meter.
//!
//! A write happens on the calling thread, so these are for failures and
//! once-per-event notes, never for anything that repeats per packet or frame.
//! Before `install` (and in unit tests) a line is dropped; a `probes` build
//! also echoes each line to its console.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use meter_core::logger::AppLogger;

use crate::data_folder::log_dir;

static LOGGER: OnceLock<Arc<AppLogger>> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Installs the app's logger on first call and returns it. A live run, after
/// `data_folder::settle`, writes to `belisk.log`; replays, captures and the
/// `--apply-update` helper write to a file in the temp folder and never create
/// the player's log folder.
pub fn install(live: bool) -> Arc<AppLogger> {
    Arc::clone(LOGGER.get_or_init(|| {
        let path = LOG_PATH.get_or_init(|| path(live)).clone();
        Arc::new(AppLogger::new(path, Box::new(|_| {})))
    }))
}

/// Appends a panic's report to the log file without the logger's lock: the
/// panicking thread may hold that lock, and a hook that waits for it hangs
/// the process instead of recording why it died. Dropped before `install`.
pub fn record_panic(report: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs_f64()).unwrap_or_default();
    let _ = writeln!(file, "[{timestamp:.3}] [ERROR] {report}");
}

fn path(live: bool) -> PathBuf {
    match log_dir().filter(|_| live) {
        Some(dir) => dir.join("belisk.log"),
        None => std::env::temp_dir().join("belisk").join("gpui-app.log"),
    }
}

pub fn info(message: impl AsRef<str>) {
    echo("INFO", message.as_ref());
    if let Some(logger) = LOGGER.get() {
        logger.info(message);
    }
}

pub fn error(message: impl AsRef<str>) {
    echo("ERROR", message.as_ref());
    if let Some(logger) = LOGGER.get() {
        logger.error(message);
    }
}

pub fn warn(message: impl AsRef<str>) {
    echo("WARN", message.as_ref());
    if let Some(logger) = LOGGER.get() {
        logger.warn(message);
    }
}

#[cfg(feature = "probes")]
fn echo(level: &str, message: &str) {
    eprintln!("[{level}] {message}");
}

#[cfg(not(feature = "probes"))]
fn echo(_level: &str, _message: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_that_never_settled_the_folders_logs_to_the_temp_folder() {
        let expected = std::env::temp_dir().join("belisk").join("gpui-app.log");

        assert_eq!(path(false), expected);
        assert_eq!(path(true), expected, "a live run before data_folder::settle has no player log folder");
    }
}
