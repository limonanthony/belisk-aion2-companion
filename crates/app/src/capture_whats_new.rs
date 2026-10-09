//! Off-screen captures and checks of What's new (docs/gpui-mockups/approved/
//! whatsnew.html), run by `--capture --fixtures` after the toolbar's. The
//! window opens as the launch opens it, under the toolbar with the meter
//! closed, on the approved mockup's invented versions as a 1.6.0 build: one
//! version, skipped versions, the same scrolled to the end; then "Full
//! changelog". Every window stays past every monitor; the only input is
//! mouse messages posted to the window itself, and nothing asks Windows for
//! the foreground.

use std::path::Path;

use gpui::{AsyncApp, Entity};
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, MA_NOACTIVATE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE};

use crate::capture::{self, MK_LBUTTON, SETTLE};
use crate::capture_reminders::{self, shoot_together};
use crate::capture_secondary::{close, pack, pause, post, window_handle, window_hwnd, TICK};
use crate::changelog::{Version, WhatsNew};
use crate::fixtures;
use crate::i18n;
use crate::meter::MeterModel;
use crate::secondary::{self, WindowKind};
use crate::settings::{self, SettingsView};
use crate::toolbar;
use crate::whats_new::WhatsNewView;

type Result<T, E = String> = std::result::Result<T, E>;

/// From the card's bottom-right corner to the chevron at the end of "Full
/// changelog", at 100%: the card's 8 px padding, the footer's 2 px, the
/// button's 10 px and half its 12 px icon; up through the 26 px footer's
/// middle.
const FULL_CHANGELOG_FROM_CORNER: (f32, f32) = (8. + 2. + 10. + 6., 8. + 13.);

const RUNNING: Version = Version::new(1, 6, 0);

pub async fn from_fixtures(model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let foreground_before = capture_reminders::foreground();
    cx.update(toolbar::close_meter);
    let changelog = fixtures::changelog(i18n::language());
    let states = [
        ("whatsnew-01-one-version", Version::new(1, 5, 1), false),
        ("whatsnew-02-skipped-versions", Version::new(1, 4, 0), false),
        ("whatsnew-03-scrolled", Version::new(1, 4, 0), true),
    ];
    for (name, since, scrolled) in states {
        let whats_new = WhatsNew::select(&changelog, Some(since), RUNNING).ok_or("the invented changelog has nothing new")?;
        open(model, whats_new, cx).await?;
        if scrolled {
            window_handle::<WhatsNewView>(WindowKind::WhatsNew, cx)?
                .update(cx, |view, _, cx| view.scroll_to_end(cx))
                .map_err(|error| format!("the What's new window is gone: {error:#}"))?;
        }
        check_under_toolbar(name, cx)?;
        capture_reminders::check_never_focused(WindowKind::WhatsNew, name, foreground_before, cx)?;
        let toolbar_hwnd = toolbar_hwnd(cx)?;
        let window = window_hwnd(WindowKind::WhatsNew, cx)?;
        shoot_together(&[toolbar_hwnd, window], dir, name, cx).await?;
        capture::check_click_activation(window, name, MA_NOACTIVATE)?;
        close(WindowKind::WhatsNew, cx).await?;
    }
    full_changelog(model, &changelog, cx).await?;
    cx.update(toolbar::open_meter);
    Ok(())
}

async fn open(model: &Entity<MeterModel>, whats_new: WhatsNew, cx: &mut AsyncApp) -> Result<()> {
    cx.update(|cx| secondary::open_whats_new(model, whats_new, cx));
    pause(cx, SETTLE).await;
    if !cx.update(|cx| secondary::is_open(WindowKind::WhatsNew, cx)) {
        return Err("the What's new window did not open".to_string());
    }
    Ok(())
}

fn toolbar_hwnd(cx: &mut AsyncApp) -> Result<windows::Win32::Foundation::HWND> {
    cx.update(|cx| toolbar::toolbar_hwnd(cx)).ok_or_else(|| "the toolbar is not open".to_string())
}

fn window_rect(hwnd: windows::Win32::Foundation::HWND) -> Result<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|error| format!("GetWindowRect failed: {error}"))?;
    Ok(rect)
}

/// 6 px under the toolbar at 100%, left edges aligned.
fn check_under_toolbar(label: &str, cx: &mut AsyncApp) -> Result<()> {
    let (bar, card) = (window_rect(toolbar_hwnd(cx)?)?, window_rect(window_hwnd(WindowKind::WhatsNew, cx)?)?);
    println!("{label}: left edges {} and {}, gap {} px, {} px tall", bar.left, card.left, card.top - bar.bottom, card.bottom - card.top);
    if card.left != bar.left || card.top <= bar.bottom {
        return Err(format!("{label}: What's new is not under the toolbar"));
    }
    Ok(())
}

/// A click on "Full changelog" opens Settings on About › Changelog and closes
/// What's new.
async fn full_changelog(model: &Entity<MeterModel>, changelog: &crate::changelog::Changelog, cx: &mut AsyncApp) -> Result<()> {
    let whats_new = WhatsNew::select(changelog, Some(Version::new(1, 5, 1)), RUNNING).ok_or("nothing new")?;
    open(model, whats_new, cx).await?;
    let hwnd = window_hwnd(WindowKind::WhatsNew, cx)?;
    let dpi = window_handle::<WhatsNewView>(WindowKind::WhatsNew, cx)?
        .update(cx, |_, window, _| window.scale_factor())
        .map_err(|error| format!("the What's new window is gone: {error:#}"))?;
    let scale = dpi * cx.update(|cx| settings::global(cx).read(cx).get().overlay.scale());
    let rect = window_rect(hwnd)?;
    let (from_right, from_bottom) = FULL_CHANGELOG_FROM_CORNER;
    let at = (
        rect.right - rect.left - (from_right * scale).round() as i32,
        rect.bottom - rect.top - (from_bottom * scale).round() as i32,
    );
    post(hwnd, WM_MOUSEMOVE, 0, pack(at));
    post(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(at));
    post(hwnd, WM_LBUTTONUP, 0, pack(at));
    pause(cx, TICK * 4).await;

    let still_open = cx.update(|cx| secondary::is_open(WindowKind::WhatsNew, cx));
    let page = window_handle::<SettingsView>(WindowKind::Settings, cx)?
        .update(cx, |view, _, _| view.changelog_state())
        .map_err(|error| format!("the Settings window is gone: {error:#}"))?;
    println!("a click on Full changelog: What's new open {still_open}, Settings on the Changelog page {page:?}");
    if still_open || page.is_none() {
        return Err("Full changelog did not open About › Changelog and close What's new".to_string());
    }
    close(WindowKind::Settings, cx).await
}
