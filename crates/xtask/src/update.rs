//! The update archive's signing: `cargo xtask keygen` and `cargo xtask sign`.
//!
//! The format is the app's (`crates/app/src/update/verify.rs`): an Ed25519
//! signature over `"belisk update <version>\n"` followed by the
//! archive's bytes, written as 128 hex digits to `<archive>.sig`. Keys are 32
//! bytes as 64 hex digits, one per file, `#` lines are comments. The private
//! key is refused anywhere inside the repository.

use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

use crate::Result;

pub const ARCHIVE_SUFFIX: &str = "-windows-x64-update.zip";
const PACKAGE_PREFIX: &str = "belisk-";

pub fn signed_message(version: &str, archive: &[u8]) -> Vec<u8> {
    let mut message = format!("belisk update {version}\n").into_bytes();
    message.extend_from_slice(archive);
    message
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 || !text.is_ascii() {
        return None;
    }
    (0..text.len()).step_by(2).map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok()).collect()
}

fn first_key_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty() && !line.starts_with('#'))
}

/// `belisk-1.2.3-windows-x64-update.zip` is version 1.2.3's archive.
pub fn version_of_archive(file_name: &str) -> Result<&str> {
    file_name
        .strip_prefix(PACKAGE_PREFIX)
        .and_then(|rest| rest.strip_suffix(ARCHIVE_SUFFIX))
        .filter(|version| !version.is_empty())
        .ok_or_else(|| format!("{file_name} is not named {PACKAGE_PREFIX}<version>{ARCHIVE_SUFFIX}").into())
}

pub fn keygen(root: &Path, out: &Path) -> Result<()> {
    refuse_inside_repo(root, out)?;
    if out.exists() {
        return Err(format!("{} already exists; keygen never overwrites a key", out.display()).into());
    }
    let public_path = public_path(out);
    if public_path.exists() {
        return Err(format!("{} already exists", public_path.display()).into());
    }

    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|error| format!("cannot get random bytes: {error}"))?;
    let key = SigningKey::from_bytes(&seed);
    let public = hex(key.verifying_key().as_bytes());

    fs::write(out, format!("# PRIVATE update signing key. Never commit it, never share it.\n{}\n", hex(&seed)))
        .map_err(|error| format!("cannot write {}: {error}", out.display()))?;
    fs::write(&public_path, format!("# Ed25519 public key, 64 hex digits.\n{public}\n"))
        .map_err(|error| format!("cannot write {}: {error}", public_path.display()))?;

    println!("private key: {}", out.display());
    println!("public key:  {}", public_path.display());
    println!("Copy the public key file over crates/app/update-key.pub, then rebuild; releases signed with this key verify.");
    Ok(())
}

pub fn sign(root: &Path, key_path: &Path, archive_path: &Path) -> Result<()> {
    refuse_inside_repo(root, key_path)?;
    let file_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{} has no file name", archive_path.display()))?;
    let version = version_of_archive(file_name)?;
    let key = read_signing_key(key_path)?;
    let archive = fs::read(archive_path).map_err(|error| format!("cannot read {}: {error}", archive_path.display()))?;

    let message = signed_message(version, &archive);
    let signature = key.sign(&message);
    embedded_verifying_key(root)?.verify_strict(&message, &signature).map_err(|_| {
        format!(
            "{} was not signed: the key {} holds is not the private half of crates/app/update-key.pub, so no installed \
             app would accept the signature. Sign with the key `cargo xtask keygen` made for that public key.",
            archive_path.display(),
            key_path.display()
        )
    })?;
    let signature_path = PathBuf::from(format!("{}.sig", archive_path.display()));
    fs::write(&signature_path, format!("{}\n", hex(&signature.to_bytes())))
        .map_err(|error| format!("cannot write {}: {error}", signature_path.display()))?;
    println!("{}", signature_path.display());
    Ok(())
}

fn read_signing_key(path: &Path) -> Result<SigningKey> {
    let text = fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let line = first_key_line(&text).ok_or_else(|| format!("{} holds no key", path.display()))?;
    let seed: [u8; 32] = decode_hex(line)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| format!("{} is not 64 hex digits", path.display()))?;
    Ok(SigningKey::from_bytes(&seed))
}

fn public_path(private: &Path) -> PathBuf {
    PathBuf::from(format!("{}.pub", private.display()))
}

/// Resolves what exists of `path` (its folder, when the file is new) and
/// compares it with the repository root.
fn refuse_inside_repo(root: &Path, path: &Path) -> Result<()> {
    let root = fs::canonicalize(root)?;
    let absolute = std::path::absolute(path)?;
    let folder = absolute.parent().ok_or_else(|| format!("{} has no folder", path.display()))?;
    let folder = fs::canonicalize(folder).map_err(|error| format!("{} does not exist: {error}", folder.display()))?;
    if folder.starts_with(&root) {
        return Err(format!("{} is inside the repository; keep the private key outside it", path.display()).into());
    }
    Ok(())
}

/// Whether `crates/app/update-key.pub` is still the development key.
pub fn embedded_key_is_development(root: &Path) -> Result<bool> {
    let path = root.join("crates/app/update-key.pub");
    Ok(fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?.contains("DEVELOPMENT"))
}

