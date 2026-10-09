//! What can go wrong with the JSON files the app keeps beside the core's data
//! (`settings.json`, `todos.json`): told as data, so the log says it in
//! English and Settings in the language in force. Paths and the system's or
//! the JSON reader's own errors are kept as they came.

use std::fmt;
use std::path::PathBuf;

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
