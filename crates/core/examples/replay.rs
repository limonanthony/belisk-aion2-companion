//! Replays an `.aetherpc` recording through the full pipeline and prints a
//! summary of the final snapshot. No capture driver is loaded and no elevation
//! is needed.
//!
//!     cargo run --example replay -- <recording.aetherpc> [--names] [--tickets] [--fights on|off|from-identity|from-tickets] [--speed N]
//!
//! Other players' names are only printed with `--names`. `--tickets` lists the
//! ticket events the core emitted, with the capture time as Unix seconds.
//! `--fights off` replays with fight tracking off, as with the DPS meter
//! closed; `--fights from-identity` turns it on when the game first names the
//! player's character, and `--fights from-tickets` at the first ticket event,
//! as if the meter were opened then.
//! `--speed N` replays at N times the recorded pace, which the boss wipes need:
//! the meter closes a wiped attempt on its snapshot tick (200 ms), so an
//! unpaced replay would close every wipe at the end. The boss wipes the meter
//! closed and the history it saved are listed at the end.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use meter_core::engine::meter::DpsMeter;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::tickets::{TicketContent, TicketReport};
use meter_core::logger::AppLogger;

const REPLAY_TIMEOUT: Duration = Duration::from_secs(600);
const DRAIN_POLL: Duration = Duration::from_millis(250);
const DRAIN_STABLE_POLLS: u32 = 8;
const DRAIN_TIMEOUT: Duration = Duration::from_secs(60);

enum ReplayOutcome {
    Finished(u64),
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FightMode {
    On,
    Off,
    FromIdentity,
    FromTickets,
}

struct CollectingSink {
    counts: Mutex<BTreeMap<&'static str, usize>>,
    tickets: Mutex<Vec<TicketReport>>,
    wipes: Mutex<Vec<String>>,
    outcome: Mutex<Sender<ReplayOutcome>>,
    meter: OnceLock<Arc<DpsMeter>>,
    fights: FightMode,
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
            MeterEvent::Tickets(_) => "tickets",
            MeterEvent::PacketReplayFinished(_) => "packet-replay-finished",
            MeterEvent::PacketReplayFailed(_) => "packet-replay-failed",
            MeterEvent::DpsIdleHidden(_) => "dps-idle-hidden",
            MeterEvent::BossWiped(_) => "boss-wiped",
        };
        *self.counts.lock().unwrap().entry(name).or_default() += 1;

        if let MeterEvent::Tickets(report) = &event {
            self.tickets.lock().unwrap().push(report.clone());
        }
        if let MeterEvent::BossWiped(wiped) = &event {
            let target = wiped.snapshot.combat_infos.target_infos.get(&wiped.target_id);
            self.wipes.lock().unwrap().push(format!(
                "target {} {} hp {:?}/{:?} players {} record {:?}",
                wiped.target_id,
                target.and_then(|target| target.target_name.as_deref()).unwrap_or("(unnamed)"),
                target.and_then(|target| target.current_hp),
                target.and_then(|target| target.max_hp),
                wiped.snapshot.by_target_player_stats.get(&wiped.target_id).map_or(0, |players| players.len()),
                wiped.record_id
            ));
        }
        let opens_the_meter = match &event {
            MeterEvent::MainActorDetected(payload) => {
                self.fights == FightMode::FromIdentity && !payload.actor_name.is_empty()
            }
            MeterEvent::Tickets(_) => self.fights == FightMode::FromTickets,
            _ => false,
        };
        if opens_the_meter {
            if let Some(meter) = self.meter.get() {
                meter.set_fight_tracking(true);
            }
        }
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
    let mut show_tickets = false;
    let mut fights = FightMode::On;
    let mut speed = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--names" => show_names = true,
            "--tickets" => show_tickets = true,
            "--speed" => match args.next().and_then(|value| value.parse::<f64>().ok()) {
                Some(value) if value > 0.0 => speed = Some(value),
                _ => {
                    eprintln!("--speed takes a positive number");
                    return ExitCode::FAILURE;
                }
            },
            "--fights" => {
                fights = match args.next().as_deref() {
                    Some("on") => FightMode::On,
                    Some("off") => FightMode::Off,
                    Some("from-identity") => FightMode::FromIdentity,
                    Some("from-tickets") => FightMode::FromTickets,
                    other => {
                        eprintln!("--fights takes on, off, from-identity or from-tickets, not {other:?}");
                        return ExitCode::FAILURE;
                    }
                };
            }
            _ if path.is_none() => path = Some(arg),
            _ => {
                eprintln!("unexpected argument: {arg}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: replay <recording.aetherpc> [--names] [--tickets] [--fights on|off|from-identity|from-tickets] [--speed N]");
        return ExitCode::FAILURE;
    };

