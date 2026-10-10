use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::crates::Metadata;
use crate::{exe_check, notices, update, Result, WINDOWS_TARGET};

const PACKAGE: &str = "belisk";
const FOLDER: &str = "belisk";

/// Official WinDivert-2.2.2-A.zip, x64 (see docs/technical-notes.md, "WinDivert files").
const WINDIVERT_FILES: [(&str, &str); 2] = [
    ("WinDivert.dll", "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2"),
    ("WinDivert64.sys", "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2"),
];

pub enum SourceDelivery {
    Bundled,
    WrittenOffer { contact: String },
}

pub struct Options {
    pub allow_dirty: bool,
    pub allow_development_key: bool,
    pub source: SourceDelivery,
}

pub fn run(root: &Path, options: &Options) -> Result<()> {
    let metadata = Metadata::read(root)?;
    let version = &metadata.version;
    refuse_dirty_tree(root, options)?;
    refuse_development_key(root, options)?;
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
    let (output_directory, source_archive) = match &options.source {
        SourceDelivery::Bundled => {
            fs::create_dir_all(&dist_directory)?;
            let archive = dist_directory.join(format!("{PACKAGE}-{version}-source.zip"));
            archive_head(root, version, &archive)?;
            (dist_directory, Some(archive))
        }
        SourceDelivery::WrittenOffer { .. } => (dist_directory.join("no-source"), None),
    };
    fs::create_dir_all(&output_directory)?;

    let commit = String::from_utf8(git(root, &["rev-parse", "--short", "HEAD"])?)?.trim().to_owned();
    let source_statement = source_statement(&options.source, version, &commit);
    let readme = include_str!("../dist/README.txt")
        .replace("{version}", version)
        .replace("{source_statement}", &source_statement);
    let license = read_text(&root.join("LICENSE"))?;
    let third_party = notices::third_party_notices(root, &metadata, &source_statement)?;

    let zip_path = output_directory.join(format!("{PACKAGE}-{version}-windows-x64.zip"));
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
    if let Some(source_archive) = source_archive {
        add(
            &format!("source/{PACKAGE}-{version}-source.zip"),
            &fs::read(&source_archive)?,
            CompressionMethod::Stored,
        )?;
    }
    zip.finish()?;

    let update_archive = write_update_archive(&output_directory, version, &exe, &windivert)?;
    report(&zip_path)?;
    report(&update_archive)?;
    println!("\n{}.sha256 written; sign it with: cargo xtask sign --key <private key> {}", update_archive.display(), update_archive.display());
    Ok(())
}

/// The exe's updater never installs with the placeholder key, so a build made with
/// it cannot update itself; a release needs a key from `cargo xtask keygen`.
fn refuse_development_key(root: &Path, options: &Options) -> Result<()> {
    if !update::embedded_key_is_development(root)? {
        return Ok(());
    }
    if options.allow_development_key {
        eprintln!(
            "warning: --allow-development-key: crates/app/update-key.pub is still the DEVELOPMENT placeholder, so the exe \
             built here never installs an update. For local testing only; never use this flag in CI or for a release."
        );
        return Ok(());
    }
    Err("crates/app/update-key.pub is still the DEVELOPMENT placeholder, so the exe could not install any update. \
         Make the real key with `cargo xtask keygen --out <path outside the repository>`, copy the .pub file over \
         crates/app/update-key.pub, commit it, and run dist again (docs/updates.md). \
         `--allow-development-key` packages anyway, for local testing only; never use it in CI."
        .into())
}

