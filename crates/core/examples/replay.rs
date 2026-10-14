//! Replays a recording through the full pipeline and prints a
//! summary of the final snapshot. No capture driver is loaded and no elevation
//! is needed.
//!
//!     cargo run --example replay -- <recording> [--names] [--tickets] [--pets] [--remembered <name>:<server>:<CLASS>]... [--fights on|off|from-identity|from-tickets] [--speed N] [--data-dir <dir> [--delete-first-record] [--record]]
//!
//! Other players' names are only printed with `--names`. `--tickets` lists the
//! ticket events the core emitted, with the capture time as Unix seconds.
//! `--pets` lists the pet events (collection snapshot, soul gains with their
//! source, counter gains with the mobs and pets whose loot was just picked up).
//! `--remembered <name>:<server>:<CLASS>` (repeatable) hands the meter the
//! characters earlier runs saw the player as, most recent first, as the app
//! does from its settings; without it the replay knows none.
//! `--fights off` replays with fight tracking off, as with the DPS meter
//! closed; `--fights from-identity` turns it on when the game first names the
//! player's character, and `--fights from-tickets` at the first ticket event,
//! as if the meter were opened then.
//! `--speed N` replays at N times the recorded pace, which the boss wipes need:
//! the meter closes a wiped attempt on its snapshot tick (100 ms in this tool,
//! as the core's default; the app sets 200 ms), so an
//! unpaced replay would close every wipe at the end. The boss wipes the meter
//! closed and the history it saved are listed at the end.
//! `--data-dir` gives the meter a folder for its history and personal bests (a
//! throwaway one, never the app's real data folder), stops the meter at the
//! end so the fight on it is saved and every pending write goes out, and shows
//! what the files hold; `--delete-first-record` then deletes the first history
//! record through the meter and shows the files again.
//! `--record` (with `--data-dir`) records the replayed packets again through the
//! meter's recording thread into `<dir>/recordings`, then reads that file back
//! and prints how many packets it holds and how many were dropped on the way.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use meter_core::capture::recorder::replay_recording;
use meter_core::engine::meter::DpsMeter;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::identity::KnownCharacter;
use meter_core::pets::{PetContent, PetReport};
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
    pets: Mutex<Vec<PetReport>>,
    wipes: Mutex<Vec<String>>,
    outcome: Mutex<Sender<ReplayOutcome>>,
    meter: OnceLock<Arc<DpsMeter>>,
    fights: FightMode,
    /// Total damage and named session players of the last frame sent.
    last_frame: Mutex<(u64, usize)>,
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
            MeterEvent::Pets(_) => "pets",
            MeterEvent::PacketReplayFinished(_) => "packet-replay-finished",
            MeterEvent::PacketReplayFailed(_) => "packet-replay-failed",
            MeterEvent::DpsIdleHidden(_) => "dps-idle-hidden",
            MeterEvent::BossWiped(_) => "boss-wiped",
        };
        *self.counts.lock().unwrap().entry(name).or_default() += 1;
        if let MeterEvent::DpsSnapshot(frame) = &event {
            let named = frame.session.players.iter().filter(|player| !player.actor_name.is_empty()).count();
            let mut last = self.last_frame.lock().unwrap();
            if frame.total_damage > 0 && frame.total_damage == last.0 {
                let mut counts = self.counts.lock().unwrap();
                *counts.entry("  of which sent with no new damage").or_default() += 1;
                if named > last.1 {
                    *counts.entry("  of which a name arrived with no new damage").or_default() += 1;
                }
            }
            *last = (frame.total_damage, named);
        }

        if let MeterEvent::Tickets(report) = &event {
            self.tickets.lock().unwrap().push(report.clone());
        }
        if let MeterEvent::Pets(report) = &event {
            self.pets.lock().unwrap().push(report.clone());
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

fn remembered_character(text: &str) -> Option<KnownCharacter> {
    let mut parts = text.split(':');
    let name = parts.next().filter(|name| !name.is_empty())?;
    let server_id = parts.next()?.parse().ok()?;
    let class = parts.next().filter(|class| !class.is_empty())?;
    if parts.next().is_some() {
        return None;
    }
    Some(KnownCharacter { name: name.to_string(), server_id, class: class.to_string(), key: None })
}

fn main() -> ExitCode {
    let mut path = None;
    let mut show_names = false;
    let mut show_tickets = false;
    let mut show_pets = false;
    let mut fights = FightMode::On;
    let mut speed = None;
    let mut data_dir = None;
    let mut delete_first_record = false;
    let mut record = false;
    let mut remembered = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--names" => show_names = true,
            "--tickets" => show_tickets = true,
            "--pets" => show_pets = true,
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
            "--data-dir" => match args.next() {
                Some(dir) => data_dir = Some(PathBuf::from(dir)),
                None => {
                    eprintln!("--data-dir takes a folder");
                    return ExitCode::FAILURE;
                }
            },
            "--delete-first-record" => delete_first_record = true,
            "--record" => record = true,
            "--remembered" => match args.next().as_deref().and_then(remembered_character) {
                Some(character) => remembered.push(character),
                None => {
                    eprintln!("--remembered takes <name>:<server>:<CLASS>, for example Lyvara:1302:CHANTER");
                    return ExitCode::FAILURE;
                }
            },
            _ if path.is_none() => path = Some(arg),
            _ => {
                eprintln!("unexpected argument: {arg}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: replay <recording> [--names] [--tickets] [--pets] [--remembered <name>:<server>:<CLASS>]... [--fights on|off|from-identity|from-tickets] [--speed N] [--data-dir <dir> [--delete-first-record] [--record]]");
        return ExitCode::FAILURE;
    };
    if (delete_first_record || record) && data_dir.is_none() {
        eprintln!("--delete-first-record and --record need --data-dir");
        return ExitCode::FAILURE;
    }

    match run(path, show_names, show_tickets, show_pets, fights, speed, data_dir, delete_first_record, record, remembered) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("replay failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    path: String,
    show_names: bool,
    show_tickets: bool,
    show_pets: bool,
    fights: FightMode,
    speed: Option<f64>,
    data_dir: Option<PathBuf>,
    delete_first_record: bool,
    record: bool,
    remembered: Vec<KnownCharacter>,
) -> Result<(), String> {
    let (outcome_tx, outcome_rx) = mpsc::channel();
    let sink = Arc::new(CollectingSink {
        counts: Mutex::new(BTreeMap::new()),
        tickets: Mutex::new(Vec::new()),
        pets: Mutex::new(Vec::new()),
        wipes: Mutex::new(Vec::new()),
        outcome: Mutex::new(outcome_tx),
        meter: OnceLock::new(),
        fights,
        last_frame: Mutex::new((0, 0)),
    });
    let log_path: PathBuf = std::env::temp_dir().join("belisk").join("replay-example.log");
    let logger = Arc::new(AppLogger::new(log_path, Box::new(|_| {})));
    let meter = Arc::new(DpsMeter::new(sink.clone(), Arc::clone(&logger), data_dir.clone()));
    let _ = sink.meter.set(Arc::clone(&meter));
    meter.set_fight_tracking(fights == FightMode::On);

    let mut config = meter.current_config();
    config.idle_reset_secs = 0;
    config.auto_record_unknown_server = false;
    config.hide_unknown_players = false;
    config.max_player_count = 1_000;
    meter.apply_config(config);
    meter.set_known_characters(remembered);

    meter.start_for_replay();
    if record {
        meter.start_packet_recording()?;
    }
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
            if show_pets {
                print_pets(&sink.pets.lock().unwrap(), show_names);
            }
            print_events(&sink, &meter);
            return Ok(());
        }
        None => return Err("the recording produced no combat data".to_string()),
    };

    println!("packets replayed: {packets}");

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
    if show_pets {
        print_pets(&sink.pets.lock().unwrap(), show_names);
    }

    print_events(&sink, &meter);
    if record {
        show_recording(&meter, packets)?;
    }
    if let Some(dir) = data_dir {
        save_and_show_files(&meter, &logger, &dir, delete_first_record);
    }
    Ok(())
}

