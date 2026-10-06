mod args;
mod capture;
mod clock;
mod commands;
mod fields;
mod output;

use std::process::ExitCode;

fn main() -> ExitCode {
    let result = args::Args::parse(std::env::args().skip(1)).and_then(|args| commands::run(&args));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("packet-explorer: {message}");
            ExitCode::FAILURE
        }
    }
}
