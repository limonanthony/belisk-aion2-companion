/// `owner/name` of the GitHub repository whose releases are the updates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    owner: String,
    name: String,
}

impl Repo {
    /// The repository this build was made for: the `UPDATE_REPO` environment
    /// variable at build time (`owner/name`, set by CI from the repository it
    /// runs in). Without one, or with one that is not `owner/name`, updates
    /// are off in this build.
    pub fn of_this_build() -> Option<Self> {
        let configured = option_env!("UPDATE_REPO")?;
        match configured.parse() {
            Ok(repo) => Some(repo),
            Err(error) => {
                crate::app_log::warn(format!("updates are off: UPDATE_REPO is not usable: {error}"));
                None
            }
        }
    }

    pub fn latest_release_url(&self) -> String {
        format!("https://api.github.com/repos/{}/{}/releases/latest", self.owner, self.name)
    }

    /// The release's page on GitHub, for "Release notes": built here from the
    /// repository and the version, never taken from what GitHub's answer says,
    /// so the button can only open a page of this repository.
    pub fn release_page_url(&self, version: crate::changelog::Version) -> String {
        format!("https://github.com/{}/{}/releases/tag/v{version}", self.owner, self.name)
    }
}

impl std::str::FromStr for Repo {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (owner, name) = text.split_once('/').ok_or_else(|| format!("{text:?} is not owner/name"))?;
        for part in [owner, name] {
            let plain = !part.is_empty()
                && part != "."
                && part != ".."
                && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
            if !plain {
                return Err(format!("{text:?} is not owner/name"));
            }
        }
        Ok(Self { owner: owner.to_owned(), name: name.to_owned() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_is_owner_slash_name_and_builds_the_latest_release_url() {
        let repo: Repo = "some-owner/Some_Repo.rs".parse().unwrap();
        assert_eq!(repo.latest_release_url(), "https://api.github.com/repos/some-owner/Some_Repo.rs/releases/latest");
    }

    #[test]
    fn the_release_page_is_this_repositorys_tag_page_whatever_the_answer_said() {
        let repo: Repo = "o/r".parse().unwrap();
        assert_eq!(repo.release_page_url(crate::changelog::Version::new(1, 4, 0)), "https://github.com/o/r/releases/tag/v1.4.0");
    }

    #[test]
    fn anything_that_could_change_the_url_is_refused() {
        for bad in ["", "owner", "owner/", "/name", "a/b/c", "a b/c", "a/../b", "../x", "a/b?x=1", "a/b#c", "a\\b/c", "a/.."] {
            assert!(bad.parse::<Repo>().is_err(), "{bad:?}");
        }
    }
}
