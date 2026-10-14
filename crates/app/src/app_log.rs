//! Where the app's own diagnostics go. The shipped exe has no console, so
//! `eprintln!` reaches nobody: failures are written to the log file the core
//! writes to (`belisk.log`), through the one logger `run` installs and hands
//! to the meter.
//!
//! A write happens on the calling thread, so these are for failures and
//! once-per-event notes, never for anything that repeats per packet or frame.
//! Before `install` (and in unit tests) a line is dropped; a `probes` build
//! also echoes each line to its console.

use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
        if log::set_boxed_logger(Box::new(Forward::default())).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
        Arc::new(AppLogger::new(path, Box::new(|_| {})))
    }))
}

/// GPUI and the crates under it report their failures through the `log`
/// crate, which had no logger, so they went nowhere. Their warnings and
/// errors go to the log file now; debug and info are not even formatted.
#[derive(Default)]
struct Forward {
    gate: Mutex<Gate>,
}

impl log::Log for Forward {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = Line { target: record.target(), file: record.file(), line: record.line(), args: record.args() };
        let pass = self.lock_gate().admit(&line, Instant::now());
        let Pass::Write { line, repeated, dropped } = pass else {
            return;
        };
        report_counts(repeated, dropped);
        if is_benign(record.target(), &line) {
            return;
        }
        match record.level() {
            log::Level::Error => error(line),
            _ => warn(line),
        }
    }

    fn flush(&self) {
        let (repeated, dropped) = self.lock_gate().take_counts();
        report_counts(repeated, dropped);
    }
}

impl Forward {
    fn lock_gate(&self) -> std::sync::MutexGuard<'_, Gate> {
        self.gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn report_counts(repeated: u32, dropped: u32) {
    if repeated > 0 {
        info(format!("(the previous GPUI message came {repeated} more times)"));
    }
    if dropped > 0 {
        info(format!("({dropped} GPUI messages were left out: more than {GATE_LINES} a minute)"));
    }
}

/// What GPUI says at every launch on a PC without the Windows graphics
/// debug layer, which is nearly every PC: the DXGI debug interface is not
/// there, and GPUI goes on without it. The HRESULT's text follows the
/// Windows language, so its code is what is matched.
fn is_benign(target: &str, line: &str) -> bool {
    let missing_debug_layer = target.is_empty() && line.ends_with("(0x887A002D)");
    let no_debug_interface = target == "gpui_windows::directx_devices" && line.contains("Failed to get DXGI debug interface");
    missing_debug_layer || no_debug_interface
}

/// A forwarded record as the log writes it, built only for a line written.
/// GPUI sets the target of a record from its own source path, so one raised
/// by code from a registry has none and the file and line say where it came
/// from instead.
struct Line<'a> {
    target: &'a str,
    file: Option<&'a str>,
    line: Option<u32>,
    args: &'a fmt::Arguments<'a>,
}

impl fmt::Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.target, self.file) {
            ("", Some(file)) => {
                write!(f, "[{file}")?;
                if let Some(line) = self.line {
                    write!(f, ":{line}")?;
                }
                write!(f, "] {}", self.args)
            }
            _ => write!(f, "[{}] {}", self.target, self.args),
        }
    }
}

/// Whether `line` reads exactly `text`, without building it: the
/// comparison stops at the first difference.
fn reads_as(line: &impl fmt::Display, text: &str) -> bool {
    struct Rest<'a>(&'a str);
    impl fmt::Write for Rest<'_> {
        fn write_str(&mut self, part: &str) -> fmt::Result {
            self.0 = self.0.strip_prefix(part).ok_or(fmt::Error)?;
            Ok(())
        }
    }
    let mut rest = Rest(text);
    fmt::write(&mut rest, format_args!("{line}")).is_ok() && rest.0.is_empty()
}

/// At most this many forwarded lines a minute, so a failure repeated each
/// frame cannot fill the file.
const GATE_LINES: u32 = 20;
const GATE_WINDOW: Duration = Duration::from_secs(60);

/// Which forwarded lines are written: a line just like the one before is
/// counted instead, and past [`GATE_LINES`] in a minute the rest are
/// counted until the next minute.
#[derive(Default)]
struct Gate {
    last: Option<String>,
    repeated: u32,
    window: Option<Instant>,
    written: u32,
    dropped: u32,
}

#[derive(Debug, PartialEq, Eq)]
enum Pass {
    Write { line: String, repeated: u32, dropped: u32 },
    Skip,
}

impl Gate {
    fn admit(&mut self, line: &impl fmt::Display, now: Instant) -> Pass {
        if self.last.as_deref().is_some_and(|last| reads_as(line, last)) {
            self.repeated += 1;
            return Pass::Skip;
        }
        if self.window.is_none_or(|start| now.duration_since(start) >= GATE_WINDOW) {
            self.window = Some(now);
            self.written = 0;
        }
        if self.written == GATE_LINES {
            self.dropped += 1;
            return Pass::Skip;
        }
        self.written += 1;
        let line = line.to_string();
        self.last = Some(line.clone());
        let (repeated, dropped) = self.take_counts();
        Pass::Write { line, repeated, dropped }
    }

