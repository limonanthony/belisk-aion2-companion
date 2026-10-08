//! Off-screen captures and checks of the Settings window, run by `--capture
//! --fixtures` after the other windows'. The window is opened from the
//! toolbar's gear, placed past every monitor, and driven with keys and
//! clicks posted to our own window; nothing asks Windows for the foreground.
//! The meter's state the page shows (capture status, recordings, personal
//! bests, the capture check) is invented and held still.

use std::path::Path;
use std::time::Duration;

use meter_core::preflight::{Check, CheckId, Report, Weight};
use gpui::{AsyncApp, Bounds, Entity, Pixels, WindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    MA_ACTIVATE, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::capture::{self, shoot_hwnd, MK_LBUTTON};
use crate::capture_reminders;
use crate::capture_secondary::{
    close, pack, pause, post, shoot_window, show_scene, until_handled, window_handle, window_hwnd,
};
use crate::fixtures;
use crate::i18n::fl;
use crate::meter::{Layout, MeterModel, MeterView, OverlaySettings, PctMode};
use crate::overlay::{self, off_screen};
use crate::sample;
use crate::secondary::{self, WindowKind};
use crate::settings::{
    self, BestRow, CaptureCheck, CaptureStatus, MeterTile, RecordingItem, RecordingOrigin, RecordingProgress,
    ReplayProgress,
    RuntimeModel, Section, SettingsView, StorageState, StorageSummary,
};
use crate::todos;
use crate::capture_toolbar;
use crate::toolbar::{Slot, Tool};

/// Inside the page's top-left padding, where no control is.
const BACKGROUND_POINT: (i32, i32) = (216, 44);
const TICK: Duration = Duration::from_millis(150);
const KEY_GAP: Duration = Duration::from_millis(40);
const KEY_UP: isize = 0xC000_0001;
/// The middle of the 36 px title bar.
const TITLE_BAR_Y: f32 = 18.0;

const VK_TAB: usize = 0x09;
const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const VK_SPACE: usize = 0x20;
const VK_NEXT: usize = 0x22;
const VK_LEFT: usize = 0x25;
const VK_DOWN: usize = 0x28;

/// What the layout check sets, one case after the other, and the shot it
/// takes of it.
const LAYOUT_CASES: [(&str, Option<&str>, fn(&mut OverlaySettings)); 11] = [
    ("Total damage on at 100%", Some("settings-07-layout-damage-on-100"), |overlay| overlay.show_damage = true),
    ("Total damage off at 150%", Some("settings-07-layout-damage-off-150"), |overlay| {
        overlay.show_damage = false;
        overlay.content_scale = 1.5;
    }),
    ("Total damage on at 150%", Some("settings-07-layout-damage-on-150"), |overlay| overlay.show_damage = true),
    ("Players shown 5", Some("settings-07-layout-players-5"), |overlay| overlay.players_shown = 5),
    ("Players shown 20", Some("settings-07-layout-players-20"), |overlay| overlay.players_shown = 20),
    ("Capsule layout", Some("settings-07-layout-capsule"), |overlay| overlay.layout = Layout::Capsule),
    ("Full layout, DPS off", None, |overlay| {
        overlay.layout = Layout::Full;
        overlay.show_dps = false;
    }),
    ("Server on", None, |overlay| overlay.show_server = true),
    ("Combat power off", None, |overlay| overlay.show_combat_power = false),
    ("Percentage on share", None, |overlay| overlay.pct_mode = PctMode::Share),
    ("Content scale 70%", None, |overlay| overlay.content_scale = 0.7),
];
/// The Overlay page's marks whose bounds must not change.
const ROW_MARKS: [&str; 3] = ["overlay-rows", "opacity", "scale"];

type Result<T, E = String> = std::result::Result<T, E>;

struct Page {
    hwnd: HWND,
    view: WindowHandle<SettingsView>,
    runtime: Entity<RuntimeModel>,
}

pub async fn from_fixtures(
    meter: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    dir: &Path,
    cx: &mut AsyncApp,
) -> Result<()> {
    open_from_toolbar(model, cx).await?;
    let page = Page {
        hwnd: window_hwnd(WindowKind::Settings, cx)?,
        view: window_handle::<SettingsView>(WindowKind::Settings, cx)?,
        runtime: cx
            .update(|cx| secondary::model_of::<RuntimeModel>(WindowKind::Settings, cx))
            .ok_or("the Settings window has no runtime model")?,
    };
    report_window(&page, cx)?;
    page.hold(cx, |runtime| {
        runtime.capture = running_capture();
        runtime.recordings = recordings();
        runtime.bests = bests();
        runtime.storage = StorageState::Loaded(storage());
    });

    overlay_page(&page, meter, dir, cx).await?;
    overlay_layout(&page, dir, cx).await?;
    behaviour_page(&page, dir, cx).await?;
    capture_page(&page, dir, cx).await?;
    bests_page(&page, dir, cx).await?;
    reminders_page(&page, dir, cx).await?;
    todos_page(&page, dir, cx).await?;
    general_page(&page, dir, cx).await?;
    about_page(&page, dir, cx).await?;
    report_title_bar(&page, cx);
    close(WindowKind::Settings, cx).await?;
    println!("settings: closed, the registry let go of it");
    Ok(())
}

/// The real entry point: a click on the toolbar's gear. Opening it again
/// makes no second window.
async fn open_from_toolbar(model: &Entity<MeterModel>, cx: &mut AsyncApp) -> Result<()> {
    show_scene(model, fixtures::open_world(), cx);
    capture_toolbar::click(Slot::Tool(Tool::Settings), cx).await?;
    pause(cx, TICK * 2).await;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Settings, cx).is_some());
    println!("entry point: click on Settings on the toolbar: Settings window open {open}");
    if !open {
        return Err("the Settings window did not open from the toolbar".to_string());
    }
    let before = cx.update(|cx| cx.windows().len());
    cx.update(|cx| secondary::open_settings(model, cx));
    pause(cx, TICK).await;
    let after = cx.update(|cx| cx.windows().len());
    println!("single instance: Settings opened twice, {before} windows before the second open, {after} after");
    Ok(())
}

