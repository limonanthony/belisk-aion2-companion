//! The folder an update waits in, beside the exe.
//!
//! ```text
//! <install dir>/update-staging/
//!     download/            what is being fetched (archive.zip.part)
//!     ready/               a verified update:
//!         update.zip           the signed archive, as published
//!         update.sha256        its published SHA-256 text
//!         update.sig           its published signature text
//!         READY                the version, as a hint only
//! ```
//!
//! Nothing in `ready/` is trusted because it is there. `READY` is a hint at
//! which version the archive claims to be; every use of the update (showing it
//! as ready at launch, the restart, the helper that swaps the files) reads the
//! archive's bytes once and checks hash and signature of those bytes, so a file
//! planted or changed in this folder can only make an update fail, never run.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use ed25519_dalek::VerifyingKey;

use super::package::{self, MAX_ARCHIVE_BYTES};
use super::state::Failure;
use super::verify;
use crate::changelog::Version;

const SMALL_FILE_LIMIT: u64 = 4096;

#[derive(Clone, Debug)]
pub struct Layout {
    install_dir: PathBuf,
}

impl Layout {
    pub fn new(install_dir: PathBuf) -> Self {
        Self { install_dir }
    }

    pub fn install_dir(&self) -> &Path {
        &self.install_dir
    }

    pub fn root(&self) -> PathBuf {
        self.install_dir.join("update-staging")
    }

    pub fn download_dir(&self) -> PathBuf {
        self.root().join("download")
    }

    pub fn archive_part(&self) -> PathBuf {
        self.download_dir().join("archive.zip.part")
    }

    pub fn ready_dir(&self) -> PathBuf {
        self.root().join("ready")
    }

    fn ready_archive(&self) -> PathBuf {
        self.ready_dir().join("update.zip")
    }

    fn ready_checksum(&self) -> PathBuf {
        self.ready_dir().join("update.sha256")
    }

    fn ready_signature(&self) -> PathBuf {
        self.ready_dir().join("update.sig")
    }

    fn ready_marker(&self) -> PathBuf {
        self.ready_dir().join("READY")
    }
}

/// Empties the staging folder and makes a fresh `download/`.
pub fn begin_download(layout: &Layout) -> io::Result<()> {
    remove_all(&layout.root())?;
    fs::create_dir_all(layout.download_dir())
}

/// Deletes the staging folder with everything in it.
pub fn remove_all(dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(dir) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Deletes only the download in progress, leaving a staged update alone.
pub fn discard_download(layout: &Layout) {
    if let Err(error) = remove_all(&layout.download_dir()) {
        crate::app_log::warn(format!("cannot delete {}: {error}", layout.download_dir().display()));
    }
}

/// Keeps a verified archive as the staged update, replacing what was staged.
/// The archive must also be one an update may install (see `package`), so a
/// bad one fails now and not at the restart.
pub fn stage(layout: &Layout, archive: &[u8], checksum_text: &str, signature_text: &str, version: Version) -> Result<(), Failure> {
    package::extract(archive)?;
    let failed = |what: &str, error: io::Error| Failure::Stage(format!("cannot write {what}: {error}"));
    remove_all(&layout.ready_dir()).map_err(|error| Failure::Stage(format!("cannot replace the staged update: {error}")))?;
    fs::create_dir_all(layout.ready_dir()).map_err(|error| failed("the staging folder", error))?;
    fs::write(layout.ready_archive(), archive).map_err(|error| failed("the archive", error))?;
    fs::write(layout.ready_checksum(), checksum_text).map_err(|error| failed("its SHA-256", error))?;
    fs::write(layout.ready_signature(), signature_text).map_err(|error| failed("its signature", error))?;
    fs::write(layout.ready_marker(), version.to_string()).map_err(|error| failed("its version", error))?;
    discard_download(layout);
    Ok(())
}

/// A staged update whose archive passed both checks just now.
pub struct Verified {
    pub version: Version,
    /// The bytes that were checked; what is installed comes from these.
    pub archive: Vec<u8>,
}

/// Reads the staged update and checks it: newer than `running`, and its hash
/// and signature, against `key`.
pub fn load_verified(layout: &Layout, running: Version, key: &VerifyingKey) -> Result<Verified, Failure> {
    let unusable = |what: &str, error: io::Error| Failure::Stage(format!("the staged update has no readable {what}: {error}"));
    let hint = read_small(&layout.ready_marker()).map_err(|error| unusable("READY", error))?;
    let version: Version = hint
        .trim()
        .parse()
        .map_err(|_| Failure::Stage("the staged update does not say which version it is".to_owned()))?;
    if version <= running {
        return Err(Failure::Stage(format!("the staged update {version} is not newer than {running}")));
    }
    let archive = read_archive(&layout.ready_archive()).map_err(|error| unusable("archive", error))?;
    let checksum = read_small(&layout.ready_checksum()).map_err(|error| unusable("SHA-256", error))?;
    let signature = read_small(&layout.ready_signature()).map_err(|error| unusable("signature", error))?;
    verify::verify_archive(&archive, version, &checksum, &signature, key)?;
    Ok(Verified { version, archive })
}

/// The version waiting in `ready/` when it verifies and is newer than
/// `running`. Anything else in the staging folder is stale or not to be
/// trusted, and is deleted.
pub fn staged_version(layout: &Layout, running: Version, key: &VerifyingKey) -> Option<Version> {
    match load_verified(layout, running, key) {
        Ok(verified) => {
            discard_download(layout);
            Some(verified.version)
        }
        Err(failure) => {
            if layout.root().exists() {
                crate::app_log::warn(format!("the staged update is deleted: {failure}"));
            }
            if let Err(error) = remove_all(&layout.root()) {
                crate::app_log::warn(format!("cannot delete the old staging folder: {error}"));
            }
            None
        }
    }
}

fn read_small(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    File::open(path)?.take(SMALL_FILE_LIMIT).read_to_string(&mut text)?;
    Ok(text)
}

fn read_archive(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_ARCHIVE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "larger than an update archive may be"));
    }
    Ok(bytes)
}

