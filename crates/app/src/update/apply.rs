//! Putting a staged update in place.
//!
//! The running exe cannot be replaced while it runs, so a second copy of it
//! does the swap. "Restart to update" starts *this same exe* again, as
//! `belisk.exe --apply-update <pid>`, and quits. The staged update is
//! never executed: it is only data, an archive whose hash and signature are
//! checked again by the exe that is already trusted.
//!
//! The helper (before any window, setting or capture exists):
//!
//! 1. takes everything from where it runs: the install folder is its own exe's
//!    folder, the staged archive is `<install>/update-staging/ready/`, the files
//!    it may write are the exe (under its own name) and the two WinDivert files.
//!    The command line carries nothing but the pid, and the pid must be the
//!    helper's real parent: it is not a way to name a file;
//! 2. waits for that parent to end, which also releases the single-instance
//!    lock and the WinDivert driver;
//! 3. reads the staged archive once, checks SHA-256 and signature of those bytes
//!    against the key built into the exe, and unpacks the allowed files from
//!    those same bytes;
//! 4. writes each as `<name>.new`, renames the files it replaces to `<name>.old`
//!    (Windows allows renaming a running exe, not overwriting it) and the new
//!    ones into place; any failure puts everything back, so the old version stays
//!    runnable;
//! 5. deletes the staging folder and starts the installed exe, which deletes the
//!    `.old` files on its next start.
//!
//! A child of an elevated process is elevated too, so no UAC prompt appears.

use std::ffi::c_void;
use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ed25519_dalek::VerifyingKey;
use windows::core::{Owned, HRESULT};
use windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, WaitForSingleObject, PROCESS_BASIC_INFORMATION, PROCESS_SYNCHRONIZE,
};

use super::package::{self, Entry, COMPANION_FILES, EXE_ENTRY};
use super::stage::{self, Layout};
use super::verify;
use crate::changelog::Version;

const FLAG: &str = "--apply-update";
const WAIT_FOR_OLD_APP: Duration = Duration::from_secs(60);
const DETACHED_PROCESS: u32 = 0x0000_0008;

/// The helper's command line: the flag and the pid of the app that started it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub parent_pid: u32,
}

impl Request {
    /// `None` when this is not the helper's command line. Anything else after
    /// the flag than one non-zero process id is refused.
    pub fn from_args(args: &[String]) -> Option<Result<Self, String>> {
        let [flag, rest @ ..] = args else {
            return None;
        };
        if flag != FLAG {
            return None;
        }
        Some(match rest {
            [pid] => match pid.parse::<u32>() {
                Ok(parent_pid) if parent_pid != 0 => Ok(Self { parent_pid }),
                _ => Err(format!("{FLAG}: {pid:?} is not a process id")),
            },
            _ => Err(format!("{FLAG} takes only the process id of the app that started it")),
        })
    }
}

/// Where an update is installed: the folder of the running exe, and the exe's
/// own file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    pub dir: PathBuf,
    pub exe_name: String,
}

impl Install {
    /// The install for the exe at `exe`, refused when its name could be taken
    /// for something else an update writes.
    pub fn of_exe(exe: &Path) -> Result<Self, String> {
        let dir = exe.parent().filter(|dir| !dir.as_os_str().is_empty()).ok_or_else(|| format!("{} has no folder", exe.display()))?;
        let name = exe
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{} has no usable file name", exe.display()))?;
        let is_exe = name.len() > ".exe".len() && name.to_ascii_lowercase().ends_with(".exe");
        let is_companion = COMPANION_FILES.iter().any(|companion| companion.eq_ignore_ascii_case(name));
        if !is_exe || is_companion {
            return Err(format!("{name} is not an exe name an update installs under"));
        }
        Ok(Self { dir: dir.to_owned(), exe_name: name.to_owned() })
    }

    pub fn layout(&self) -> Layout {
        Layout::new(self.dir.clone())
    }