async fn overlay_page(page: &Page, meter: WindowHandle<MeterView>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    shoot_window(WindowKind::Settings, dir, "settings-01-overlay", cx).await?;

    page.tab_to("opacity", cx).await?;
    println!("keyboard: Tab reached {:?}", page.focused(cx));
    shoot_window(WindowKind::Settings, dir, "settings-02-focus-ring", cx).await?;

    for vk in [VK_NEXT, VK_NEXT, VK_LEFT, VK_LEFT, VK_LEFT, VK_LEFT, VK_LEFT] {
        page.key(vk, cx).await;
    }
    let alpha = cx.update(|cx| settings::global(cx).read(cx).get().overlay.background_alpha);
    println!("keyboard: PageDown twice and Left five times on Background opacity: {alpha} of 255 ({:.0}%)", f64::from(alpha) / 2.55);
    shoot_window(WindowKind::Settings, dir, "settings-03-opacity-by-keyboard", cx).await?;
    shoot_meter(meter, dir, "settings-04-meter-after-opacity", cx).await?;

    let track = page.bounds_of("scale", cx)?;
    let at = |fraction: f32| {
        let x = track.left() + track.size.width * fraction;
        (f32::from(x).round() as i32, f32::from(track.center().y).round() as i32)
    };
    post(page.hwnd, WM_MOUSEMOVE, 0, pack(at(0.25)));
    post(page.hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(at(0.25)));
    pause(cx, TICK).await;
    let pressed = content_scale(cx);
    post(page.hwnd, WM_MOUSEMOVE, MK_LBUTTON, pack(at(0.5)));
    post(page.hwnd, WM_LBUTTONUP, 0, pack(at(0.5)));
    pause(cx, TICK).await;
    println!(
        "mouse: press at a quarter of Content scale: {:.0}%, dragged to the middle: {:.0}%, focused {:?}",
        pressed * 100.,
        content_scale(cx) * 100.,
        page.focused(cx)
    );
    shoot_window(WindowKind::Settings, dir, "settings-05-scale-after-drag", cx).await?;

    page.tab_to("total-damage", cx).await?;
    page.key(VK_SPACE, cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-06-total-damage-on", cx).await?;
    page.tab_to("detail-opens", cx).await?;
    shoot_window(WindowKind::Settings, dir, "settings-06b-overlay-window", cx).await?;
    page.tab_to("layout", cx).await?;
    let damage = cx.update(|cx| settings::global(cx).read(cx).get().overlay.show_damage);
    println!("keyboard: Space on Total damage: shown {damage}");
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-06-preview-follows", cx).await?;
    shoot_meter(meter, dir, "settings-07-meter-follows", cx).await?;
    reset_settings(cx);
    Ok(())
}

/// No overlay setting moves the Overlay page's rows: each case is set on the
/// store, one after the other, and the rows (their column and the two
/// sliders' tracks) must sit exactly where they sat with the defaults. The
/// preview keeps its place and width and never reaches below the rows, so
/// the page's height holds too.
async fn overlay_layout(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::Behaviour, cx).await?;
    page.show(Section::Overlay, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-07-layout-damage-off-100", cx).await?;
    let (rows, preview) = overlay_layout_bounds(page, cx)?;
    println!("layout: defaults: rows {:?}, preview {:?}", rows[0], preview);
    let mut moved = Vec::new();
    for (case, shot, change) in LAYOUT_CASES {
        cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| change(&mut all.overlay))));
        pause(cx, TICK).await;
        if let Some(shot) = shot {
            shoot_window(WindowKind::Settings, dir, shot, cx).await?;
        }
        let (now, now_preview) = overlay_layout_bounds(page, cx)?;
        let rows_held = now == rows;
        let preview_held = now_preview.origin == preview.origin && now_preview.size.width == preview.size.width;
        let preview_above = now_preview.bottom() <= now[0].bottom();
        println!(
            "layout: {case}: rows held {rows_held}, preview place and width held {preview_held}, preview {:.0}x{:.0} ends above the rows' end {preview_above}",
            f32::from(now_preview.size.width),
            f32::from(now_preview.size.height),
        );
        if !(rows_held && preview_held && preview_above) {
            moved.push(format!("{case}: rows {now:?}, preview {now_preview:?}"));
        }
    }
    reset_settings(cx);
    if moved.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "overlay settings moved the Overlay page's layout (defaults: rows {rows:?}, preview {preview:?}): {}",
            moved.join("; ")
        ))
    }
}

