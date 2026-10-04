use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::crates::Metadata;
use crate::{exe_check, notices, Result, WINDOWS_TARGET};

const PACKAGE: &str = "aion-2-dps-meter";
const FOLDER: &str = "aion-2-dps-meter";

/// Official WinDivert-2.2.2-A.zip, x64 (see FORK.md, "WinDivert files").
const WINDIVERT_FILES: [(&str, &str); 2] = [
    ("WinDivert.dll", "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2"),
    ("WinDivert64.sys", "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2"),
];

pub struct Options {
    pub allow_dirty: bool,
}

pub fn run(root: &Path, options: &Options) -> Result<()> {
    let metadata = Metadata::read(root)?;
    let version = &metadata.version;
    refuse_dirty_tree(root, options)?;
    let windivert = read_verified_windivert(root)?;

    let exe_path = metadata
        .target_directory
        .join(WINDOWS_TARGET)
        .join("release")
        .join(format!("{PACKAGE}.exe"));
    refuse_locked_exe(&exe_path)?;
    build_release_with_static_crt(root)?;
    refuse_locked_exe(&exe_path)?;
    let exe = fs::read(&exe_path).map_err(|error| format!("cannot read {}: {error}", exe_path.display()))?;
    exe_check::verify_shipping_exe(&exe)?;
    refuse_profile_path_in_exe(&exe)?;

    let dist_directory = metadata.target_directory.join("dist");
    fs::create_dir_all(&dist_directory)?;
    let source_archive = dist_directory.join(format!("{PACKAGE}-{version}-source.zip"));
    archive_head(root, version, &source_archive)?;

    let readme = include_str!("../dist/README.txt").replace("{version}", version);
    let license = read_text(&root.join("LICENSE"))?;
    let third_party = notices::third_party_notices(root, &metadata)?;

    let zip_path = dist_directory.join(format!("{PACKAGE}-{version}-windows-x64.zip"));
    let mut zip = ZipWriter::new(File::create(&zip_path)?);
    let mut add = |name: &str, bytes: &[u8], method: CompressionMethod| -> Result<()> {
        zip.start_file(
            format!("{FOLDER}/{name}"),
            SimpleFileOptions::default().compression_method(method),
        )?;
        zip.write_all(bytes)?;
        Ok(())
    };
    add(&format!("{PACKAGE}.exe"), &exe, CompressionMethod::Deflated)?;
    for (name, bytes) in &windivert {
        add(name, bytes, CompressionMethod::Deflated)?;
    }
    add("README.txt", crlf(&readme).as_bytes(), CompressionMethod::Deflated)?;
    add("LICENSE.txt", crlf(&license).as_bytes(), CompressionMethod::Deflated)?;
    add("THIRD-PARTY-NOTICES.txt", crlf(&third_party).as_bytes(), CompressionMethod::Deflated)?;
    add(
        &format!("source/{PACKAGE}-{version}-source.zip"),
        &fs::read(&source_archive)?,
        CompressionMethod::Stored,
    )?;
    zip.finish()?;

    report(&zip_path)
}

fn refuse_dirty_tree(root: &Path, options: &Options) -> Result<()> {
    let status = git(root, &["status", "--porcelain"])?;
    let changes = String::from_utf8_lossy(&status);
    if changes.trim().is_empty() {
        return Ok(());
    }
    if options.allow_dirty {
        println!(
            "warning: --allow-dirty: the working tree has uncommitted changes. The exe is built from them, but the \
             source zip is `git archive HEAD` and will NOT contain them:\n{changes}"
        );
        return Ok(());
    }
    Err(format!(
        "the working tree has uncommitted changes, so the source zip (git archive of HEAD) would not match the \
         exe built from it. Commit them, or pass --allow-dirty to package anyway:\n{changes}"
    )
    .into())
}

fn read_verified_windivert(root: &Path) -> Result<Vec<(&'static str, Vec<u8>)>> {
    let mut files = Vec::new();
    for (name, expected) in WINDIVERT_FILES {
        let path = root.join("crates/app/windivert").join(name);
        let bytes = fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let actual = sha256_hex(&bytes);
        if actual != expected {
            return Err(format!("{name} sha256 is {actual}, expected the official WinDivert 2.2.2 {expected}").into());
        }
        println!("check: {name} sha256 {actual} (official WinDivert 2.2.2)");
        files.push((name, bytes));
    }
    Ok(files)
}

