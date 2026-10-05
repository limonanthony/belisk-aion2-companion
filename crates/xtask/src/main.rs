mod crates;
mod dist;
mod exe_check;
mod notices;
mod spdx;

use std::path::Path;
use std::process::ExitCode;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

const USAGE: &str = "usage: cargo xtask dist [--allow-dirty] [--no-source [--source-contact <text>]]

dist   build the release exe with the static CRT and package target/dist/aion-2-dps-meter-<version>-windows-x64.zip
       --allow-dirty    package although the working tree has uncommitted changes (the source zip will not contain them)
       --no-source      leave the source zip out and put a GPL-3.0 section 6(b) written offer in README.txt instead;
                        writes target/dist/no-source/aion-2-dps-meter-<version>-windows-x64.zip
       --source-contact who to ask for the source (default: \"the person who gave you this copy\")";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("xtask is not two levels below the workspace root")?;
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("dist") => {
            let mut allow_dirty = false;
            let mut no_source = false;
            let mut contact = None;
            while let Some(argument) = args.next() {
                match argument.as_str() {
                    "--allow-dirty" => allow_dirty = true,
                    "--no-source" => no_source = true,
                    "--source-contact" => {
                        contact = Some(args.next().ok_or_else(|| format!("--source-contact needs a value\n{USAGE}"))?)
                    }
                    other => return Err(format!("unknown option {other}\n{USAGE}").into()),
                }
            }
            let source = match (no_source, contact) {
                (false, None) => dist::SourceDelivery::Bundled,
                (false, Some(_)) => return Err(format!("--source-contact only applies with --no-source\n{USAGE}").into()),
                (true, contact) => dist::SourceDelivery::WrittenOffer {
                    contact: contact.unwrap_or_else(|| "the person who gave you this copy".to_owned()),
                },
            };
            dist::run(root, &dist::Options { allow_dirty, source })
        }
        _ => Err(USAGE.into()),
    }
}
