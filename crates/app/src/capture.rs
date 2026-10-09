//! `--capture <dir>`: renders the overlay outside every monitor and saves
//! what it draws with `PrintWindow(PW_RENDERFULLCONTENT)`, so the design can
//! be checked while someone is playing on the same machine. Nothing is ever
//! shown on a monitor: every capture first checks that the window is still
//! off screen, and hides it and stops otherwise.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use meter_core::events::MeterEvent;
use meter_core::models::diagnostics::MemorySnapshot;
use gpui::{App, AsyncApp, Context, Entity, WindowHandle};
use windows::Win32::Foundation::{FILETIME, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, PostMessageW, SendMessageW, HTCAPTION, HTCLIENT, HTCLOSE, HTMINBUTTON,
    MA_ACTIVATE, MA_ACTIVATEANDEAT, MA_NOACTIVATE, MA_NOACTIVATEANDEAT, PW_RENDERFULLCONTENT, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST,
};

use crate::capture_secondary;
use crate::fixtures;
use crate::meter::{
    Board, BossChoice, Layout, MeterModel, MeterView, OverlaySettings, PctMode, ReplayState,
};
use crate::overlay::{self, off_screen};
use crate::sample;

/// Long enough for a 240 ms tween, a fade and the window fitting its card.
pub(crate) const SETTLE: Duration = Duration::from_millis(700);
const REPLAY_INTERVAL: Duration = Duration::from_secs(15);
/// After the replay, a provisional summary needs 15 s of quiet.
const AFTER_REPLAY: Duration = Duration::from_secs(25);
const REPLAY_LIMIT: Duration = Duration::from_secs(600);
/// Past every tween and fade before idle sampling starts.
const IDLE_SETTLE: Duration = Duration::from_secs(2);
/// Settle plus sample stays under the 15 s after which a quiet boss fight
/// puts up its provisional summary, so the live sample is truly idle.
const IDLE_SAMPLE: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Fixtures,
    Replay,
}

pub fn run(
    window: WindowHandle<MeterView>,
    model: Entity<MeterModel>,
    dir: PathBuf,
    source: Source,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        if let Ok(Ok(hwnd)) = window.update(cx, |_, window, _| overlay::hwnd(window)) {
            println!("frame: {}", off_screen::frame_report(hwnd));
        }
        let result = match std::fs::create_dir_all(&dir) {
            Err(error) => Err(format!("cannot create {}: {error}", dir.display())),
            Ok(()) => match source {
                Source::Fixtures => match capture_fixtures(window, &model, &dir, cx).await {
                    Ok(()) => capture_secondary::from_fixtures(window, &model, &dir, cx).await,
                    failed => failed,
                },
                Source::Replay => match capture_replay(window, &model, &dir, cx).await {
                    Ok(()) => capture_secondary::from_replay(&model, &dir, cx).await,
                    failed => failed,
                },
            },
        };
        if let Err(error) = result {
            eprintln!("capture stopped: {error}");
            crate::FAILED.store(true, Ordering::SeqCst);
        }
        cx.update(|cx| cx.quit());
    })
    .detach();
}

#[derive(Clone, Copy)]
enum Scene {
    Waiting,
    OpenWorld,
    /// The open world with only you on the mob: a one-row board.
    Alone,
    /// Several players per class, on the Session board and on a boss.
    SameClassSession,
    SameClassBoss,
    Unidentified,
    /// The boss fight with players the game has not named.
    BossUnknownPlayers,
    Boss,
    LongBoss,
    Final,
    /// The kill before the game has named you: the summary is the party's.
    FinalParty,
    Provisional,
    /// The boss's health went back to full with it at 12.8%.
    Wiped,
}

struct Fixture {
    name: &'static str,
    settings: OverlaySettings,
    scene: Scene,
    best: Option<f64>,
    hide_unknown_players: bool,
    pointer: bool,
    locked: bool,
    pin: Option<Board>,
    hover: Option<Board>,
    menu: bool,
    copied: bool,
    correct: Option<BossChoice>,
    unavailable: Option<&'static str>,
    /// The pointer on the summary card's report button.
    report_hover: bool,
}

