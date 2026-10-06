mod assets;
mod bridge;
#[cfg(feature = "probes")]
mod capture;
#[cfg(feature = "probes")]
mod capture_reminders;
#[cfg(feature = "probes")]
mod capture_secondary;
#[cfg(feature = "probes")]
mod capture_settings;
#[cfg(feature = "probes")]
mod capture_todos;
mod cli;
mod clock;
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
mod i18n;
mod live_session;
mod meter;
mod overlay;
#[cfg(feature = "probes")]
mod probe;
mod pvp;
mod quit;
mod reminders;
mod replay;
mod sample;
mod secondary;
mod settings;
mod style;
mod todos;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use meter_core::engine::meter::DpsMeter;
use gpui::{App, AppContext as _};

use cli::Args;
use i18n::LanguageChoice;
use meter::{MeterModel, OverlaySettings};
use overlay::Placement;
use settings::{AppSettings, Settings};

static FAILED: AtomicBool = AtomicBool::new(false);

/// The whole program, so the unit tests live in this library: the Administrator
/// manifest is linked into the binary only, and tests must run unelevated.
pub fn run() -> ! {
    quit::terminate(run_app())
}

fn run_app() -> u32 {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}\n{}", cli::USAGE);
            return 2;
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

    exit_code()
}

fn exit_code() -> u32 {
    u32::from(FAILED.load(Ordering::SeqCst))
}

fn start(args: Args, cx: &mut App) -> Result<(), String> {
    quit::install_deadline(cx);
    quit::install_exit(cx, exit_code);
    fonts::register(cx)?;
    cx.set_reduce_motion(!overlay::system_animates());

    #[cfg(feature = "probes")]
    if args.probe == Some(cli::Probe::Font) {
        return font_probe::open(cx, args.capture.clone());
    }

    let (overlay, placement) = overlay_setup(&args);
    let settings = if args.wants_live() {
        Settings::load(live_session::data_dir().as_deref())
    } else {
        Settings::in_memory(non_live_settings(overlay, args.replay.is_some(), capture_language(&args)))
    };
    let initial = settings.get().clone();

    let (sink, events) = bridge::channel();
    let sink = Arc::new(sink);
    let mut unavailable = None;
    let meter: Option<Arc<DpsMeter>> = match &args.replay {
        Some(recording) => Some(replay::start(sink, recording, args.speed)?),
        None if args.wants_live() => {
            let (meter, started) = live_session::start(sink, &initial);
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

    let settings = cx.new(|_| settings);
    settings::install(&settings, cx);
    reminders::install(&settings, args.wants_live(), cx);
    todos::install(&settings, args.wants_live(), cx);
    cx.on_app_quit({
        let settings = settings.clone();
        move |cx| {
            settings.update(cx, |settings, _| settings.flush());
            async {}
        }
    })
    .detach();

    let meter_for_settings = meter.clone();
    let model = cx.new(|_| {
        let mut model = MeterModel::new(initial.overlay.clone(), meter);
        if let Some(reason) = unavailable {
            model.set_unavailable(reason);
        }
        model
    });
    settings::connect(&settings, &model, meter_for_settings, cx);
    bridge::spawn_pump(model.clone(), events, args.exit_after_replay, cx).detach();
    #[cfg(feature = "probes")]
    let foreground_before_open = probe::foreground_window();
    let window = overlay::open(cx, model.clone(), placement)?;
    meter::install_window(window, cx);
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

/// Settings for runs that never read the user's file. A replay starts from
/// the options `replay::start` gives the core, so the first change made in
/// Settings does not overwrite them with the live defaults.
fn non_live_settings(overlay: OverlaySettings, replaying: bool, language: LanguageChoice) -> AppSettings {
    let mut settings = AppSettings { overlay, ..AppSettings::default() };
    settings.general.language = language;
    if replaying {
        settings.behaviour.end_fight_after_secs = 0;
        settings.capture.record_unfamiliar_servers = false;
    }
    settings
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

#[cfg(not(feature = "probes"))]
fn capture_language(_: &Args) -> LanguageChoice {
    LanguageChoice::System
}

#[cfg(feature = "probes")]
fn capture_language(args: &Args) -> LanguageChoice {
    args.language.unwrap_or_default()
}