/// What the exe needs to run, as flat files, with its SHA-256 beside it; the
/// signature is made later, with a private key this command never sees.
fn write_update_archive(output_directory: &Path, version: &str, exe: &[u8], windivert: &[(&'static str, Vec<u8>)]) -> Result<PathBuf> {
    let name = format!("{PACKAGE}-{version}{}", update::ARCHIVE_SUFFIX);
    let path = output_directory.join(&name);
    let mut zip = ZipWriter::new(File::create(&path)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(format!("{PACKAGE}.exe"), options)?;
    zip.write_all(exe)?;
    for (file_name, bytes) in windivert {
        zip.start_file(*file_name, options)?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;

    let checksum_path = PathBuf::from(format!("{}.sha256", path.display()));
    fs::write(&checksum_path, format!("{}  {name}\n", sha256_hex(&fs::read(&path)?)))?;
    Ok(path)
}

fn source_statement(source: &SourceDelivery, version: &str, commit: &str) -> String {
    match source {
        SourceDelivery::Bundled => format!(
            "The complete source code of this build is in source\\{PACKAGE}-{version}-source.zip."
        ),
        SourceDelivery::WrittenOffer { contact } => format!(
            "The source code is not included in this zip. As section 6(b) of the GPL-3.0 allows, this is a\n\
             written offer: the complete corresponding source code for this exact build (version {version},\n\
             commit {commit}) is available free of charge, on request, for at least three years from {contact}.\n\
             The offer holds for anyone who received this copy."
        ),
    }
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
             source (the zip, or the commit a written offer names) is HEAD and will NOT contain them:\n{changes}"
        );
        return Ok(());
    }
    Err(format!(
        "the working tree has uncommitted changes, so the source (HEAD) would not match the \
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
    use std::fs;

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

    fn root_with_key_file(name: &str, key_file: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("belisk-xtask-{name}-{}", std::process::id()));
        fs::create_dir_all(root.join("crates/app")).unwrap();
        fs::write(root.join("crates/app/update-key.pub"), key_file).unwrap();
        root
    }

    fn options(allow_development_key: bool) -> super::Options {
        super::Options { allow_dirty: false, allow_development_key, source: super::SourceDelivery::Bundled }
    }

    #[test]
    fn dist_fails_while_the_update_key_is_the_placeholder_and_says_how_to_fix_it() {
        let root = root_with_key_file("placeholder-refused", "# DEVELOPMENT KEY\nab\n");

        let error = super::refuse_development_key(&root, &options(false)).unwrap_err().to_string();

        assert!(error.contains("DEVELOPMENT") && error.contains("cargo xtask keygen") && error.contains("--allow-development-key"), "{error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_explicit_flag_lets_a_local_build_through_and_a_real_key_needs_no_flag() {
        let placeholder = root_with_key_file("placeholder-allowed", "# DEVELOPMENT KEY\nab\n");
        let real = root_with_key_file("real-key", "# Ed25519 public key\nab\n");

        assert!(super::refuse_development_key(&placeholder, &options(true)).is_ok());
        assert!(super::refuse_development_key(&real, &options(false)).is_ok());
        fs::remove_dir_all(placeholder).unwrap();
        fs::remove_dir_all(real).unwrap();
    }

    #[test]
    fn the_update_archive_holds_the_exe_and_the_driver_files_flat_with_its_sha256_beside_it() {
        let dir = std::env::temp_dir().join(format!("belisk-xtask-lean-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let windivert = vec![("WinDivert.dll", b"dll".to_vec()), ("WinDivert64.sys", b"sys".to_vec())];

        let path = super::write_update_archive(&dir, "1.2.3", b"exe bytes", &windivert).unwrap();

        assert_eq!(path.file_name().unwrap(), "belisk-1.2.3-windows-x64-update.zip");
        let mut archive = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
        let mut names: Vec<String> = (0..archive.len()).map(|index| archive.by_index(index).unwrap().name().to_owned()).collect();
        names.sort();
        assert_eq!(names, ["WinDivert.dll", "WinDivert64.sys", "belisk.exe"]);
        let checksum = fs::read_to_string(format!("{}.sha256", path.display())).unwrap();
        assert_eq!(checksum, format!("{}  belisk-1.2.3-windows-x64-update.zip\n", sha256_hex(&fs::read(&path).unwrap())));
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod source_statement_tests {
    use super::{source_statement, SourceDelivery};

    #[test]
    fn written_offer_names_the_commit_version_and_contact() {
        let offer = SourceDelivery::WrittenOffer { contact: "the person who gave you this copy".into() };
        let text = source_statement(&offer, "2.4.0", "276d5e1");
        assert!(text.contains("commit 276d5e1"));
        assert!(text.contains("version 2.4.0"));
        assert!(text.contains("from the person who gave you this copy"));
        assert!(text.contains("three years"));
    }

    #[test]
    fn bundled_statement_points_at_the_source_zip() {
        let text = source_statement(&SourceDelivery::Bundled, "2.4.0", "276d5e1");
        assert!(text.contains("source\\belisk-2.4.0-source.zip"));
    }
}
