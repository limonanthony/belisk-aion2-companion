//! The toolbar's update icon (approved `toolbar.html`, tokens.md "Toolbar
//! update icon"): what it shows for each state of the updates, and its
//! tooltip. It is there only while there is something to do or know.

use crate::changelog::Version;
use crate::i18n::fl;
use crate::update::{Failure, UpdateState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateIcon {
    /// Also after a failed download, which shows the same look.
    Available { version: Version, last_failure: Option<Failure> },
    Downloading { version: Version, percent: u8, paused_for_fight: bool },
    Verifying { version: Version },
    Ready { version: Version },
}

/// What a click on the icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateClick {
    StartDownload,
    ShowDetails,
    Restart,
}

impl UpdateIcon {
    /// None while up to date, checking, offline, limited, failed or off: those
    /// are told in Settings › About only.
    pub fn of(state: &UpdateState) -> Option<Self> {
        Some(match state {
            UpdateState::Available { release, last_failure } => {
                Self::Available { version: release.version, last_failure: last_failure.clone() }
            }
            &UpdateState::Downloading { version, downloaded, total, paused_for_fight } => {
                Self::Downloading { version, percent: whole_percent(downloaded, total), paused_for_fight }
            }
            &UpdateState::Verifying { version } => Self::Verifying { version },
            &UpdateState::Ready { version } => Self::Ready { version },
            UpdateState::Disabled
            | UpdateState::UpToDate { .. }
            | UpdateState::Checking
            | UpdateState::Failed { .. }
            | UpdateState::Unreachable { .. }
            | UpdateState::RateLimited { .. } => return None,
        })
    }

    pub fn click(&self) -> UpdateClick {
        match self {
            Self::Available { .. } => UpdateClick::StartDownload,
            Self::Downloading { .. } | Self::Verifying { .. } => UpdateClick::ShowDetails,
            Self::Ready { .. } => UpdateClick::Restart,
        }
    }

    /// The progress line's fill, 0 to 1, and whether it is paused; none
    /// without a download under way.
    pub fn progress(&self) -> Option<(f32, bool)> {
        match *self {
            Self::Downloading { percent, paused_for_fight, .. } => Some((f32::from(percent) / 100., paused_for_fight)),
            Self::Verifying { .. } => Some((1., false)),
            Self::Available { .. } | Self::Ready { .. } => None,
        }
    }

    /// The tooltip's two lines; the first is the button's accessible name.
    pub fn tip(&self, in_fight: bool) -> (String, String) {
        match self {
            Self::Available { version, last_failure } => {
                let line = match last_failure {
                    None => fl!("toolbar-update-available-line"),
                    Some(Failure::ChecksumMismatch | Failure::SignatureMismatch) => fl!("toolbar-update-failed-line"),
                    Some(_) => fl!("toolbar-update-retry-line"),
                };
                (fl!("toolbar-update-available", version = version.to_string()), line)
            }
            Self::Downloading { version, percent, paused_for_fight: false } => (
                fl!("toolbar-update-downloading", version = version.to_string(), percent = percent_words(*percent)),
                fl!("toolbar-update-downloading-line"),
            ),
            Self::Downloading { percent, paused_for_fight: true, .. } => {
                (fl!("toolbar-update-paused", percent = percent_words(*percent)), fl!("toolbar-update-paused-line"))
            }
            Self::Verifying { version } => {
                (fl!("toolbar-update-verifying", version = version.to_string()), fl!("toolbar-update-verifying-line"))
            }
            Self::Ready { version } => {
                let line = if in_fight { fl!("toolbar-update-ready-fight-line") } else { fl!("toolbar-update-ready-line") };
                (fl!("toolbar-update-ready", version = version.to_string()), line)
            }
        }
    }
}

/// Whole percent of a download, never past 100; 0 while its size is unknown.
fn whole_percent(done: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    (done.min(total) * 100 / total) as u8
}

