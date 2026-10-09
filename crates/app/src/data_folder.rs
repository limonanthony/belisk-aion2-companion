//! Where the app keeps its files, and the one-time move out of the folders the
//! builds before the rename to Belisk used.
//!
//! `%APPDATA%/Belisk` holds `settings.json`, the to-dos, the fight history, the
//! personal bests, the recordings and `mobs.json`; `%LOCALAPPDATA%/Belisk/logs`
//! holds `belisk.log`. Earlier builds used `%APPDATA%/com.helveticxa.aether` and
//! `%LOCALAPPDATA%/com.helveticxa.aether/logs/aether.log`. [`settle`] runs once
//! at the start of a live run, before anything opens those files:
//!
//! - the new folder is missing and the old one exists: the old folder is
//!   renamed into place (same volume, so nothing is copied);
//! - the rename fails (a file in it is open, access denied): this run keeps
//!   using the old folder, says why in the log, and the next launch tries again;
//! - both exist: nothing is merged. The new folder wins, the old one is left
//!   untouched and the log names it, so no file is overwritten or lost. A new
//!   folder that holds no file, only empty folders, counts as missing, and is
//!   removed before the rename; if it cannot be removed the move fails as above;
//! - the old folder is missing: the new one is used as it is.
//!
//! The data and the logs are settled apart, so one failing never holds the
//! other back. Only the `logs` folder moves out of the old local folder: the
//! rest of it is not ours to move.
//!
//! Nothing may create the Belisk folders before [`settle`] has chosen them,
//! or "both exist" would strand the old data on the next launch. So until
//! [`settle`] has run, [`data_dir`] and [`log_dir`] answer `None`, and a run
//! that never settles (replays, captures, the `--apply-update` helper) has no
//! player folder to write to at all.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const FOLDER: &str = "Belisk";
const LEGACY_FOLDER: &str = "com.helveticxa.aether";
const LOGS: &str = "logs";
const LOG_FILE: &str = "belisk.log";
const LEGACY_LOG_FILE: &str = "aether.log";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Folders {
    data: Option<PathBuf>,
    logs: PathBuf,
}

static SETTLED: OnceLock<Folders> = OnceLock::new();

/// Something the move did that the log should say, once the logger exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    Info(String),
    Warning(String),
}

impl Note {
    pub fn log(&self) {
        match self {
            Note::Info(text) => crate::app_log::info(text),
            Note::Warning(text) => crate::app_log::warn(text),
        }
    }
}

/// `%APPDATA%/Belisk`, where history, personal bests, recordings and
/// `settings.json` live. `None` before [`settle`] has run, and when `%APPDATA%`
/// is not set.
pub fn data_dir() -> Option<PathBuf> {
    SETTLED.get()?.data.clone()
}

/// `%LOCALAPPDATA%/Belisk/logs`, where `belisk.log` lives. `None` before
/// [`settle`] has run.
pub fn log_dir() -> Option<PathBuf> {
    Some(SETTLED.get()?.logs.clone())
}

/// Whether [`settle`] has chosen the folders, which only a live run does.
pub fn is_settled() -> bool {
    SETTLED.get().is_some()
}

/// Moves the old folders into place if they are still there and fixes what
/// [`data_dir`] and [`log_dir`] answer for the rest of the run. Call it once,
/// before the logger or the settings open a file.
pub fn settle() -> Vec<Note> {
    let (folders, notes) = settle_in(
        std::env::var_os("APPDATA").map(PathBuf::from).as_deref(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from).as_deref(),
        &std::env::temp_dir(),
    );
    let _ = SETTLED.set(folders);
    notes
}

fn settle_in(roaming: Option<&Path>, local: Option<&Path>, temp: &Path) -> (Folders, Vec<Note>) {
    let mut notes = Vec::new();
    let data = roaming.map(|roaming| {
        settle_folder("settings and history", &roaming.join(LEGACY_FOLDER), &roaming.join(FOLDER), &mut notes)
    });
    let logs = match local {
        Some(local) => settle_logs(local, &mut notes),
        None => temp.join("belisk"),
    };
    (Folders { data, logs }, notes)
}