    fn installed_name(&self, entry: &Entry) -> String {
        if entry.name == EXE_ENTRY {
            self.exe_name.clone()
        } else {
            entry.name.to_owned()
        }
    }
}

/// The exe's file, held open for as long as the app runs, without the right for
/// anyone else to write, rename or delete it. Windows lets a running exe be
/// renamed, so without this a process of the same user could move the running
/// image aside, put another exe at its path, and have Restart start that one
/// elevated. With the file held, the path keeps meaning the image that is
/// running (a folder holding an open file cannot be renamed either).
///
/// Not covered: a swap made before the app started, which is also what
/// replacing the exe by hand does.
#[derive(Debug)]
pub struct ImageLock {
    path: PathBuf,
    _held: fs::File,
}

impl ImageLock {
    pub fn acquire(exe: &Path) -> Result<Self, String> {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(exe)
            .map_err(|error| format!("cannot hold {} against replacement: {error}", exe.display()))?;
        Ok(Self { path: exe.to_owned(), _held: held })
    }

    #[cfg(test)]
    pub fn of_this_test_exe() -> Self {
        Self::acquire(&std::env::current_exe().unwrap()).unwrap()
    }
}

/// Before the app quits: the staged update must still verify, and the helper
/// (a copy of this exe, the file `image` holds) must start. When either fails
/// nothing has changed and the app keeps running.
pub fn start_helper(layout: &Layout, key: &VerifyingKey, image: &ImageLock) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| format!("cannot find this exe: {error}"))?;
    if exe != image.path {
        return Err(format!("{} is not the exe that was held at start ({})", exe.display(), image.path.display()));
    }
    let install = Install::of_exe(&exe)?;
    if layout.install_dir() != install.dir {
        return Err(format!("the staged update is not beside {}", exe.display()));
    }
    stage::load_verified(layout, Version::current(), key).map_err(|failure| failure.to_string())?;
    Command::new(&exe)
        .arg(FLAG)
        .arg(std::process::id().to_string())
        .current_dir(&install.dir)
        .creation_flags(DETACHED_PROCESS)
        .spawn()
        .map(drop)
        .map_err(|error| format!("cannot start {}: {error}", exe.display()))
}

/// The helper's whole life; the process exit code.
pub fn run(request: &Request) -> u32 {
    let key = match verify::trusted_key() {
        Ok(key) => key,
        Err(error) => {
            crate::app_log::error(format!("the update was not applied: {error}"));
            return 1;
        }
    };
    let install = match std::env::current_exe().map_err(|error| error.to_string()).and_then(|exe| Install::of_exe(&exe)) {
        Ok(install) => install,
        Err(error) => {
            crate::app_log::error(format!("the update was not applied: {error}"));
            return 1;
        }
    };
    if let Err(error) = confirm_parent(request.parent_pid, parent_process_id()) {
        crate::app_log::error(format!("the update was not applied: {error}"));
        return 2;
    }
    crate::app_log::info(format!("applying the staged update: waiting for process {} to end", request.parent_pid));
    if let Err(error) = wait_for_exit(request.parent_pid, WAIT_FOR_OLD_APP) {
        crate::app_log::error(format!("the update was not applied, the old app is still running: {error}"));
        return 1;
    }

    let outcome = apply_staged(&install, Version::current(), &key, &|from, to| fs::rename(from, to));
    if let Err(error) = stage::remove_all(&install.layout().root()) {
        crate::app_log::warn(format!("cannot delete the staging folder: {error}; the next start does"));
    }
    let code = match outcome {
        Ok(version) => {
            crate::app_log::info(format!("update {version} is in place"));
            0
        }
        Err(error) => {
            crate::app_log::error(format!("the update was not applied, the old version stays: {error}"));
            1
        }
    };
    let exe = install.dir.join(&install.exe_name);
    let started = Command::new(&exe).current_dir(&install.dir).creation_flags(DETACHED_PROCESS).spawn();
    if let Err(error) = started {
        crate::app_log::error(format!("cannot start {}: {error}", exe.display()));
        return 1;
    }
    code
}

