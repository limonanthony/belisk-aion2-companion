//! How `settings.json`, `todos.json` and `pets.json` are kept on disk while
//! the app runs: a change is written once nothing has changed for
//! [`SAVE_AFTER`], on the background executor, so the UI thread never waits
//! for the disk. Writes carry the number of the value they hold, so one that
//! finishes late never replaces a newer one. A write that fails is tried
//! again every [`RETRY_AFTER`] until it goes through; [`BackgroundSave::flush`]
//! writes at once, on the calling thread, for quitting.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use gpui::{Context, Task};

use crate::stored_file::SaveError;

/// A run of changes (a batch of pickups) causes one write.
pub const SAVE_AFTER: Duration = Duration::from_millis(400);
pub const RETRY_AFTER: Duration = Duration::from_secs(10);

/// The entity that owns a stored value and its [`BackgroundSave`].
pub trait Persisted: 'static {
    type Value: Clone + Send + 'static;

    fn value(&self) -> &Self::Value;

    fn background_save(&mut self) -> &mut BackgroundSave<Self::Value>;
}

pub struct BackgroundSave<T> {
    file: Option<PathBuf>,
    write: fn(&Path, &T) -> Result<(), SaveError>,
    task: Option<Task<()>>,
    saved: bool,
    error: Option<SaveError>,
    /// Counts changes; a write carries the number of the value it holds.
    revision: u64,
    /// The revision last written. Writers take the lock, so an older value,
    /// finishing late, never replaces a newer one on disk.
    written: Arc<Mutex<u64>>,
}

impl<T: Clone + Send + 'static> BackgroundSave<T> {
    /// Writes to `file` with `write`; without a file (a replay or a capture)
    /// nothing is ever written.
    pub fn new(file: Option<PathBuf>, write: fn(&Path, &T) -> Result<(), SaveError>) -> Self {
        Self { file, write, task: None, saved: true, error: None, revision: 0, written: Arc::default() }
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// Why the last write failed; cleared by the next one that succeeds.
    pub fn error(&self) -> Option<&SaveError> {
        self.error.as_ref()
    }

    /// The owner's value changed: it is written after the pause. Notifies the
    /// owner when the outcome of a write changes [`error`](Self::error).
    pub fn changed<O: Persisted<Value = T>>(&mut self, cx: &mut Context<O>) {
        self.pending();
        let Some(file) = self.file.clone() else {
            return;
        };
        let write = self.write;
        self.task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_AFTER).await;
            loop {
                let Ok(Some((revision, value, written))) = this.update(cx, |owner, _| {
                    let save = owner.background_save();
                    if save.saved {
                        return None;
                    }
                    let (revision, written) = (save.revision, Arc::clone(&save.written));
                    Some((revision, owner.value().clone(), written))
                }) else {
                    return;
                };
                let target = file.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { write_in_order(write, &written, revision, &target, &value) })
                    .await;
                match this.update(cx, |owner, cx| owner.background_save().finished(revision, result, cx)) {
                    Ok(true) | Err(_) => return,
                    Ok(false) => cx.background_executor().timer(RETRY_AFTER).await,
                }
            }
        }));
    }

    /// Marks the value as not on disk, without scheduling a write: for a
    /// value that [`flush`](Self::flush) writes at once.
    pub fn pending(&mut self) {
        self.saved = false;
        self.revision += 1;
    }

    /// Writes any change not yet on disk now, on this thread.
    pub fn flush(&mut self, value: &T) {
        let Some(file) = self.file.as_ref().filter(|_| !self.saved) else {
            return;
        };
        match write_in_order(self.write, &self.written, self.revision, file, value) {
            Ok(()) => {
                self.task = None;
                self.saved = true;
                self.error = None;
            }
            Err(error) => {
                self.failed(error);
            }
        }
    }

    /// Records a finished background write; whether the value is on disk.
    fn finished<O: 'static>(&mut self, revision: u64, result: Result<(), SaveError>, cx: &mut Context<O>) -> bool {
        let had_error = self.error.is_some();
        match result {
            Ok(()) => {
                self.saved = self.saved || self.revision == revision;
                self.error = None;
                if had_error {
                    cx.notify();
                }
                true
            }
            Err(error) => {
                if self.failed(error) {
                    cx.notify();
                }
                false
            }
        }
    }

    /// Keeps the error, and logs it unless it is the one already told (a
    /// retry that fails the same way). Whether it is new.
    fn failed(&mut self, error: SaveError) -> bool {
        let is_new = self.error.as_ref() != Some(&error);
        if is_new {
            crate::app_log::error(error.to_string());
        }
        self.error = Some(error);
        is_new
    }
}