/// Every state of docs/gpui-mockups/approved/meter.html, in its order, then
/// the boss menu and the settings variants.
async fn capture_fixtures(
    window: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    dir: &Path,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let full = OverlaySettings::default();
    let capsule = OverlaySettings { layout: Layout::Capsule, ..OverlaySettings::default() };
    let fixture = |name, scene| Fixture {
        name,
        settings: full.clone(),
        scene,
        best: None,
        hide_unknown_players: false,
        pointer: false,
        locked: false,
        pin: None,
        hover: None,
        menu: false,
        copied: false,
        correct: None,
        unavailable: None,
        report_hover: false,
    };
    let ahead = Some(46_265.0);
    let behind = Some(51_740.0);
    let fixtures = [
        fixture("d-01-session", Scene::OpenWorld),
        Fixture { pointer: true, ..fixture("d-02-session-pointer", Scene::OpenWorld) },
        Fixture { pointer: true, hover: Some(Board::Target), ..fixture("d-03-switch-hover", Scene::OpenWorld) },
        Fixture { pointer: true, copied: true, ..fixture("d-04-copied", Scene::OpenWorld) },
        Fixture { best: ahead, ..fixture("d-05-target-pb", Scene::Boss) },
        Fixture { best: ahead, pointer: true, ..fixture("d-06-target-pointer", Scene::Boss) },
        fixture("d-07-target-no-pb", Scene::Boss),
        Fixture { best: behind, locked: true, ..fixture("d-08-locked", Scene::Boss) },
        Fixture { pin: Some(Board::Session), ..fixture("d-09-pinned", Scene::Boss) },
        Fixture {
            pin: Some(Board::Session),
            pointer: true,
            hover: Some(Board::Session),
            ..fixture("d-10-pinned-hover", Scene::Boss)
        },
        Fixture { pin: Some(Board::Target), ..fixture("d-11-pinned-target", Scene::OpenWorld) },
        fixture("d-12-unidentified", Scene::Unidentified),
        fixture("d-13-long", Scene::LongBoss),
        Fixture { pointer: true, ..fixture("d-14-long-pointer", Scene::LongBoss) },
        fixture("d-15-waiting", Scene::Waiting),
        Fixture { pointer: true, ..fixture("d-16-waiting-pointer", Scene::Waiting) },
        Fixture { settings: capsule.clone(), ..fixture("d-17-capsule-session", Scene::OpenWorld) },
        Fixture { settings: capsule.clone(), best: ahead, ..fixture("d-18-capsule-target", Scene::Boss) },
        Fixture { settings: capsule.clone(), ..fixture("d-19-capsule-idle", Scene::Waiting) },
        Fixture { best: Some(45_440.0), ..fixture("d-20-final", Scene::Final) },
        Fixture { best: behind, ..fixture("d-21-provisional", Scene::Provisional) },
        Fixture { pointer: true, menu: true, ..fixture("menu-full-card", Scene::OpenWorld) },
        Fixture {
            pointer: true,
            menu: true,
            pin: Some(Board::Target),
            ..fixture("menu-one-row", Scene::Alone)
        },
        Fixture { pointer: true, menu: true, ..fixture("menu-no-target", Scene::Waiting) },
        Fixture {
            pointer: true,
            menu: true,
            correct: Some(BossChoice::Boss),
            ..fixture("menu-after-treat-as-boss", Scene::OpenWorld)
        },
        Fixture {
            settings: OverlaySettings {
                content_scale: 1.3,
                background_alpha: 220,
                show_damage: true,
                show_server: true,
                pct_mode: PctMode::Share,
                ..OverlaySettings::default()
            },
            best: behind,
            ..fixture("settings-scaled-damage", Scene::Boss)
        },
        Fixture {
            unavailable: Some(
                "No capture backend available; Npcap: Npcap is not installed; WinDivert: Access is denied (os error 5)",
            ),
            ..fixture("unavailable", Scene::Waiting)
        },
        Fixture { settings: capsule.clone(), ..fixture("capsule-unidentified", Scene::Unidentified) },
        fixture("shades-session", Scene::SameClassSession),
        Fixture { best: ahead, ..fixture("shades-target", Scene::SameClassBoss) },
        Fixture { settings: capsule.clone(), ..fixture("shades-capsule", Scene::SameClassSession) },
        fixture("unknown-players-shown", Scene::BossUnknownPlayers),
        Fixture { hide_unknown_players: true, ..fixture("unknown-players-hidden", Scene::BossUnknownPlayers) },
        Fixture { best: Some(45_440.0), report_hover: true, ..fixture("summary-final-hover", Scene::Final) },
        fixture("summary-party", Scene::FinalParty),
        Fixture { best: Some(45_440.0), locked: true, ..fixture("summary-locked", Scene::Final) },
        Fixture { best: behind, ..fixture("summary-wiped", Scene::Wiped) },
        Fixture { best: behind, locked: true, ..fixture("summary-wiped-locked", Scene::Wiped) },
    ];

    for fixture in &fixtures {
        show_fixture(window, model, fixture, cx)?;
        shoot(window, dir, fixture.name, cx).await?;
    }
    for fixture in [&fixtures[14], &fixtures[0], &fixtures[4]] {
        show_fixture(window, model, fixture, cx)?;
        measure_idle(window, fixture.name, cx).await?;
    }
    check_hit_test(window, model, &[&fixtures[14], &fixtures[0], &fixtures[19]], cx).await?;
    show_fixture(window, model, &fixtures[0], cx)?;
    check_menu_dismissal(window, model, cx).await?;
    // GPUI routes the press by the last frame drawn, so the menu the
    // previous check closed must be gone from it first.
    cx.background_executor().timer(SETTLE).await;
    let (hwnd, scale) = window
        .update(cx, |_, window, _| overlay::hwnd(window).map(|hwnd| (hwnd, window.scale_factor())))
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    check_click_activation(hwnd, "a click on the meter", MA_NOACTIVATE)?;
    check_drag(hwnd, scale, NAME_POINT, "the meter's name", cx).await
}