fn embedded_verifying_key(root: &Path) -> Result<VerifyingKey> {
    let path = root.join("crates/app/update-key.pub");
    let text = fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let bytes: [u8; 32] = first_key_line(&text)
        .and_then(decode_hex)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| format!("{} does not hold a key of 64 hex digits", path.display()))?;
    VerifyingKey::from_bytes(&bytes).map_err(|error| format!("{} is not a valid public key: {error}", path.display()).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("belisk-xtask-{name}-{}-{serial}", std::process::id()));
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

    fn repo() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap()
    }

    #[test]
    fn the_signed_bytes_are_the_header_line_then_the_archive() {
        assert_eq!(signed_message("1.2.3", b"zip"), b"belisk update 1.2.3\nzip");
    }

    /// The same vector is asserted by the app's verify tests: the two programs agree.
    #[test]
    fn a_known_key_version_and_archive_give_the_known_signature() {
        let key = SigningKey::from_bytes(&[0x41; 32]);
        let signature = key.sign(&signed_message("1.2.3", b"pretend zip bytes"));
        assert_eq!(hex(&signature.to_bytes()), KNOWN_SIGNATURE);
    }

    const KNOWN_SIGNATURE: &str = "8c07ab81643d2db4eca02daaee91a260b8b15b46a2f08becdc08d1c3bfc70460716dbc62943adffb4e6565fe68b57bd9170fc088a487daf8bf0a3d0be8b2ac0b";

    #[test]
    fn the_version_comes_from_the_archive_name() {
        assert_eq!(version_of_archive("belisk-1.2.3-windows-x64-update.zip").unwrap(), "1.2.3");
        for bad in ["belisk-1.2.3-windows-x64.zip", "other-1.2.3-windows-x64-update.zip", "belisk--windows-x64-update.zip"] {
            assert!(version_of_archive(bad).is_err(), "{bad}");
        }
    }

    fn repo_trusting(public_key_file: &Path) -> TempDir {
        let root = TempDir::new("trusting-repo");
        fs::create_dir_all(root.0.join("crates/app")).unwrap();
        fs::copy(public_key_file, root.0.join("crates/app/update-key.pub")).unwrap();
        root
    }

    #[test]
    fn a_key_the_app_does_not_trust_signs_nothing() {
        let dir = TempDir::new("sign-untrusted");
        let trusted_key = dir.0.join("trusted.key");
        let other_key = dir.0.join("other.key");
        let archive = dir.0.join("belisk-2.0.0-windows-x64-update.zip");
        fs::write(&archive, b"archive bytes").unwrap();
        keygen(repo(), &trusted_key).unwrap();
        keygen(repo(), &other_key).unwrap();
        let trusting = repo_trusting(&public_path(&trusted_key));

        let error = sign(&trusting.0, &other_key, &archive).unwrap_err().to_string();

        assert!(error.contains("update-key.pub"), "{error}");
        assert!(!Path::new(&format!("{}.sig", archive.display())).exists());
    }

    #[test]
    fn keygen_then_sign_writes_a_signature_the_public_key_verifies() {
        let dir = TempDir::new("keygen-sign");
        let key = dir.0.join("update.key");
        let archive = dir.0.join("belisk-2.0.0-windows-x64-update.zip");
        fs::write(&archive, b"archive bytes").unwrap();

        keygen(repo(), &key).unwrap();
        let trusting = repo_trusting(&public_path(&key));
        sign(&trusting.0, &key, &archive).unwrap();

        let public = fs::read_to_string(public_path(&key)).unwrap();
        let public: [u8; 32] = decode_hex(public.lines().find(|line| !line.starts_with('#')).unwrap()).unwrap().try_into().unwrap();
        let signature = fs::read_to_string(format!("{}.sig", archive.display())).unwrap();
        let signature: [u8; 64] = decode_hex(signature.trim()).unwrap().try_into().unwrap();
        ed25519_dalek::VerifyingKey::from_bytes(&public)
            .unwrap()
            .verify_strict(&signed_message("2.0.0", b"archive bytes"), &ed25519_dalek::Signature::from_bytes(&signature))
            .unwrap();
    }

    #[test]
    fn keygen_never_overwrites_a_key() {
        let dir = TempDir::new("keygen-overwrite");
        let key = dir.0.join("update.key");
        keygen(repo(), &key).unwrap();
        let before = fs::read(&key).unwrap();

        assert!(keygen(repo(), &key).is_err());

        assert_eq!(fs::read(&key).unwrap(), before);
    }

    #[test]
    fn a_private_key_inside_the_repository_is_refused() {
        let inside = repo().join("target").join("belisk-xtask-test.key");
        fs::create_dir_all(inside.parent().unwrap()).unwrap();

        assert!(keygen(repo(), &inside).is_err());
        assert!(!inside.exists());

        let dir = TempDir::new("sign-inside");
        let archive = dir.0.join("belisk-2.0.0-windows-x64-update.zip");
        fs::write(&archive, b"x").unwrap();
        assert!(sign(repo(), &inside, &archive).is_err());
    }

    #[test]
    fn the_development_marker_in_the_public_key_file_is_noticed() {
        let dir = TempDir::new("marker");
        fs::create_dir_all(dir.0.join("crates/app")).unwrap();
        let key_file = dir.0.join("crates/app/update-key.pub");

        fs::write(&key_file, "# DEVELOPMENT KEY\nab\n").unwrap();
        assert!(embedded_key_is_development(&dir.0).unwrap());

        fs::write(&key_file, "# Ed25519 public key\nab\n").unwrap();
        assert!(!embedded_key_is_development(&dir.0).unwrap());
    }
}
