use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::crates::APP_PACKAGE;
use crate::Result;

const RELEASE_REF: &str = "refs/heads/main";

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    Publish,
    NotMain,
    TagExists,
}

impl Decision {
    fn notice(&self, git_ref: &str, version: &str) -> String {
        match self {
            Self::Publish => format!("Tag v{version} does not exist yet: this push releases {version}."),
            Self::NotMain => format!("{git_ref} is not main: build and test only, nothing is published."),
            Self::TagExists => format!(
                "Tag v{version} already exists: build and test only, nothing is published. Bump the version in crates/app/Cargo.toml to release."
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Plan {
    version: String,
    prerelease: bool,
    publish: bool,
}

impl Plan {
    fn lines(&self) -> String {
        format!("version={}\nprerelease={}\npublish={}\n", self.version, self.prerelease, self.publish)
    }
}

pub fn run(root: &Path) -> Result<()> {
    let version = app_version(root)?;
    validate_version(&version)?;
    let git_ref = current_ref(root)?;
    let decision = decide(&git_ref, || tag_exists(root, &version))?;
    println!("::notice::{}", decision.notice(&git_ref, &version));
    let plan = Plan {
        prerelease: is_prerelease(&version),
        publish: decision == Decision::Publish,
        version,
    };
    match std::env::var_os("GITHUB_OUTPUT") {
        Some(path) => append_outputs(Path::new(&path), &plan)?,
        None => print!("{}", plan.lines()),
    }
    Ok(())
}

fn app_version(root: &Path) -> Result<String> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr).trim()).into());
    }
    version_in_metadata(&serde_json::from_slice(&output.stdout)?)
}

fn version_in_metadata(metadata: &Value) -> Result<String> {
    let package = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages")?
        .iter()
        .find(|package| package["name"] == APP_PACKAGE)
        .ok_or_else(|| format!("package {APP_PACKAGE} not found in the workspace"))?;
    Ok(package["version"].as_str().ok_or("the app's version is not a string")?.to_owned())
}

fn validate_version(version: &str) -> Result<()> {
    let (core, suffix) = match version.split_once('-') {
        Some((core, suffix)) => (core, Some(suffix)),
        None => (version, None),
    };
    let numbers: Vec<&str> = core.split('.').collect();
    let is_number = |part: &&str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let suffix_is_valid = suffix.is_none_or(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
    });
    if numbers.len() == 3 && numbers.iter().all(is_number) && suffix_is_valid {
        Ok(())
    } else {
        Err(format!("Cannot read a version like 1.2.3 or 1.2.3-beta.1 from crates/app/Cargo.toml (got '{version}').").into())
    }
}

fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

fn current_ref(root: &Path) -> Result<String> {
    if let Ok(git_ref) = std::env::var("GITHUB_REF") {
        return Ok(git_ref);
    }
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Ok("(detached HEAD)".to_owned())
    }
}

fn decide(git_ref: &str, tag_exists: impl FnOnce() -> Result<bool>) -> Result<Decision> {
    if git_ref != RELEASE_REF {
        return Ok(Decision::NotMain);
    }
    Ok(if tag_exists()? { Decision::TagExists } else { Decision::Publish })
}

/// An unreachable remote is a failure, not "no tag": reading it as "no tag" could release a version that is already
/// published, and reading it as "tag exists" would skip a release without saying why.
fn tag_exists(root: &Path, version: &str) -> Result<bool> {
    let tag = format!("refs/tags/v{version}");
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "origin", &tag])
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not run git ls-remote: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Cannot tell whether tag v{version} exists: `git ls-remote --tags origin {tag}` failed ({}). Re-run once the remote is reachable.",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(!output.stdout.iter().all(u8::is_ascii_whitespace))
}