/// Asks our own window what a left click on its client area would answer to
/// `WM_MOUSEACTIVATE`. A sent message activates nothing; only a real click,
/// which a capture never makes, would act on the answer.
pub(crate) fn check_click_activation(hwnd: HWND, label: &str, expected: u32) -> Result<(), String> {
    let hit = LPARAM(((WM_LBUTTONDOWN as isize) << 16) | HTCLIENT as isize);
    let answer = unsafe { SendMessageW(hwnd, WM_MOUSEACTIVATE, Some(WPARAM(hwnd.0 as usize)), Some(hit)) }.0 as u32;
    let name = |code: u32| match code {
        MA_ACTIVATE => "MA_ACTIVATE".to_string(),
        MA_ACTIVATEANDEAT => "MA_ACTIVATEANDEAT".to_string(),
        MA_NOACTIVATE => "MA_NOACTIVATE".to_string(),
        MA_NOACTIVATEANDEAT => "MA_NOACTIVATEANDEAT".to_string(),
        other => format!("{other}"),
    };
    println!("{label}: WM_MOUSEACTIVATE answers {} (expected {})", name(answer), name(expected));
    if answer != expected {
        return Err(format!("{label}: WM_MOUSEACTIVATE answers {}, not {}", name(answer), name(expected)));
    }
    Ok(())
}

/// Client points on the Session card at 100% (310x260): a row, the switch's
/// Target side, and the "…" tool.
pub(crate) const ROW_POINT: (i32, i32) = (40, 60);
const SWITCH_TARGET_POINT: (i32, i32) = (276, 243);
const MORE_POINT: (i32, i32) = (266, 20);
pub(crate) const MK_LBUTTON: usize = 0x0001;

