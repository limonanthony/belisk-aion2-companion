//! Quitting ends the process. The quit hooks save everything and stop capture
//! in milliseconds; what follows (GPUI's window and Direct3D teardown, DLL
//! detach) is outside this program's control and was seen to hang with the
//! window hidden and the exe still running. The process is therefore ended
//! from the future of the last quit hook, which GPUI polls once every hook has
//! run and the windows are released, so teardown is never reached. A deadline
//! ends it if a hook itself hangs; every file is written atomically, so being
//! cut by it keeps the previous contents.

use std::thread;
use std::time::Duration;

use gpui::App;
use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};

const HOOK_DEADLINE: Duration = Duration::from_secs(3);

/// Must be installed before any other quit hook, so the deadline covers them all.
pub fn install_deadline(cx: &mut App) {
    cx.on_app_quit(|_| {
        arm_deadline();
        async {}
    })
    .detach();
}

/// Ends the process once every quit hook has run: GPUI polls the hooks'
/// futures only after all of them, and after the windows are released.
pub fn install_exit(cx: &mut App, exit_code: impl Fn() -> u32 + 'static) {
    cx.on_app_quit(move |_| {
        let code = exit_code();
        async move { terminate(code) }
    })
    .detach();
}

fn arm_deadline() {
    let spawned = thread::Builder::new().name("quit-deadline".to_owned()).spawn(|| {
        thread::sleep(HOOK_DEADLINE);
        terminate(0)
    });
    if let Err(error) = spawned {
        crate::app_log::warn(format!("cannot arm the quit deadline ({error}); a hung quit hook would keep the meter running"));
    }
}

pub fn terminate(code: u32) -> ! {
    // SAFETY: the pseudo-handle of the current process is always valid.
    let _ = unsafe { TerminateProcess(GetCurrentProcess(), code) };
    std::process::exit(code as i32)
}