fn overlay_layout_bounds(page: &Page, cx: &mut AsyncApp) -> Result<(Vec<Bounds<Pixels>>, Bounds<Pixels>)> {
    let rows = ROW_MARKS.iter().map(|mark| page.bounds_of(mark, cx)).collect::<Result<_>>()?;
    Ok((rows, page.bounds_of("preview", cx)?))
}

async fn behaviour_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::Behaviour, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-08-behaviour", cx).await?;

    page.tab_to("end-fight", cx).await?;
    page.key(VK_DOWN, cx).await;
    println!("select: Down opens it: open {}", page.select_open(cx));
    shoot_window(WindowKind::Settings, dir, "settings-09-select-open", cx).await?;
    page.key(VK_DOWN, cx).await;
    page.key(VK_RETURN, cx).await;
    let secs = cx.update(|cx| settings::global(cx).read(cx).get().behaviour.end_fight_after_secs);
    println!("select: Down, Enter picks the next option: {secs} s, open {}, focused {:?}", page.select_open(cx), page.focused(cx));
    page.key(VK_SPACE, cx).await;
    let opened = page.select_open(cx);
    page.key(VK_ESCAPE, cx).await;
    println!(
        "select: Space opens it ({opened}), Escape closes it: open {}, focused {:?}",
        page.select_open(cx),
        page.focused(cx)
    );

    page.key(VK_DOWN, cx).await;
    let opened = page.select_open(cx);
    page.click(BACKGROUND_POINT, cx).await;
    println!(
        "pointer: a press outside the open select ({opened}): open {}, focused {:?}",
        page.select_open(cx),
        page.focused(cx)
    );
    page.tab_to("players", cx).await?;
    let before = page.focused(cx);
    page.click(BACKGROUND_POINT, cx).await;
    println!("pointer: a press on the page's background with {before:?} focused: focused {:?}", page.focused(cx));

    let track = page.bounds_of("players", cx)?;
    let point = (f32::from(track.right()).round() as i32 - 2, f32::from(track.center().y).round() as i32);
    page.click(point, cx).await;
    let players = cx.update(|cx| settings::global(cx).read(cx).get().overlay.players_shown);
    println!("pointer: a press at the right end of Players shown: {players}, focused {:?}", page.focused(cx));
    reset_settings(cx);
    Ok(())
}

