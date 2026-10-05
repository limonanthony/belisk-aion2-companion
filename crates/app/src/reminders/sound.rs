//! The reminder's sound: Windows' own "chimes" sound, played from the user's
//! Windows install (`<Windows folder>\Media\chimes.wav`). It is Microsoft's, so
//! no audio file ships with the app. If the file is not there, the Windows
//! notification sound (the "Notification" event under Settings → System → Sound)
//! plays instead.
//!
//! `SND_ASYNC` returns at once and winmm plays on its own thread;
//! `SND_NODEFAULT` keeps a PC without the sound silent instead of beeping. The
//! call still goes through the background executor, because the first one reads
//! the sound file from disk.

use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use gpui::App;
use windows::core::{w, PCWSTR};
use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};

fn chimes_path(windows_dir: &Path) -> PathBuf {
    windows_dir.join("Media").join("chimes.wav")
}

fn installed_chimes() -> Option<PathBuf> {
    let windows_dir = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir"))?;
    let path = chimes_path(Path::new(&windows_dir));
    path.is_file().then_some(path)
}

fn wide_nul_terminated(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = OsString::from(path).encode_wide().collect();
    wide.push(0);
    wide
}

pub fn play_notification(cx: &App) {
    cx.background_executor()
        .spawn(async {
            let played = match installed_chimes() {
                Some(path) => {
                    let wide = wide_nul_terminated(&path);
                    unsafe { PlaySoundW(PCWSTR(wide.as_ptr()), None, SND_FILENAME | SND_ASYNC | SND_NODEFAULT) }
                }
                None => unsafe { PlaySoundW(w!("SystemNotification"), None, SND_ALIAS | SND_ASYNC | SND_NODEFAULT) },
            };
            if !played.as_bool() {
                eprintln!("the reminder sound could not be played (no chimes.wav in the Windows folder and no \"SystemNotification\" sound set in Windows)");
            }
        })
        .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chimes_live_under_the_windows_folders_media_directory() {
        assert_eq!(chimes_path(Path::new(r"D:\WinNT")), Path::new(r"D:\WinNT\Media\chimes.wav"));
    }
}