/// Posts real mouse messages to our own window (nothing else receives any)
/// and reports how the "…" menu reacts: a press elsewhere closes it without
/// reaching the control under it, a second press on "…" closes it, and the
/// pointer leaving the window closes it after 400 ms. The leave is whatever
/// Windows sends after GPUI's own `TrackMouseEvent(TME_LEAVE)` on our HWND.
async fn check_menu_dismissal(
    window: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let hwnd = window
        .update(cx, |view, window, cx| {
            view.forced_pointer = None;
            cx.notify();
            overlay::hwnd(window)
        })
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    let post = |message: u32, wparam: usize, (x, y): (i32, i32)| unsafe {
        let lparam = LPARAM((((y as u16) as isize) << 16) | (x as u16) as isize);
        let _ = PostMessageW(Some(hwnd), message, WPARAM(wparam), lparam);
    };
    let open_menu = |cx: &mut AsyncApp| {
        window
            .update(cx, |view, _, cx| view.force_hover(None, true, false, cx))
            .map_err(|error| format!("overlay window gone: {error:#}"))
    };
    let state = |cx: &mut AsyncApp| {
        let (over, open) = window
            .update(cx, |view, _, _| (view.pointer_over, view.menu_is_open()))
            .map_err(|error| format!("overlay window gone: {error:#}"))?;
        let board = cx.update(|cx| model.read(cx).active_board());
        Ok::<_, String>((over, open, board))
    };
    let settle = Duration::from_millis(150);
    let click = |point| {
        post(WM_MOUSEMOVE, 0, point);
        post(WM_LBUTTONDOWN, MK_LBUTTON, point);
        post(WM_LBUTTONUP, 0, point);
    };

    open_menu(cx)?;
    cx.background_executor().timer(settle * 2).await;
    let (_, open, _) = state(cx)?;
    println!("menu-check control: {} ms with no press: menu open {open}", (settle * 2).as_millis());

    open_menu(cx)?;
    cx.background_executor().timer(settle).await;
    click(ROW_POINT);
    cx.background_executor().timer(settle).await;
    let (_, open, board) = state(cx)?;
    println!("menu-check press on a row: menu open {open}, board {board:?}");

    open_menu(cx)?;
    cx.background_executor().timer(settle).await;
    click(SWITCH_TARGET_POINT);
    cx.background_executor().timer(settle).await;
    let (_, open, board) = state(cx)?;
    println!("menu-check press on the switch's Target side: menu open {open}, board {board:?} (unchanged means the press went no further)");

    click(SWITCH_TARGET_POINT);
    cx.background_executor().timer(settle).await;
    let (_, _, board) = state(cx)?;
    println!("menu-check control: the same press with the menu closed: board {board:?}");

    open_menu(cx)?;
    cx.background_executor().timer(settle).await;
    click(MORE_POINT);
    cx.background_executor().timer(settle).await;
    let (_, open, _) = state(cx)?;
    println!("menu-check second press on \"…\": menu open {open}");

    // GPUI sets its hover flag on WM_MOUSEMOVE (arming TrackMouseEvent with
    // TME_LEAVE on our HWND) and clears it only on WM_MOUSELEAVE. The real
    // cursor is on a monitor, not over this window, so Windows answers the
    // tracking request with a leave at once. We post no WM_MOUSELEAVE.
    let hover = |cx: &mut AsyncApp| {
        window
            .update(cx, |_, window, _| (window.is_window_hovered(), window.mouse_position()))
            .map_err(|error| format!("overlay window gone: {error:#}"))
    };
    let (hovered_before, _) = hover(cx)?;
    open_menu(cx)?;
    post(WM_MOUSEMOVE, 0, ROW_POINT);
    let started = Instant::now();
    cx.background_executor().timer(Duration::from_millis(100)).await;
    let (hovered_after, position) = hover(cx)?;
    println!(
        "menu-check leave: hovered before {hovered_before}; after one posted WM_MOUSEMOVE the pointer is at {position:?} (the move was handled) and hovered is {hovered_after} (a WM_MOUSELEAVE arrived from Windows)"
    );
    let mut closed_at = None;
    while started.elapsed() < Duration::from_millis(1500) {
        let (_, open, _) = state(cx)?;
        if !open {
            closed_at = Some(started.elapsed().as_millis());
            break;
        }
        cx.background_executor().timer(Duration::from_millis(10)).await;
    }
    println!("menu-check leave: menu closed {closed_at:?} ms after the move (400 ms timer)");
    Ok(())
}

/// What a posted drag did to the window, in device pixels.
pub(crate) struct DragOutcome {
    /// How far the pointer moved with the button held.
    pub pointer: (i32, i32),
    pub moved: (i32, i32),
    /// How far a move without the button, after the release, moved it.
    pub after_release: (i32, i32),
}

