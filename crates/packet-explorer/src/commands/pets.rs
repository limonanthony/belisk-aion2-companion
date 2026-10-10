use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use meter_core::capture::parser::loot_record::{
    decode_loot_pickup, decode_loot_spawns, LOOT_PICKUP_OPCODE, LOOT_SPAWN_OPCODE, PURCHASE_OPCODE,
};
use meter_core::capture::parser::pet_record::{
    decode_counter_gains, decode_snapshot, decode_soul_gains, decode_unlock, CounterGain, PetCounter, PetLevel,
    PetProgress, SoulGain, COUNTER_GAINS_OPCODE, SNAPSHOT_OPCODE, SOUL_GAINS_OPCODE, UNLOCK_OPCODE,
};
use meter_core::capture::parser::spawn_record::spawned_mob;
use meter_core::pet_sources::{PickedUpLoot, SoulSource, SoulSourceTracker};
use meter_core::pets::{PetCollection, PetState};
use serde::Serialize;

use crate::args::Args;
use crate::capture::{Capture, Direction, Packet};
use crate::clock;

const SPAWN_OPCODE: [u8; 2] = [0x41, 0x36];

/// The app's pet catalogue, read here only to name the pet a looted mob gives.
static PET_OF_MOB: LazyLock<HashMap<u32, u32>> = LazyLock::new(|| {
    let catalogue: serde_json::Value =
        serde_json::from_str(include_str!("../../../app/assets/game/data/pets.json")).expect("pets.json is valid");
    let mut pet_of_mob = HashMap::new();
    for (pet_id, pet) in catalogue.as_object().into_iter().flatten() {
        let Ok(pet_id) = pet_id.parse::<u32>() else { continue };
        for mob in pet["mobs"].as_array().into_iter().flatten().filter_map(serde_json::Value::as_u64) {
            pet_of_mob.insert(mob as u32, pet_id);
        }
    }
    pet_of_mob
});

/// A loot object picked up just before a message, with what was known of it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LootRow {
    dropper_entity: Option<u32>,
    mob_code: Option<u32>,
    pet_id: Option<u32>,
    item_id: Option<u32>,
    milliseconds_before: u32,
}

