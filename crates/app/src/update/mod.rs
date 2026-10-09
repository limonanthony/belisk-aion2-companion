//! Updates from GitHub Releases (docs/updates.md).
//!
//! `service` is the one entity the UI observes ([`Updates`]); `engine` is the
//! worker thread's logic: `release` reads GitHub's answer, `schedule` decides
//! when to ask, `http` is the network, `throttle` caps the download, `verify`
//! checks hash and signature, `stage` keeps the update beside the exe and
//! `apply` puts it in place after a restart.
//!
//! Only GitHub is contacted, only to read the latest release and download the
//! update archive; nothing about the player or the game is sent.

mod apply;
mod engine;
mod http;
mod package;
mod release;
mod repo;
mod schedule;
mod service;
mod settings;
mod stage;
mod state;
mod throttle;
mod verify;

pub use service::{global, install, Updates};
#[cfg(test)]
pub use engine::Command;
#[cfg(test)]
pub use service::install_recording_fixture;
#[cfg(any(test, feature = "probes"))]
pub use service::install_fixture;
pub use settings::UpdateSettings;
#[cfg(any(test, feature = "probes"))]
pub use state::ReleaseInfo;
pub use state::{Failure, Snapshot, UpdateState};

/// When this process was started by an update's restart to put the staged
/// files in place (`--apply-update ...`), does that and returns the exit code;
/// `None` for an ordinary start. It runs before any window, setting or capture
/// exists. It logs to the temp folder: it runs before the data folders are
/// settled, and creating the player's log folder here would make the next
/// launch find both the old and the new one.
pub fn apply_if_requested(args: &[String]) -> Option<u32> {
    let request = apply::Request::from_args(args)?;
    crate::app_log::install(false);
    Some(match request {
        Ok(request) => apply::run(&request),
        Err(error) => {
            crate::app_log::error(error);
            2
        }
    })
}