/// A press on `from` (logical client pixels), a move with the button held,
/// the release, then a move without it, all posted to our own window in one
/// batch so the capture GPUI takes on the press never waits on the real
/// mouse.
pub(crate) async fn post_drag(hwnd: HWND, scale: f32, from: (i32, i32), cx: &mut AsyncApp) -> Result<DragOutcome, String> {
    let post = |message: u32, wparam: usize, (x, y): (i32, i32)| unsafe {
        let lparam = LPARAM((((y as u16) as isize) << 16) | (x as u16) as isize);
        let _ = PostMessageW(Some(hwnd), message, WPARAM(wparam), lparam);
    };
    let origin = || {
        let mut rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(hwnd, &mut rect);
        }
        (rect.left, rect.top)
    };
    let settle = Duration::from_millis(150);
    let device = |(x, y): (i32, i32)| ((x as f32 * scale).round() as i32, (y as f32 * scale).round() as i32);
    let (from, to) = (device(from), device((from.0 - 30, from.1 + 12)));

    let before = origin();
    post(WM_MOUSEMOVE, 0, from);
    post(WM_LBUTTONDOWN, MK_LBUTTON, from);
    post(WM_MOUSEMOVE, MK_LBUTTON, to);
    post(WM_LBUTTONUP, 0, to);
    cx.background_executor().timer(settle).await;
    off_screen::ensure_off_screen(hwnd)?;
    let dragged = origin();
    post(WM_MOUSEMOVE, 0, from);
    cx.background_executor().timer(settle).await;
    off_screen::ensure_off_screen(hwnd)?;
    let released = origin();
    Ok(DragOutcome {
        pointer: (to.0 - from.0, to.1 - from.1),
        moved: (dragged.0 - before.0, dragged.1 - before.1),
        after_release: (released.0 - dragged.0, released.1 - dragged.1),
    })
}

/// A drag on `handle` at `from` must move the window exactly with the held
/// move and ignore the later one.
pub(crate) async fn check_drag(
    hwnd: HWND,
    scale: f32,
    from: (i32, i32),
    handle: &str,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let DragOutcome { pointer, moved, after_release } = post_drag(hwnd, scale, from, cx).await?;
    println!(
        "drag-check: pressed on {handle} at {from:?}, pointer moved {pointer:?} with the button held: window moved {moved:?}; a move after the release moved it {after_release:?}",
    );
    if moved != pointer {
        return Err(format!("drag-check on {handle}: the window moved {moved:?}, expected {pointer:?}"));
    }
    if after_release != (0, 0) {
        return Err(format!("drag-check on {handle}: a move after the release still moved the window by {after_release:?}"));
    }
    Ok(())
}

/// On the header's name, left of anything the pointer shows.
const NAME_POINT: (i32, i32) = (60, 20);

/// Header and row hit tests, left to right, with the tools shown and at rest.
/// Windows reports the pointer crossing between client and non-client area
/// as a leave, and GPUI clears its hover flag on it, so the card must have no
/// non-client part.
async fn check_hit_test(
    window: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    fixtures: &[&Fixture],
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let (hwnd, scale) = window
        .update(cx, |_, window, _| overlay::hwnd(window).map(|hwnd| (hwnd, window.scale_factor())))
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    let mut not_client = Vec::new();
    for fixture in fixtures {
        for pointer in [true, false] {
            show_fixture(window, model, fixture, cx)?;
            set_view(window, Some(pointer), false, cx)?;
            cx.background_executor().timer(SETTLE).await;
            for y in [HEADER_Y, ROW_POINT.1 as f32] {
                let runs = hit_test_runs(hwnd, scale, y);
                let state = format!("{} pointer {} y {y}", fixture.name, if pointer { "over" } else { "away" });
                println!("hit-test {state}: {}", describe_runs(&runs));
                if runs.iter().any(|(code, _, _)| *code as u32 != HTCLIENT) {
                    not_client.push(state);
                }
            }
        }
    }
    if !not_client.is_empty() {
        return Err(format!("hit-test: not all client area in {}", not_client.join("; ")));
    }
    Ok(())
}

const HEADER_Y: f32 = 20.0;

