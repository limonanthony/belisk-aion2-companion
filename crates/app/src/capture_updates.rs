//! Off-screen captures of the update UI (docs/gpui-mockups/approved/
//! toolbar.html update states, settings.html About › Updates states), run by
//! `--capture --fixtures` after What's new. The updates sit in invented
//! states (`update::install_fixture`): nothing is contacted. The toolbar's
//! icon is drawn in each state, then a click on it while downloading opens
//! Settings › About, whose Updates group is drawn in each state. Every window
//! stays past every monitor; the only input is messages posted to our own
//! windows, and nothing asks Windows for the foreground.

use std::path::Path;
use std::time::{Duration, SystemTime};

use gpui::AsyncApp;
use windows::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_KEYUP};

use crate::capture_secondary::{close, pause, post, shoot_window, until_handled, window_handle, TICK};
use crate::capture_todos::set_reset_ahead;
use crate::capture_toolbar::{click, login, shot, tickets, toolbar_window, view};
use crate::changelog::{Date, Version};
use crate::secondary::{self, WindowKind};
use crate::settings::{self, Section, SettingsView};
use crate::toolbar::{self, Slot};
use crate::update::{self, Failure, ReleaseInfo, Snapshot, UpdateState};

type Result<T, E = String> = std::result::Result<T, E>;

const NEXT: Version = Version::new(1, 6, 0);
const SIZE: u64 = 4_200_000;
const VK_LEFT: usize = 0x25;
const VK_RIGHT: usize = 0x27;
const KEY_UP: isize = 0xC000_0001;

fn release() -> ReleaseInfo {
    ReleaseInfo {
        version: NEXT,
        size: SIZE,
        notes_url: "https://github.com/owner/repo/releases/tag/v1.6.0".to_string(),
        published: Some(Date { year: 2026, month: 10, day: 21 }),
    }
}

fn available(last_failure: Option<Failure>) -> UpdateState {
    UpdateState::Available { release: release(), last_failure }
}

fn downloading(paused_for_fight: bool) -> UpdateState {
    UpdateState::Downloading { version: NEXT, downloaded: SIZE * 62 / 100, total: SIZE, paused_for_fight }
}

/// The updates in `state`, checked ten minutes ago; Check now open again
/// after `check_again_in`.
fn show(state: UpdateState, check_again_in: Duration, cx: &mut AsyncApp) {
    let now = SystemTime::now();
    let snapshot = Snapshot { state, last_checked: Some(now - Duration::from_secs(600)), check_again_at: now + check_again_in };
    cx.update(|cx| {
        let store = settings::global(cx);
        update::install_fixture(&store, snapshot, cx);
    });
}

fn set_download_automatically(on: bool, cx: &mut AsyncApp) {
    cx.update(|cx| {
        settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.updates.download_automatically = on))
    });
}

pub async fn from_fixtures(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    cx.update(toolbar::close_meter);
    set_reset_ahead(5, cx);
    login(&tickets(&[]), cx);
    toolbar_states(dir, cx).await?;
    about_states(dir, cx).await?;
    show(UpdateState::Disabled, Duration::ZERO, cx);
    Ok(())
}

async fn toolbar_states(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let tip = |slot: Option<Slot>, cx: &mut AsyncApp| view(cx, |view, cx| view.force_tip(slot, cx));

    show(UpdateState::UpToDate { checked_at: SystemTime::now() }, Duration::ZERO, cx);
    tip(None, cx)?;
    shot(dir, "updates-00-up-to-date-no-icon", cx).await?;

    let states: [(&str, UpdateState, bool); 10] = [
        ("updates-01-available", available(None), false),
        ("updates-02-available-tip", available(None), true),
        ("updates-03-downloading", downloading(false), false),
        ("updates-04-downloading-tip", downloading(false), true),
        ("updates-05-paused-tip", downloading(true), true),
        ("updates-06-verifying-tip", UpdateState::Verifying { version: NEXT }, true),
        ("updates-07-ready", UpdateState::Ready { version: NEXT }, false),
        ("updates-08-ready-tip", UpdateState::Ready { version: NEXT }, true),
        ("updates-10-failed", available(Some(Failure::SignatureMismatch)), false),
        ("updates-11-failed-tip", available(Some(Failure::SignatureMismatch)), true),
    ];
    for (name, state, with_tip) in states {
        show(state, Duration::ZERO, cx);
        tip(with_tip.then_some(Slot::Update), cx)?;
        shot(dir, name, cx).await?;
    }

    show(UpdateState::Ready { version: NEXT }, Duration::ZERO, cx);
    view(cx, |view, cx| view.force_fight(Some(true), cx))?;
    tip(Some(Slot::Update), cx)?;
    shot(dir, "updates-09-ready-fight-tip", cx).await?;
    view(cx, |view, cx| view.force_fight(None, cx))?;

    set_reset_ahead(1, cx);
    login(&tickets(&[(8, 7, 0), (4, 14, 0)]), cx);
    show(available(None), Duration::ZERO, cx);
    tip(None, cx)?;
    shot(dir, "updates-12-beside-the-warning", cx).await?;

    login(&tickets(&[(1, 21, 3), (8, 7, 0)]), cx);
    view(cx, |view, cx| view.set_folded(true, cx))?;
    shot(dir, "updates-13-folded", cx).await?;
    show(downloading(false), Duration::ZERO, cx);
    shot(dir, "updates-14-folded-downloading", cx).await?;
    view(cx, |view, cx| view.set_folded(false, cx))?;

    set_reset_ahead(5, cx);
    login(&tickets(&[]), cx);
    show(available(None), Duration::ZERO, cx);
    keyboard_focus(dir, cx).await
}