fn settle_logs(local: &Path, notes: &mut Vec<Note>) -> PathBuf {
    let legacy_local = local.join(LEGACY_FOLDER);
    let old = legacy_local.join(LOGS);
    let new = local.join(FOLDER).join(LOGS);
    let in_use = settle_folder("logs", &old, &new, notes);
    if in_use == new && !old.exists() {
        rename_log_files(&new, notes);
        let _ = fs::remove_dir(&legacy_local);
    }
    in_use
}

fn rename_log_files(logs: &Path, notes: &mut Vec<Note>) {
    for (old, new) in [(LEGACY_LOG_FILE, LOG_FILE), ("aether.log.1", "belisk.log.1")] {
        let (old, new) = (logs.join(old), logs.join(new));
        if !old.is_file() || new.exists() {
            continue;
        }
        if let Err(error) = fs::rename(&old, &new) {
            notes.push(Note::Warning(format!("{} could not be renamed to {}: {error}", old.display(), new.display())));
        }
    }
}

/// Settles one folder as the module documentation says and returns the one to use.
fn settle_folder(what: &str, old: &Path, new: &Path, notes: &mut Vec<Note>) -> PathBuf {
    if !old.is_dir() {
        return new.to_path_buf();
    }
    match clear_unused(new) {
        Ok(true) => {}
        Ok(false) => {
            notes.push(Note::Warning(format!(
                "the {what} folder exists both at {} and at {}; {} is used and nothing was merged, {} is left as it is",
                old.display(),
                new.display(),
                new.display(),
                old.display(),
            )));
            return new.to_path_buf();
        }
        Err(error) => return keep_old(what, old, new, &error, notes),
    }
    let moved = match new.parent() {
        Some(parent) => fs::create_dir_all(parent).and_then(|()| fs::rename(old, new)),
        None => fs::rename(old, new),
    };
    match moved {
        Ok(()) => {
            notes.push(Note::Info(format!("the {what} folder moved from {} to {}", old.display(), new.display())));
            new.to_path_buf()
        }
        Err(error) => keep_old(what, old, new, &error, notes),
    }
}

fn keep_old(what: &str, old: &Path, new: &Path, error: &std::io::Error, notes: &mut Vec<Note>) -> PathBuf {
    notes.push(Note::Warning(format!(
        "the {what} folder could not move from {} to {}: {error}; {} is used for this run and the move is tried again at the next launch",
        old.display(),
        new.display(),
        old.display(),
    )));
    old.to_path_buf()
}

/// Makes room for the rename: `Ok(true)` when `new` is missing or was only
/// empty folders (now removed), `Ok(false)` when it holds a file or is not a
/// folder and must be left alone.
fn clear_unused(new: &Path) -> std::io::Result<bool> {
    if !new.exists() {
        return Ok(true);
    }
    if !new.is_dir() || !holds_no_file(new)? {
        return Ok(false);
    }
    remove_empty_folders(new)?;
    Ok(true)
}