/// `WM_NCHITTEST` along the client row at `y` (logical), as runs of equal
/// answers `(code, first x, last x)` in logical x. GPUI answers from the last
/// mouse move it handled, not from the point in the message, so each point
/// is sent as a move first.
pub(crate) fn hit_test_runs(hwnd: HWND, scale: f32, y: f32) -> Vec<(isize, i32, i32)> {
    let mut client = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    let pack = |point: POINT| LPARAM((((point.y as u16) as isize) << 16) | (point.x as u16) as isize);
    let mut runs: Vec<(isize, i32, i32)> = Vec::new();
    for x in 0..client.right {
        let mut point = POINT { x, y: (y * scale).round() as i32 };
        let code = unsafe {
            let _ = SendMessageW(hwnd, WM_MOUSEMOVE, Some(WPARAM(0)), Some(pack(point)));
            let _ = ClientToScreen(hwnd, &mut point);
            SendMessageW(hwnd, WM_NCHITTEST, Some(WPARAM(0)), Some(pack(point))).0
        };
        let logical = (x as f32 / scale) as i32;
        match runs.last_mut() {
            Some((last, _, end)) if *last == code => *end = logical,
            _ => runs.push((code, logical, logical)),
        }
    }
    runs
}

/// "HTCLIENT 0-309" or "HTCLIENT 0-3, HTCAPTION 4-201, …".
pub(crate) fn describe_runs(runs: &[(isize, i32, i32)]) -> String {
    let name = |code: isize| match code as u32 {
        HTCLIENT => "HTCLIENT".to_string(),
        HTCAPTION => "HTCAPTION".to_string(),
        HTMINBUTTON => "HTMINBUTTON".to_string(),
        HTCLOSE => "HTCLOSE".to_string(),
        other => format!("hit {other}"),
    };
    runs.iter().map(|(code, start, end)| format!("{} {start}-{end}", name(*code))).collect::<Vec<_>>().join(", ")
}

fn show_fixture(
    window: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    fixture: &Fixture,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    cx.update(|cx| {
        model.update(cx, |model, cx| {
            *model = MeterModel::new(fixture.settings.clone(), None);
            model.use_memory_mobs();
            model.set_hide_unknown_players(fixture.hide_unknown_players);
            if let Some(best) = fixture.best {
                model.seed_personal_best(sample::YOU, sample::BOSS_MOB, best);
            }
            let now = Instant::now();
            let quiet = now - Duration::from_secs(125);
            model.apply(MeterEvent::DpsMemory(sample_memory()), now);
            if let Some(reason) = fixture.unavailable {
                model.set_unavailable(reason.to_string());
            }
            let snapshots = match fixture.scene {
                Scene::Waiting => vec![],
                Scene::OpenWorld => vec![(fixtures::open_world(), now)],
                Scene::Alone => vec![(fixtures::alone(), now)],
                Scene::SameClassSession => vec![(fixtures::same_classes(fixtures::open_world()), now)],
                Scene::SameClassBoss => vec![
                    (fixtures::same_classes(fixtures::open_world()), now),
                    (fixtures::same_classes(sample::boss_fight(221.0, 624)), now),
                ],
                Scene::Unidentified => vec![(fixtures::unidentified(), now)],
                Scene::BossUnknownPlayers => vec![(fixtures::boss_with_unknown_players(), now)],
                Scene::Boss => vec![(sample::boss_fight(221.0, 624), now)],
                Scene::LongBoss => vec![(fixtures::long_boss_fight(221.0, 624), now)],
                Scene::Final => vec![
                    (sample::boss_fight(247.0, 40), now),
                    (sample::boss_fight(248.0, 0), now),
                ],
                Scene::FinalParty => {
                    let unnamed = |mut snapshot: meter_core::models::combat::CombatSnapshot| {
                        snapshot.combat_infos.main_actor_id = None;
                        snapshot.combat_infos.main_actor_name = None;
                        snapshot
                    };
                    vec![(unnamed(sample::boss_fight(247.0, 40)), now), (unnamed(sample::boss_fight(248.0, 0)), now)]
                }
                Scene::Provisional => vec![(sample::boss_fight(177.0, 183), quiet)],
                Scene::Wiped => vec![(sample::boss_fight(177.0, 128), now)],
            };
            for (snapshot, at) in snapshots {
                model.apply(MeterEvent::DpsSnapshot(snapshot), at);
            }
            if matches!(fixture.scene, Scene::Wiped) {
                let snapshot = sample::boss_fight(177.0, 128);
                let report = meter_core::report::fight_report::FightReport::from_snapshot(&snapshot, sample::BOSS_ID, None);
                let wipe = meter_core::events::WipedAttempt { target_id: sample::BOSS_ID, snapshot, record_id: None, report };
                model.apply(MeterEvent::BossWiped(wipe), now);
            }
            if let Some(side) = fixture.pin {
                model.select_board(side);
            }
            if let Some(choice) = fixture.correct {
                model.set_boss_choice(choice);
            }
            model.arm_deadline(cx);
            cx.notify();
        })
    });
    set_view(window, Some(fixture.pointer), fixture.locked, cx)?;
    window
        .update(cx, |view, _, cx| {
            view.force_hover(fixture.hover, fixture.menu, fixture.copied, cx);
            view.force_report_hover(fixture.report_hover, cx);
        })
        .map_err(|error| format!("overlay window gone: {error:#}"))
}

