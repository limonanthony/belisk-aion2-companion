use serde::Deserialize;

use super::repo::Repo;
use super::state::ReleaseInfo;
use crate::changelog::{Date, Version};

/// What the update archive of a release is called, after its version:
/// `belisk-1.2.3-windows-x64-update.zip`. Its hash and signature are
/// the assets of the same name plus `.sha256` and `.sig`.
pub const ARCHIVE_SUFFIX: &str = "-windows-x64-update.zip";
pub const CHECKSUM_SUFFIX: &str = ".sha256";
pub const SIGNATURE_SUFFIX: &str = ".sig";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub info: ReleaseInfo,
    pub archive_url: String,
    pub checksum_url: Option<String>,
    pub signature_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Latest {
    /// Nothing newer than the running version, or nothing installable.
    NothingNewer,
    Newer(Release),
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    /// `2026-10-21T09:30:00Z`; only the day is read.
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// `v1.2.3` is version 1.2.3; anything else (no `v`, a prerelease suffix, a
/// fourth number) is not a release of this app.
pub fn parse_tag(tag: &str) -> Result<Version, String> {
    let number = tag.strip_prefix('v').ok_or_else(|| format!("tag {tag:?} does not start with v"))?;
    number.parse().map_err(|error| format!("tag {tag:?}: {error}"))
}

/// Reads GitHub's `releases/latest` answer.
pub fn read_latest(body: &[u8], running: Version, repo: &Repo) -> Result<Latest, String> {
    let release: ApiRelease =
        serde_json::from_slice(body).map_err(|error| format!("the release answer is not what GitHub documents: {error}"))?;
    if release.draft || release.prerelease {
        return Ok(Latest::NothingNewer);
    }
    let version = parse_tag(&release.tag_name)?;
    if version <= running {
        return Ok(Latest::NothingNewer);
    }

    let archive_name = format!("belisk-{version}{ARCHIVE_SUFFIX}");
    let asset = |name: &str| release.assets.iter().find(|asset| asset.name == name);
    let Some(archive) = asset(&archive_name) else {
        crate::app_log::warn(format!("release {} has no {archive_name}; it is not offered", release.tag_name));
        return Ok(Latest::NothingNewer);
    };
    Ok(Latest::Newer(Release {
        info: ReleaseInfo {
            version,
            size: archive.size,
            notes_url: repo.release_page_url(version),
            published: release.published_at.as_deref().and_then(|at| at.get(..10)).and_then(Date::parse),
        },
        archive_url: archive.browser_download_url.clone(),
        checksum_url: asset(&format!("{archive_name}{CHECKSUM_SUFFIX}")).map(|asset| asset.browser_download_url.clone()),
        signature_url: asset(&format!("{archive_name}{SIGNATURE_SUFFIX}")).map(|asset| asset.browser_download_url.clone()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNNING: Version = Version::new(1, 0, 0);

    fn repo() -> Repo {
        "o/r".parse().unwrap()
    }

    fn answer(tag: &str, extra: &str) -> Vec<u8> {
        let name = format!("belisk-{}{ARCHIVE_SUFFIX}", tag.trim_start_matches('v'));
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/o/r/releases/tag/{tag}","draft":false,"prerelease":false,
            "assets":[
              {{"name":"{name}","size":4200000,"browser_download_url":"https://example.test/{name}"}},
              {{"name":"{name}.sha256","size":90,"browser_download_url":"https://example.test/{name}.sha256"}},
              {{"name":"{name}.sig","size":128,"browser_download_url":"https://example.test/{name}.sig"}}
            ]{extra}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn a_v_tag_is_a_version_and_anything_else_is_refused() {
        assert_eq!(parse_tag("v1.2.3"), Ok(Version::new(1, 2, 3)));
        for bad in ["1.2.3", "v1.2", "v1.2.3-beta.1", "v1.2.3.4", "V1.2.3", "v", "vx.y.z", ""] {
            assert!(parse_tag(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_newer_release_carries_its_archive_hash_signature_and_notes_page() {
        let Latest::Newer(release) = read_latest(&answer("v1.4.0", ""), RUNNING, &repo()).unwrap() else { panic!("expected an update") };
        assert_eq!(release.info.version, Version::new(1, 4, 0));
        assert_eq!(release.info.size, 4_200_000);
        assert_eq!(release.info.notes_url, "https://github.com/o/r/releases/tag/v1.4.0");
        assert_eq!(release.archive_url, "https://example.test/belisk-1.4.0-windows-x64-update.zip");
        assert!(release.checksum_url.unwrap().ends_with(".zip.sha256"));
        assert!(release.signature_url.unwrap().ends_with(".zip.sig"));
    }

    #[test]
    fn the_release_day_is_read_from_published_at_when_github_gives_one() {
        let dated = answer("v1.4.0", r#","published_at":"2026-10-21T09:30:00Z""#);
        let Latest::Newer(release) = read_latest(&dated, RUNNING, &repo()).unwrap() else { panic!("expected an update") };
        assert_eq!(release.info.published, Some(Date { year: 2026, month: 10, day: 21 }));

        let Latest::Newer(undated) = read_latest(&answer("v1.4.0", ""), RUNNING, &repo()).unwrap() else { panic!("expected an update") };
        assert_eq!(undated.info.published, None);
    }

    #[test]
    fn the_notes_page_is_this_repositorys_whatever_html_url_says() {
        let hostile = String::from_utf8(answer("v1.4.0", ""))
            .unwrap()
            .replace("https://github.com/o/r/releases/tag/v1.4.0", "https://evil.example/phish");

        let Latest::Newer(release) = read_latest(hostile.as_bytes(), RUNNING, &repo()).unwrap() else { panic!("expected an update") };

        assert_eq!(release.info.notes_url, "https://github.com/o/r/releases/tag/v1.4.0");
    }

    #[test]
    fn the_running_version_and_older_ones_are_nothing_new() {
        assert_eq!(read_latest(&answer("v1.0.0", ""), RUNNING, &repo()).unwrap(), Latest::NothingNewer);
        assert_eq!(read_latest(&answer("v0.9.9", ""), RUNNING, &repo()).unwrap(), Latest::NothingNewer);
    }

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(matches!(read_latest(&answer("v1.10.0", ""), Version::new(1, 9, 0), &repo()).unwrap(), Latest::Newer(_)));
    }

    #[test]
    fn drafts_and_prereleases_are_never_offered() {
        let draft = String::from_utf8(answer("v2.0.0", "")).unwrap().replace("\"draft\":false", "\"draft\":true");
        let pre = String::from_utf8(answer("v2.0.0", "")).unwrap().replace("\"prerelease\":false", "\"prerelease\":true");
        assert_eq!(read_latest(draft.as_bytes(), RUNNING, &repo()).unwrap(), Latest::NothingNewer);
        assert_eq!(read_latest(pre.as_bytes(), RUNNING, &repo()).unwrap(), Latest::NothingNewer);
    }

    #[test]
    fn a_release_without_the_update_archive_is_not_offered() {
        let body = br#"{"tag_name":"v3.0.0","html_url":"https://x","assets":[{"name":"other.zip","size":1,"browser_download_url":"https://x/o"}]}"#;
        assert_eq!(read_latest(body, RUNNING, &repo()).unwrap(), Latest::NothingNewer);
    }

    #[test]
    fn a_missing_hash_or_signature_asset_is_carried_as_missing() {
        let body = br#"{"tag_name":"v1.1.0","html_url":"https://x","assets":[{"name":"belisk-1.1.0-windows-x64-update.zip","size":5,"browser_download_url":"https://x/a"}]}"#;
        let Latest::Newer(release) = read_latest(body, RUNNING, &repo()).unwrap() else { panic!("expected an update") };
        assert!(release.checksum_url.is_none() && release.signature_url.is_none());
    }

    #[test]
    fn an_answer_that_is_not_a_release_or_has_a_bad_tag_is_an_error() {
        assert!(read_latest(b"<html>", RUNNING, &repo()).is_err());
        assert!(read_latest(br#"{"message":"Not Found"}"#, RUNNING, &repo()).is_err());
        assert!(read_latest(&answer("nightly", ""), RUNNING, &repo()).is_err());
    }
}
