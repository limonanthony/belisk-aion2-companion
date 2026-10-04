mod assets;
mod bridge;
#[cfg(feature = "probes")]
mod capture;
#[cfg(feature = "probes")]
mod capture_secondary;
mod cli;
mod detail;
#[cfg(feature = "probes")]
mod fixtures;
#[cfg(feature = "probes")]
mod font_probe;
mod fonts;
mod format;
mod game_data;
mod history;
mod hotkey;
mod live_session;
mod meter;
mod overlay;
#[cfg(feature = "probes")]
mod probe;
mod pvp;
mod replay;
mod secondary;
mod style;

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use aether_core::engine::meter::DpsMeter;
use gpui::{App, AppContext as _};

use cli::Args;
use meter::{MeterModel, OverlaySettings};
use overlay::Placement;

static FAILED: AtomicBool = AtomicBool::new(false);

/// The whole program, so the unit tests live in this library: the Administrator
/// manifest is linked into the binary only, and tests must run unelevated.
pub fn run() -> ExitCode {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };

    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            if let Err(error) = start(args, cx) {
                eprintln!("{error}");
                FAILED.store(true, Ordering::SeqCst);
                cx.quit();
            }
        });

    if FAILED.load(Ordering::SeqCst) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn start(args: Args, cx: &mut App) -> Result<(), String> {
    fonts::register(cx)?;
    cx.set_reduce_motion(!overlay::system_animates());

    #[cfg(feature = "probes")]
    if args.probe == Some(cli::Probe::Font) {
        return font_probe::open(cx, args.capture.clone());
    }

    let (sink, events) = bridge::channel();
    let sink = Arc::new(sink);
    let mut unavailable = None;
    let meter: Option<Arc<DpsMeter>> = match &args.replay {
        Some(recording) => Some(replay::start(sink, recording, args.speed)?),
        None if args.wants_live() => {
            let (meter, started) = live_session::start(sink, args.collect_mobs);
            match started {
                Ok(()) => Some(meter),
                Err(reason) => {
                    eprintln!("the meter could not start: {reason}");
                    unavailable = Some(reason);
                    None
                }
            }
        }
        None => None,
    };
    if let Some(meter) = meter.clone() {
        cx.on_app_quit(move |_| {
            meter.cancel_packet_replay();
            meter.stop_dps_meter();
            async {}
        })
        .detach();
    }

    let (settings, placement) = overlay_setup(&args);
    let model = cx.new(|_| {
        let mut model = MeterModel::new(settings, meter);
        if args.debug_log {
            model.enable_decision_log();
        }
        if let Some(reason) = unavailable {
            model.set_unavailable(reason);
        }
        model
    });
    bridge::spawn_pump(model.clone(), events, args.exit_after_replay, cx).detach();
    #[cfg(feature = "probes")]
    let foreground_before_open = probe::foreground_window();
    let window = overlay::open(cx, model.clone(), placement)?;
    secondary::init(&window, placement, cx)?;

    #[cfg(feature = "probes")]
    if let Some(dir) = args.capture.clone() {
        // Captures may run while someone plays: no global shortcut, which
        // would swallow their Alt+CapsLock. The button is drawn as it would be.
        window
            .update(cx, |view, _, cx| view.enable_lock_shortcut(cx))
            .map_err(|error| format!("cannot reach the overlay window: {error:#}"))?;
        let source = if args.fixtures { capture::Source::Fixtures } else { capture::Source::Replay };
        capture::run(window, model, dir, source, cx);
        return Ok(());
    }

    match hotkey::register_lock_toggle(window, cx) {
        Ok(()) => window
            .update(cx, |view, _, cx| view.enable_lock_shortcut(cx))
            .map_err(|error| format!("cannot reach the overlay window: {error:#}"))?,
        Err(error) => eprintln!("{error}; the lock button is hidden, since nothing could unlock it"),
    }

    #[cfg(feature = "probes")]
    if args.probe == Some(cli::Probe::Overlay) {
        probe::run(window, foreground_before_open, args.replay.is_some(), cx);
    }
    Ok(())
}

#[cfg(not(feature = "probes"))]
fn overlay_setup(_: &Args) -> (OverlaySettings, Placement) {
    (OverlaySettings::default(), Placement::Screen)
}

#[cfg(feature = "probes")]
fn overlay_setup(args: &Args) -> (OverlaySettings, Placement) {
    if args.capture.is_none() {
        return (OverlaySettings::default(), Placement::Screen);
    }
    // Replay captures are written to disk; other players' names stay out of
    // them.
    let settings = OverlaySettings { mask_nicknames: args.replay.is_some(), ..OverlaySettings::default() };
    (settings, Placement::OffScreen)
}