    match run(path, show_names, show_tickets, fights, speed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("replay failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(path: String, show_names: bool, show_tickets: bool, fights: FightMode, speed: Option<f64>) -> Result<(), String> {
    let (outcome_tx, outcome_rx) = mpsc::channel();
    let sink = Arc::new(CollectingSink {
        counts: Mutex::new(BTreeMap::new()),
        tickets: Mutex::new(Vec::new()),
        wipes: Mutex::new(Vec::new()),
        outcome: Mutex::new(outcome_tx),
        meter: OnceLock::new(),
        fights,
    });
    let log_path: PathBuf = std::env::temp_dir().join("aether").join("replay-example.log");
    let logger = Arc::new(AppLogger::new(log_path, Box::new(|_| {})));
    let meter = Arc::new(DpsMeter::new(sink.clone(), logger, None));
    let _ = sink.meter.set(Arc::clone(&meter));
    meter.set_fight_tracking(fights == FightMode::On);

    let mut config = meter.current_config();
    config.idle_reset_secs = 0;
    config.auto_record_unknown_server = false;
    config.hide_unknown_players = false;
    config.max_player_count = 1_000;
    meter.apply_config(config);

    meter.start_for_replay()?;
    match speed {
        Some(speed) => meter.replay_packet_recording_paced(path.clone(), speed)?,
        None => meter.replay_packet_recording(path.clone())?,
    }

    let packets = match outcome_rx
        .recv_timeout(REPLAY_TIMEOUT)
        .map_err(|error| format!("replay of {path} did not finish: {error}"))?
    {
        ReplayOutcome::Finished(count) => count,
        ReplayOutcome::Failed(error) => return Err(error),
    };

    wait_for_pipeline_to_drain(&meter);
    let snapshot = match meter.get_dps_snapshot(0) {
        Some(snapshot) => snapshot,
        None if fights == FightMode::Off => {
            println!("packets replayed: {packets}");
            println!("fight tracking off: no combat data, as expected");
            print_own_character(&meter);
            if show_tickets {
                print_tickets(&sink.tickets.lock().unwrap(), show_names);
            }
            print_events(&sink, &meter);
            return Ok(());
        }
        None => return Err("the recording produced no combat data".to_string()),
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

    if show_tickets {
        print_tickets(&sink.tickets.lock().unwrap(), show_names);
    }

    print_events(&sink, &meter);
    Ok(())
}

fn print_own_character(meter: &DpsMeter) {
    match meter.main_actor_name() {
        Some(name) => println!("own character named: {}", !name.is_empty()),
        None => println!("own character: not named"),
    }
}

fn print_events(sink: &CollectingSink, meter: &DpsMeter) {
    println!("events emitted:");
    for (name, count) in sink.counts.lock().unwrap().iter() {
        println!("  {name}: {count}");
    }
    let wipes = sink.wipes.lock().unwrap();
    println!("boss wipes closed: {}", wipes.len());
    for wipe in wipes.iter() {
        println!("  {wipe}");
    }
    println!("history records:");
    for record in meter.get_history() {
        let target = record.target_info.as_ref();
        println!(
            "  {} {} wiped: {} hp {:?}/{:?} damage {} players {}",
            record.id,
            target.and_then(|target| target.target_name.as_deref()).unwrap_or("(unnamed)"),
            record.wiped,
            target.and_then(|target| target.current_hp),
            target.and_then(|target| target.max_hp),
            record.total_damage,
            record.player_stats.len()
        );
    }
}

fn print_tickets(reports: &[TicketReport], show_names: bool) {
    println!("ticket events: {}", reports.len());
    for report in reports {
        let character = &report.character;
        let name = if show_names { character.name.clone() } else { format!("##({})", character.name.chars().count()) };
        let who = format!("{name} server {} key {}", character.server_id, character.key.as_deref().unwrap_or("-"));
        match &report.content {
            TicketContent::Snapshot(records) => {
                println!("  {:.3} snapshot of {} tickets for {who}", report.captured_at, records.len());
                for record in records {
                    println!(
                        "      {} count {} charge {} remaining {:?}",
                        record.id, record.count, record.charge, record.remaining_ms
                    );
                }
            }
            TicketContent::Change { record, reason, delta } => println!(
                "  {:.3} change for {who}: {} count {} charge {} remaining {:?} {reason:?} delta {delta:?}",
                report.captured_at, record.id, record.count, record.charge, record.remaining_ms
            ),
        }
    }
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
