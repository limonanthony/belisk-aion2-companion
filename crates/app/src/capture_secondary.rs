//! Off-screen captures and checks of the history, player detail, PvP and
//! to-do windows, run by `--capture` after the meter's own. Every window is
//! placed outside all monitors (`Placement::OffScreen`) and nothing here asks
//! Windows for the foreground: the typing focus exception is checked on the
//! window style, and keys and clicks are posted to our own windows only. The
//! data is invented (`--fixtures`) or a recording's, with names masked.

use std::path::Path;
use std::time::{Duration, Instant};

use meter_core::events::MeterEvent;
use meter_core::models::combat::CombatSnapshot;
use gpui::{AsyncApp, Entity};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageW, HTCLIENT, MA_ACTIVATE, MA_NOACTIVATE, WM_CHAR, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::capture::{self, shoot_hwnd, MK_LBUTTON, ROW_POINT};
use crate::capture_reminders;
use crate::capture_settings;
use crate::capture_todos;
use crate::detail::{DetailModel, DetailSource, DetailView};
use crate::fixtures;
use crate::history::{HistoryModel, HistoryView};
use crate::meter::{MeterModel, MeterView, OverlaySettings};
use crate::overlay::{self, off_screen};
use crate::pvp::{PvpModel, PvpView, StatsSort, Tab};
use crate::secondary::{self, typing, WindowKind};

const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const TICK: Duration = Duration::from_millis(150);
/// The menu's first two entries at 100% scale, in the meter's client area.
const HISTORY_ITEM_POINT: (i32, i32) = (150, 47);
const PVP_ITEM_POINT: (i32, i32) = (150, 73);
/// On the title, left of anything else in the header, at 100% scale.
pub(crate) const TITLE_POINT: (i32, i32) = (60, 20);
/// Only a `WindowControlArea` answers anything but HTCLIENT, and none would
/// be shorter than a button, so every sixth logical row is enough.
const HIT_TEST_STEP: f32 = 6.0;

type Result<T, E = String> = std::result::Result<T, E>;

pub async fn from_fixtures(
    meter: gpui::WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    dir: &Path,
    cx: &mut AsyncApp,
) -> Result<()> {
    show_scene(model, fixtures::detail_fight(221.0, 624), cx);
    history_window(model, dir, cx).await?;
    detail_window(model, dir, cx).await?;
    pvp_window(model, dir, cx).await?;
    entry_points(meter, model, cx).await?;
    capture_todos::from_fixtures(meter, model, dir, cx).await?;
    capture_settings::from_fixtures(meter, model, dir, cx).await?;
    capture_reminders::from_fixtures(meter, model, dir, cx).await
}

/// After a replay: the detail window on the recording's local player, on both
/// boards, then the history window refreshing when the fight is saved.
pub async fn from_replay(model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let (actor_id, target_id) = cx.update(|cx| {
        let meter = model.read(cx);
        let snapshot = meter.snapshot();
        let local = snapshot.and_then(|snapshot| snapshot.combat_infos.main_actor_id);
        let top = snapshot.and_then(|snapshot| snapshot.session.players.first().map(|player| player.actor_id));
        (local.or(top), meter.board_target_id())
    });
    let Some(actor_id) = actor_id else {
        return Err("the recording ended with no player on the meter".to_string());
    };
    println!("replay: detail for actor {actor_id}, board target {target_id:?}");

    for (label, source) in [
        ("target", DetailSource::Target { actor_id, target_id }),
        ("session", DetailSource::Session { actor_id }),
    ] {
        cx.update(|cx| secondary::open_detail(model, source, cx));
        pause(cx, TICK).await;
        println!("replay detail ({label}): {}", describe_detail(cx)?);
        shoot_window(WindowKind::Detail, dir, &format!("replay-detail-{label}"), cx).await?;
    }
    close(WindowKind::Detail, cx).await?;

    cx.update(|cx| secondary::open_history(model, cx));
    pause(cx, TICK).await;
    let before = history_records(cx)?;
    let meter = cx.update(|cx| model.read(cx).meter().cloned());
    if let Some(meter) = meter {
        meter.reset_dps_meter(true);
    }
    let started = Instant::now();
    let mut after = before;
    while after == before && started.elapsed() < Duration::from_secs(5) {
        pause(cx, Duration::from_millis(50)).await;
        after = history_records(cx)?;
    }
    if after == before {
        println!("replay history: {before} records before and after the reset: no target in the recording took the 1,000,000 damage history keeps, so nothing was saved");
    } else {
        println!("replay history: {before} records before the reset, {after} after (the open window refreshed itself on the event)");
    }
    shoot_window(WindowKind::History, dir, "replay-history", cx).await?;
    close(WindowKind::History, cx).await
}

