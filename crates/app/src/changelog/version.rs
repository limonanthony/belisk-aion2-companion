use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A release number, `major.minor.patch`; ordered the way releases are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionError(String);

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a version like 1.2.3", self.0)
    }
}

impl std::error::Error for VersionError {}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// The version of this build, from Cargo's package version; a prerelease or
    /// build suffix (`1.2.3-beta.1`) is not part of the number.
    pub fn current() -> Self {
        let package = env!("CARGO_PKG_VERSION");
        let number = package.split(['-', '+']).next().unwrap_or(package);
        number.parse().expect("Cargo's package version starts with major.minor.patch")
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split('.');
        let mut next = || {
            let part = parts.next()?;
            let digits_only = !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
            if digits_only { part.parse::<u32>().ok() } else { None }
        };
        let parsed = (next(), next(), next(), parts.next());
        match parsed {
            (Some(major), Some(minor), Some(patch), None) => Ok(Self { major, minor, patch }),
            _ => Err(VersionError(text.to_owned())),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl TryFrom<String> for Version {
    type Error = VersionError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Version> for String {
    fn from(version: Version) -> Self {
        version.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_by_number_not_by_text() {
        assert!(Version::new(1, 10, 0) > Version::new(1, 9, 9));
        assert!(Version::new(2, 0, 0) > Version::new(1, 99, 99));
        assert_eq!(Version::new(1, 0, 0), "1.0.0".parse().unwrap());
    }

    #[test]
    fn only_major_minor_patch_numbers_parse() {
        for bad in ["", "1", "1.0", "1.0.0.0", "1.0.x", "1.0.-1", "v1.0.0", "1.0.0-beta", " 1.0.0", "1..0"] {
            assert!(bad.parse::<Version>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn this_build_has_a_version() {
        assert_eq!(Version::current().to_string(), env!("CARGO_PKG_VERSION").split(['-', '+']).next().unwrap());
    }

    #[test]
    fn a_version_is_stored_as_text() {
        assert_eq!(serde_json::to_string(&Version::new(1, 2, 3)).unwrap(), "\"1.2.3\"");
        assert_eq!(serde_json::from_str::<Version>("\"1.2.3\"").unwrap(), Version::new(1, 2, 3));
        assert!(serde_json::from_str::<Version>("\"one\"").is_err());
    }
}
