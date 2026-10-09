//! Checking an update archive before anything of it is unpacked or run.
//!
//! The exe runs as Administrator, so an archive that only *looks* like a
//! release would run as Administrator on every player's PC. Two checks, both
//! of which must pass:
//!
//! - SHA-256 of the archive against the release's `.sha256` file: catches a
//!   download that was damaged on the way.
//! - an Ed25519 signature (the release's `.sig` file, 64 bytes as 128 hex
//!   digits) against the public key built into the exe (`update-key.pub`).
//!   This is what a forged release cannot pass.
//!
//! What is signed is not the archive alone: it is
//! `"belisk update <version>\n"` followed by the archive's bytes. The
//! version binds the signature to the release it was made for, so an old, genuine,
//! signed archive cannot be served as a newer release. `cargo xtask sign`
//! writes exactly this.
//!
//! The checks take the archive's bytes, not a path: whoever verifies keeps
//! using those same bytes, so the file cannot change between the check and
//! the use.

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use super::state::Failure;
use crate::changelog::Version;

const EMBEDDED_KEY: &str = include_str!("../../update-key.pub");

/// The marker in the key file of a placeholder key; `cargo xtask dist` refuses
/// to build while it is there and the app never installs with such a key.
pub const PLACEHOLDER_MARKER: &str = "DEVELOPMENT";

pub fn signed_message(version: Version, archive: &[u8]) -> Vec<u8> {
    let mut message = format!("belisk update {version}\n").into_bytes();
    message.extend_from_slice(archive);
    message
}

/// The public key this exe trusts for updates; an error while the key file is
/// still the placeholder, so a build that was never given a real key never
/// installs anything.
pub fn trusted_key() -> Result<VerifyingKey, String> {
    parse_trusted_key(EMBEDDED_KEY)
}

fn parse_trusted_key(text: &str) -> Result<VerifyingKey, String> {
    if text.contains(PLACEHOLDER_MARKER) {
        return Err("crates/app/update-key.pub is still the DEVELOPMENT placeholder; make a key with `cargo xtask keygen`".to_owned());
    }
    parse_key(text)
}

/// A key file: 64 hex digits (32 bytes); blank lines and lines starting with `#` are ignored.
pub fn parse_key(text: &str) -> Result<VerifyingKey, String> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .ok_or("the update key file has no key")?;
    let bytes: [u8; 32] = decode_hex(line)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or("the update key is not 64 hex digits")?;
    VerifyingKey::from_bytes(&bytes).map_err(|error| format!("the update key is not a valid Ed25519 key: {error}"))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Checks `archive` against what the release published. The first token of the
/// checksum text is the hash (a `sha256sum` line is fine); the signature text
/// is the 128 hex digits.
pub fn verify_archive(
    archive: &[u8],
    version: Version,
    checksum_text: &str,
    signature_text: &str,
    key: &VerifyingKey,
) -> Result<(), Failure> {
    let published = checksum_text.split_whitespace().next().unwrap_or("");
    if decode_hex(published).is_none_or(|bytes| bytes.len() != 32) {
        return Err(Failure::MissingChecksum);
    }
    if !published.eq_ignore_ascii_case(&sha256_hex(archive)) {
        return Err(Failure::ChecksumMismatch);
    }

    let signature: [u8; 64] = decode_hex(signature_text.trim())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(Failure::MissingSignature)?;
    key.verify_strict(&signed_message(version, archive), &Signature::from_bytes(&signature))
        .map_err(|_| Failure::SignatureMismatch)
}

pub fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok())
        .collect()
}