/// Deletes what an earlier update's swap left beside the exe: the `.old` copies
/// of the files it replaced and any `.new` file it did not finish with.
pub fn remove_leftovers(install_dir: &Path) {
    const SUFFIXES: [&str; 6] = [".exe.old", ".dll.old", ".sys.old", ".exe.new", ".dll.new", ".sys.new"];
    let Ok(entries) = fs::read_dir(install_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let left_by_a_swap = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| SUFFIXES.iter().any(|suffix| name.ends_with(suffix)));
        if left_by_a_swap && path.is_file() {
            if let Err(error) = fs::remove_file(&path) {
                crate::app_log::warn(format!("cannot delete {}: {error}", path.display()));
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::update::package::tests::zip_of;
    use crate::update::package::EXE_ENTRY;
    use crate::update::verify::tests::{fixture_key, sign};

    pub struct TempDir(pub PathBuf);

    impl TempDir {
        pub fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("belisk-update-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub fn key() -> VerifyingKey {
        fixture_key().verifying_key()
    }

    pub struct Published {
        pub archive: Vec<u8>,
        pub checksum: String,
        pub signature: String,
    }

    /// An archive of `files`, signed for `version` by the fixture key.
    pub fn published(files: &[(&str, &[u8])], version: Version) -> Published {
        let archive = zip_of(files);
        Published { checksum: verify::sha256_hex(&archive), signature: sign(&fixture_key(), version, &archive), archive }
    }

    pub fn stage_published(layout: &Layout, published: &Published, version: Version) {
        begin_download(layout).unwrap();
        stage(layout, &published.archive, &published.checksum, &published.signature, version).unwrap();
    }

    const RUNNING: Version = Version::new(1, 0, 0);
    const NEXT: Version = Version::new(1, 2, 0);

    fn layout_in(dir: &TempDir) -> Layout {
        Layout::new(dir.0.join("install"))
    }

    #[test]
    fn a_verified_archive_is_kept_as_it_was_published_and_reads_back_verified() {
        let dir = TempDir::new("stage-ok");
        let layout = layout_in(&dir);
        let update = published(&[(EXE_ENTRY, b"new exe"), ("WinDivert.dll", b"dll")], NEXT);

        stage_published(&layout, &update, NEXT);

        let verified = load_verified(&layout, RUNNING, &key()).unwrap();
        assert_eq!(verified.version, NEXT);
        assert_eq!(verified.archive, update.archive);
        assert_eq!(staged_version(&layout, RUNNING, &key()), Some(NEXT));
        assert!(!layout.download_dir().exists(), "the download folder is gone once staged");
    }

    #[test]
    fn staging_a_newer_release_replaces_the_staged_one() {
        let dir = TempDir::new("stage-replace");
        let layout = layout_in(&dir);
        let first = published(&[(EXE_ENTRY, b"one"), ("WinDivert.dll", b"x")], Version::new(1, 1, 0));
        let second = published(&[(EXE_ENTRY, b"two")], NEXT);

        stage_published(&layout, &first, Version::new(1, 1, 0));
        stage_published(&layout, &second, NEXT);

        let verified = load_verified(&layout, RUNNING, &key()).unwrap();
        assert_eq!((verified.version, verified.archive), (NEXT, second.archive));
    }

    #[test]
    fn an_archive_an_update_may_not_install_is_refused_before_it_is_kept() {
        let dir = TempDir::new("stage-bad");
        let layout = layout_in(&dir);
        for files in [
            vec![(EXE_ENTRY, &b"x"[..]), ("sub/WinDivert.dll", b"y")],
            vec![(EXE_ENTRY, &b"x"[..]), ("../evil.dll", b"y")],
            vec![("WinDivert.dll", &b"y"[..])],
        ] {
            let update = published(&files, NEXT);
            begin_download(&layout).unwrap();

            let result = stage(&layout, &update.archive, &update.checksum, &update.signature, NEXT);

            assert!(matches!(result, Err(Failure::Stage(_))), "{files:?}");
            assert!(!layout.ready_dir().exists());
        }
    }

    #[test]
    fn a_staged_archive_changed_afterwards_is_not_ready_and_is_deleted() {
        let dir = TempDir::new("stage-tampered");
        let layout = layout_in(&dir);
        stage_published(&layout, &published(&[(EXE_ENTRY, b"genuine")], NEXT), NEXT);

        fs::write(layout.ready_dir().join("update.zip"), zip_of(&[(EXE_ENTRY, b"planted")])).unwrap();

        assert!(matches!(load_verified(&layout, RUNNING, &key()), Err(Failure::ChecksumMismatch)));
        assert_eq!(staged_version(&layout, RUNNING, &key()), None);
        assert!(!layout.root().exists());
    }

    #[test]
    fn a_planted_archive_with_its_own_hash_but_not_the_publishers_signature_is_not_ready() {
        let dir = TempDir::new("stage-forged");
        let layout = layout_in(&dir);
        stage_published(&layout, &published(&[(EXE_ENTRY, b"genuine")], NEXT), NEXT);
        let planted = zip_of(&[(EXE_ENTRY, b"planted")]);

        fs::write(layout.ready_dir().join("update.zip"), &planted).unwrap();
        fs::write(layout.ready_dir().join("update.sha256"), verify::sha256_hex(&planted)).unwrap();

        assert!(matches!(load_verified(&layout, RUNNING, &key()), Err(Failure::SignatureMismatch)));
    }

    #[test]
    fn a_ready_marker_that_lies_about_the_version_cannot_make_an_old_archive_pass() {
        let dir = TempDir::new("stage-lying-marker");
        let layout = layout_in(&dir);
        let old = Version::new(1, 1, 0);
        stage_published(&layout, &published(&[(EXE_ENTRY, b"old but genuine")], old), old);

        fs::write(layout.ready_dir().join("READY"), "9.9.9").unwrap();

        assert!(matches!(load_verified(&layout, RUNNING, &key()), Err(Failure::SignatureMismatch)));
    }

    #[test]
    fn a_staged_version_that_is_not_newer_than_the_running_one_is_deleted() {
        let dir = TempDir::new("stage-not-newer");
        let layout = layout_in(&dir);
        stage_published(&layout, &published(&[(EXE_ENTRY, b"x")], RUNNING), RUNNING);

        assert!(matches!(load_verified(&layout, RUNNING, &key()), Err(Failure::Stage(_))));
        assert_eq!(staged_version(&layout, RUNNING, &key()), None);
        assert!(!layout.root().exists());
    }

    #[test]
    fn missing_or_unreadable_parts_are_not_ready() {
        let dir = TempDir::new("stage-missing");
        let layout = layout_in(&dir);

        assert!(matches!(load_verified(&layout, RUNNING, &key()), Err(Failure::Stage(_))), "nothing staged");

        for part in ["READY", "update.zip", "update.sha256", "update.sig"] {
            stage_published(&layout, &published(&[(EXE_ENTRY, b"x")], NEXT), NEXT);
            fs::remove_file(layout.ready_dir().join(part)).unwrap();
            assert!(load_verified(&layout, RUNNING, &key()).is_err(), "without {part}");
        }

        stage_published(&layout, &published(&[(EXE_ENTRY, b"x")], NEXT), NEXT);
        fs::write(layout.ready_dir().join("READY"), "not a version").unwrap();
        assert!(matches!(load_verified(&layout, RUNNING, &key()), Err(Failure::Stage(_))));
    }

    #[test]
    fn a_staged_update_checked_against_another_key_is_not_ready() {
        let dir = TempDir::new("stage-other-key");
        let layout = layout_in(&dir);
        stage_published(&layout, &published(&[(EXE_ENTRY, b"x")], NEXT), NEXT);
        let other = ed25519_dalek::SigningKey::from_bytes(&[0x13; 32]).verifying_key();

        assert!(matches!(load_verified(&layout, RUNNING, &other), Err(Failure::SignatureMismatch)));
    }

    #[test]
    fn a_download_left_by_a_run_that_ended_is_discarded_but_the_staged_update_stays() {
        let dir = TempDir::new("stage-leftover");
        let layout = layout_in(&dir);
        stage_published(&layout, &published(&[(EXE_ENTRY, b"x")], NEXT), NEXT);
        fs::create_dir_all(layout.download_dir()).unwrap();
        fs::write(layout.archive_part(), b"half").unwrap();

        assert_eq!(staged_version(&layout, RUNNING, &key()), Some(NEXT));
        assert!(!layout.download_dir().exists());
    }

    #[test]
    fn leftovers_of_an_earlier_swap_are_deleted_and_nothing_else() {
        let dir = TempDir::new("leftovers");
        let names = [
            "belisk.exe.old",
            "WinDivert.dll.old",
            "WinDivert64.sys.old",
            "belisk.exe.new",
            "WinDivert.dll.new",
            "belisk.exe",
            "settings.json",
            "notes.txt.old",
        ];
        for name in names {
            fs::write(dir.0.join(name), b"x").unwrap();
        }

        remove_leftovers(&dir.0);

        let mut left: Vec<_> = fs::read_dir(&dir.0).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
        left.sort();
        assert_eq!(left, ["belisk.exe", "notes.txt.old", "settings.json"]);
    }
}