/// Opening for write without truncating fails while Windows has the exe mapped by a running process.
fn refuse_locked_exe(exe: &Path) -> Result<()> {
    if !exe.exists() {
        return Ok(());
    }
    fs::OpenOptions::new().write(true).open(exe).map(drop).map_err(|error| {
        format!(
            "{} is locked or not writable ({error}). Close the running meter and run again; nothing was killed.",
            exe.display()
        )
        .into()
    })
}

/// The flags are set through the environment of this one cargo invocation, with `--target` so that
/// they apply to the app and not to build scripts, and so that the output lands in
/// `target/<triple>/release`, leaving `target/release` and dev builds alone.
fn build_release_with_static_crt(root: &Path) -> Result<()> {
    println!("build: cargo build --release -p {PACKAGE} --target {WINDOWS_TARGET} (static CRT, build paths remapped)");
    let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["build", "--release", "--locked", "-p", PACKAGE, "--target", WINDOWS_TARGET])
        .env("CARGO_ENCODED_RUSTFLAGS", release_rustflags(root)?)
        .env_remove("RUSTFLAGS")
        .current_dir(root)
        .status()
        .map_err(|error| format!("could not run cargo build: {error}"))?;
    if !status.success() {
        return Err(format!("the release build failed ({status}); nothing was packaged").into());
    }
    Ok(())
}

/// Panic and `file!()` strings in the exe carry source paths; mapping them keeps the builder's
/// user profile out of a binary that is sent to other people.
fn release_rustflags(root: &Path) -> Result<String> {
    let cargo_home = match std::env::var_os("CARGO_HOME") {
        Some(home) => PathBuf::from(home),
        None => user_profile()?.join(".cargo"),
    };
    let sysroot = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--print", "sysroot"])
        .output()
        .map_err(|error| format!("could not run rustc --print sysroot: {error}"))?;
    let sysroot = PathBuf::from(String::from_utf8_lossy(&sysroot.stdout).trim());
    let rustup_home = sysroot
        .ancestors()
        .nth(2)
        .ok_or_else(|| format!("sysroot {} is not inside a rustup toolchains folder", sysroot.display()))?;
    let flags = [
        "-Ctarget-feature=+crt-static".to_owned(),
        format!("--remap-path-prefix={}=/cargo", cargo_home.display()),
        format!("--remap-path-prefix={}=/rustup", rustup_home.display()),
        format!("--remap-path-prefix={}=/src", root.display()),
    ];
    Ok(flags.join("\x1f"))
}

fn user_profile() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "USERPROFILE is not set".into())
}

/// Fails when the exe still contains the builder's profile folder, in either slash style.
fn refuse_profile_path_in_exe(exe: &[u8]) -> Result<()> {
    let profile = user_profile()?.to_string_lossy().to_ascii_lowercase();
    let lowered = exe.to_ascii_lowercase();
    for needle in [profile.clone(), profile.replace('\\', "/")] {
        if let Some(offset) = lowered.windows(needle.len()).position(|window| window == needle.as_bytes()) {
            let context = String::from_utf8_lossy(&exe[offset..(offset + 80).min(exe.len())]);
            return Err(format!(
                "the exe still contains the build machine's user profile path at byte {offset} (\"{}\"); it would \
                 leak the builder's Windows username",
                context.trim_end_matches('\0')
            )
            .into());
        }
    }
    println!("check: exe contains no build-machine user profile path");
    Ok(())
}

fn archive_head(root: &Path, version: &str, output: &Path) -> Result<()> {
    let prefix = format!("{PACKAGE}-{version}-source/");
    let output = output.to_str().ok_or("dist path is not valid UTF-8")?;
    git(root, &["archive", "--format=zip", "--prefix", &prefix, "--output", output, "HEAD"])?;
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if !output.status.success() {
        return Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(output.stdout)
}

fn report(zip_path: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(File::open(zip_path)?)?;
    println!("\n{}", zip_path.display());
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        println!("  {:>12} bytes  {}", entry.size(), entry.name());
    }
    let size = fs::metadata(zip_path)?.len();
    println!("total: {size} bytes ({:.1} MiB)", size as f64 / (1024.0 * 1024.0));
    Ok(())
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()).into())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Windows Notepad users get CRLF whatever line endings the repository file was checked out with.
pub fn crlf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

#[cfg(test)]
mod tests {
    use super::{crlf, sha256_hex};

    #[test]
    fn crlf_is_idempotent_and_converts_bare_line_feeds() {
        assert_eq!(crlf("a\nb\r\nc"), "a\r\nb\r\nc");
        assert_eq!(crlf(&crlf("a\nb")), crlf("a\nb"));
    }

    #[test]
    fn sha256_hex_matches_the_known_digest_of_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
