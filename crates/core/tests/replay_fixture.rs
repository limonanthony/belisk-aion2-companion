//! Replays a real fight through the whole core -- recording reader, TCP
//! reassembly, dispatcher, framing, parsers, storage and the snapshot -- and
//! checks what the player would read off the meter. See
//! `tests/fixtures/README.md` for what the fixture is and how it was made.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meter_core::engine::meter::DpsMeter;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;
use meter_core::models::combat::CombatSnapshot;

const FIXTURE: &str = "tests/fixtures/fight-anonymised.belisk";
const FIXTURE_FRAMES_REPLAYED_AS_PACKETS: u64 = 1_685;
const TOTAL_DAMAGE: u64 = 2_068_011;
const PLAYERS_SEEN: usize = 14;
const OWN_PLACEHOLDER_NAME: &str = "P00000";
const DRAIN_POLL: Duration = Duration::from_millis(40);
const DRAIN_STABLE_POLLS: u32 = 12;
const REPLAY_TIMEOUT: Duration = Duration::from_secs(60);

struct ReplayDone(Mutex<Sender<MeterEvent>>);

impl EventSink for ReplayDone {
    fn emit(&self, event: MeterEvent) {
        if matches!(event, MeterEvent::PacketReplayFinished(_) | MeterEvent::PacketReplayFailed(_)) {
            let _ = self.0.lock().unwrap().send(event);
        }
    }
}

fn fixture_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE).to_string_lossy().into_owned()
}

/// The meter after the fixture went through it at `speed` times the recorded
/// pace (`None`: as fast as the pipeline takes it), once nothing more arrives.
fn replayed(speed: Option<f64>) -> CombatSnapshot {
    let (finished_tx, finished_rx) = channel();
    let log = std::env::temp_dir().join("belisk-replay-fixture").join("replay.log");
    let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
    let meter = DpsMeter::new(Arc::new(ReplayDone(Mutex::new(finished_tx))), logger, None);
    meter.set_fight_tracking(true);
    let mut config = meter.current_config();
    config.idle_reset_secs = 0;
    config.auto_record_unknown_server = false;
    config.hide_unknown_players = false;
    config.max_player_count = 1_000;
    meter.apply_config(config);

    meter.start_for_replay();
    match speed {
        Some(speed) => meter.replay_packet_recording_paced(fixture_path(), speed),
        None => meter.replay_packet_recording(fixture_path()),
    }
    .expect("the replay starts");
    match finished_rx.recv_timeout(REPLAY_TIMEOUT).expect("the replay finishes") {
        MeterEvent::PacketReplayFinished(packets) => assert_eq!(packets, FIXTURE_FRAMES_REPLAYED_AS_PACKETS),
        other => panic!("the replay failed: {other:?}"),
    }

    let deadline = Instant::now() + REPLAY_TIMEOUT;
    let (mut last, mut stable) = (None, 0);
    while stable < DRAIN_STABLE_POLLS {
        assert!(Instant::now() < deadline, "the pipeline never went quiet");
        std::thread::sleep(DRAIN_POLL);
        let total = meter.get_dps_snapshot(0).map(|snapshot| (snapshot.total_damage, snapshot.session.total_damage));
        if total.is_some() && total == last {
            stable += 1;
        } else {
            stable = 0;
            last = total;
        }
    }
    meter.get_dps_snapshot(0).expect("the fight is on the meter")
}

#[test]
fn a_real_fight_replays_to_the_damage_that_was_dealt_and_names_its_players() {
    let snapshot = replayed(None);

    assert_eq!(snapshot.total_damage, TOTAL_DAMAGE);
    let session = &snapshot.session;
    assert_eq!(session.players.len(), PLAYERS_SEEN);
    assert_eq!(
        session.players.iter().map(|player| player.total_damage).sum::<u64>(),
        session.total_damage,
        "the rows add up to the session's damage"
    );
    assert!(session.players.iter().all(|player| player.actor_name.starts_with('P')), "only placeholders are named");
    let own = session
        .players
        .iter()
        .find(|player| player.actor_name == OWN_PLACEHOLDER_NAME)
        .expect("the player the game named is on the board");
    assert_eq!(Some(own.actor_id), snapshot.combat_infos.main_actor_id, "and is the local player");
    assert!(own.total_damage > 0 && own.dps > 0.0);
}

#[test]
fn a_fight_lasts_as_long_as_it_was_recorded_however_fast_it_is_replayed() {
    let at_50_times = replayed(Some(50.0));
    let at_200_times = replayed(Some(200.0));

    let fight_secs = at_50_times.session.combat_time_secs;
    assert!(fight_secs > 30.0, "the recorded fight lasted a minute, not the second or so it took to replay: {fight_secs}");
    assert_eq!(at_200_times.session.combat_time_secs, fight_secs);
    assert_eq!(at_200_times.total_damage, at_50_times.total_damage);
    let rows = |snapshot: &CombatSnapshot| {
        snapshot
            .session
            .players
            .iter()
            .map(|player| (player.actor_id, player.total_damage, player.combat_time_secs, player.dps))
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(&at_200_times), rows(&at_50_times));
}