#[cfg(test)]
pub mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    /// A key made for the tests; its private half is public on purpose, so
    /// nothing signed with it may ever be released, and the key file in the
    /// repository is not this key.
    pub const FIXTURE_SEED: [u8; 32] = [0x41; 32];

    pub fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn fixture_key() -> SigningKey {
        SigningKey::from_bytes(&FIXTURE_SEED)
    }

    pub fn sign(key: &SigningKey, version: Version, archive: &[u8]) -> String {
        hex(&key.sign(&signed_message(version, archive)).to_bytes())
    }

    const V: Version = Version::new(1, 2, 3);

    fn fixture_public() -> VerifyingKey {
        fixture_key().verifying_key()
    }

    /// The same vector is asserted by `cargo xtask sign`'s tests: the signer and the verifier agree.
    #[test]
    fn the_signature_the_signer_tool_makes_for_a_known_key_version_and_archive_is_accepted() {
        let signature = "8c07ab81643d2db4eca02daaee91a260b8b15b46a2f08becdc08d1c3bfc70460716dbc62943adffb4e6565fe68b57bd9170fc088a487daf8bf0a3d0be8b2ac0b";
        let bytes = b"pretend zip bytes";

        assert_eq!(verify_archive(bytes, V, &sha256_hex(bytes), signature, &fixture_public()), Ok(()));
        assert_eq!(sign(&fixture_key(), V, bytes), signature);
    }

    #[test]
    fn a_good_archive_hash_and_signature_pass() {
        let bytes = b"pretend zip bytes";
        let checksum = format!("{}  belisk-1.2.3-windows-x64-update.zip\n", sha256_hex(bytes));
        let signature = sign(&fixture_key(), V, bytes);

        assert_eq!(verify_archive(bytes, V, &checksum, &signature, &fixture_public()), Ok(()));
    }

    #[test]
    fn a_wrong_hash_is_rejected_even_when_the_signature_is_good() {
        let signature = sign(&fixture_key(), V, b"the archive");

        assert_eq!(
            verify_archive(b"the archive", V, &sha256_hex(b"another archive"), &signature, &fixture_public()),
            Err(Failure::ChecksumMismatch)
        );
    }

    #[test]
    fn a_signature_by_another_key_is_rejected_even_when_the_hash_is_good() {
        let bytes = b"the archive";
        let forger = SigningKey::from_bytes(&[0x42; 32]);

        assert_eq!(
            verify_archive(bytes, V, &sha256_hex(bytes), &sign(&forger, V, bytes), &fixture_public()),
            Err(Failure::SignatureMismatch)
        );
    }

    #[test]
    fn a_signature_of_other_bytes_is_rejected() {
        let signature = sign(&fixture_key(), V, b"a different archive");

        assert_eq!(
            verify_archive(b"the archive", V, &sha256_hex(b"the archive"), &signature, &fixture_public()),
            Err(Failure::SignatureMismatch)
        );
    }

    #[test]
    fn a_genuine_old_archive_cannot_pass_as_a_newer_version() {
        let bytes = b"version 1.2.3 of the app";
        let signature = sign(&fixture_key(), V, bytes);

        assert_eq!(
            verify_archive(bytes, Version::new(9, 9, 9), &sha256_hex(bytes), &signature, &fixture_public()),
            Err(Failure::SignatureMismatch)
        );
    }

    #[test]
    fn an_unreadable_hash_or_signature_is_missing_not_a_pass() {
        let bytes = b"the archive";
        let key = fixture_public();
        let signature = sign(&fixture_key(), V, bytes);

        assert_eq!(verify_archive(bytes, V, "", &signature, &key), Err(Failure::MissingChecksum));
        assert_eq!(verify_archive(bytes, V, "not hex", &signature, &key), Err(Failure::MissingChecksum));
        assert_eq!(verify_archive(bytes, V, &sha256_hex(bytes), "", &key), Err(Failure::MissingSignature));
        assert_eq!(verify_archive(bytes, V, &sha256_hex(bytes), "abcd", &key), Err(Failure::MissingSignature));
    }

    #[test]
    fn key_files_may_have_comments_and_must_hold_one_valid_key() {
        let key = fixture_public();
        let text = format!("# a comment\n\n{}\n", hex(key.as_bytes()));
        assert_eq!(parse_key(&text), Ok(key));
        assert!(parse_key("# only a comment\n").is_err());
        assert!(parse_key("zz").is_err());
        assert!(parse_key(&"ab".repeat(31)).is_err());
    }

    #[test]
    fn a_key_file_marked_as_the_placeholder_is_never_trusted() {
        let key = hex(fixture_public().as_bytes());

        assert!(parse_trusted_key(&format!("# DEVELOPMENT KEY\n{key}\n")).unwrap_err().contains("placeholder"));
        assert_eq!(parse_trusted_key(&format!("# Ed25519 public key\n{key}\n")), Ok(fixture_public()));
    }

    #[test]
    fn the_key_file_in_the_repository_parses_and_is_never_the_test_key() {
        assert_ne!(parse_key(EMBEDDED_KEY).unwrap(), fixture_public(), "a key whose private half is public must not ship");
    }

    #[test]
    fn hex_decoding_is_strict() {
        assert_eq!(decode_hex("00ff10"), Some(vec![0, 255, 16]));
        assert_eq!(decode_hex("0"), None);
        assert_eq!(decode_hex("0g"), None);
        assert_eq!(decode_hex("é0"), None);
    }
}