fn append_outputs(path: &Path, plan: &Plan) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|error| format!("could not open GITHUB_OUTPUT {}: {error}", path.display()))?;
    file.write_all(plan.lines().as_bytes())
        .map_err(|error| format!("could not write GITHUB_OUTPUT {}: {error}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("belisk-xtask-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn reads_the_app_version_from_cargo_metadata() {
        let metadata = serde_json::json!({"packages": [
            {"name": "xtask", "version": "0.0.0"},
            {"name": "belisk", "version": "1.0.1"},
        ]});
        assert_eq!(version_in_metadata(&metadata).unwrap(), "1.0.1");
    }

    #[test]
    fn a_workspace_without_the_app_has_no_version() {
        let metadata = serde_json::json!({"packages": [{"name": "xtask", "version": "0.0.0"}]});
        assert!(version_in_metadata(&metadata).unwrap_err().to_string().contains("belisk"));
    }

    #[test]
    fn reads_the_real_app_version() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap();
        let version = app_version(root).unwrap();
        validate_version(&version).unwrap();
    }

    #[test]
    fn accepts_release_and_prerelease_versions() {
        for version in ["1.0.1", "10.20.30", "1.2.3-beta.1", "1.2.3-rc-1"] {
            validate_version(version).unwrap_or_else(|error| panic!("{version}: {error}"));
        }
    }

    #[test]
    fn rejects_versions_that_are_not_three_numbers_and_an_optional_suffix() {
        for version in ["", "1", "1.2", "1.2.3.4", "1.2.x", "v1.2.3", "1.2.3-", "1.2.3+build", "1.2.3-be ta", "1..3"] {
            assert!(validate_version(version).is_err(), "{version:?} should be refused");
        }
    }

    #[test]
    fn a_suffix_makes_a_prerelease() {
        assert!(!is_prerelease("1.0.1"));
        assert!(is_prerelease("1.1.0-beta.1"));
    }

    #[test]
    fn publishes_on_main_when_the_tag_is_missing() {
        assert_eq!(decide("refs/heads/main", || Ok(false)).unwrap(), Decision::Publish);
    }

    #[test]
    fn does_not_publish_on_main_when_the_tag_exists() {
        assert_eq!(decide("refs/heads/main", || Ok(true)).unwrap(), Decision::TagExists);
    }

    #[test]
    fn does_not_publish_off_main_and_never_asks_the_remote() {
        for git_ref in ["refs/heads/dev", "refs/pull/7/merge", "(detached HEAD)", "refs/heads/main2"] {
            let decision = decide(git_ref, || panic!("the remote must not be asked off main")).unwrap();
            assert_eq!(decision, Decision::NotMain, "{git_ref}");
        }
    }

    #[test]
    fn an_unreachable_remote_fails_the_decision() {
        let failure = decide("refs/heads/main", || Err("remote unreachable".into())).unwrap_err();
        assert_eq!(failure.to_string(), "remote unreachable");
    }

    #[test]
    fn notices_say_why_nothing_is_published() {
        assert_eq!(
            Decision::NotMain.notice("refs/heads/dev", "1.0.2"),
            "refs/heads/dev is not main: build and test only, nothing is published."
        );
        assert!(Decision::TagExists.notice("refs/heads/main", "1.0.2").starts_with("Tag v1.0.2 already exists"));
        assert_eq!(
            Decision::Publish.notice("refs/heads/main", "1.0.2"),
            "Tag v1.0.2 does not exist yet: this push releases 1.0.2."
        );
    }

    #[test]
    fn appends_the_plan_to_the_github_output_file() {
        let path = temp_file("github-output");
        std::fs::write(&path, "earlier=1\n").unwrap();
        let plan = Plan { version: "1.2.3-beta.1".into(), prerelease: true, publish: false };
        append_outputs(&path, &plan).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "earlier=1\nversion=1.2.3-beta.1\nprerelease=true\npublish=false\n"
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn creates_the_github_output_file_when_missing() {
        let path = temp_file("github-output-new");
        let plan = Plan { version: "1.0.1".into(), prerelease: false, publish: true };
        append_outputs(&path, &plan).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "version=1.0.1\nprerelease=false\npublish=true\n");
        std::fs::remove_file(&path).unwrap();
    }
}