async fn capture_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::Capture, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-10-capture", cx).await?;

    page.hold(cx, |runtime| runtime.capture_check = CaptureCheck::Running);
    shoot_window(WindowKind::Settings, dir, "settings-11-capture-check-running", cx).await?;
    page.hold(cx, |runtime| runtime.capture_check = CaptureCheck::Done(check_report()));
    shoot_window(WindowKind::Settings, dir, "settings-12-capture-check-done", cx).await?;
    page.hold(cx, |runtime| runtime.capture_check = CaptureCheck::Done(failed_check_report()));
    page.tab_to("record-unfamiliar", cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-12b-capture-check-failed", cx).await?;
    page.hold(cx, |runtime| runtime.capture_check = CaptureCheck::Done(check_report()));

    page.hold(cx, |runtime| runtime.recording = Some(RecordingProgress { packets: 2_418, bytes: 1_468_006 }));
    page.tab_to("recording", cx).await?;
    println!("keyboard: Tab to Stop recording scrolled the page to it: focused {:?}", page.focused(cx));
    shoot_window(WindowKind::Settings, dir, "settings-13-recording", cx).await?;

    let first = recordings().remove(0).name;
    page.hold(cx, |runtime| {
        runtime.recording = None;
        runtime.replay = ReplayProgress::Running { name: first };
    });
    page.tab_to("cancel-replay", cx).await?;
    shoot_window(WindowKind::Settings, dir, "settings-14-replaying", cx).await?;

    page.hold(cx, |runtime| runtime.replay = ReplayProgress::Finished { packets: 18_204 });
    page.tab_to("log-folder", cx).await?;
    shoot_window(WindowKind::Settings, dir, "settings-15-capture-end", cx).await?;
    Ok(())
}

async fn bests_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::Bests, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-16-bests", cx).await?;
    page.tab_to("bests-reset", cx).await?;
    page.key(VK_RETURN, cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-17-bests-confirm", cx).await?;
    page.hold(cx, |runtime| runtime.bests.clear());
    shoot_window(WindowKind::Settings, dir, "settings-18-bests-empty", cx).await?;
    Ok(())
}

