mod crates;
mod dist;
mod exe_check;
mod notices;
mod spdx;

use std::path::Path;
use std::process::ExitCode;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

const USAGE: &str = "usage: cargo xtask dist [--allow-dirty]

dist   build the release exe with the static CRT and package target/dist/aion-2-dps-meter-<version>-windows-x64.zip
       --allow-dirty   package although the working tree has uncommitted changes (the source zip will not contain them)";

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
            let mut options = dist::Options { allow_dirty: false };
            for argument in args {
                match argument.as_str() {
                    "--allow-dirty" => options.allow_dirty = true,
                    other => return Err(format!("unknown option {other}\n{USAGE}").into()),
                }
            }
            dist::run(root, &options)
        }
        _ => Err(USAGE.into()),
    }
}