/// Closes the recording the replay made (waiting for the recording thread to
/// catch up) and reads it back.
fn show_recording(meter: &DpsMeter, replayed: u64) -> Result<(), String> {
    let status = meter
        .stop_packet_recording()?
        .ok_or("no recording was running")?;
    let path = status.path.ok_or("the recording has no path")?;
    let read_back = replay_recording(Path::new(&path), |_| true)?;
    let drops = meter.packet_drops();
    println!(
        "recording {path}: {} packets written, {read_back} read back, {replayed} replayed; dropped before the dispatcher {}, left out of recordings {}",
        status.packets, drops.capture_queue, drops.recording_queue
    );
    Ok(())
}

/// Stops the meter, which saves the fight still on it and writes every
/// pending file, then shows what is on disk under `dir`; with `delete_first`,
/// deletes the first history record through the meter and shows the files again.
fn save_and_show_files(meter: &DpsMeter, logger: &Arc<AppLogger>, dir: &std::path::Path, delete_first: bool) {
    meter.stop_dps_meter();
    show_files(logger, dir, "after stop");
    if delete_first {
        match meter.get_history().first() {
            Some(record) => {
                println!("deleting history record {} through the meter", record.id);
                meter.delete_history_record(&record.id);
                meter.save_now();
                show_files(logger, dir, "after the delete");
            }
            None => println!("no history record to delete"),
        }
    }
}