/// The staged update, checked again, put in place. `rename` is how files move
/// on the way in, passed so a test can make one step fail.
pub fn apply_staged(
    install: &Install,
    running: Version,
    key: &VerifyingKey,
    rename: &dyn Fn(&Path, &Path) -> io::Result<()>,
) -> Result<Version, String> {
    let verified = stage::load_verified(&install.layout(), running, key).map_err(|failure| failure.to_string())?;
    let entries = package::extract(&verified.archive).map_err(|failure| failure.to_string())?;
    install_files(install, &entries, rename)?;
    Ok(verified.version)
}

struct Step {
    target: PathBuf,
    new: PathBuf,
    backup: PathBuf,
}

fn install_files(install: &Install, entries: &[Entry], rename: &dyn Fn(&Path, &Path) -> io::Result<()>) -> Result<(), String> {
    let mut steps: Vec<Step> = Vec::new();
    for entry in entries {
        let name = install.installed_name(entry);
        let target = install.dir.join(&name);
        if entry.name != EXE_ENTRY && fs::read(&target).is_ok_and(|current| current == entry.bytes) {
            continue;
        }
        let step = Step { new: install.dir.join(format!("{name}.new")), backup: install.dir.join(format!("{name}.old")), target };
        let written = match fs::remove_file(&step.new) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => fs::write(&step.new, &entry.bytes),
        };
        steps.push(step);
        if let Err(error) = written {
            discard_new_files(&steps);
            return Err(format!("cannot write {name}.new: {error}"));
        }
    }

    let mut set_aside: Vec<&Step> = Vec::new();
    let mut placed: Vec<&Step> = Vec::new();
    let result = (|| {
        for step in &steps {
            if step.target.exists() {
                match fs::remove_file(&step.backup) {
                    Err(error) if error.kind() != io::ErrorKind::NotFound => {
                        return Err(format!("cannot clear {}: {error}", step.backup.display()));
                    }
                    _ => {}
                }
                rename(&step.target, &step.backup)
                    .map_err(|error| format!("cannot move {} aside: {error}", step.target.display()))?;
                set_aside.push(step);
            }
        }
        for step in &steps {
            rename(&step.new, &step.target).map_err(|error| format!("cannot move {} into place: {error}", step.target.display()))?;
            placed.push(step);
        }
        Ok(())
    })();

    if let Err(error) = result {
        let problems = roll_back(&placed, &set_aside);
        discard_new_files(&steps);
        return Err(if problems.is_empty() {
            format!("{error}; the old files were put back")
        } else {
            format!("{error}; putting the old files back failed too: {}", problems.join("; "))
        });
    }
    Ok(())
}

fn roll_back(placed: &[&Step], set_aside: &[&Step]) -> Vec<String> {
    let mut problems = Vec::new();
    for step in placed.iter().rev() {
        if let Err(error) = fs::remove_file(&step.target) {
            problems.push(format!("{}: {error}", step.target.display()));
        }
    }
    for step in set_aside.iter().rev() {
        if let Err(error) = fs::rename(&step.backup, &step.target) {
            problems.push(format!("{} (it is still at {}): {error}", step.target.display(), step.backup.display()));
        }
    }
    problems
}

fn discard_new_files(steps: &[Step]) {
    for step in steps {
        let _ = fs::remove_file(&step.new);
    }
}

/// The request names a process; it is only believed when it is the process that
/// started this one.
fn confirm_parent(requested: u32, actual: Result<u32, String>) -> Result<(), String> {
    let actual = actual?;
    if requested == 0 || requested != actual {
        return Err(format!("process {requested} did not start this one (its parent is {actual})"));
    }
    Ok(())
}

