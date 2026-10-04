//! `--capture <dir>`: renders the overlay outside every monitor and saves
//! what it draws with `PrintWindow(PW_RENDERFULLCONTENT)`, so the design can
//! be checked while someone is playing on the same machine. Nothing is ever
//! shown on a monitor: every capture first checks that the window is still
//! off screen, and hides it and stops otherwise.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use aether_core::events::MeterEvent;
use aether_core::models::diagnostics::MemorySnapshot;
use gpui::{App, AsyncApp, Entity, WindowHandle};
use windows::Win32::Foundation::{FILETIME, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, PostMessageW, PW_RENDERFULLCONTENT, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE,
};

use crate::capture_secondary;
use crate::fixtures;
use crate::meter::{
    Board, BossChoice, Layout, MeterModel, MeterView, OverlaySettings, PctMode, ReplayState,
};
use crate::overlay::{self, off_screen};

/// Long enough for a 240 ms tween, a fade and the window fitting its card.
const SETTLE: Duration = Duration::from_millis(700);
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
    Boss,
    LongBoss,
    Final,
    Provisional,
}

struct Fixture {
    name: &'static str,
    settings: OverlaySettings,
    scene: Scene,
    best: Option<f64>,
    pointer: bool,
    locked: bool,
    pin: Option<Board>,
    hover: Option<Board>,
    menu: bool,
    copied: bool,
    correct: Option<BossChoice>,
    unavailable: Option<&'static str>,
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
        pointer: false,
        locked: false,
        pin: None,
        hover: None,
        menu: false,
        copied: false,
        correct: None,
        unavailable: None,
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
    ];

    for fixture in &fixtures {
        show_fixture(window, model, fixture, cx)?;
        shoot(window, dir, fixture.name, cx).await?;
    }
    for fixture in [&fixtures[14], &fixtures[0], &fixtures[4]] {
        show_fixture(window, model, fixture, cx)?;
        measure_idle(window, fixture.name, cx).await?;
    }
    show_fixture(window, model, &fixtures[0], cx)?;
    check_menu_dismissal(window, model, cx).await
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
            if let Some(best) = fixture.best {
                model.seed_personal_best(fixtures::YOU, fixtures::BOSS_MOB, best);
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
                    (fixtures::same_classes(fixtures::boss_fight(221.0, 624)), now),
                ],
                Scene::Unidentified => vec![(fixtures::unidentified(), now)],
                Scene::Boss => vec![(fixtures::boss_fight(221.0, 624), now)],
                Scene::LongBoss => vec![(fixtures::long_boss_fight(221.0, 624), now)],
                Scene::Final => vec![
                    (fixtures::boss_fight(247.0, 40), now),
                    (fixtures::boss_fight(248.0, 0), now),
                ],
                Scene::Provisional => vec![(fixtures::boss_fight(177.0, 183), quiet)],
            };
            for (snapshot, at) in snapshots {
                model.apply(MeterEvent::DpsSnapshot(snapshot), at);
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
        .update(cx, |view, _, cx| view.force_hover(fixture.hover, fixture.menu, fixture.copied, cx))
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
        cx.update(|cx| model.update(cx, |model, cx| model.toggle_layout(cx)));
        shoot(window, dir, &format!("{name}-capsule"), cx).await?;
        cx.update(|cx| model.update(cx, |model, cx| model.toggle_layout(cx)));

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
        .ok_or("captured pixels do not match the client size")?
        .save(path)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok((client_width, client_height))
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
