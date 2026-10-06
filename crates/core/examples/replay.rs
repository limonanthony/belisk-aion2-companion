//! Replays an `.aetherpc` recording through the full pipeline and prints a
//! summary of the final snapshot. No capture driver is loaded and no elevation
//! is needed.
//!
//!     cargo run --example replay -- <recording.aetherpc> [--names]
//!
//! Other players' names are only printed with `--names`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meter_core::engine::meter::DpsMeter;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;

const REPLAY_TIMEOUT: Duration = Duration::from_secs(600);
const DRAIN_POLL: Duration = Duration::from_millis(250);
const DRAIN_STABLE_POLLS: u32 = 8;
const DRAIN_TIMEOUT: Duration = Duration::from_secs(60);

enum ReplayOutcome {
    Finished(u64),
    Failed(String),
}

struct CollectingSink {
    counts: Mutex<BTreeMap<&'static str, usize>>,
    outcome: Mutex<Sender<ReplayOutcome>>,
}

impl EventSink for CollectingSink {
    fn emit(&self, event: MeterEvent) {
        let name = match &event {
            MeterEvent::DpsSnapshot(_) => "dps-snapshot",
            MeterEvent::DpsMemory(_) => "dps-memory",
            MeterEvent::DpsMeterStatus(_) => "dps-meter-status",
            MeterEvent::HistoryUpdated => "history-updated",
            MeterEvent::PersonalBestsUpdated => "personal-bests-updated",
            MeterEvent::MainActorDetected(_) => "dps-main-actor-detected",
            MeterEvent::CharacterSeen(_) => "character-seen",
            MeterEvent::OwnIdentity(_) => "own-identity",
            MeterEvent::PacketReplayFinished(_) => "packet-replay-finished",
            MeterEvent::PacketReplayFailed(_) => "packet-replay-failed",
            MeterEvent::DpsIdleHidden(_) => "dps-idle-hidden",
        };
        *self.counts.lock().unwrap().entry(name).or_default() += 1;

        let outcome = match event {
            MeterEvent::PacketReplayFinished(count) => ReplayOutcome::Finished(count),
            MeterEvent::PacketReplayFailed(error) => ReplayOutcome::Failed(error),
            _ => return,
        };
        let _ = self.outcome.lock().unwrap().send(outcome);
    }
}

fn main() -> ExitCode {
    let mut path = None;
    let mut show_names = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--names" => show_names = true,
            _ if path.is_none() => path = Some(arg),
            _ => {
                eprintln!("unexpected argument: {arg}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: replay <recording.aetherpc> [--names]");
        return ExitCode::FAILURE;
    };

    match run(path, show_names) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("replay failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(path: String, show_names: bool) -> Result<(), String> {
    let (outcome_tx, outcome_rx) = mpsc::channel();
    let sink = Arc::new(CollectingSink {
        counts: Mutex::new(BTreeMap::new()),
        outcome: Mutex::new(outcome_tx),
    });
    let log_path: PathBuf = std::env::temp_dir().join("aether").join("replay-example.log");
    let logger = Arc::new(AppLogger::new(log_path, Box::new(|_| {})));
    let meter = DpsMeter::new(sink.clone(), logger, None);

    let mut config = meter.current_config();
    config.idle_reset_secs = 0;
    config.auto_record_unknown_server = false;
    config.hide_unknown_players = false;
    config.max_player_count = 1_000;
    meter.apply_config(config);

    meter.start_for_replay()?;
    meter.replay_packet_recording(path.clone())?;

    let packets = match outcome_rx
        .recv_timeout(REPLAY_TIMEOUT)
        .map_err(|error| format!("replay of {path} did not finish: {error}"))?
    {
        ReplayOutcome::Finished(count) => count,
        ReplayOutcome::Failed(error) => return Err(error),
    };

    wait_for_pipeline_to_drain(&meter);
    let Some(snapshot) = meter.get_dps_snapshot(0) else {
        return Err("the recording produced no combat data".to_string());
    };

    println!("packets replayed: {packets}");
    println!("note: timestamps are taken at replay time, so durations are compressed");

    match &snapshot.last_target_info {
        Some(target) => {
            let started = target
                .target_start_time
                .values()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let ended = target.target_last_time.values().copied().fold(0.0, f64::max);
            let name = target.target_name.as_deref().unwrap_or("(unnamed)");
            println!(
                "target: {name} (mob code {}, boss: {})",
                target
                    .target_mob_code
                    .map_or_else(|| "?".to_string(), |code| code.to_string()),
                target.is_boss
            );
            if started.is_finite() && ended >= started {
                println!("fight duration: {:.1}s", ended - started);
            } else {
                println!("fight duration: n/a");
            }
        }
        None => println!("target: none"),
    }

    let players = &snapshot.last_target_all_players_overview_stats;
    let named = players.iter().filter(|p| !p.actor_name.is_empty()).count();
    println!(
        "actors on target: {} ({named} named, {} unnamed)",
        players.len(),
        players.len() - named
    );
    println!(
        "actors known: {}",
        snapshot.combat_infos.actor_infos.len()
    );
    println!("total damage: {}", snapshot.total_damage);

    let session = &snapshot.session;
    println!(
        "session: {} players, {} targets, total damage {}, combat time {:.1}s",
        session.players.len(),
        session.target_count,
        session.total_damage,
        session.combat_time_secs
    );
    if let Some(top) = session.players.first() {
        println!(
            "session top row: {} damage, {:.1}s combat time, {:.0} dps, {:.1}% share",
            top.total_damage,
            top.combat_time_secs,
            top.dps,
            top.damage_share * 100.0
        );
    }

    match snapshot.combat_infos.main_actor_id {
        Some(main_id) => match players.iter().find(|p| p.actor_id == main_id) {
            Some(player) => println!(
                "local player: name resolved: {}, dps: {:.0}",
                !player.actor_name.is_empty(),
                player.dps
            ),
            None => println!("local player: identified, no damage on the last target"),
        },
        None => println!("local player: not identified"),
    }

    if show_names {
        for player in players {
            println!(
                "  {:>10.0} dps  {}",
                player.dps,
                if player.actor_name.is_empty() {
                    "(unnamed)"
                } else {
                    &player.actor_name
                }
            );
        }
    }

    println!("events emitted:");
    for (name, count) in sink.counts.lock().unwrap().iter() {
        println!("  {name}: {count}");
    }
    Ok(())
}

fn wait_for_pipeline_to_drain(meter: &DpsMeter) {
    let deadline = Instant::now() + DRAIN_TIMEOUT;
    let mut last_total = None;
    let mut stable_polls = 0;
    while Instant::now() < deadline && stable_polls < DRAIN_STABLE_POLLS {
        std::thread::sleep(DRAIN_POLL);
        let total = meter.get_dps_snapshot(0).map(|s| s.total_damage);
        if total.is_some() && total == last_total {
            stable_polls += 1;
        } else {
            stable_polls = 0;
            last_total = total;
        }
    }
}