/// Frames built and CPU used by this process while the overlay shows a
/// settled state and nothing changes.
async fn measure_idle(window: WindowHandle<MeterView>, name: &str, cx: &mut AsyncApp) -> Result<(), String> {
    cx.background_executor().timer(IDLE_SETTLE).await;
    let renders = |cx: &mut AsyncApp| {
        window
            .update(cx, |view, _, _| view.renders)
            .map_err(|error| format!("overlay window gone: {error:#}"))
    };
    let (frames_before, cpu_before) = (renders(cx)?, process_cpu()?);
    cx.background_executor().timer(IDLE_SAMPLE).await;
    let (frames_after, cpu_after) = (renders(cx)?, process_cpu()?);
    let cpu = cpu_after - cpu_before;
    println!(
        "idle {name}: {} frames built, {:.0} ms of CPU in {} s ({:.2}% of one core)",
        frames_after - frames_before,
        cpu.as_secs_f64() * 1000.0,
        IDLE_SAMPLE.as_secs(),
        cpu.as_secs_f64() / IDLE_SAMPLE.as_secs_f64() * 100.0
    );
    Ok(())
}

fn process_cpu() -> Result<Duration, String> {
    let (mut creation, mut exit, mut kernel, mut user) = Default::default();
    unsafe {
        GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user)
            .map_err(|error| format!("GetProcessTimes failed: {error}"))?;
    }
    let ticks = |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    Ok(Duration::from_nanos((ticks(kernel) + ticks(user)) * 100))
}

/// Every 15 s of a paced replay: the overlay as it is, under the pointer,
/// locked, and as a capsule.
async fn capture_replay(
    window: WindowHandle<MeterView>,
    model: &Entity<MeterModel>,
    dir: &Path,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let started = Instant::now();
    let mut replay_over_at: Option<Instant> = None;
    loop {
        let name = format!("replay-{:03}s", started.elapsed().as_secs());
        set_view(window, Some(false), false, cx)?;
        shoot(window, dir, &name, cx).await?;
        set_view(window, Some(true), false, cx)?;
        shoot(window, dir, &format!("{name}-pointer"), cx).await?;
        set_view(window, Some(false), true, cx)?;
        shoot(window, dir, &format!("{name}-locked"), cx).await?;
        set_view(window, Some(false), false, cx)?;
        cx.update(|cx| model.update(cx, toggle_layout));
        shoot(window, dir, &format!("{name}-capsule"), cx).await?;
        cx.update(|cx| model.update(cx, toggle_layout));

        let replay = cx.update(|cx| model.read(cx).replay.clone());
        if let ReplayState::Failed(error) = replay {
            return Err(format!("the replay failed: {error}"));
        }
        if replay != ReplayState::NotStarted && replay_over_at.is_none() {
            replay_over_at = Some(Instant::now());
        }
        if replay_over_at.is_some_and(|at| at.elapsed() >= AFTER_REPLAY)
            || started.elapsed() >= REPLAY_LIMIT
        {
            return Ok(());
        }
        cx.background_executor().timer(REPLAY_INTERVAL).await;
    }
}

