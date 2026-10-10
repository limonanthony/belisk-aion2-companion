//! What can go wrong with the JSON files the app keeps beside the core's data
//! (`settings.json`, `todos.json`, `pets.json`), and the one way they are read,
//! set aside and written: told as data, so the log says it in
//! English and Settings in the language in force. Paths and the system's or
//! the JSON reader's own errors are kept as they came.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use meter_core::mobs::store::write_atomic;
use serde::Serialize;

use crate::i18n::fl;

/// Why a stored file was not used at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadNotice {
    Unreadable { path: PathBuf, error: String },
    /// Read but not usable, and renamed to `aside`.
    SetAside { path: PathBuf, problem: Problem, aside: PathBuf },
    /// Not usable, and the rename failed too.
    NotMoved { path: PathBuf, problem: Problem, error: String },
}

/// What is wrong with a file that could be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    NotJson(String),
    NotObject,
    Newer { version: u64, supported: u32 },
    /// Valid JSON that is not what the file stores.
    WrongShape(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    Encode(String),
    Write { path: PathBuf, error: String },
}

impl LoadNotice {
    /// The notice in the language in force, then `outcome`: what the app did
    /// instead.
    pub fn text(&self, outcome: &str) -> String {
        let what = match self {
            Self::Unreadable { path, error } => {
                fl!("stored-unreadable", file = path.display().to_string(), error = error.as_str())
            }
            Self::SetAside { path, problem, aside } => fl!(
                "stored-set-aside",
                file = path.display().to_string(),
                problem = problem.text(),
                aside = aside.display().to_string()
            ),
            Self::NotMoved { path, problem, error } => fl!(
                "stored-not-moved",
                file = path.display().to_string(),
                problem = problem.text(),
                error = error.as_str()
            ),
        };
        format!("{what} {outcome}")
    }
}

impl Problem {
    fn text(&self) -> String {
        match self {
            Self::NotJson(error) => fl!("stored-not-json", error = error.as_str()),
            Self::NotObject => fl!("stored-not-object"),
            &Self::Newer { version, supported } => fl!("stored-newer", version = version, supported = supported),
            Self::WrongShape(error) => fl!("stored-wrong-shape", error = error.as_str()),
        }
    }
}

impl SaveError {
    pub fn text(&self) -> String {
        match self {
            Self::Encode(error) => fl!("stored-encode-failed", error = error.as_str()),
            Self::Write { path, error } => {
                fl!("stored-write-failed", file = path.display().to_string(), error = error.as_str())
            }
        }
    }
}

impl fmt::Display for LoadNotice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, error } => write!(f, "cannot read {}: {error}", path.display()),
            Self::SetAside { path, problem, aside } => {
                write!(f, "{} {problem}; moved to {}", path.display(), aside.display())
            }
            Self::NotMoved { path, problem, error } => {
                write!(f, "{} {problem}, and it could not be moved aside ({error})", path.display())
            }
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotJson(error) => write!(f, "is not valid JSON: {error}"),
            Self::NotObject => f.write_str("is not a JSON object"),
            Self::Newer { version, supported } => write!(f, "is schema version {version}, newer than {supported}"),
            Self::WrongShape(error) => write!(f, "cannot be read: {error}"),
        }
    }
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(error) => write!(f, "cannot encode: {error}"),
            Self::Write { path, error } => write!(f, "cannot write {}: {error}", path.display()),
        }
    }
}

/// What reading a stored file gave.
pub enum Read<T> {
    Missing,
    Used(T),
    /// Not used; the file was set aside when it could be.
    Unusable(LoadNotice),
}

/// Reads `path` and hands its bytes to `parse`. A file `parse` refuses is
/// renamed to `<name>.bad-<now_secs>` so the next save does not overwrite it.
pub fn read<T>(path: &Path, now_secs: u64, parse: impl FnOnce(&[u8]) -> Result<T, Problem>) -> Read<T> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Read::Missing,
        Err(error) => return Read::Unusable(LoadNotice::Unreadable { path: path.to_path_buf(), error: error.to_string() }),
    };
    let problem = match parse(&bytes) {
        Ok(value) => return Read::Used(value),
        Err(problem) => problem,
    };
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bad-{now_secs}"));
    let aside = path.with_file_name(name);
    Read::Unusable(match fs::rename(path, &aside) {
        Ok(()) => LoadNotice::SetAside { path: path.to_path_buf(), problem, aside },
        Err(error) => LoadNotice::NotMoved { path: path.to_path_buf(), problem, error: error.to_string() },
    })
}

/// The JSON object in `bytes` and its `version` (0 when absent), refused when
/// a newer build than `supported` wrote it.
pub fn parse_versioned_object(bytes: &[u8], supported: u32) -> Result<(serde_json::Value, u64), Problem> {
    let stored: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| Problem::NotJson(error.to_string()))?;
    let Some(fields) = stored.as_object() else {
        return Err(Problem::NotObject);
    };
    let version = fields.get("version").and_then(serde_json::Value::as_u64).unwrap_or(0);
    if version > u64::from(supported) {
        return Err(Problem::Newer { version, supported });
    }
    Ok((stored, version))
}

pub fn encode_versioned<T: Serialize>(value: &T, version: u32) -> serde_json::Result<Vec<u8>> {
    let mut value = serde_json::to_value(value)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("version".to_string(), version.into());
    }
    serde_json::to_vec_pretty(&value)
}

pub fn save_versioned<T: Serialize>(path: &Path, value: &T, version: u32) -> Result<(), SaveError> {
    let bytes = encode_versioned(value, version).map_err(|error| SaveError::Encode(error.to_string()))?;
    write_atomic(path, &bytes).map_err(|error| SaveError::Write { path: path.to_path_buf(), error: error.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{set_language, Language};

    #[test]
    fn notices_and_save_errors_read_in_the_language_in_force() {
        let set_aside = LoadNotice::SetAside {
            path: PathBuf::from("C:/data/todos.json"),
            problem: Problem::Newer { version: 99, supported: 1 },
            aside: PathBuf::from("C:/data/todos.json.bad-5"),
        };
        let write = SaveError::Write { path: PathBuf::from("C:/data/settings.json"), error: "disk full".to_string() };
        assert_eq!(
            set_aside.text("The to-dos start empty."),
            "C:/data/todos.json was written by a newer version (schema 99; this one reads 1). It was renamed C:/data/todos.json.bad-5. The to-dos start empty."
        );
        assert_eq!(write.text(), "C:/data/settings.json could not be written: disk full");

        set_language(Language::French);
        assert_eq!(
            set_aside.text("Les tâches repartent de zéro."),
            "C:/data/todos.json a été écrit par une version plus récente (schéma 99 ; celle-ci lit 1). Il a été renommé C:/data/todos.json.bad-5. Les tâches repartent de zéro."
        );
        assert_eq!(write.text(), "C:/data/settings.json n'a pas pu être écrit : disk full");
    }
}