    /// The lines counted and not yet told, which are then told.
    fn take_counts(&mut self) -> (u32, u32) {
        (std::mem::take(&mut self.repeated), std::mem::take(&mut self.dropped))
    }
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

    #[test]
    fn gpui_messages_repeated_or_past_twenty_a_minute_are_counted_not_written() {
        let mut gate = Gate::default();
        let start = Instant::now();
        let written = |line: &str, repeated, dropped| Pass::Write { line: line.to_string(), repeated, dropped };
        assert_eq!(gate.admit(&"a", start), written("a", 0, 0));
        assert_eq!(gate.admit(&"a", start), Pass::Skip);
        assert_eq!(gate.admit(&"a", start), Pass::Skip);
        assert_eq!(gate.admit(&"b", start), written("b", 2, 0), "the repeats are told with the next line");
        for index in 2..GATE_LINES {
            assert!(matches!(gate.admit(&format!("line {index}"), start), Pass::Write { .. }));
        }
        assert_eq!(gate.admit(&"one too many", start), Pass::Skip);
        assert_eq!(gate.admit(&"two too many", start + Duration::from_secs(30)), Pass::Skip);
        assert_eq!(gate.admit(&"next minute", start + GATE_WINDOW), written("next minute", 0, 2));
    }

    #[test]
    fn the_repeats_and_left_out_lines_not_yet_told_are_taken_once_for_flush() {
        let mut gate = Gate::default();
        let start = Instant::now();
        gate.admit(&"a", start);
        gate.admit(&"a", start);
        gate.admit(&"a", start);
        assert_eq!(gate.take_counts(), (2, 0));
        assert_eq!(gate.take_counts(), (0, 0), "told once");
    }

    #[test]
    fn a_record_without_a_target_says_the_file_and_line_it_came_from() {
        let args = format_args!("RefCell already borrowed");
        let from_registry = Line { target: "", file: Some("gpui-pre-0.3.7/src/app/async_context.rs"), line: Some(93), args: &args };
        assert_eq!(from_registry.to_string(), "[gpui-pre-0.3.7/src/app/async_context.rs:93] RefCell already borrowed");

        let named = Line { target: "gpui_windows::directx_devices", file: Some("x.rs"), line: Some(1), args: &args };
        assert_eq!(named.to_string(), "[gpui_windows::directx_devices] RefCell already borrowed");

        let nowhere = Line { target: "", file: None, line: None, args: &args };
        assert_eq!(nowhere.to_string(), "[] RefCell already borrowed");
    }

    #[test]
    fn only_the_two_missing_debug_layer_lines_are_left_out_of_the_log() {
        assert!(is_benign("", "[a/b.rs:1] The application requested an operation that depends on an SDK component that is missing or mismatched. (0x887A002D)"));
        assert!(is_benign("", "[a/b.rs:1] L'application a demandé une opération qui dépend d'un composant SDK absent. (0x887A002D)"));
        assert!(is_benign(
            "gpui_windows::directx_devices",
            "[gpui_windows::directx_devices] Failed to get DXGI debug interface. DirectX debugging features will be disabled."
        ));
        assert!(!is_benign("", "[a/b.rs:1] RefCell already borrowed"));
        assert!(!is_benign("gpui_windows::directx_devices", "[gpui_windows::directx_devices] Failed to create the device (0x887A0005)"));
        assert!(!is_benign("wgpu", "[wgpu] something (0x887A002D)"));
    }

    /// A line in two parts that counts the parts a formatter took.
    struct Parts<'a> {
        parts: [&'a str; 2],
        taken: &'a std::cell::Cell<usize>,
    }

    impl fmt::Display for Parts<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for part in self.parts {
                f.write_str(part)?;
                self.taken.set(self.taken.get() + 1);
            }
            Ok(())
        }
    }

    #[test]
    fn a_message_left_out_is_never_built() {
        let mut gate = Gate::default();
        let start = Instant::now();
        for index in 0..GATE_LINES {
            gate.admit(&format!("[gpui] line {index}"), start);
        }
        let taken = std::cell::Cell::new(0);
        let left_out = Parts { parts: ["[wgpu] ", "a long report"], taken: &taken };
        assert_eq!(gate.admit(&left_out, start), Pass::Skip);
        assert_eq!(taken.get(), 0, "compared with the last line up to its first difference, and no further");

        let repeat = Parts { parts: ["[gpui] ", "line 19"], taken: &taken };
        assert_eq!(gate.admit(&repeat, start), Pass::Skip, "the last line again is a repeat");
    }

    #[test]
    fn a_line_reads_as_a_text_only_when_it_is_that_text() {
        assert!(reads_as(&"[gpui] a", "[gpui] a"));
        assert!(!reads_as(&"[gpui] a", "[gpui] ab"), "a shorter line");
        assert!(!reads_as(&"[gpui] ab", "[gpui] a"), "a longer line");
        assert!(!reads_as(&"[gpui] b", "[gpui] a"));
    }
}