/// "62%" (French: "62 %").
pub fn percent_words(percent: u8) -> String {
    fl!("unit-percent", value = percent.to_string())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::i18n::{set_language, Language};
    use crate::update::ReleaseInfo;

    const NEXT: Version = Version::new(1, 6, 0);

    fn available(last_failure: Option<Failure>) -> UpdateState {
        let release = ReleaseInfo { version: NEXT, size: 4_200_000, notes_url: String::new(), published: None };
        UpdateState::Available { release, last_failure }
    }

    fn downloading(paused_for_fight: bool) -> UpdateState {
        UpdateState::Downloading { version: NEXT, downloaded: 2_604_000, total: 4_200_000, paused_for_fight }
    }

    #[test]
    fn the_icon_shows_only_while_there_is_something_to_do_or_know() {
        let now = SystemTime::now();
        for quiet in [
            UpdateState::Disabled,
            UpdateState::UpToDate { checked_at: now },
            UpdateState::Checking,
            UpdateState::Failed { failure: Failure::Check("bad answer".into()) },
            UpdateState::Unreachable { next_try: now },
            UpdateState::RateLimited { until: now },
        ] {
            assert_eq!(UpdateIcon::of(&quiet), None, "{quiet:?}");
        }
        for shown in [
            available(None),
            available(Some(Failure::SignatureMismatch)),
            downloading(false),
            downloading(true),
            UpdateState::Verifying { version: NEXT },
            UpdateState::Ready { version: NEXT },
        ] {
            assert!(UpdateIcon::of(&shown).is_some(), "{shown:?}");
        }
    }

    #[test]
    fn a_click_downloads_an_available_update_opens_the_details_of_a_busy_one_and_restarts_a_ready_one() {
        let click = |state: UpdateState| UpdateIcon::of(&state).unwrap().click();
        assert_eq!(click(available(None)), UpdateClick::StartDownload);
        assert_eq!(click(available(Some(Failure::ChecksumMismatch))), UpdateClick::StartDownload);
        assert_eq!(click(downloading(false)), UpdateClick::ShowDetails);
        assert_eq!(click(downloading(true)), UpdateClick::ShowDetails);
        assert_eq!(click(UpdateState::Verifying { version: NEXT }), UpdateClick::ShowDetails);
        assert_eq!(click(UpdateState::Ready { version: NEXT }), UpdateClick::Restart);
    }

    #[test]
    fn the_progress_line_follows_the_download_dims_when_paused_and_is_full_while_checking() {
        let progress = |state: UpdateState| UpdateIcon::of(&state).unwrap().progress();
        assert_eq!(progress(downloading(false)), Some((0.62, false)));
        assert_eq!(progress(downloading(true)), Some((0.62, true)));
        assert_eq!(progress(UpdateState::Verifying { version: NEXT }), Some((1., false)));
        assert_eq!(progress(available(None)), None);
        assert_eq!(progress(UpdateState::Ready { version: NEXT }), None);
        let unknown_size = UpdateState::Downloading { version: NEXT, downloaded: 10, total: 0, paused_for_fight: false };
        assert_eq!(progress(unknown_size), Some((0., false)));
    }

    #[test]
    fn the_tooltips_read_as_drawn_and_warn_about_restarting_during_a_fight() {
        set_language(Language::English);
        let tip = |state: UpdateState, in_fight: bool| UpdateIcon::of(&state).unwrap().tip(in_fight);
        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());

        assert_eq!(
            tip(available(None), false),
            pair("Update available: 1.6.0", "Click to download it. It installs only when you restart.")
        );
        assert_eq!(
            tip(available(Some(Failure::SignatureMismatch)), false),
            pair("Update available: 1.6.0", "The last download failed its check and was deleted. Click to try again.")
        );
        assert_eq!(
            tip(available(Some(Failure::Download("reset".into()))), false).1,
            "The last download did not work. Click to try again."
        );
        assert_eq!(
            tip(downloading(false), false),
            pair("Downloading 1.6.0 · 62%", "Speed-capped, paused while you fight. Click for details.")
        );
        assert_eq!(tip(downloading(true), true), pair("Download paused · 62%", "Resumes when the fight ends. Click for details."));
        assert_eq!(
            tip(UpdateState::Verifying { version: NEXT }, false),
            pair("Checking 1.6.0…", "Making sure the download is complete and genuine.")
        );
        assert_eq!(
            tip(UpdateState::Ready { version: NEXT }, false),
            pair("Restart to update to 1.6.0", "Closes and opens again in the new version. Nothing is lost.")
        );
        assert_eq!(
            tip(UpdateState::Ready { version: NEXT }, true),
            pair("Restart to update to 1.6.0", "You are fighting: restarting now splits this fight in History.")
        );

        set_language(Language::French);
        assert_eq!(tip(downloading(false), false).0, "Téléchargement de 1.6.0 · 62\u{202f}%");
        assert_eq!(tip(UpdateState::Ready { version: NEXT }, true).0, "Redémarrer pour passer à 1.6.0");
    }
}