/// The defaults, the lead time's list, a test reminder (the real card opens
/// above the meter), and the master switch off.
async fn reminders_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::Reminders, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-21-reminders", cx).await?;

    page.tab_to("lead-time", cx).await?;
    page.key(VK_DOWN, cx).await;
    println!("reminders: Down on Remind me opens it: open {}", page.select_open(cx));
    shoot_window(WindowKind::Settings, dir, "settings-22-reminders-select-open", cx).await?;
    page.key(VK_ESCAPE, cx).await;

    page.tab_to("test-reminder", cx).await?;
    let foreground_before = capture_reminders::foreground();
    page.key(VK_RETURN, cx).await;
    pause(cx, TICK).await;
    let note = page.view.update(cx, |view, _, _| view.test_note_is_shown()).unwrap_or(false);
    let card = cx.update(|cx| secondary::window_of(WindowKind::Reminder, cx).is_some());
    println!("reminders: Enter on Show a test reminder: note shown {note}, reminder window open {card}");
    if !note || !card {
        return Err("Show a test reminder did not show the card and its note".to_string());
    }
    capture_reminders::check_never_focused(WindowKind::Reminder, "the test reminder", foreground_before, cx)?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-23-reminders-test", cx).await?;
    shoot_window(WindowKind::Reminder, dir, "reminder-00-test", cx).await?;

    page.tab_to("reminders", cx).await?;
    page.key(VK_SPACE, cx).await;
    page.key(VK_TAB, cx).await;
    let enabled = cx.update(|cx| settings::global(cx).read(cx).get().reminders.enabled);
    println!("reminders: Space on Event reminders: on {enabled}; Tab then goes to {:?} (the dimmed rows are skipped)", page.focused(cx));
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-24-reminders-off", cx).await?;
    reset_settings(cx);
    Ok(())
}

/// The page with five built-ins hidden on two characters, the reset hour's
/// list, Restore with its badge, and the complete reset through its dialog.
async fn todos_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let todos = cx.update(|cx| todos::global(cx));
    let characters: Vec<_> = cx.update(|cx| todos.read(cx).characters().into_iter().map(|character| character.id).collect());
    cx.update(|cx| {
        todos.update(cx, |todos, cx| {
            for (index, id) in characters.iter().take(2).enumerate() {
                for row in ["trial", "subj", "deus"].iter().skip(index) {
                    todos.hide(*id, row, cx);
                }
            }
        })
    });
    let login = |tickets: &[(u32, u32, u32)], cx: &mut AsyncApp| {
        let report = crate::fixtures::ticket_login(&crate::capture_todos::known("Mirelle"), crate::clock::now_ms(), tickets);
        cx.update(|cx| todos.update(cx, |todos, cx| todos.apply_tickets(&[report], cx)));
    };
    login(crate::fixtures::MIRELLE_TICKETS, cx);
    page.show(Section::Todos, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-25-todos", cx).await?;

    login(&[(1, 18, 3)], cx);
    pause(cx, TICK).await;
    let hint = cx.update(|cx| todos.read(cx).membership_hint());
    println!("todos: the game shows 18 Shugo keys with the membership off: hint {hint:?}");
    if hint.is_none() {
        return Err("Settings → To-dos did not ask about the membership".to_string());
    }
    shoot_window(WindowKind::Settings, dir, "settings-25b-todos-membership-hint", cx).await?;
    login(crate::fixtures::MIRELLE_TICKETS, cx);

    page.tab_to("reset-hour", cx).await?;
    page.key(VK_DOWN, cx).await;
    println!("todos: Down on Daily reset opens it: open {}", page.select_open(cx));
    shoot_window(WindowKind::Settings, dir, "settings-26-todos-hour-open", cx).await?;
    page.key(VK_ESCAPE, cx).await;

    page.tab_to("todos-restore", cx).await?;
    page.key(VK_RETURN, cx).await;
    let badge = page.view.update(cx, |view, _, _| view.todos_done_text()).ok().flatten();
    let hidden = cx.update(|cx| todos.read(cx).hidden_summary());
    println!("todos: Enter on Restore default to-dos: badge {badge:?}, still hidden {}", hidden.hidden);
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-27-todos-restored", cx).await?;

    page.tab_to("todos-reset", cx).await?;
    page.key(VK_RETURN, cx).await;
    println!("todos: Enter on Reset to-dos completely: dialog open {}, focused {:?}", page.dialog_open(cx), page.focused(cx));
    shoot_window(WindowKind::Settings, dir, "settings-28-todos-confirm", cx).await?;
    page.key(VK_TAB, cx).await;
    pause(cx, TICK).await;
    println!("todos: Tab in the dialog goes to {:?}", page.focused(cx));
    page.key(VK_RETURN, cx).await;
    pause(cx, TICK).await;
    let (own, badge) = (
        cx.update(|cx| todos.read(cx).own_total()),
        page.view.update(cx, |view, _, _| view.todos_done_text()).ok().flatten(),
    );
    println!("todos: Tab then Enter in the dialog: open {}, own to-dos left {own}, badge {badge:?}", page.dialog_open(cx));
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-29-todos-reset", cx).await?;
    if badge != Some(fl!("settings-todos-reset-done")) || own != 0 {
        return Err("Reset to-dos completely did not reset the to-dos".to_string());
    }
    Ok(())
}

