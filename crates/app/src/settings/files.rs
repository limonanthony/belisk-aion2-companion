//! What Settings shows about the files on disk, and the actions that hand a
//! folder or a document to Windows. Opening goes through `explorer.exe`: it
//! opens a folder, a file in its default program, or a URL in the browser,
//! and it touches nothing but our own files.

use std::path::{Path, PathBuf};
use std::process::Command;

use meter_core::capture::recorder::list_recordings;
use meter_core::engine::meter::DpsMeter;

use super::store;
use crate::i18n::fl;
use crate::data_folder;

pub const LICENCE_URL: &str = "https://www.gnu.org/licenses/gpl-3.0.html";
const HISTORY_FILE: &str = "history/combat_history.bin";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folder {
    /// Settings, history, personal bests, recordings and mobs.json.
    Data,
    Logs,
}

/// A document shipped beside the exe by `cargo xtask dist`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Document {
    /// Where the source is, or the written offer for it.
    Readme,
    Licence,
}

impl Document {
    fn file_name(self) -> &'static str {
        match self {
            Self::Readme => "README.txt",
            Self::Licence => "LICENSE.txt",
        }
    }
}

/// What handing a folder or document to Windows could not do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// `%APPDATA%` is not set, so the data folder has no place.
    NoDataFolder,
    /// A replay or capture run keeps no files, so it has no folder to open.
    NotLiveRun,
    NotCreated { path: PathBuf, error: String },
    NotOpened { target: String, error: String },
    /// Only `https://` addresses are handed over, so an address from outside
    /// (GitHub's answer) can never name a program or a file.
    NotAWebPage { target: String },
}

impl OpenError {
    pub fn text(&self) -> String {
        match self {
            Self::NoDataFolder => fl!("settings-open-no-data-folder"),
            Self::NotLiveRun => fl!("settings-open-not-live"),
            Self::NotCreated { path, error } => {
                fl!("settings-open-not-created", path = path.display().to_string(), error = error.as_str())
            }
            Self::NotOpened { target, error } => fl!("settings-open-failed", target = target.as_str(), error = error.as_str()),
            Self::NotAWebPage { target } => fl!("settings-open-not-web-page", target = target.as_str()),
        }
    }
}

/// What the data folder holds, as the About page lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageSummary {
    pub settings_bytes: u64,
    pub history_records: usize,
    pub history_bytes: u64,
    pub recordings: usize,
    pub recordings_bytes: u64,
}

/// Reads file sizes and counts the saved fights; run it off the UI thread.
pub fn storage_summary(data_dir: Option<&Path>, meter: Option<&DpsMeter>) -> StorageSummary {
    let size_of = |path: PathBuf| std::fs::metadata(path).map_or(0, |metadata| metadata.len());
    let recordings = data_dir.map(|dir| list_recordings(&dir.join("recordings"))).unwrap_or_default();
    StorageSummary {
        settings_bytes: data_dir.map_or(0, |dir| size_of(store::path_in(dir))),
        history_records: meter.map_or(0, |meter| meter.get_history().len()),
        history_bytes: data_dir.map_or(0, |dir| size_of(dir.join(HISTORY_FILE))),
        recordings: recordings.len(),
        recordings_bytes: recordings.iter().map(|file| file.bytes).sum(),
    }
}

pub fn open_folder(folder: Folder) -> Result<(), OpenError> {
    if !data_folder::is_settled() {
        return Err(OpenError::NotLiveRun);
    }
    let path = match folder {
        Folder::Data => data_folder::data_dir().ok_or(OpenError::NoDataFolder)?,
        Folder::Logs => data_folder::log_dir().ok_or(OpenError::NotLiveRun)?,
    };
    std::fs::create_dir_all(&path).map_err(|error| OpenError::NotCreated { path: path.clone(), error: error.to_string() })?;
    hand_to_windows(path.as_os_str())
}

/// Opens the document beside the exe; a build run from the source tree has
/// none, so the GPL-3.0 text on gnu.org opens instead.
pub fn open_document(document: Document) -> Result<(), OpenError> {
    match shipped(document) {
        Some(path) => hand_to_windows(path.as_os_str()),
        None => hand_to_windows(LICENCE_URL.as_ref()),
    }
}

/// Opens a web page in the default browser.
pub fn open_web_page(url: &str) -> Result<(), OpenError> {
    if !is_web_page(url) {
        return Err(OpenError::NotAWebPage { target: url.to_owned() });
    }
    hand_to_windows(url.as_ref())
}

fn is_web_page(url: &str) -> bool {
    url.strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\\')))
}

fn shipped(document: Document) -> Option<PathBuf> {
    let beside_exe = std::env::current_exe().ok()?.parent()?.join(document.file_name());
    beside_exe.is_file().then_some(beside_exe)
}

fn hand_to_windows(target: &std::ffi::OsStr) -> Result<(), OpenError> {
    Command::new("explorer.exe")
        .arg(target)
        .spawn()
        .map(drop)
        .map_err(|error| OpenError::NotOpened { target: target.to_string_lossy().into_owned(), error: error.to_string() })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn the_summary_counts_what_is_in_the_data_folder() {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("belisk-storage-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(dir.join("recordings")).unwrap();
        std::fs::create_dir_all(dir.join("history")).unwrap();
        std::fs::write(dir.join("settings.json"), vec![0; 10]).unwrap();
        std::fs::write(dir.join(HISTORY_FILE), vec![0; 200]).unwrap();
        std::fs::write(dir.join("recordings/a.belisk"), vec![0; 300]).unwrap();
        std::fs::write(dir.join("recordings/b.aetherpc"), vec![0; 40]).unwrap();
        std::fs::write(dir.join("recordings/notes.txt"), vec![0; 9_000]).unwrap();

        let summary = storage_summary(Some(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            summary,
            StorageSummary {
                settings_bytes: 10,
                history_records: 0,
                history_bytes: 200,
                recordings: 2,
                recordings_bytes: 340,
            }
        );
    }

    #[test]
    fn without_a_data_folder_nothing_is_stored() {
        assert_eq!(storage_summary(None, None), StorageSummary::default());
    }

    #[test]
    fn a_run_that_never_settled_the_folders_opens_and_creates_none() {
        assert_eq!(open_folder(Folder::Data), Err(OpenError::NotLiveRun));
        assert_eq!(open_folder(Folder::Logs), Err(OpenError::NotLiveRun));
    }

    #[test]
    fn only_https_addresses_are_handed_to_windows() {
        for not_a_page in ["", "https://", "http://github.com/o/r", "C:\\Windows\\notepad.exe", "file:///C:/x", "https://a b", "https://x\"y"] {
            assert_eq!(open_web_page(not_a_page), Err(OpenError::NotAWebPage { target: not_a_page.to_owned() }), "{not_a_page:?}");
        }
        assert!(is_web_page("https://github.com/o/r/releases/tag/v1.6.0"));
    }
}