/// The capsule button's effect, without the settings store: captures never
/// touch the user's `settings.json`.
fn toggle_layout(model: &mut MeterModel, cx: &mut Context<MeterModel>) {
    let mut settings = model.settings.clone();
    settings.layout = settings.layout.toggled();
    model.set_overlay_settings(settings);
    cx.notify();
}

fn set_view(
    window: WindowHandle<MeterView>,
    pointer: Option<bool>,
    locked: bool,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    window
        .update(cx, |view, window, cx| {
            view.forced_pointer = pointer;
            if view.is_locked() != locked {
                view.set_locked(locked, window, cx);
            }
            cx.notify();
        })
        .map_err(|error| format!("overlay window gone: {error:#}"))
}

async fn shoot(
    window: WindowHandle<MeterView>,
    dir: &Path,
    name: &str,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let hwnd = window
        .update(cx, |_, window, _| overlay::hwnd(window))
        .map_err(|error| format!("overlay window gone: {error:#}"))??;
    shoot_hwnd(hwnd, dir, name, cx).await
}

/// Waits for the window to settle, checks it is still off screen, and saves
/// what it draws.
pub async fn shoot_hwnd(hwnd: HWND, dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<(), String> {
    cx.background_executor().timer(SETTLE).await;
    let raw = hwnd.0 as isize;
    let path = dir.join(format!("{name}.png"));
    let target = path.clone();
    let (width, height) = cx
        .background_executor()
        .spawn(async move { grab(HWND(raw as *mut c_void), &target) })
        .await?;
    println!("captured {} ({width}x{height})", path.display());
    Ok(())
}

/// The client area as the compositor has it, with its alpha.
pub fn grab(hwnd: HWND, path: &Path) -> Result<(u32, u32), String> {
    let image = grab_image(hwnd)?;
    image.save(path).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(image.dimensions())
}

/// [`grab`] without writing a file.
pub fn grab_image(hwnd: HWND) -> Result<image::RgbaImage, String> {
    off_screen::ensure_off_screen(hwnd)?;
    let mut outer = RECT::default();
    let mut client = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer).map_err(|error| format!("GetWindowRect failed: {error}"))?;
        GetClientRect(hwnd, &mut client).map_err(|error| format!("GetClientRect failed: {error}"))?;
        if !ClientToScreen(hwnd, &mut origin).as_bool() {
            return Err("ClientToScreen failed".to_string());
        }
    }
    let (width, height) = (outer.right - outer.left, outer.bottom - outer.top);
    let pixels = print_window(hwnd, width, height)?;

    let (left, top) = (origin.x - outer.left, origin.y - outer.top);
    let (client_width, client_height) = (client.right as u32, client.bottom as u32);
    let mut rgba = Vec::with_capacity((client_width * client_height * 4) as usize);
    for y in 0..client_height as i32 {
        let row = ((y + top) * width + left) as usize * 4;
        for pixel in pixels[row..row + client_width as usize * 4].chunks_exact(4) {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    image::RgbaImage::from_raw(client_width, client_height, rgba)
        .ok_or_else(|| "captured pixels do not match the client size".to_string())
}

fn print_window(hwnd: HWND, width: i32, height: i32) -> Result<Vec<u8>, String> {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        let dc = CreateCompatibleDC(None);
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) => bitmap,
            Err(error) => {
                let _ = DeleteDC(dc);
                return Err(format!("CreateDIBSection failed: {error}"));
            }
        };
        let previous = SelectObject(dc, bitmap.into());
        let printed = PrintWindow(hwnd, dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT));
        let pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
        SelectObject(dc, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        if !printed.as_bool() {
            return Err("PrintWindow failed".to_string());
        }
        Ok(pixels)
    }
}

fn sample_memory() -> MemorySnapshot {
    MemorySnapshot {
        cpu_percent: 0.4,
        rss_mb: 80.0,
        vms_mb: 120.0,
        memory_percent: 0.2,
        system_cpu_percent: 18.0,
        system_memory_used_mb: 11_468.8,
        system_memory_total_mb: 32_768.0,
        cap_device: None,
        cap_port: None,
        packet_sizes: Default::default(),
        ping_ms: Some(42.0),
        main_actor_name: None,
    }
}