fn parent_process_id() -> Result<u32, String> {
    let mut info = PROCESS_BASIC_INFORMATION::default();
    // SAFETY: the pseudo-handle of this process is valid, and `info` is the
    // PROCESS_BASIC_INFORMATION the class asks for, with its own size.
    let status = unsafe {
        NtQueryInformationProcess(
            GetCurrentProcess(),
            ProcessBasicInformation,
            std::ptr::from_mut(&mut info).cast::<c_void>(),
            std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            std::ptr::null_mut(),
        )
    };
    if status.is_err() {
        return Err(format!("cannot ask Windows for this process's parent (status {:#x})", status.0));
    }
    u32::try_from(info.InheritedFromUniqueProcessId).map_err(|_| "the parent's process id does not fit 32 bits".to_owned())
}

/// Waits for `pid`, which `confirm_parent` showed to be this process's parent.
/// A process that no longer exists has ended, so that is success.
fn wait_for_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    // SAFETY: OpenProcess has no preconditions; the handle is closed by `Owned`.
    let process = match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
        Ok(handle) => unsafe { Owned::new(handle) },
        Err(error) if error.code() == HRESULT::from_win32(ERROR_INVALID_PARAMETER.0) => return Ok(()),
        Err(error) => return Err(format!("cannot watch process {pid}: {error}")),
    };
    // SAFETY: the handle is valid for the call.
    let waited = unsafe { WaitForSingleObject(*process, u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX)) };
    match waited {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => Err(format!("process {pid} did not end within {} s", timeout.as_secs())),
        other => Err(format!("waiting for process {pid} failed ({other:?})")),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::update::package::tests::zip_of;
    use crate::update::stage::tests::{key, published, stage_published, TempDir};

    const RUNNING: Version = Version::new(1, 0, 0);
    const NEXT: Version = Version::new(2, 0, 0);
    const EXE: &str = "meter.exe";

    fn real_rename(from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    /// An install folder holding `installed`, and the update `files` staged for `NEXT`.
    fn install_with(dir: &TempDir, installed: &[(&str, &str)], files: &[(&str, &[u8])]) -> Install {
        let install_dir = dir.0.join("install");
        fs::create_dir_all(&install_dir).unwrap();
        for (name, contents) in installed {
            fs::write(install_dir.join(name), contents).unwrap();
        }
        let install = Install { dir: install_dir, exe_name: EXE.to_owned() };
        stage_published(&install.layout(), &published(files, NEXT), NEXT);
        install
    }

    fn read(install: &Install, name: &str) -> Option<String> {
        fs::read_to_string(install.dir.join(name)).ok()
    }

    fn apply(install: &Install) -> Result<Version, String> {
        apply_staged(install, RUNNING, &key(), &real_rename)
    }

    #[test]
    fn the_staged_files_replace_the_old_ones_which_are_kept_aside() {
        let dir = TempDir::new("apply-ok");
        let install = install_with(
            &dir,
            &[(EXE, "old exe"), ("WinDivert.dll", "old dll")],
            &[(EXE_ENTRY, b"new exe"), ("WinDivert.dll", b"new dll"), ("WinDivert64.sys", b"new sys")],
        );

        assert_eq!(apply(&install), Ok(NEXT));

        assert_eq!(read(&install, EXE).as_deref(), Some("new exe"), "the update's exe takes the running exe's name");
        assert_eq!(read(&install, "WinDivert.dll").as_deref(), Some("new dll"));
        assert_eq!(read(&install, "WinDivert64.sys").as_deref(), Some("new sys"));
        assert_eq!(read(&install, "meter.exe.old").as_deref(), Some("old exe"));
        assert_eq!(read(&install, "WinDivert.dll.old").as_deref(), Some("old dll"));
        assert!(!install.dir.join(EXE_ENTRY).exists(), "nothing is written under the archive's name");
        assert!(!install.dir.join("meter.exe.new").exists());
    }

    #[test]
    fn a_file_that_did_not_change_is_left_alone() {
        let dir = TempDir::new("apply-same");
        let install = install_with(
            &dir,
            &[(EXE, "old"), ("WinDivert64.sys", "driver")],
            &[(EXE_ENTRY, b"new"), ("WinDivert64.sys", b"driver")],
        );

        apply(&install).unwrap();

        assert_eq!(read(&install, "WinDivert64.sys").as_deref(), Some("driver"));
        assert!(!install.dir.join("WinDivert64.sys.old").exists(), "nothing was moved aside");
    }

    #[test]
    fn a_staged_archive_changed_after_staging_installs_nothing() {
        let dir = TempDir::new("apply-tampered");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"genuine")]);
        fs::write(install.layout().ready_dir().join("update.zip"), zip_of(&[(EXE_ENTRY, b"planted")])).unwrap();

        assert!(apply(&install).is_err());

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
        assert!(!install.dir.join("meter.exe.old").exists() && !install.dir.join("meter.exe.new").exists());
    }

    #[test]
    fn a_planted_archive_with_a_matching_hash_but_no_signature_of_the_publisher_installs_nothing() {
        let dir = TempDir::new("apply-forged");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"genuine")]);
        let planted = zip_of(&[(EXE_ENTRY, b"planted")]);
        fs::write(install.layout().ready_dir().join("update.zip"), &planted).unwrap();
        fs::write(install.layout().ready_dir().join("update.sha256"), verify::sha256_hex(&planted)).unwrap();

        assert!(apply(&install).unwrap_err().contains("signature"));

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
    }

    #[test]
    fn a_signature_by_another_key_installs_nothing() {
        let dir = TempDir::new("apply-other-key");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"new")]);
        let other = ed25519_dalek::SigningKey::from_bytes(&[0x13; 32]).verifying_key();

        assert!(apply_staged(&install, RUNNING, &other, &real_rename).is_err());

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
    }

    #[test]
    fn an_update_that_is_not_newer_than_the_running_version_installs_nothing() {
        let dir = TempDir::new("apply-not-newer");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"same version")]);

        assert!(apply_staged(&install, NEXT, &key(), &real_rename).is_err());

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
    }

    #[test]
    fn nothing_staged_installs_nothing() {
        let dir = TempDir::new("apply-nothing");
        let install = Install { dir: dir.0.clone(), exe_name: EXE.to_owned() };
        fs::write(dir.0.join(EXE), "old exe").unwrap();

        assert!(apply(&install).is_err());

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
    }

    #[test]
    fn a_failure_while_placing_files_puts_everything_back() {
        let dir = TempDir::new("apply-fail-place");
        let install = install_with(
            &dir,
            &[(EXE, "old exe"), ("WinDivert.dll", "old dll")],
            &[(EXE_ENTRY, b"new exe"), ("WinDivert.dll", b"new dll")],
        );
        let moves = Cell::new(0);
        // Two files go aside, then the first placement works and the second does not.
        let rename = |from: &Path, to: &Path| {
            moves.set(moves.get() + 1);
            if moves.get() == 4 {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
            }
            fs::rename(from, to)
        };

        let error = apply_staged(&install, RUNNING, &key(), &rename).unwrap_err();

        assert!(error.contains("locked") && error.contains("put back"), "{error}");
        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"), "the old exe is back and runnable");
        assert_eq!(read(&install, "WinDivert.dll").as_deref(), Some("old dll"));
        for left_over in ["meter.exe.old", "WinDivert.dll.old", "meter.exe.new", "WinDivert.dll.new"] {
            assert!(!install.dir.join(left_over).exists(), "{left_over}");
        }
    }

    #[test]
    fn a_new_file_that_failed_to_land_is_removed_again() {
        let dir = TempDir::new("apply-fail-new-file");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"new exe"), ("WinDivert64.sys", b"new sys")]);
        let moves = Cell::new(0);
        // The exe goes aside and in; the driver, which was not installed before, then fails.
        let rename = |from: &Path, to: &Path| {
            moves.set(moves.get() + 1);
            if moves.get() == 3 {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
            }
            fs::rename(from, to)
        };

        assert!(apply_staged(&install, RUNNING, &key(), &rename).is_err());

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
        assert!(!install.dir.join("WinDivert64.sys").exists() && !install.dir.join("WinDivert64.sys.new").exists());
    }

    #[test]
    fn a_failure_while_moving_the_old_exe_aside_changes_nothing() {
        let dir = TempDir::new("apply-fail-aside");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"new exe")]);
        let rename = |_: &Path, _: &Path| Err(io::Error::new(io::ErrorKind::PermissionDenied, "in use"));

        assert!(apply_staged(&install, RUNNING, &key(), &rename).unwrap_err().contains("in use"));

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
        assert!(!install.dir.join("meter.exe.new").exists());
    }

    #[test]
    fn a_new_file_that_cannot_be_written_stops_before_anything_is_moved() {
        let dir = TempDir::new("apply-fail-write");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"new exe")]);
        fs::create_dir(install.dir.join("meter.exe.new")).unwrap();

        assert!(apply(&install).unwrap_err().contains("cannot write"));

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
        assert!(!install.dir.join("meter.exe.old").exists());
    }

    #[test]
    fn a_stale_old_file_from_an_earlier_update_is_replaced() {
        let dir = TempDir::new("apply-stale-old");
        let install = install_with(&dir, &[(EXE, "old exe"), ("meter.exe.old", "ancient")], &[(EXE_ENTRY, b"new exe")]);

        apply(&install).unwrap();

        assert_eq!(read(&install, "meter.exe.old").as_deref(), Some("old exe"));
    }

    #[test]
    fn an_archive_with_other_names_writes_nothing_outside_the_three_files() {
        let dir = TempDir::new("apply-names");
        let install = install_with(&dir, &[(EXE, "old exe")], &[(EXE_ENTRY, b"new exe")]);
        let hostile = published(&[(EXE_ENTRY, b"new exe"), ("../outside.dll", b"evil"), ("sethc.exe", b"evil")], NEXT);
        fs::write(install.layout().ready_dir().join("update.zip"), &hostile.archive).unwrap();
        fs::write(install.layout().ready_dir().join("update.sha256"), &hostile.checksum).unwrap();
        fs::write(install.layout().ready_dir().join("update.sig"), &hostile.signature).unwrap();

        assert!(apply(&install).is_err(), "even a validly signed archive may only hold the allowed names");

        assert_eq!(read(&install, EXE).as_deref(), Some("old exe"));
        assert!(!dir.0.join("outside.dll").exists() && !install.dir.join("sethc.exe").exists());
    }

    #[test]
    fn only_the_apply_flag_with_one_process_id_is_the_helper_and_everything_else_is_refused() {
        let args = |parts: &[&str]| parts.iter().map(|part| (*part).to_owned()).collect::<Vec<_>>();

        assert_eq!(Request::from_args(&[]), None);
        assert_eq!(Request::from_args(&args(&["--replay", "x"])), None);
        assert_eq!(Request::from_args(&args(&[FLAG, "4242"])), Some(Ok(Request { parent_pid: 4242 })));
        for refused in [
            args(&[FLAG]),
            args(&[FLAG, "0"]),
            args(&[FLAG, "abc"]),
            args(&[FLAG, "-1"]),
            args(&[FLAG, "4242", "C:/Windows/System32"]),
            args(&[FLAG, "4242", "C:/install", "C:/staged", "sethc.exe"]),
        ] {
            assert!(matches!(Request::from_args(&refused), Some(Err(_))), "{refused:?}");
        }
    }

    #[test]
    fn a_process_that_did_not_start_this_one_is_not_waited_for_and_nothing_proceeds() {
        assert_eq!(confirm_parent(4242, Ok(4242)), Ok(()));
        assert!(confirm_parent(4242, Ok(77)).unwrap_err().contains("did not start"));
        assert!(confirm_parent(0, Ok(0)).is_err(), "no parent is not a process that ended");
        assert!(confirm_parent(4242, Err("no answer".to_owned())).is_err());
    }

    #[test]
    fn this_process_reports_its_real_parent() {
        let parent = parent_process_id().unwrap();

        assert_ne!(parent, 0);
        assert_ne!(parent, std::process::id());
    }

    #[test]
    fn the_exe_name_an_update_installs_under_is_a_plain_exe_name() {
        let install = Install::of_exe(Path::new("C:/Tools/meter/belisk.exe")).unwrap();
        assert_eq!(install, Install { dir: PathBuf::from("C:/Tools/meter"), exe_name: "belisk.exe".to_owned() });

        for refused in [
            "C:/Tools/meter/WinDivert.dll",
            "C:/Tools/meter/windivert.dll",
            "C:/Tools/meter/WinDivert64.sys",
            "C:/Tools/meter/notes.txt",
            "C:/Tools/meter/.exe",
            "C:/Tools/meter/meter.exe.old",
            "meter.exe",
        ] {
            assert!(Install::of_exe(Path::new(refused)).is_err(), "{refused}");
        }
    }

    #[test]
    fn a_restart_does_not_start_when_the_staged_update_is_not_beside_this_exe() {
        let dir = TempDir::new("apply-restart-elsewhere");
        let layout = Layout::new(dir.0.join("elsewhere"));

        assert!(start_helper(&layout, &key(), &ImageLock::of_this_test_exe()).unwrap_err().contains("not beside"));
    }

    #[test]
    fn a_restart_does_not_start_when_nothing_verifiable_is_staged_beside_this_exe() {
        let exe = std::env::current_exe().unwrap();
        let layout = Layout::new(exe.parent().unwrap().to_owned());
        assert!(!layout.root().exists(), "the test exe's folder holds no staged update");

        assert!(start_helper(&layout, &key(), &ImageLock::of_this_test_exe()).unwrap_err().contains("no readable"));
    }

    #[test]
    fn a_restart_does_not_start_an_exe_other_than_the_one_that_was_held() {
        let dir = TempDir::new("apply-restart-other-image");
        let other = dir.0.join("other.exe");
        fs::write(&other, "x").unwrap();
        let held = ImageLock::acquire(&other).unwrap();
        let layout = Layout::new(std::env::current_exe().unwrap().parent().unwrap().to_owned());

        assert!(start_helper(&layout, &key(), &held).unwrap_err().contains("not the exe that was held"));
    }

    #[test]
    fn a_held_exe_cannot_be_renamed_deleted_or_written_by_anyone_else_until_it_is_released() {
        let dir = TempDir::new("apply-image-lock");
        let exe = dir.0.join("meter.exe");
        fs::write(&exe, "genuine").unwrap();
        let held = ImageLock::acquire(&exe).unwrap();

        assert!(fs::rename(&exe, dir.0.join("meter.exe.moved")).is_err(), "renaming the image aside is refused");
        assert!(fs::remove_file(&exe).is_err());
        assert!(fs::write(&exe, "planted").is_err());
        assert!(fs::rename(&dir.0, dir.0.with_extension("moved")).is_err(), "so is moving its folder");
        assert_eq!(fs::read_to_string(&exe).unwrap(), "genuine", "reading stays possible");

        drop(held);
        fs::rename(&exe, dir.0.join("meter.exe.moved")).unwrap();
    }

    #[test]
    fn an_exe_that_is_missing_cannot_be_held() {
        let dir = TempDir::new("apply-image-missing");

        assert!(ImageLock::acquire(&dir.0.join("gone.exe")).unwrap_err().contains("cannot hold"));
    }

    #[test]
    fn waiting_for_a_process_returns_when_it_ends_and_times_out_while_it_runs() {
        let mut quick = Command::new("cmd").args(["/c", "exit", "0"]).spawn().unwrap();
        quick.wait().unwrap();
        assert_eq!(wait_for_exit(quick.id(), Duration::from_secs(5)), Ok(()), "an ended process is not waited for");

        let mut slow = Command::new("cmd").args(["/c", "ping", "-n", "6", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let waited = wait_for_exit(slow.id(), Duration::from_millis(200));
        slow.kill().unwrap();
        slow.wait().unwrap();
        assert!(waited.unwrap_err().contains("did not end"));
    }
}
