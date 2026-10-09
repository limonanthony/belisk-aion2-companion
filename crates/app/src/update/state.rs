use std::fmt;
use std::time::SystemTime;

use crate::changelog::{Date, Version};

/// A release the player could move to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: Version,
    /// Bytes of the update archive.
    pub size: u64,
    /// The release's page on GitHub, for "Release notes".
    pub notes_url: String,
    /// The day GitHub published it, when its answer said.
    pub published: Option<Date>,
}

/// Why a step of an update did not work; the window words each kind in the
/// player's language, `Display` is the English text for the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Check(String),
    Download(String),
    ChecksumMismatch,
    SignatureMismatch,
    MissingChecksum,
    MissingSignature,
    Stage(String),
    Restart(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Check(detail) => write!(f, "the check for updates failed: {detail}"),
            Self::Download(detail) => write!(f, "the download failed: {detail}"),
            Self::ChecksumMismatch => f.write_str("the download does not match its SHA-256"),
            Self::SignatureMismatch => f.write_str("the download's signature does not match"),
            Self::MissingChecksum => f.write_str("the release has no usable SHA-256 file"),
            Self::MissingSignature => f.write_str("the release has no usable signature file"),
            Self::Stage(detail) => write!(f, "the update could not be unpacked: {detail}"),
            Self::Restart(detail) => write!(f, "the restart could not start: {detail}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    /// This build has no repository to ask (`UPDATE_REPO` was not set when it
    /// was built), or it is not a live run: nothing is ever contacted.
    Disabled,
    UpToDate { checked_at: SystemTime },
    /// A check is running, shown only when no update is known; while one is
    /// available or staged the hourly check stays out of sight.
    Checking,
    /// A newer release exists. `last_failure` is why the last download of it
    /// was thrown away, if it was.
    Available { release: ReleaseInfo, last_failure: Option<Failure> },
    Downloading { version: Version, downloaded: u64, total: u64, paused_for_fight: bool },
    /// The download is complete; its hash and signature are being checked.
    Verifying { version: Version },
    /// Verified and unpacked beside the exe: a restart puts it in place.
    Ready { version: Version },
    /// The answer could not be used, or the restart could not start.
    Failed { failure: Failure },
    /// GitHub could not be reached; the next attempt is the next hourly check.
    Unreachable { next_try: SystemTime },
    /// GitHub asked to wait until `until`.
    RateLimited { until: SystemTime },
}

/// What the window shows: the state, and what its "Check now" needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub state: UpdateState,
    /// When GitHub last answered a check, successfully or not.
    pub last_checked: Option<SystemTime>,
    /// The earliest time "Check now" does anything.
    pub check_again_at: SystemTime,
}

impl Snapshot {
    pub fn disabled() -> Self {
        Self { state: UpdateState::Disabled, last_checked: None, check_again_at: SystemTime::UNIX_EPOCH }
    }
}