fn write_in_order<T>(
    write: fn(&Path, &T) -> Result<(), SaveError>,
    written: &Mutex<u64>,
    revision: u64,
    file: &Path,
    value: &T,
) -> Result<(), SaveError> {
    let mut last = written.lock().unwrap_or_else(PoisonError::into_inner);
    if revision <= *last {
        return Ok(());
    }
    write(file, value)?;
    *last = revision;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use gpui::{AppContext as _, Entity, TestAppContext};

    use super::*;

    /// The disk is "full" for a file while a `full` file lies beside it.
    fn disk_full_marker(file: &Path) -> PathBuf {
        file.with_file_name("full")
    }

    fn write_text(path: &Path, value: &u32) -> Result<(), SaveError> {
        if disk_full_marker(path).exists() {
            return Err(SaveError::Write { path: path.to_path_buf(), error: "disk full".to_string() });
        }
        std::fs::write(path, value.to_string()).map_err(|error| SaveError::Write { path: path.to_path_buf(), error: error.to_string() })
    }

    struct Owner {
        value: u32,
        save: BackgroundSave<u32>,
    }

    impl Persisted for Owner {
        type Value = u32;

        fn value(&self) -> &u32 {
            &self.value
        }

        fn background_save(&mut self) -> &mut BackgroundSave<u32> {
            &mut self.save
        }
    }

    fn owner(cx: &mut TestAppContext, name: &str) -> (Entity<Owner>, PathBuf) {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("belisk-background-save-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("value.txt");
        let entity = cx.new(|_| Owner { value: 0, save: BackgroundSave::new(Some(file.clone()), write_text) });
        (entity, file)
    }

    fn set(owner: &Entity<Owner>, value: u32, cx: &mut TestAppContext) {
        owner.update(cx, |owner, cx| {
            owner.value = value;
            owner.save.changed(cx);
        });
    }

    fn on_disk(file: &Path) -> Option<String> {
        std::fs::read_to_string(file).ok()
    }

    #[gpui::test]
    fn a_change_is_written_after_the_pause_and_a_run_of_changes_once(cx: &mut TestAppContext) {
        let (owner, file) = owner(cx, "pause");
        set(&owner, 1, cx);
        cx.executor().advance_clock(SAVE_AFTER / 2);
        set(&owner, 2, cx);
        cx.executor().advance_clock(SAVE_AFTER * 3 / 4);
        cx.run_until_parked();
        assert_eq!(on_disk(&file), None, "the pause starts again at each change");

        cx.executor().advance_clock(SAVE_AFTER);
        cx.run_until_parked();
        assert_eq!(on_disk(&file).as_deref(), Some("2"));
    }

    #[gpui::test]
    fn a_failed_write_is_tried_again_without_another_change(cx: &mut TestAppContext) {
        let (owner, file) = owner(cx, "retry");
        std::fs::write(disk_full_marker(&file), "").unwrap();
        set(&owner, 7, cx);
        cx.executor().advance_clock(SAVE_AFTER * 2);
        cx.run_until_parked();
        assert_eq!(on_disk(&file), None);
        assert!(owner.read_with(cx, |owner, _| owner.save.error().is_some()), "the failure is kept for the window to show");

        std::fs::remove_file(disk_full_marker(&file)).unwrap();
        cx.executor().advance_clock(RETRY_AFTER * 2);
        cx.run_until_parked();
        assert_eq!(on_disk(&file).as_deref(), Some("7"));
        assert!(owner.read_with(cx, |owner, _| owner.save.error().is_none()));
    }

    #[gpui::test]
    fn flush_writes_at_once_what_the_pause_had_not_yet_written(cx: &mut TestAppContext) {
        let (owner, file) = owner(cx, "flush");
        set(&owner, 5, cx);
        owner.update(cx, |owner, _| owner.save.flush(&owner.value));
        assert_eq!(on_disk(&file).as_deref(), Some("5"));
    }

    #[test]
    fn a_value_written_late_never_replaces_a_newer_one() {
        let file = std::env::temp_dir().join(format!("belisk-background-save-order-{}.txt", std::process::id()));
        let written = Mutex::new(0);
        write_in_order(write_text, &written, 2, &file, &20).unwrap();
        write_in_order(write_text, &written, 1, &file, &10).unwrap();
        assert_eq!(on_disk(&file).as_deref(), Some("20"));
        let _ = std::fs::remove_file(&file);
    }
}
