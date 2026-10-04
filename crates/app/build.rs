//! Windows build steps: embeds the manifest and icon in the binary, and copies
//! the WinDivert driver files beside it.
//!
//! Capture needs Administrator, so the shipped binary asks for it; a build with
//! the `probes` feature asks for nothing, so the development checks run from an
//! ordinary terminal. The binary imports WinDivert.dll at load time, so the DLL
//! (and the driver it installs) must sit in the same folder as the exe.

use std::path::{Path, PathBuf};

const WINDIVERT_FILES: [&str; 2] = ["WinDivert.dll", "WinDivert64.sys"];

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    embed_manifest_and_icon();
    copy_windivert_beside_the_binary();
}

fn embed_manifest_and_icon() {
    let resource = if std::env::var_os("CARGO_FEATURE_PROBES").is_some() {
        "windows/unelevated.rc"
    } else {
        "windows/admin.rc"
    };
    println!("cargo:rerun-if-changed=windows");
    embed_resource::compile(resource, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}

fn copy_windivert_beside_the_binary() {
    println!("cargo:rerun-if-changed=windivert");
    let source_dir = Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("windivert");
    let profile_dir = profile_dir();
    for name in WINDIVERT_FILES {
        let (source, destination) = (source_dir.join(name), profile_dir.join(name));
        println!("cargo:rerun-if-changed={}", destination.display());
        if std::fs::read(&destination).ok() == std::fs::read(&source).ok() {
            continue;
        }
        std::fs::copy(&source, &destination).unwrap_or_else(|error| {
            panic!(
                "copying {} to {} failed: {error}",
                source.display(),
                destination.display()
            )
        });
    }
}

/// `target/<profile>`: the folder the exe lands in, three levels above OUT_DIR
/// (`target/<profile>/build/<package>-<hash>/out`).
fn profile_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR is nested under target/<profile>/build")
        .to_path_buf()
}