fn holds_no_file(folder: &Path) -> std::io::Result<bool> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || !holds_no_file(&entry.path())? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn remove_empty_folders(folder: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(folder)? {
        remove_empty_folders(&entry?.path())?;
    }
    fs::remove_dir(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let root = std::env::temp_dir().join(format!("belisk-data-folder-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("roaming")).unwrap();
            fs::create_dir_all(root.join("local")).unwrap();
            Scratch(root)
        }

        fn roaming(&self) -> PathBuf {
            self.0.join("roaming")
        }

        fn local(&self) -> PathBuf {
            self.0.join("local")
        }

        fn settle(&self) -> (Folders, Vec<Note>) {
            settle_in(Some(&self.roaming()), Some(&self.local()), &self.0.join("temp"))
        }

        fn write(&self, path: PathBuf, text: &str) {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn warnings(notes: &[Note]) -> Vec<&str> {
        notes.iter().filter_map(|note| if let Note::Warning(text) = note { Some(text.as_str()) } else { None }).collect()
    }

    #[test]
    fn a_fresh_install_uses_the_belisk_folders_and_creates_nothing() {
        let scratch = Scratch::new("fresh");

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert_eq!(folders.logs, scratch.local().join("Belisk").join("logs"));
        assert!(notes.is_empty());
        assert!(!scratch.roaming().join("Belisk").exists());
    }

    #[test]
    fn the_old_data_folder_moves_whole_into_the_belisk_folder() {
        let scratch = Scratch::new("move-data");
        let old = scratch.roaming().join("com.helveticxa.aether");
        scratch.write(old.join("settings.json"), "{\"general\":1}");
        scratch.write(old.join("history/fights.dpsh"), "history");
        scratch.write(old.join("recordings/rolling-1.aetherpc"), "AETHERPC");

        let (folders, notes) = scratch.settle();

        let new = scratch.roaming().join("Belisk");
        assert_eq!(folders.data, Some(new.clone()));
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new.join("settings.json")).unwrap(), "{\"general\":1}");
        assert_eq!(fs::read_to_string(new.join("history/fights.dpsh")).unwrap(), "history");
        assert_eq!(fs::read_to_string(new.join("recordings/rolling-1.aetherpc")).unwrap(), "AETHERPC");
        assert!(warnings(&notes).is_empty());
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn an_empty_belisk_folder_counts_as_missing() {
        let scratch = Scratch::new("empty-new");
        scratch.write(scratch.roaming().join("com.helveticxa.aether/settings.json"), "old");
        fs::create_dir_all(scratch.roaming().join("Belisk")).unwrap();

        let (folders, _) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert_eq!(fs::read_to_string(scratch.roaming().join("Belisk/settings.json")).unwrap(), "old");
        assert!(!scratch.roaming().join("com.helveticxa.aether").exists());
    }

    #[test]
    fn a_belisk_folder_of_only_empty_folders_counts_as_missing() {
        let scratch = Scratch::new("empty-tree");
        scratch.write(scratch.roaming().join("com.helveticxa.aether/settings.json"), "old");
        fs::create_dir_all(scratch.roaming().join("Belisk/recordings/nested")).unwrap();

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert_eq!(fs::read_to_string(scratch.roaming().join("Belisk/settings.json")).unwrap(), "old");
        assert!(!scratch.roaming().join("Belisk/recordings").exists());
        assert!(warnings(&notes).is_empty());
    }

    #[test]
    fn a_belisk_folder_holding_any_file_wins_and_loses_no_empty_folder() {
        let scratch = Scratch::new("tree-with-file");
        scratch.write(scratch.roaming().join("com.helveticxa.aether/settings.json"), "old");
        scratch.write(scratch.roaming().join("Belisk/desktop.ini"), "");
        fs::create_dir_all(scratch.roaming().join("Belisk/recordings")).unwrap();

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert!(scratch.roaming().join("Belisk/recordings").is_dir(), "a folder with a file in it is not touched");
        assert_eq!(fs::read_to_string(scratch.roaming().join("com.helveticxa.aether/settings.json")).unwrap(), "old");
        assert_eq!(warnings(&notes).len(), 1);
    }

    #[test]
    fn a_belisk_path_that_is_a_file_is_never_replaced() {
        let scratch = Scratch::new("new-is-file");
        scratch.write(scratch.roaming().join("com.helveticxa.aether/settings.json"), "old");
        scratch.write(scratch.roaming().join("Belisk"), "a file");

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert_eq!(fs::read_to_string(scratch.roaming().join("Belisk")).unwrap(), "a file");
        assert_eq!(warnings(&notes).len(), 1);
    }

    #[test]
    fn before_settle_there_is_no_folder_to_write_to() {
        assert!(!is_settled());
        assert_eq!(data_dir(), None);
        assert_eq!(log_dir(), None);
    }

    #[test]
    fn when_both_folders_exist_the_belisk_one_wins_and_the_old_one_is_left_alone() {
        let scratch = Scratch::new("both");
        scratch.write(scratch.roaming().join("com.helveticxa.aether/settings.json"), "old");
        scratch.write(scratch.roaming().join("Belisk/settings.json"), "new");

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert_eq!(fs::read_to_string(scratch.roaming().join("Belisk/settings.json")).unwrap(), "new");
        assert_eq!(fs::read_to_string(scratch.roaming().join("com.helveticxa.aether/settings.json")).unwrap(), "old");
        let warnings = warnings(&notes);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("nothing was merged") && warnings[0].contains("com.helveticxa.aether"), "{warnings:?}");
    }

    #[test]
    fn a_move_that_fails_keeps_the_old_folder_for_this_run_and_says_why() {
        let scratch = Scratch::new("blocked");
        let old = scratch.roaming().join("com.helveticxa.aether");
        scratch.write(old.join("settings.json"), "old");
        // The parent of the new folder is a file, so the new folder cannot be created.
        let blocker = scratch.0.join("blocker");
        fs::write(&blocker, "not a folder").unwrap();
        let mut notes = Vec::new();

        let in_use = settle_folder("settings and history", &old, &blocker.join("Belisk"), &mut notes);

        assert_eq!(in_use, old);
        assert_eq!(fs::read_to_string(old.join("settings.json")).unwrap(), "old");
        let warnings = warnings(&notes);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("could not move") && warnings[0].contains("next launch"), "{warnings:?}");
    }

    #[test]
    fn the_logs_move_and_the_log_files_are_renamed() {
        let scratch = Scratch::new("logs");
        let old = scratch.local().join("com.helveticxa.aether/logs");
        scratch.write(old.join("aether.log"), "current");
        scratch.write(old.join("aether.log.1"), "previous");

        let (folders, notes) = scratch.settle();

        let logs = scratch.local().join("Belisk/logs");
        assert_eq!(folders.logs, logs);
        assert_eq!(fs::read_to_string(logs.join("belisk.log")).unwrap(), "current");
        assert_eq!(fs::read_to_string(logs.join("belisk.log.1")).unwrap(), "previous");
        assert!(!logs.join("aether.log").exists());
        assert!(!scratch.local().join("com.helveticxa.aether").exists(), "the emptied old folder is removed");
        assert!(warnings(&notes).is_empty());
    }

    #[test]
    fn the_old_local_folder_stays_when_it_holds_more_than_the_logs() {
        let scratch = Scratch::new("logs-other");
        scratch.write(scratch.local().join("com.helveticxa.aether/logs/aether.log"), "current");
        scratch.write(scratch.local().join("com.helveticxa.aether/EBWebView/data"), "tauri");

        let (folders, _) = scratch.settle();

        assert_eq!(folders.logs, scratch.local().join("Belisk/logs"));
        assert_eq!(fs::read_to_string(scratch.local().join("com.helveticxa.aether/EBWebView/data")).unwrap(), "tauri");
    }

    #[test]
    fn when_both_log_folders_exist_the_new_one_wins_and_the_old_logs_stay() {
        let scratch = Scratch::new("logs-both");
        scratch.write(scratch.local().join("com.helveticxa.aether/logs/aether.log"), "old");
        scratch.write(scratch.local().join("Belisk/logs/belisk.log"), "new");

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.logs, scratch.local().join("Belisk/logs"));
        assert_eq!(fs::read_to_string(scratch.local().join("Belisk/logs/belisk.log")).unwrap(), "new");
        assert_eq!(fs::read_to_string(scratch.local().join("com.helveticxa.aether/logs/aether.log")).unwrap(), "old");
        assert_eq!(warnings(&notes).len(), 1);
    }

    #[test]
    fn the_data_and_the_logs_are_settled_apart() {
        let scratch = Scratch::new("apart");
        scratch.write(scratch.roaming().join("com.helveticxa.aether/settings.json"), "old");
        scratch.write(scratch.roaming().join("Belisk/settings.json"), "new");
        scratch.write(scratch.local().join("com.helveticxa.aether/logs/aether.log"), "old log");

        let (folders, notes) = scratch.settle();

        assert_eq!(folders.data, Some(scratch.roaming().join("Belisk")));
        assert_eq!(fs::read_to_string(scratch.local().join("Belisk/logs/belisk.log")).unwrap(), "old log");
        assert_eq!(warnings(&notes).len(), 1, "only the data folder had both");
    }

    #[test]
    fn without_the_windows_folders_the_data_is_unknown_and_the_logs_go_to_the_temp_folder() {
        let (folders, notes) = settle_in(None, None, Path::new("/tmp-root"));

        assert_eq!(folders.data, None);
        assert_eq!(folders.logs, Path::new("/tmp-root").join("belisk"));
        assert!(notes.is_empty());
    }
}
