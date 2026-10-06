use std::io::Write;

use serde::Serialize;

/// Writes one line to stdout; a closed pipe (`| head`) ends the program quietly.
#[macro_export]
macro_rules! out {
    () => { $crate::output::write_line("") };
    ($($arg:tt)*) => { $crate::output::write_line(&format!($($arg)*)) };
}

pub fn write_line(line: &str) {
    if writeln!(std::io::stdout().lock(), "{line}").is_err() {
        std::process::exit(0);
    }
}

pub fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|error| format!("cannot write JSON: {error}"))?;
    write_line(&text);
    Ok(())
}