pub(crate) fn show_scene(model: &Entity<MeterModel>, snapshot: CombatSnapshot, cx: &mut AsyncApp) {
    cx.update(|cx| {
        model.update(cx, |model, cx| {
            *model = MeterModel::new(OverlaySettings::default(), None);
            model.use_memory_mobs();
            model.apply(MeterEvent::DpsSnapshot(snapshot), Instant::now());
            cx.notify();
        })
    });
}

async fn history_window(model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    cx.update(|cx| secondary::open_history(model, cx));
    cx.update(|cx| secondary::open_history(model, cx));
    pause(cx, TICK).await;
    let windows = cx.update(|cx| cx.windows().len());
    println!("single instance: History opened twice, {windows} windows now (meter + history = 2)");
    report_window(WindowKind::History, cx)?;
    shoot_window(WindowKind::History, dir, "history-01-empty", cx).await?;

    let records = fixtures::history_records();
    let ids: Vec<String> = records.iter().map(|record| record.id.clone()).collect();
    let history = history_model(cx)?;
    cx.update(|cx| {
        history.update(cx, |history, cx| {
            history.load(records);
            cx.notify();
        })
    });
    shoot_window(WindowKind::History, dir, "history-02-records", cx).await?;
    print_size(WindowKind::History, "records", cx)?;
    check_title(WindowKind::History, "records", cx).await?;

    let handle = window_handle::<HistoryView>(WindowKind::History, cx)?;
    cx.update(|cx| {
        history.update(cx, |history, cx| {
            history.list.filter_target(Some("Veilwarden Orsikh".to_string()));
            cx.notify();
        })
    });
    handle
        .update(cx, |view, _, cx| view.show_state(Some(true), None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    pause(cx, TICK).await;
    let shown = cx.update(|cx| history.read(cx).list.count_label());
    println!("history filter: target Veilwarden Orsikh shows {shown}, dropdown open");
    shoot_window(WindowKind::History, dir, "history-03-filter-open", cx).await?;
    print_size(WindowKind::History, "target dropdown open", cx)?;
    check_client_area(WindowKind::History, "target dropdown open", cx)?;
    let (hwnd, scale, _) = frame(WindowKind::History, cx)?;
    let drag = capture::post_drag(hwnd, scale, TITLE_POINT, cx).await?;
    let open = handle.update(cx, |view, _, _| view.menu_is_open()).map_err(|error| format!("history window gone: {error:#}"))?;
    println!(
        "history dropdown: pressed on the title and dragged {:?} with the dropdown open: dropdown open {open}, window moved {:?} (the scrim takes the press, as it did over the native caption)",
        drag.pointer, drag.moved
    );
    if open || drag.moved != (0, 0) {
        return Err("history dropdown: a press on the title with the dropdown open must only close it".to_string());
    }
    handle
        .update(cx, |view, _, cx| view.show_state(Some(true), None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    pause(cx, TICK).await;

    cx.update(|cx| {
        history.update(cx, |history, cx| {
            history.list.filter_target(None);
            history.list.filter_player(None);
            cx.notify();
        })
    });
    handle
        .update(cx, |view, _, cx| view.show_state(Some(false), None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    pause(cx, TICK).await;
    shoot_window(WindowKind::History, dir, "history-04-player-filter-open", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_state(None, None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;

    let newest = ids[0].clone();
    cx.update(|cx| history.update(cx, |history, cx| history.toggle_open(&newest, cx)));
    shoot_window(WindowKind::History, dir, "history-05-record-open", cx).await?;
    cx.update(|cx| history.update(cx, |history, cx| history.toggle_open(&newest, cx)));

    let third = cx.update(|cx| history.read(cx).list.visible()[2].id.clone());
    handle
        .update(cx, |view, _, cx| view.show_state(None, Some(third), false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    shoot_window(WindowKind::History, dir, "history-06-delete-one", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_state(None, None, true, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    shoot_window(WindowKind::History, dir, "history-07-delete-all", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_state(None, None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;

    let record = cx.update(|cx| history.read(cx).list.record(&newest).cloned());
    let record = record.ok_or("the newest record is missing")?;
    cx.update(|cx| secondary::open_detail(model, DetailSource::History { actor_id: 2, record }, cx));
    pause(cx, TICK).await;
    report_window(WindowKind::Detail, cx)?;
    shoot_window(WindowKind::Detail, dir, "detail-05-history", cx).await?;
    close(WindowKind::Detail, cx).await?;

    cx.update(|cx| history.update(cx, |history, cx| history.delete(&newest, cx)));
    let remaining = cx.update(|cx| history.read(cx).list.total());
    println!("history delete: without a meter nothing is deleted, {remaining} records still listed");

    // Nothing matches both filters, so the card is short and the target
    // dropdown hangs below it.
    cx.update(|cx| {
        history.update(cx, |history, cx| {
            history.list.filter_target(Some("Thornback Boar".to_string()));
            history.list.filter_player(Some("Vaelith".to_string()));
            cx.notify();
        })
    });
    pause(cx, TICK).await;
    print_size(WindowKind::History, "no record matches", cx)?;
    shoot_window(WindowKind::History, dir, "history-09-no-match", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_state(Some(true), None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    pause(cx, TICK).await;
    print_size(WindowKind::History, "no record matches, dropdown open", cx)?;
    shoot_window(WindowKind::History, dir, "history-10-dropdown-grows-window", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_state(None, None, false, cx))
        .map_err(|error| format!("history window gone: {error:#}"))?;
    pause(cx, TICK).await;
    print_size(WindowKind::History, "no record matches, dropdown closed", cx)?;
    cx.update(|cx| history.update(cx, |history, cx| history.show_deleted_all(cx)));
    shoot_window(WindowKind::History, dir, "history-08-after-delete-all", cx).await?;
    close(WindowKind::History, cx).await
}

async fn detail_window(model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    cx.update(|cx| secondary::open_detail(model, DetailSource::Target { actor_id: 999, target_id: None }, cx));
    pause(cx, TICK).await;
    report_window(WindowKind::Detail, cx)?;
    println!("detail (nobody): {}", describe_detail(cx)?);
    shoot_window(WindowKind::Detail, dir, "detail-01-empty", cx).await?;

    cx.update(|cx| secondary::open_detail(model, DetailSource::Target { actor_id: 2, target_id: None }, cx));
    pause(cx, TICK).await;
    println!("detail (target board): {}", describe_detail(cx)?);
    shoot_window(WindowKind::Detail, dir, "detail-02-live", cx).await?;
    check_title(WindowKind::Detail, "live", cx).await?;

    let handle = window_handle::<DetailView>(WindowKind::Detail, cx)?;
    handle
        .update(cx, |view, _, cx| view.show_open(Some(11_020_150), false, cx))
        .map_err(|error| format!("detail window gone: {error:#}"))?;
    shoot_window(WindowKind::Detail, dir, "detail-03-skill-open", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_open(None, true, cx))
        .map_err(|error| format!("detail window gone: {error:#}"))?;
    shoot_window(WindowKind::Detail, dir, "detail-04-buffs-open", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_open(None, false, cx))
        .map_err(|error| format!("detail window gone: {error:#}"))?;

    cx.update(|cx| secondary::open_detail(model, DetailSource::Target { actor_id: 8, target_id: None }, cx));
    pause(cx, TICK).await;
    println!("detail (healer, no hits): {}", describe_detail(cx)?);
    shoot_window(WindowKind::Detail, dir, "detail-06-no-skill-data", cx).await?;

    cx.update(|cx| secondary::open_detail(model, DetailSource::Session { actor_id: 2 }, cx));
    pause(cx, TICK).await;
    let windows = cx.update(|cx| cx.windows().len());
    println!("single instance: Detail opened again retargets it, {windows} windows now (meter + detail = 2)");
    println!("detail (session board): {}", describe_detail(cx)?);
    shoot_window(WindowKind::Detail, dir, "detail-07-session", cx).await?;
    handle
        .update(cx, |view, _, cx| view.show_open(None, true, cx))
        .map_err(|error| format!("detail window gone: {error:#}"))?;
    shoot_window(WindowKind::Detail, dir, "detail-08-session-buffs-open", cx).await?;
    close(WindowKind::Detail, cx).await
}

async fn pvp_window(model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    show_scene(model, fixtures::open_world(), cx);
    cx.update(|cx| secondary::open_pvp(model, cx));
    pause(cx, TICK).await;
    report_window(WindowKind::Pvp, cx)?;
    shoot_window(WindowKind::Pvp, dir, "pvp-01-just-opened", cx).await?;

    show_scene(model, fixtures::pvp_fight(), cx);
    let pvp = pvp_model(cx)?;
    let (names, response, stats) = fixtures::pvp_watch();
    cx.update(|cx| pvp.update(cx, |pvp, cx| pvp.show_fixture(&names, response.clone(), stats.clone(), cx)));
    shoot_window(WindowKind::Pvp, dir, "pvp-02-kills-by-damage", cx).await?;
    check_title(WindowKind::Pvp, "kills", cx).await?;
    cx.update(|cx| pvp.update(cx, |pvp, cx| pvp.sort_by(StatsSort::Kills, cx)));
    shoot_window(WindowKind::Pvp, dir, "pvp-03-kills-by-kills", cx).await?;
    cx.update(|cx| pvp.update(cx, |pvp, cx| pvp.sort_by(StatsSort::Damage, cx)));

    let handle = window_handle::<PvpView>(WindowKind::Pvp, cx)?;
    let show_tab = |tab: Tab, cx: &mut AsyncApp| {
        handle.update(cx, |view, _, cx| view.show_tab(tab, cx)).map_err(|error| format!("pvp window gone: {error:#}"))
    };
    show_tab(Tab::Damage, cx)?;
    shoot_window(WindowKind::Pvp, dir, "pvp-04-damage", cx).await?;
    show_tab(Tab::Kills, cx)?;

    let hwnd = window_hwnd(WindowKind::Pvp, cx)?;
    handle
        .update(cx, |view, window, cx| view.start_editing_without_activation(true, window, cx))
        .map_err(|error| format!("pvp window gone: {error:#}"))?;
    pause(cx, TICK).await;
    for letter in "Ka".chars() {
        post(hwnd, WM_CHAR, letter as usize, 0);
    }
    pause(cx, TICK).await;
    shoot_window(WindowKind::Pvp, dir, "pvp-05-adding", cx).await?;
    print_size(WindowKind::Pvp, "suggestions open", cx)?;

    post(hwnd, WM_KEYDOWN, VK_RETURN, 0);
    pause(cx, TICK).await;
    let (editing, watching) = (
        handle.update(cx, |view, _, _| view.is_editing()).unwrap_or(true),
        cx.update(|cx| pvp.read(cx).watch().iter().map(|row| row.query_name.clone()).collect::<Vec<_>>()),
    );
    println!("typing: \"Ka\" typed and Enter pressed: still editing {editing}, watch rows {watching:?}");
    shoot_window(WindowKind::Pvp, dir, "pvp-06-added", cx).await?;
    print_size(WindowKind::Pvp, "suggestions closed", cx)?;

    handle
        .update(cx, |view, window, cx| view.start_editing_without_activation(true, window, cx))
        .map_err(|error| format!("pvp window gone: {error:#}"))?;
    post(hwnd, WM_CHAR, 'x' as usize, 0);
    post(hwnd, WM_KEYDOWN, VK_ESCAPE, 0);
    pause(cx, TICK).await;
    let editing = handle.update(cx, |view, _, _| view.is_editing()).unwrap_or(true);
    println!("typing: Escape cancels: still editing {editing}");

    handle
        .update(cx, |view, window, cx| view.start_editing_without_activation(true, window, cx))
        .map_err(|error| format!("pvp window gone: {error:#}"))?;
    pause(cx, TICK).await;
    let (_, scale, _) = frame(WindowKind::Pvp, cx)?;
    capture::check_drag(hwnd, scale, TITLE_POINT, "the PvP title while editing", cx).await?;
    let editing = handle.update(cx, |view, _, _| view.is_editing()).unwrap_or(false);
    println!("pvp edit: after a drag on the title: still editing {editing}");
    if !editing {
        return Err("pvp edit: a drag on the title ended the edit".to_string());
    }
    post(hwnd, WM_KEYDOWN, VK_ESCAPE, 0);
    pause(cx, TICK).await;

    handle
        .update(cx, |view, window, cx| view.start_editing_without_activation(false, window, cx))
        .map_err(|error| format!("pvp window gone: {error:#}"))?;
    pause(cx, TICK * 5).await;
    let editing = handle.update(cx, |view, _, _| view.is_editing()).unwrap_or(true);
    println!("typing: the pointer is not over the window: still editing {editing} after {} ms", (TICK * 5).as_millis());

    check_typing_style(hwnd)?;

    cx.update(|cx| pvp.update(cx, |pvp, cx| pvp.show_fixture(&names, response, fixtures::pvp_stats_long(), cx)));
    shoot_window(WindowKind::Pvp, dir, "pvp-07-long-list", cx).await?;

    cx.update(|cx| pvp.update(cx, |pvp, cx| pvp.clear_stats(cx)));
    let stats = cx.update(|cx| pvp.read(cx).stats().len());
    println!("pvp clear: {stats} kill stat rows left");
    shoot_window(WindowKind::Pvp, dir, "pvp-08-cleared", cx).await?;
    close(WindowKind::Pvp, cx).await
}

/// The window's client size, to show it grows while a popover is open.
fn print_size(kind: WindowKind, label: &str, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(kind, cx)?;
    println!("window {kind:?} ({label}): {}", off_screen::frame_report(hwnd));
    Ok(())
}

fn frame(kind: WindowKind, cx: &mut AsyncApp) -> Result<(HWND, f32, f32)> {
    let handle = cx.update(|cx| secondary::window_of(kind, cx)).ok_or_else(|| format!("no {kind:?} window"))?;
    handle
        .update(cx, |_, window, _| {
            overlay::hwnd(window).map(|hwnd| (hwnd, window.scale_factor(), window.viewport_size().height.as_f32()))
        })
        .map_err(|error| format!("{kind:?} window gone: {error:#}"))?
}

/// Windows reports the pointer crossing between client and non-client area
/// as a leave, and GPUI clears its hover flag on it, so no part of the window
/// may be non-client.
fn check_client_area(kind: WindowKind, state: &str, cx: &mut AsyncApp) -> Result<()> {
    let (hwnd, scale, height) = frame(kind, cx)?;
    let title = capture::hit_test_runs(hwnd, scale, TITLE_POINT.1 as f32);
    println!("hit-test {kind:?} {state}, title row y {}: {}", TITLE_POINT.1, capture::describe_runs(&title));
    let mut rows = 0;
    let mut not_client = Vec::new();
    let mut y = 0.0;
    while y < height {
        let runs = capture::hit_test_runs(hwnd, scale, y);
        if runs.iter().any(|(code, _, _)| *code as u32 != HTCLIENT) {
            not_client.push(format!("y {y}: {}", capture::describe_runs(&runs)));
        }
        rows += 1;
        y += HIT_TEST_STEP;
    }
    println!("hit-test {kind:?} {state}: {rows} rows every {HIT_TEST_STEP} px down to {height:.0}, {} not all HTCLIENT", not_client.len());
    if title.iter().any(|(code, _, _)| *code as u32 != HTCLIENT) || !not_client.is_empty() {
        return Err(format!("hit-test {kind:?} {state}: not all client area: {}", not_client.join("; ")));
    }
    Ok(())
}

/// The whole window is client area, and a drag on the title moves it by the
/// pointer's delta while a later move without the button does not.
pub(crate) async fn check_title(kind: WindowKind, state: &str, cx: &mut AsyncApp) -> Result<()> {
    check_client_area(kind, state, cx)?;
    let (hwnd, scale, _) = frame(kind, cx)?;
    capture::check_drag(hwnd, scale, TITLE_POINT, &format!("the {kind:?} title"), cx).await
}

/// The window turns focusable while a field is edited and non-activating
/// again after, and a click activates it only in between. The style is all
/// that is exercised: asking for the foreground would take focus from
/// whatever the user is doing.
fn check_typing_style(hwnd: HWND) -> Result<()> {
    let before = typing::is_activatable(hwnd);
    typing::set_activatable(hwnd, true);
    let during = typing::is_activatable(hwnd);
    let editing = capture::check_click_activation(hwnd, "a click on the PvP window while editing", MA_ACTIVATE);
    typing::end(hwnd);
    let after = typing::is_activatable(hwnd);
    println!("typing focus: activatable before {before}, while editing {during}, after {after} (expected false, true, false)");
    editing?;
    capture::check_click_activation(hwnd, "a click on the PvP window after the edit", MA_NOACTIVATE)
}

/// The real entry points, with mouse messages posted to the meter's own
/// window: the "…" menu's two links, and a row.
async fn entry_points(meter: gpui::WindowHandle<MeterView>, model: &Entity<MeterModel>, cx: &mut AsyncApp) -> Result<()> {
    show_scene(model, fixtures::open_world(), cx);
    let hwnd = meter
        .update(cx, |view, window, cx| {
            view.forced_pointer = None;
            cx.notify();
            overlay::hwnd(window)
        })
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    let click = |point: (i32, i32)| {
        post(hwnd, WM_MOUSEMOVE, 0, pack(point));
        post(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(point));
        post(hwnd, WM_LBUTTONUP, 0, pack(point));
    };
    let open_menu = |cx: &mut AsyncApp| meter.update(cx, |view, _, cx| view.force_hover(None, true, false, cx));

    for (label, point, kind) in [
        ("Fight history", HISTORY_ITEM_POINT, WindowKind::History),
        ("PvP meter", PVP_ITEM_POINT, WindowKind::Pvp),
    ] {
        open_menu(cx).map_err(|error| format!("overlay window gone: {error:#}"))?;
        pause(cx, TICK).await;
        click(point);
        pause(cx, TICK * 2).await;
        let open = cx.update(|cx| secondary::window_of(kind, cx).is_some());
        println!("entry point: click on \"{label}\" in the menu: {kind:?} window open {open}");
        close(kind, cx).await?;
    }

    click(ROW_POINT);
    pause(cx, TICK * 2).await;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Detail, cx).is_some());
    println!("entry point: click on a Session row: Detail window open {open}: {}", describe_detail(cx).unwrap_or_default());
    close(WindowKind::Detail, cx).await
}

fn describe_detail(cx: &mut AsyncApp) -> Result<String> {
    let model = detail_model(cx)?;
    Ok(cx.update(|cx| match model.read(cx).data() {
        None => "no data".to_string(),
        Some(data) => {
            let summary = &data.summary;
            let top: Vec<String> = data
                .skills
                .iter()
                .take(3)
                .map(|skill| format!("{} {} ({:.0}%)", skill.name, skill.total, skill.share * 100.0))
                .collect();
            format!(
                "{:?} {} class {:?} server {:?} power {:?}; damage {} dps {:.0} fight {:.0}s hits {} crit {:?}; {} skills, top {top:?}; buffs {} on the player, {} on the target",
                data.mode,
                data.identity.name,
                data.identity.class,
                data.identity.server,
                data.identity.combat_power,
                summary.damage,
                summary.dps,
                summary.fight_secs,
                summary.hits,
                summary.critical.map(|crit| format!("{:.1}%", crit * 100.0)),
                data.skills.len(),
                data.player_buffs.len(),
                data.target_buffs.len(),
            )
        }
    }))
}

fn history_records(cx: &mut AsyncApp) -> Result<usize> {
    let history = history_model(cx)?;
    Ok(cx.update(|cx| history.read(cx).list.total()))
}

fn history_model(cx: &mut AsyncApp) -> Result<Entity<HistoryModel>> {
    cx.update(|cx| secondary::model_of::<HistoryModel>(WindowKind::History, cx)).ok_or_else(|| "no history window".to_string())
}

fn detail_model(cx: &mut AsyncApp) -> Result<Entity<DetailModel>> {
    cx.update(|cx| secondary::model_of::<DetailModel>(WindowKind::Detail, cx)).ok_or_else(|| "no detail window".to_string())
}

fn pvp_model(cx: &mut AsyncApp) -> Result<Entity<PvpModel>> {
    cx.update(|cx| secondary::model_of::<PvpModel>(WindowKind::Pvp, cx)).ok_or_else(|| "no PvP window".to_string())
}

pub(crate) fn window_handle<V: gpui::Render + 'static>(kind: WindowKind, cx: &mut AsyncApp) -> Result<gpui::WindowHandle<V>> {
    cx.update(|cx| secondary::window_of(kind, cx))
        .and_then(|handle| handle.downcast::<V>())
        .ok_or_else(|| format!("no {kind:?} window of the expected kind"))
}

pub(crate) fn window_hwnd(kind: WindowKind, cx: &mut AsyncApp) -> Result<HWND> {
    let handle = cx.update(|cx| secondary::window_of(kind, cx)).ok_or_else(|| format!("no {kind:?} window"))?;
    handle
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("{kind:?} window gone: {error:#}"))?
}

pub(crate) async fn shoot_window(kind: WindowKind, dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(kind, cx)?;
    shoot_hwnd(hwnd, dir, name, cx).await
}

/// The window's style as the capture sees it: topmost, tool window, never
/// activated, frameless.
pub(crate) fn report_window(kind: WindowKind, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(kind, cx)?;
    let style = overlay::ex_style(hwnd);
    println!(
        "window {kind:?}: topmost {}, tool window {}, non-activating {}; {}",
        style & WS_EX_TOPMOST.0 != 0,
        style & WS_EX_TOOLWINDOW.0 != 0,
        style & WS_EX_NOACTIVATE.0 != 0,
        off_screen::frame_report(hwnd)
    );
    capture::check_click_activation(hwnd, &format!("a click on the {kind:?} window"), MA_NOACTIVATE)
}

/// Closes the window as its × would and checks the registry let go of it.
pub(crate) async fn close(kind: WindowKind, cx: &mut AsyncApp) -> Result<()> {
    let handle = cx.update(|cx| secondary::window_of(kind, cx)).ok_or_else(|| format!("no {kind:?} window to close"))?;
    handle
        .update(cx, |_, window, _| window.remove_window())
        .map_err(|error| format!("{kind:?} window gone: {error:#}"))?;
    pause(cx, TICK).await;
    let still_open = cx.update(|cx| secondary::window_of(kind, cx).is_some());
    if still_open {
        return Err(format!("the {kind:?} window is still registered after closing"));
    }
    Ok(())
}

pub(crate) async fn pause(cx: &mut AsyncApp, duration: Duration) {
    cx.background_executor().timer(duration).await;
}

pub(crate) fn pack((x, y): (i32, i32)) -> isize {
    (((y as u16) as isize) << 16) | (x as u16) as isize
}

pub(crate) fn post(hwnd: HWND, message: u32, wparam: usize, lparam: isize) {
    unsafe {
        let _ = PostMessageW(Some(hwnd), message, WPARAM(wparam), LPARAM(lparam));
    }
}
