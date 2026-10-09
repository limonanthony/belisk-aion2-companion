use super::parser::{Changelog, Release};
use super::version::Version;

/// What the What's new window lists: the versions the player has not run yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhatsNew {
    /// The last version the player ran; none for a settings file from before
    /// versions were remembered.
    pub since: Option<Version>,
    pub current: Version,
    /// Newest first, never empty, every one with bullets.
    pub releases: Vec<Release>,
}

impl WhatsNew {
    /// The versions newer than `last_seen`, up to `current`, skipped ones
    /// included. A version with no bullets is not listed, and when nothing is
    /// left there is nothing to announce. No `last_seen` counts as older than
    /// every version.
    pub fn select(changelog: &Changelog, last_seen: Option<Version>, current: Version) -> Option<Self> {
        let releases: Vec<Release> = changelog
            .releases
            .iter()
            .filter(|release| {
                release.version <= current
                    && last_seen.is_none_or(|seen| release.version > seen)
                    && !release.bullets.is_empty()
            })
            .cloned()
            .collect();
        (!releases.is_empty()).then_some(Self { since: last_seen, current, releases })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::parser::{Date, Span};

    fn release(major: u32, minor: u32, patch: u32, bullets: usize) -> Release {
        Release {
            version: Version::new(major, minor, patch),
            date: Date { year: 2026, month: 10, day: 9 },
            summary: vec![Span { text: "Summary.".into(), bold: false }],
            bullets: (0..bullets).map(|n| vec![Span { text: format!("Bullet {n}."), bold: false }]).collect(),
        }
    }

    fn history() -> Changelog {
        Changelog { releases: vec![release(1, 3, 0, 2), release(1, 2, 0, 0), release(1, 1, 0, 1), release(1, 0, 0, 3)] }
    }

    fn versions(shown: &Option<WhatsNew>) -> Vec<String> {
        shown.as_ref().map_or_else(Vec::new, |shown| shown.releases.iter().map(|r| r.version.to_string()).collect())
    }

    #[test]
    fn an_older_build_with_no_remembered_version_gets_the_first_notes_once() {
        let shown = WhatsNew::select(&history(), None, Version::new(1, 0, 0));
        assert_eq!(versions(&shown), ["1.0.0"]);
        assert_eq!(shown.unwrap().since, None);
    }

    #[test]
    fn no_remembered_version_counts_as_older_than_every_version_up_to_the_current() {
        let shown = WhatsNew::select(&history(), None, Version::new(1, 3, 0));
        assert_eq!(versions(&shown), ["1.3.0", "1.1.0", "1.0.0"]);
    }

    #[test]
    fn skipped_versions_are_listed_newest_first_up_to_the_current_one() {
        let shown = WhatsNew::select(&history(), Some(Version::new(1, 0, 0)), Version::new(1, 3, 0));
        assert_eq!(versions(&shown), ["1.3.0", "1.1.0"]);
        let shown = shown.unwrap();
        assert_eq!(shown.since, Some(Version::new(1, 0, 0)));
        assert_eq!(shown.current, Version::new(1, 3, 0));
    }

    #[test]
    fn versions_newer_than_the_running_build_are_not_listed() {
        let shown = WhatsNew::select(&history(), Some(Version::new(1, 0, 0)), Version::new(1, 1, 0));
        assert_eq!(versions(&shown), ["1.1.0"]);
    }

    #[test]
    fn a_version_without_bullets_announces_nothing() {
        assert_eq!(WhatsNew::select(&history(), Some(Version::new(1, 1, 0)), Version::new(1, 2, 0)), None);
        let shown = WhatsNew::select(&history(), Some(Version::new(1, 1, 0)), Version::new(1, 3, 0));
        assert_eq!(versions(&shown), ["1.3.0"]);
    }

    #[test]
    fn nothing_is_listed_once_the_current_version_has_been_seen() {
        assert_eq!(WhatsNew::select(&history(), Some(Version::new(1, 3, 0)), Version::new(1, 3, 0)), None);
        assert_eq!(WhatsNew::select(&history(), Some(Version::new(2, 0, 0)), Version::new(1, 3, 0)), None);
    }
}