/// The page, and the language list opened from the keyboard.
async fn general_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::General, cx).await?;
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-30-general", cx).await?;
    page.tab_to("language", cx).await?;
    page.key(VK_DOWN, cx).await;
    println!("general: Down on Language opens it: open {}", page.select_open(cx));
    shoot_window(WindowKind::Settings, dir, "settings-31-general-language-open", cx).await?;
    page.key(VK_ESCAPE, cx).await;
    Ok(())
}

/// Settings back to their defaults, in the language the capture draws.
fn reset_settings(cx: &mut AsyncApp) {
    cx.update(|cx| {
        settings::global(cx).update(cx, |settings, cx| {
            let language = settings.get().general.language;
            settings.reset(cx);
            settings.update(cx, |all| all.general.language = language);
        })
    });
}

async fn about_page(page: &Page, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    page.show(Section::About, cx).await?;
    // Opening About sizes the data folder; the capture shows a held summary.
    page.hold(cx, |runtime| runtime.storage = StorageState::Loaded(storage()));
    page.blur(cx).await;
    shoot_window(WindowKind::Settings, dir, "settings-19-about", cx).await?;

    page.tab_to("reset-settings", cx).await?;
    page.key(VK_RETURN, cx).await;
    println!("dialog: Enter on Reset settings: open {}, focused {:?}", page.dialog_open(cx), page.focused(cx));
    shoot_window(WindowKind::Settings, dir, "settings-20-reset-dialog", cx).await?;
    page.key(VK_TAB, cx).await;
    let second = page.focused(cx);
    page.key(VK_TAB, cx).await;
    println!("dialog: Tab goes to {second:?}, then back to {:?}", page.focused(cx));
    page.key(VK_ESCAPE, cx).await;
    println!("dialog: Escape: open {}, focused {:?}", page.dialog_open(cx), page.focused(cx));
    Ok(())
}

impl Page {
    fn hold(&self, cx: &mut AsyncApp, state: impl FnOnce(&mut RuntimeModel)) {
        cx.update(|cx| self.runtime.update(cx, |runtime, cx| runtime.hold_for_capture(cx, state)));
    }

    /// A key pressed and released, as the keyboard would send it.
    async fn key(&self, vk: usize, cx: &mut AsyncApp) {
        post(self.hwnd, WM_KEYDOWN, vk, 1);
        post(self.hwnd, WM_KEYUP, vk, KEY_UP);
        until_handled(self.hwnd, cx).await;
        pause(cx, KEY_GAP).await;
    }

    async fn click(&self, point: (i32, i32), cx: &mut AsyncApp) {
        post(self.hwnd, WM_MOUSEMOVE, 0, pack(point));
        post(self.hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(point));
        post(self.hwnd, WM_LBUTTONUP, 0, pack(point));
        until_handled(self.hwnd, cx).await;
        pause(cx, TICK).await;
    }