fn loot_rows(pickups: &[PickedUpLoot], message_at: f64) -> Vec<LootRow> {
    pickups
        .iter()
        .map(|pickup| LootRow {
            dropper_entity: pickup.spawn.map(|spawn| spawn.dropper_entity),
            mob_code: pickup.mob_code,
            pet_id: pickup.mob_code.and_then(|mob| PET_OF_MOB.get(&mob).copied()),
            item_id: pickup.spawn.map(|spawn| spawn.item_id),
            milliseconds_before: ((message_at - pickup.at_seconds) * 1000.0).round() as u32,
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PetRow {
    pet_id: u32,
    level: u32,
    progress: Option<u32>,
}

/// A gain and the pet as it is after it; `None` when no snapshot came before,
/// so the starting point is unknown.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GainRow {
    #[serde(flatten)]
    gain: SoulGain,
    after: Option<PetState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum Event {
    Snapshot {
        packet: usize,
        time: String,
        levelled_pets: usize,
        progress_entries: usize,
        counters: Vec<PetCounter>,
        rows: Vec<PetRow>,
    },
    SoulGains {
        packet: usize,
        time: String,
        source: SoulSource,
        gains: Vec<GainRow>,
        picked_up: Vec<LootRow>,
    },
    Unlock {
        packet: usize,
        time: String,
        pet_id: u32,
        before: Option<PetState>,
        after: Option<PetState>,
    },
    CounterGains {
        packet: usize,
        time: String,
        gains: Vec<CounterGain>,
        looted_mobs: Vec<u32>,
        picked_up: Vec<LootRow>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Rejected {
    packet: usize,
    time: String,
    opcode: String,
    len: usize,
    why: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    events: Vec<Event>,
    rejected: Vec<Rejected>,
}

fn rows_of(levels: &[PetLevel], progress: &[PetProgress]) -> Vec<PetRow> {
    let mut rows: BTreeMap<u32, PetRow> = BTreeMap::new();
    for entry in levels {
        rows.insert(entry.pet_id, PetRow { pet_id: entry.pet_id, level: entry.level, progress: None });
    }
    for entry in progress {
        rows.entry(entry.pet_id)
            .or_insert(PetRow { pet_id: entry.pet_id, level: 0, progress: None })
            .progress = Some(entry.progress);
    }
    rows.into_values().collect()
}

fn rejection(packet: &Packet, time: String, opcode: [u8; 2], why: String) -> Rejected {
    Rejected {
        packet: packet.id,
        time,
        opcode: format!("{:02x}{:02x}", opcode[0], opcode[1]),
        len: packet.data.len(),
        why,
    }
}

/// Walks the recordings in the order given, applying each message to the
/// collection the way the app does, from the first snapshot on.
fn analyze(packets: &[Packet], only_pets: &[u32]) -> Report {
    let wants = |pet_id: u32| only_pets.is_empty() || only_pets.contains(&pet_id);
    let mut report = Report { events: Vec::new(), rejected: Vec::new() };
    let mut collection: Option<PetCollection> = None;
    let mut sources = SoulSourceTracker::default();
    let mut mobs: HashMap<u32, u32> = HashMap::new();
    for packet in packets.iter().filter(|packet| packet.dir == Direction::ServerToClient) {
        let Some(opcode) = packet.opcode() else { continue };
        let time = clock::format_full(packet.t);
        match opcode {
            SNAPSHOT_OPCODE => match decode_snapshot(&packet.data) {
                Ok(snapshot) => {
                    let rows = rows_of(&snapshot.levels, &snapshot.progress)
                        .into_iter()
                        .filter(|row| wants(row.pet_id))
                        .collect();
                    let mut listed = PetCollection::default();
                    listed.apply_snapshot(&snapshot, packet.t);
                    collection = Some(listed);
                    report.events.push(Event::Snapshot {
                        packet: packet.id,
                        time,
                        levelled_pets: snapshot.levels.len(),
                        progress_entries: snapshot.progress.len(),
                        counters: snapshot.counters,
                        rows,
                    });
                }
                Err(why) => report.rejected.push(rejection(packet, time, opcode, why)),
            },
            SOUL_GAINS_OPCODE => match decode_soul_gains(&packet.data) {
                Ok(gains) => {
                    let cause = sources.soul_gain_cause(packet.t);
                    let rows: Vec<_> = gains
                        .into_iter()
                        .map(|gain| GainRow { gain, after: collection.as_mut().map(|known| known.apply_soul_gain(&gain)) })
                        .filter(|row| wants(row.gain.pet_id))
                        .collect();
                    if !rows.is_empty() {
                        report.events.push(Event::SoulGains {
                            packet: packet.id,
                            time,
                            source: cause.source,
                            gains: rows,
                            picked_up: loot_rows(&cause.pickups, packet.t),
                        });
                    }
                }
                Err(why) => report.rejected.push(rejection(packet, time, opcode, why)),
            },
            UNLOCK_OPCODE => match decode_unlock(&packet.data) {
                Ok(unlock) => {
                    let before = collection.as_ref().map(|known| known.pet(unlock.pet_id));
                    let after = collection.as_mut().map(|known| known.apply_unlock(&unlock));
                    if wants(unlock.pet_id) {
                        report.events.push(Event::Unlock { packet: packet.id, time, pet_id: unlock.pet_id, before, after });
                    }
                }
                Err(why) => report.rejected.push(rejection(packet, time, opcode, why)),
            },
            COUNTER_GAINS_OPCODE => match decode_counter_gains(&packet.data) {
                Ok(gains) => {
                    let cause = sources.counter_gain_cause(packet.t);
                    if only_pets.is_empty() {
                        report.events.push(Event::CounterGains {
                            packet: packet.id,
                            time,
                            gains,
                            looted_mobs: cause.looted_mobs,
                            picked_up: loot_rows(&cause.pickups, packet.t),
                        });
                    }
                }
                Err(why) => report.rejected.push(rejection(packet, time, opcode, why)),
            },
            LOOT_SPAWN_OPCODE => {
                for spawn in decode_loot_spawns(&packet.data).unwrap_or_default() {
                    sources.loot_spawned(packet.t, spawn);
                }
            }
            LOOT_PICKUP_OPCODE => {
                if let Ok(pickup) = decode_loot_pickup(&packet.data) {
                    sources.picked_up(packet.t, pickup, |entity| mobs.get(&entity).copied());
                }
            }
            PURCHASE_OPCODE => sources.purchased(packet.t),
            SPAWN_OPCODE => {
                if let Some((entity, mob_code)) = spawned_mob(&packet.data) {
                    mobs.insert(entity, mob_code);
                }
            }
            _ => {}
        }
    }
    report
}

fn pet_text(state: Option<PetState>) -> String {
    match state {
        Some(state) => format!("level {}, {} souls", state.level, state.souls),
        None => "start unknown (no snapshot before it)".to_string(),
    }
}

fn source_text(source: SoulSource) -> &'static str {
    match source {
        SoulSource::Loot => "loot",
        SoulSource::BlackMarket => "black market",
        SoulSource::Other => "other",
    }
}

fn print_loot(picked_up: &[LootRow]) {
    for row in picked_up {
        let known = |value: Option<u32>| value.map_or_else(|| "?".to_string(), |value| value.to_string());
        crate::out!(
            "    picked up {} ms before: item {} from entity {}, mob {}, pet {}",
            row.milliseconds_before,
            known(row.item_id),
            known(row.dropper_entity),
            known(row.mob_code),
            known(row.pet_id)
        );
    }
}

fn print_text(report: &Report) {
    let count = |matches: fn(&Event) -> bool| report.events.iter().filter(|event| matches(event)).count();
    crate::out!(
        "{} snapshot(s) (00 90), {} soul gain message(s) (0D 90), {} unlock(s) (09 90), {} counter gain message(s) (11 90), {} rejected frame(s)",
        count(|event| matches!(event, Event::Snapshot { .. })),
        count(|event| matches!(event, Event::SoulGains { .. })),
        count(|event| matches!(event, Event::Unlock { .. })),
        count(|event| matches!(event, Event::CounterGains { .. })),
        report.rejected.len()
    );
    for event in &report.events {
        crate::out!();
        match event {
            Event::Snapshot { packet, time, levelled_pets, progress_entries, counters, rows } => {
                crate::out!(
                    "SNAPSHOT #{packet} {time}  {levelled_pets} levelled pet(s), {progress_entries} progress entr(ies)"
                );
                let counters: Vec<_> = counters.iter().map(|counter| format!("{}={}", counter.kind, counter.value)).collect();
                crate::out!("  counters {}", counters.join(" "));
                crate::out!("  {:>6} {:>5} {:>8}", "pet", "level", "progress");
                for row in rows {
                    let progress = row.progress.map_or_else(|| "-".to_string(), |progress| progress.to_string());
                    crate::out!("  {:>6} {:>5} {:>8}", row.pet_id, row.level, progress);
                }
            }
            Event::SoulGains { packet, time, source, gains, picked_up } => {
                let gains: Vec<_> = gains
                    .iter()
                    .map(|row| format!("pet {} +{} -> {}", row.gain.pet_id, row.gain.amount, pet_text(row.after)))
                    .collect();
                crate::out!("SOUL GAIN #{packet} {time}  [{}]  {}", source_text(*source), gains.join("; "));
                print_loot(picked_up);
            }
            Event::Unlock { packet, time, pet_id, before, after } => {
                crate::out!("UNLOCK #{packet} {time}  pet {pet_id}: {} -> {}", pet_text(*before), pet_text(*after));
            }
            Event::CounterGains { packet, time, gains, looted_mobs, picked_up } => {
                let gains: Vec<_> = gains.iter().map(|gain| format!("kind {} +{}", gain.kind, gain.amount)).collect();
                let mobs: Vec<_> = looted_mobs.iter().map(u32::to_string).collect();
                crate::out!("COUNTER GAIN #{packet} {time}  {}  [looted mobs: {}]", gains.join(", "), mobs.join(", "));
                print_loot(picked_up);
            }
        }
    }
    for rejected in &report.rejected {
        crate::out!();
        crate::out!(
            "REJECTED #{} {} {} len {}: {}",
            rejected.packet,
            rejected.time,
            rejected.opcode,
            rejected.len,
            rejected.why
        );
    }
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let only_pets = args
        .values("pet")
        .iter()
        .flat_map(|list| list.split(','))
        .map(|text| text.trim().parse::<u32>().map_err(|_| format!("--pet: cannot read '{text}' as a pet id")))
        .collect::<Result<Vec<_>, _>>()?;
    let report = analyze(&capture.packets, &only_pets);
    if args.flag("json") {
        return crate::output::print_json(&report);
    }
    print_text(&report);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(id: usize, data: Vec<u8>) -> Packet {
        Packet { id, file: 0, t: 1_000.0 + id as f64, dir: Direction::ServerToClient, in_bundle: false, data }
    }

    fn soul_gains_frame(gains: &[(u32, u32)]) -> Vec<u8> {
        let mut bytes = SOUL_GAINS_OPCODE.to_vec();
        bytes.push(gains.len() as u8);
        for (pet_id, amount) in gains {
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(amount.to_le_bytes());
        }
        bytes
    }

    fn snapshot_frame(levels: &[(u32, u32)], progress: &[(u32, u32)]) -> Vec<u8> {
        let mut bytes = SNAPSHOT_OPCODE.to_vec();
        bytes.extend(0u32.to_le_bytes());
        bytes.push(levels.len() as u8);
        for (pet_id, level) in levels {
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(level.to_le_bytes());
        }
        bytes.push(progress.len() as u8);
        for (pet_id, souls) in progress {
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(souls.to_le_bytes());
        }
        bytes.push(0);
        bytes
    }

    fn unlock_frame(pet_id: u32) -> Vec<u8> {
        let mut bytes = UNLOCK_OPCODE.to_vec();
        bytes.extend([0xAB, 0x35]);
        bytes.extend(pet_id.to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.push(3);
        bytes.extend(6u32.to_le_bytes());
        bytes.extend(10_850u32.to_le_bytes());
        bytes.extend(0u32.to_le_bytes());
        bytes
    }

    #[test]
    fn a_snapshot_row_joins_the_level_and_the_progress_of_a_pet_and_a_pet_without_a_level_is_level_zero() {
        let packets = vec![packet(1, snapshot_frame(&[(1086, 2)], &[(1086, 27), (1119, 4)]))];

        let report = analyze(&packets, &[]);

        let Event::Snapshot { rows, levelled_pets, progress_entries, .. } = &report.events[0] else { panic!("snapshot") };
        assert_eq!((*levelled_pets, *progress_entries), (1, 2));
        assert_eq!(
            rows.iter().map(|row| (row.pet_id, row.level, row.progress)).collect::<Vec<_>>(),
            vec![(1086, 2, Some(27)), (1119, 0, Some(4))]
        );
    }

    #[test]
    fn the_pet_filter_keeps_only_that_pets_rows_and_gains() {
        let packets = vec![
            packet(1, snapshot_frame(&[(1086, 2), (1214, 3)], &[])),
            packet(2, soul_gains_frame(&[(1086, 1), (1119, 1)])),
        ];

        let report = analyze(&packets, &[1086]);

        let Event::Snapshot { rows, .. } = &report.events[0] else { panic!("snapshot") };
        assert_eq!(rows.len(), 1);
        let Event::SoulGains { gains, .. } = &report.events[1] else { panic!("gains") };
        assert_eq!(gains.iter().map(|row| row.gain).collect::<Vec<_>>(), vec![SoulGain { pet_id: 1086, amount: 1 }]);
    }

    #[test]
    fn a_malformed_frame_is_listed_as_rejected() {
        let packets = vec![packet(1, vec![0x0D, 0x90, 0x02, 1, 2])];

        let report = analyze(&packets, &[]);

        assert!(report.events.is_empty());
        assert_eq!(report.rejected.len(), 1);
    }

    #[test]
    fn a_gain_shows_the_level_and_souls_after_it_when_a_snapshot_came_before() {
        let packets = vec![
            packet(1, snapshot_frame(&[(1012, 1)], &[(1012, 21), (1108, 4)])),
            packet(2, soul_gains_frame(&[(1012, 5), (1108, 1)])),
            packet(3, soul_gains_frame(&[(1012, 1)])),
        ];

        let report = analyze(&packets, &[]);

        let after = |event: &Event| match event {
            Event::SoulGains { gains, .. } => gains.iter().map(|row| row.after).collect::<Vec<_>>(),
            _ => panic!("gains"),
        };
        let state = |level, souls| Some(PetState { level, souls });
        assert_eq!(after(&report.events[1]), vec![state(2, 1), state(1, 0)]);
        assert_eq!(after(&report.events[2]), vec![state(2, 2)], "each gain builds on the one before");
    }

    #[test]
    fn without_a_snapshot_before_it_the_starting_point_is_unknown() {
        let packets = vec![
            packet(1, soul_gains_frame(&[(1182, 5)])),
            packet(2, unlock_frame(1134)),
            packet(3, snapshot_frame(&[(1182, 1)], &[])),
            packet(4, soul_gains_frame(&[(1182, 1)])),
        ];

        let report = analyze(&packets, &[]);

        let Event::SoulGains { gains, .. } = &report.events[0] else { panic!("gains") };
        assert_eq!(gains[0].after, None);
        let Event::Unlock { before, after, .. } = &report.events[1] else { panic!("unlock") };
        assert_eq!((*before, *after), (None, None));
        let Event::SoulGains { gains, .. } = &report.events[3] else { panic!("gains") };
        assert_eq!(gains[0].after, Some(PetState { level: 1, souls: 1 }));
    }

    #[test]
    fn an_unlock_lists_the_pet_before_and_after_and_a_new_snapshot_starts_over() {
        let packets = vec![
            packet(1, snapshot_frame(&[], &[(1192, 3)])),
            packet(2, unlock_frame(1192)),
            packet(3, snapshot_frame(&[], &[])),
            packet(4, unlock_frame(1192)),
        ];

        let report = analyze(&packets, &[]);

        let Event::Unlock { pet_id, before, after, .. } = &report.events[1] else { panic!("unlock") };
        assert_eq!(*pet_id, 1192);
        assert_eq!(*before, Some(PetState { level: 0, souls: 3 }));
        assert_eq!(*after, Some(PetState { level: 1, souls: 3 }));
        let Event::Unlock { before, after, .. } = &report.events[3] else { panic!("unlock") };
        assert_eq!((*before, *after), (Some(PetState::default()), Some(PetState { level: 1, souls: 0 })));
    }

    fn packet_at(id: usize, t: f64, data: Vec<u8>) -> Packet {
        Packet { t, ..packet(id, data) }
    }

    fn loot_spawn_frame(dropper_entity: u32, object_id: u32, serial: u32) -> Vec<u8> {
        let mut bytes = LOOT_SPAWN_OPCODE.to_vec();
        bytes.push(1);
        bytes.extend(dropper_entity.to_le_bytes());
        bytes.extend([0u8; 24]);
        bytes.extend(object_id.to_le_bytes());
        bytes.extend(serial.to_le_bytes());
        bytes.extend([0u8; 9]);
        bytes.extend(238_906u32.to_le_bytes());
        bytes.extend([0u8; 14]);
        bytes.push(1);
        bytes
    }

    fn pickup_frame(object_id: u32, serial: u32) -> Vec<u8> {
        let mut bytes = LOOT_PICKUP_OPCODE.to_vec();
        bytes.extend([0, 0]);
        bytes.extend(object_id.to_le_bytes());
        bytes.extend(serial.to_le_bytes());
        bytes.push(1);
        bytes
    }

    fn counter_gains_frame(kind: u8) -> Vec<u8> {
        let mut bytes = COUNTER_GAINS_OPCODE.to_vec();
        bytes.push(1);
        bytes.push(kind);
        bytes.extend(1u32.to_le_bytes());
        bytes.push(2);
        bytes
    }

    #[test]
    fn a_soul_gain_is_listed_with_its_source_and_the_loot_picked_up_before_it() {
        let packets = vec![
            packet_at(1, 100.0, loot_spawn_frame(48_710, 0x580c, 7)),
            packet_at(2, 100.5, pickup_frame(0x580c, 7)),
            packet_at(3, 100.55, soul_gains_frame(&[(1086, 1)])),
            packet_at(4, 200.0, PURCHASE_OPCODE.to_vec()),
            packet_at(5, 200.05, soul_gains_frame(&[(1182, 5)])),
            packet_at(6, 300.0, soul_gains_frame(&[(1091, 1)])),
        ];

        let report = analyze(&packets, &[]);

        let sources: Vec<_> = report
            .events
            .iter()
            .map(|event| match event {
                Event::SoulGains { source, picked_up, .. } => (*source, picked_up.len()),
                _ => panic!("soul gains"),
            })
            .collect();
        assert_eq!(sources, vec![(SoulSource::Loot, 1), (SoulSource::BlackMarket, 0), (SoulSource::Other, 0)]);
        let Event::SoulGains { picked_up, .. } = &report.events[0] else { panic!("soul gains") };
        assert_eq!((picked_up[0].dropper_entity, picked_up[0].item_id, picked_up[0].milliseconds_before), (Some(48_710), Some(238_906), 50));
    }

    #[test]
    fn a_pet_filter_does_not_stop_a_gain_from_claiming_its_loot() {
        let packets = vec![
            packet_at(1, 100.0, loot_spawn_frame(48_710, 1, 1)),
            packet_at(2, 100.5, pickup_frame(1, 1)),
            packet_at(3, 100.55, soul_gains_frame(&[(1086, 1)])),
            packet_at(4, 100.6, soul_gains_frame(&[(1119, 1)])),
        ];

        let report = analyze(&packets, &[1119]);

        let Event::SoulGains { source, .. } = &report.events[0] else { panic!("soul gains") };
        assert_eq!(*source, SoulSource::Other, "the first gain took the pickup");
    }

    #[test]
    fn a_counter_gain_lists_the_loot_picked_up_before_it_and_names_no_mob_the_world_never_spawned() {
        let packets = vec![
            packet_at(1, 100.0, loot_spawn_frame(48_710, 1, 1)),
            packet_at(2, 100.5, pickup_frame(1, 1)),
            packet_at(3, 100.55, counter_gains_frame(4)),
            packet_at(4, 200.0, counter_gains_frame(6)),
        ];

        let report = analyze(&packets, &[]);

        let Event::CounterGains { looted_mobs, picked_up, .. } = &report.events[0] else { panic!("counter gains") };
        assert!(looted_mobs.is_empty());
        assert_eq!(picked_up.len(), 1);
        let Event::CounterGains { picked_up, .. } = &report.events[1] else { panic!("counter gains") };
        assert!(picked_up.is_empty());
    }
}