/// The keyboard given to the toolbar on the icon (Right then Left, as keys),
/// so the focus ring shows.
async fn keyboard_focus(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let (window, hwnd) = toolbar_window(cx)?;
    window
        .update(cx, |view, window, cx| view.take_keyboard(Slot::Update, window, cx))
        .map_err(|error| format!("the toolbar window is gone: {error:#}"))?;
    for vk in [VK_RIGHT, VK_LEFT] {
        post(hwnd, WM_KEYDOWN, vk, 1);
        post(hwnd, WM_KEYUP, vk, KEY_UP);
        until_handled(hwnd, cx).await;
        pause(cx, TICK).await;
    }
    shot(dir, "updates-15-keyboard-focus", cx).await?;
    window
        .update(cx, |_, window, cx| window.blur(cx))
        .map_err(|error| format!("the toolbar window is gone: {error:#}"))?;
    Ok(())
}

/// The real entry point first: a click on the icon while downloading opens
/// Settings on About.
async fn about_states(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    show(downloading(false), Duration::ZERO, cx);
    view(cx, |view, cx| view.force_tip(None, cx))?;
    click(Slot::Update, cx).await?;
    pause(cx, TICK * 3).await;
    let settings_view = window_handle::<SettingsView>(WindowKind::Settings, cx)?;
    let section = settings_view.update(cx, |view, _, _| view.section()).map_err(|error| format!("the Settings window is gone: {error:#}"))?;
    println!("entry point: a click on the update icon while downloading: Settings open on {section:?}");
    if section != Section::About {
        return Err(format!("the update icon opened Settings on {section:?}, not About"));
    }

    let minute = Duration::from_secs(60);
    let states: [(&str, UpdateState, Duration, bool); 11] = [
        ("about-01-up-to-date", UpdateState::UpToDate { checked_at: SystemTime::now() - Duration::from_secs(600) }, Duration::ZERO, true),
        ("about-02-checking", UpdateState::Checking, minute, true),
        ("about-03-just-checked", UpdateState::UpToDate { checked_at: SystemTime::now() }, minute, true),
        ("about-04-available-auto-off", available(None), Duration::ZERO, false),
        ("about-05-downloading", downloading(false), Duration::ZERO, true),
        ("about-06-paused", downloading(true), Duration::ZERO, true),
        ("about-07-verifying", UpdateState::Verifying { version: NEXT }, Duration::ZERO, true),
        ("about-08-ready", UpdateState::Ready { version: NEXT }, Duration::ZERO, true),
        ("about-09-failed-its-check", available(Some(Failure::SignatureMismatch)), Duration::ZERO, true),
        ("about-10-unreachable", UpdateState::Unreachable { next_try: SystemTime::now() + Duration::from_secs(3600) }, Duration::ZERO, true),
        ("about-11-rate-limited", UpdateState::RateLimited { until: SystemTime::now() + Duration::from_secs(720) }, Duration::from_secs(720), true),
    ];
    for (name, state, check_again_in, automatic) in states {
        set_download_automatically(automatic, cx);
        show(state, check_again_in, cx);
        pause(cx, TICK).await;
        shoot_window(WindowKind::Settings, dir, name, cx).await?;
    }
    set_download_automatically(true, cx);
    show(UpdateState::Disabled, Duration::ZERO, cx);
    pause(cx, TICK).await;
    shoot_window(WindowKind::Settings, dir, "about-12-updates-off", cx).await?;

    close(WindowKind::Settings, cx).await?;
    let still_open = cx.update(|cx| secondary::is_open(WindowKind::Settings, cx));
    println!("updates: Settings closed {}", !still_open);
    Ok(())
}