    /// A press on the page's background, so a shot shows no focus.
    async fn blur(&self, cx: &mut AsyncApp) {
        self.click(BACKGROUND_POINT, cx).await;
    }

    async fn tab_to(&self, key: &str, cx: &mut AsyncApp) -> Result<()> {
        let mut walked = Vec::new();
        for _ in 0..60 {
            self.key(VK_TAB, cx).await;
            let focused = self.focused(cx);
            if focused.as_deref() == Some(key) {
                return Ok(());
            }
            walked.push(focused.unwrap_or_else(|| "-".to_string()));
        }
        Err(format!("Tab never reached {key} in the Settings window; focus after each Tab: {}", walked.join(", ")))
    }

    /// The section's sidebar entry, reached with Tab and opened with Enter.
    async fn show(&self, section: Section, cx: &mut AsyncApp) -> Result<()> {
        self.tab_to(section.key(), cx).await?;
        self.key(VK_RETURN, cx).await;
        pause(cx, TICK).await;
        Ok(())
    }

    fn focused(&self, cx: &mut AsyncApp) -> Option<String> {
        self.view.update(cx, |view, window, _| view.focused_control(window)).ok().flatten()
    }

    fn select_open(&self, cx: &mut AsyncApp) -> bool {
        self.view.update(cx, |view, _, _| view.select_is_open()).unwrap_or(false)
    }

    fn dialog_open(&self, cx: &mut AsyncApp) -> bool {
        self.view.update(cx, |view, _, _| view.reset_dialog_is_open()).unwrap_or(false)
    }

    fn bounds_of(&self, key: &str, cx: &mut AsyncApp) -> Result<gpui::Bounds<gpui::Pixels>> {
        self.view
            .update(cx, |view, _, _| view.bounds_of(key))
            .ok()
            .flatten()
            .ok_or_else(|| format!("{key} is not drawn in the Settings window"))
    }
}

fn content_scale(cx: &mut AsyncApp) -> f32 {
    cx.update(|cx| settings::global(cx).read(cx).get().overlay.content_scale)
}

async fn shoot_meter(meter: WindowHandle<MeterView>, dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = meter
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    shoot_hwnd(hwnd, dir, name, cx).await
}

/// Off screen the window is a tool window that never activates, so the
/// capture shows nothing in the taskbar and takes no focus; on screen it is a
/// normal one. It is not an overlay, so a click answers GPUI's own
/// `MA_ACTIVATE` whatever its style.
fn report_window(page: &Page, cx: &mut AsyncApp) -> Result<()> {
    let style = overlay::ex_style(page.hwnd);
    let viewport = page.view.update(cx, |_, window, _| window.viewport_size()).ok();
    println!(
        "window Settings: topmost {}, app window {}, tool window {}, non-activating {}; GPUI viewport {viewport:?}; {}",
        style & WS_EX_TOPMOST.0 != 0,
        style & WS_EX_APPWINDOW.0 != 0,
        style & WS_EX_TOOLWINDOW.0 != 0,
        style & WS_EX_NOACTIVATE.0 != 0,
        off_screen::frame_report(page.hwnd)
    );
    capture::check_click_activation(page.hwnd, "a click on the Settings window", MA_ACTIVATE)
}

/// The title bar stays Windows' caption with its minimise and close buttons.
/// Crossing into them is a leave that clears GPUI's hover flag, which nothing
/// in the Settings window reads; hover styles follow GPUI's hit test, which
/// non-client moves keep current.
fn report_title_bar(page: &Page, cx: &mut AsyncApp) {
    let scale = page.view.update(cx, |_, window, _| window.scale_factor()).unwrap_or(1.0);
    let runs = capture::hit_test_runs(page.hwnd, scale, TITLE_BAR_Y);
    println!("hit-test Settings title bar y {TITLE_BAR_Y} (native caption kept): {}", capture::describe_runs(&runs));
}

