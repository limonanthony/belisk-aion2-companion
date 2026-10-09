//! A panic writes its message, thread, location and backtrace to the log
//! file before the process goes on to die. A panic that cannot unwind (one
//! inside a window procedure) aborts with no message, so without this hook
//! the log would end at the last line written before it.

use std::backtrace::Backtrace;
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};

use crate::app_log;

/// Keeps the previous hook, which still prints the default message.
pub fn install() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        record(info);
        previous(info);
    }));
}

/// Must not panic, since a panic inside a panic hook aborts at once.
fn record(info: &PanicHookInfo<'_>) {
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        let thread = std::thread::current();
        let location = info.location().map_or_else(|| "an unknown location".to_string(), ToString::to_string);
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("a payload that is not text");
        let backtrace = Backtrace::force_capture();
        app_log::record_panic(&format!(
            "panic in thread '{}' at {location}: {message}\n{backtrace}",
            thread.name().unwrap_or("unnamed")
        ));
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_reaches_the_log_with_its_message_thread_and_location() {
        app_log::install(false);
        install();

        let panicked = std::thread::Builder::new()
            .name("panic-probe".to_string())
            .spawn(|| {
                let lease = std::cell::RefCell::new(());
                let _held = lease.borrow_mut();
                let _again = lease.borrow_mut();
            })
            .expect("a thread")
            .join();
        assert!(panicked.is_err(), "the probe thread did not panic");

        let log = std::fs::read_to_string(std::env::temp_dir().join("belisk").join("gpui-app.log")).expect("the log file");
        let report = log
            .lines()
            .rev()
            .find(|line| line.contains("panic in thread 'panic-probe'"))
            .expect("the panic's line in the log");
        assert!(report.contains("[ERROR]"), "{report}");
        assert!(report.contains("panic_log.rs"), "the location: {report}");
        assert!(report.contains("already borrowed"), "the message: {report}");
    }
}
