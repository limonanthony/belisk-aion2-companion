//! Build steps: lists the skill icons to compile into the binary, and on
//! Windows embeds the manifest, the icon and the version information in it
//! and copies the WinDivert driver files beside it.
//!
//! Capture needs Administrator, so the shipped binary asks for it; a build with
//! the `probes` feature asks for nothing, so the development checks run from an
//! ordinary terminal. The binary delay-loads WinDivert.dll: with Npcap the meter
//! starts without the DLL and the driver beside it, which only the fallback needs.

use std::path::{Path, PathBuf};

const WINDIVERT_FILES: [&str; 2] = ["WinDivert.dll", "WinDivert64.sys"];

fn main() {
    list_skill_icons();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    embed_manifest_and_icon();
    copy_windivert_beside_the_binary();
    delay_load_windivert();
}

/// WinDivert.dll only matters when Npcap cannot capture, so the exe must start
/// without it. The guard in meter-core's `windivert_capturer` checks the DLL
/// loads before the first WinDivert call.
fn delay_load_windivert() {
    println!("cargo:rustc-link-arg-bins=/DELAYLOAD:WinDivert.dll");
    println!("cargo:rustc-link-arg-bins=delayimp.lib");
}

/// Writes `skill_icons.rs` into OUT_DIR: one `("skills/<file>", bytes)` entry
/// per PNG in `assets/game/skills`, so a new icon needs no code change.
fn list_skill_icons() {
    let root = Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/game/skills");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files: Vec<String> = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", root.display()))
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    let entries: String = files
        .iter()
        .map(|name| {
            let path = root.join(name);
            format!("    (\"skills/{name}\", include_bytes!({:?})),\n", path.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("skill_icons.rs");
    std::fs::write(&out, format!("&[\n{entries}]\n"))
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", out.display()));
}

fn embed_manifest_and_icon() {
    let resource = if std::env::var_os("CARGO_FEATURE_PROBES").is_some() {
        "windows/unelevated.rc"
    } else {
        "windows/admin.rc"
    };
    println!("cargo:rerun-if-changed=windows");
    let version = |part: &str| std::env::var(format!("CARGO_PKG_VERSION_{part}")).unwrap();
    let numbers = format!("{},{},{},0", version("MAJOR"), version("MINOR"), version("PATCH"));
    let text = format!("{}.{}.{}", version("MAJOR"), version("MINOR"), version("PATCH"));
    let macros = [format!("BELISK_VERSION_NUMBERS={numbers}"), format!("BELISK_VERSION_TEXT={text}")];
    embed_resource::compile(resource, macros)
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