fn running_capture() -> CaptureStatus {
    CaptureStatus {
        meter: MeterTile::Running { via: Some("Npcap") },
        game_found: true,
        game_address: Some("203.0.113.24".to_string()),
        character: Some(sample::YOU.to_string()),
        region: Some("Global"),
        servers: vec!["2011".to_string(), "2014".to_string()],
    }
}

fn recordings() -> Vec<RecordingItem> {
    [
        ("manual-2026-10-03-2142.aetherpc", 2, 2_516_582, RecordingOrigin::Manual),
        ("rolling-1790000000000.aetherpc", 5, 412_483_912, RecordingOrigin::Rolling),
        ("auto-2026-10-02-2031.aetherpc", 27, 3_250_585, RecordingOrigin::UnfamiliarServer),
        ("manual-2026-09-29-2210.aetherpc", 100, 1_887_436, RecordingOrigin::Manual),
    ]
    .into_iter()
    .map(|(name, hours_ago, bytes, origin)| RecordingItem {
        path: std::env::temp_dir().join(name),
        name: name.to_string(),
        bytes,
        when: hours_before_now(hours_ago),
        origin,
    })
    .collect()
}

fn bests() -> Vec<BestRow> {
    [
        ("Veilwarden Orsikh", "Mirelle", 51_740.0, 50),
        ("Ashmaw the Hollow", "Mirelle", 47_210.0, 74),
        ("Thane Ulmor", "Mirelle", 44_020.0, 400),
        ("Tidecaller Vennis", "Saebrin", 39_810.0, 3),
        ("Veilwarden Orsikh", "Saebrin", 36_120.0, 120),
        ("Sister Calyx", "Saebrin", 33_400.0, 450),
    ]
    .into_iter()
    .map(|(boss, character, dps, hours_ago)| BestRow {
        boss: boss.to_string(),
        character: character.to_string(),
        dps,
        achieved_at_ms: 0,
        when: hours_before_now(hours_ago),
    })
    .collect()
}

/// "Today 21:42", "Mon 22:10" or "21 Sep" in the language drawn.
fn hours_before_now(hours: u64) -> String {
    let now = u64::try_from(crate::clock::now_ms()).unwrap_or(0);
    crate::history::record_time(now.saturating_sub(hours * 3_600_000))
}

fn storage() -> StorageSummary {
    StorageSummary {
        settings_bytes: 6_144,
        history_records: 14,
        history_bytes: 2_202_009,
        recordings: 5,
        recordings_bytes: 12_373_196,
    }
}

fn check_report() -> Report {
    let check = |id, label, detail: &str, ok, weight| Check { id, label, detail: detail.to_string(), ok, weight, fix: None };
    let checks = vec![
        check(CheckId::Elevation, "Administrator rights", "Running elevated, which packet capture requires.", true, Weight::Required),
        check(CheckId::Npcap, "Npcap", "Installed and answering -- 3 network adapters visible.", true, Weight::Optional),
        check(
            CheckId::Windivert,
            "WinDivert",
            "Bundled as a fallback. Its driver is loaded only if Npcap cannot capture.",
            true,
            Weight::Optional,
        ),
    ];
    Report { ready: true, summary: "Ready to capture.".to_string(), checks }
}

/// Not elevated, Npcap missing and WinDivert refused: the core's own error
/// lines show under the translated ones.
fn failed_check_report() -> Report {
    let check = |id, label, detail: &str, weight| Check { id, label, detail: detail.to_string(), ok: false, weight, fix: None };
    let checks = vec![
        check(CheckId::Elevation, "Administrator rights", "The meter is not running as Administrator.", Weight::Required),
        check(CheckId::Npcap, "Npcap", "Npcap is not installed", Weight::Required),
        check(CheckId::Windivert, "WinDivert", "Access is denied (os error 5)", Weight::Optional),
    ];
    Report { ready: false, summary: "Administrator rights and Npcap have to be sorted out.".to_string(), checks }
}