fn show_files(logger: &Arc<AppLogger>, dir: &std::path::Path, when: &str) {
    let size = |path: PathBuf| std::fs::metadata(path).map_or_else(|_| "missing".to_string(), |meta| format!("{} bytes", meta.len()));
    let history = meter_core::history::HistoryStore::open(Some(dir.to_path_buf()), Arc::clone(logger));
    println!(
        "files {when}: combat_history.bin {}, {} records read back; personal_bests.json {}",
        size(dir.join("history").join("combat_history.bin")),
        history.get_records().len(),
        size(dir.join("history").join("personal_bests.json")),
    );
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

/// The app's pet catalogue, read here only to name the pet a looted mob gives.
fn pet_of_mob(mob_code: u32) -> Option<u32> {
    let catalogue: serde_json::Value =
        serde_json::from_str(include_str!("../../app/assets/game/data/pets.json")).expect("pets.json is valid");
    catalogue.as_object()?.iter().find_map(|(pet_id, pet)| {
        let drops_it = pet["mobs"].as_array()?.iter().any(|mob| mob.as_u64() == Some(u64::from(mob_code)));
        if drops_it { pet_id.parse().ok() } else { None }
    })
}

fn print_pets(reports: &[PetReport], show_names: bool) {
    println!("pet events: {}", reports.len());
    for report in reports {
        let character = &report.character;
        let name = if show_names { character.name.clone() } else { format!("##({})", character.name.chars().count()) };
        let who = format!("{name} server {}", character.server_id);
        let at = report.captured_at;
        match &report.content {
            PetContent::Snapshot(snapshot) => {
                println!(
                    "  {at:.3} snapshot for {who}: {} levelled pets, {} progress entries, counters {:?}",
                    snapshot.levels.len(),
                    snapshot.progress.len(),
                    snapshot.counters.iter().map(|counter| (counter.kind, counter.value)).collect::<Vec<_>>()
                );
            }
            PetContent::SoulGains { gains, source } => {
                let gains: Vec<_> = gains.iter().map(|gain| format!("pet {} +{}", gain.pet_id, gain.amount)).collect();
                println!("  {at:.3} soul gains for {who} [{source:?}]: {}", gains.join(", "));
            }
            PetContent::Unlock(unlock) => println!("  {at:.3} unlock for {who}: pet {}", unlock.pet_id),
            PetContent::CounterGains { gains, looted_mobs } => {
                let gains: Vec<_> = gains.iter().map(|gain| format!("kind {} +{}", gain.kind, gain.amount)).collect();
                let mobs: Vec<_> = looted_mobs
                    .iter()
                    .map(|mob| match pet_of_mob(*mob) {
                        Some(pet_id) => format!("mob {mob} (pet {pet_id})"),
                        None => format!("mob {mob} (no pet)"),
                    })
                    .collect();
                println!("  {at:.3} counter gains for {who}: {}; looted {}", gains.join(", "), mobs.join(", "));
            }
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
