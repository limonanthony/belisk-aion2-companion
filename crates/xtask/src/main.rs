mod crates;
mod dist;
mod exe_check;
mod notices;
mod spdx;
mod update;

use std::path::Path;
use std::process::ExitCode;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

const USAGE: &str = "usage: cargo xtask dist [--allow-dirty] [--allow-development-key] [--no-source [--source-contact <text>]]
       cargo xtask keygen --out <path outside the repository>
       cargo xtask sign --key <private key file> <belisk-<version>-windows-x64-update.zip>

dist   build the release exe with the static CRT and package target/dist/belisk-<version>-windows-x64.zip
       and, beside it, the lean update archive belisk-<version>-windows-x64-update.zip with its .sha256
       --allow-dirty    package although the working tree has uncommitted changes (the source zip will not contain them)
       --allow-development-key
                        package although crates/app/update-key.pub is still the DEVELOPMENT placeholder. For local testing
                        only: never in CI, never for a build players get (its updater stays off with that key)
       --no-source      leave the source zip out and put a GPL-3.0 section 6(b) written offer in README.txt instead;
                        writes target/dist/no-source/belisk-<version>-windows-x64.zip
       --source-contact who to ask for the source (default: \"the person who gave you this copy\")
keygen make an Ed25519 update signing key pair: <path> (private) and <path>.pub (copy it over crates/app/update-key.pub)
sign   write <file>.sig, the signature the app checks before it installs an update (docs/updates.md); fails, writing nothing,
       when the key is not the private half of crates/app/update-key.pub";

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
            let mut allow_development_key = false;
            let mut no_source = false;
            let mut contact = None;
            while let Some(argument) = args.next() {
                match argument.as_str() {
                    "--allow-dirty" => allow_dirty = true,
                    "--allow-development-key" => allow_development_key = true,
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
            dist::run(root, &dist::Options { allow_dirty, allow_development_key, source })
        }
        Some("keygen") => {
            let (Some(flag), Some(out), None) = (args.next(), args.next(), args.next()) else {
                return Err(format!("keygen needs --out <path>\n{USAGE}").into());
            };
            if flag != "--out" {
                return Err(format!("unknown option {flag}\n{USAGE}").into());
            }
            update::keygen(root, Path::new(&out))
        }
        Some("sign") => {
            let rest: Vec<String> = args.collect();
            let [flag, key, archive] = rest.as_slice() else {
                return Err(format!("sign needs --key <path> <file>\n{USAGE}").into());
            };
            if flag != "--key" {
                return Err(format!("unknown option {flag}\n{USAGE}").into());
            }
            update::sign(root, Path::new(key), Path::new(archive))
        }
        _ => Err(USAGE.into()),
    }
}
