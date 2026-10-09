//! Replays the recordings through the shipped `StreamAssembler` and prints
//! each boss entity's HP as the meter's storage holds it: where it appears,
//! where it drops by a tenth of its maximum, where it is back at full health
//! after a loss, where it reaches zero, and every wipe the core's detector
//! would close (the attempt is then taken off, as the meter does).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use meter_core::capture::assembler::StreamAssembler;
use meter_core::config::DpsMeterConfig;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;
use meter_core::mobs::catalog::MobCatalog;
use meter_core::storage::data_storage::DataStorage;

use crate::args::Args;
use crate::capture::{self, contains_heartbeat};
use crate::clock;

const DROP_STEP_TENTHS: u64 = 1;

struct DiscardEvents;

impl EventSink for DiscardEvents {
    fn emit(&self, _event: MeterEvent) {}
}

struct BossHp {
    current: u64,
    max: u64,
    lowest_tenth: u64,
}

fn tenths_left(current: u64, max: u64) -> u64 {
    if max == 0 {
        0
    } else {
        current * 10 / max
    }
}

fn percent(current: u64, max: u64) -> f64 {
    if max == 0 {
        0.0
    } else {
        current as f64 * 100.0 / max as f64
    }
}

pub fn run(files: &[PathBuf], _args: &Args) -> Result<(), String> {
    let log = std::env::temp_dir().join("packet-explorer").join("core.log");
    let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));

    for path in files {
        let raw = capture::read_raw(path).map_err(|error| format!("{}: {error}", path.display()))?;
        crate::out!("== {}", path.display());
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = Arc::new(DataStorage::new(Arc::new(DiscardEvents), Arc::clone(&config), mobs));
        storage.set_fight_tracking(true);
        let mut assemblers: HashMap<usize, StreamAssembler> = HashMap::new();
        let mut seen: HashMap<u32, BossHp> = HashMap::new();

        for chunk in &raw.chunks {
            if contains_heartbeat(&chunk.data) {
                assemblers.entry(chunk.flow).or_insert_with(|| {
                    StreamAssembler::new(
                        Arc::clone(&storage),
                        Arc::clone(&logger),
                        format!("flow{}", chunk.flow),
                        Arc::clone(&config),
                    )
                });
            }
            let Some(assembler) = assemblers.get_mut(&chunk.flow) else {
                continue;
            };
            assembler.process_chunk(&chunk.data, true, chunk.t);

            for (entity, (current, max)) in storage.mob_id_hp_snapshot() {
                if seen.get(&entity).is_some_and(|known| known.current == current && known.max == max) {
                    continue;
                }
                let Some(mob_code) = storage.get_mob_code(entity) else {
                    continue;
                };
                if !storage.is_boss(mob_code) {
                    continue;
                }
                let name = storage.mob_name(mob_code).unwrap_or_else(|| "?".to_string());
                let label = format!(
                    "{} entity {entity} {name} ({mob_code}) hp {current}/{max} ({:.1}%)",
                    clock::format_full(chunk.t),
                    percent(current, max)
                );
                let tenth = tenths_left(current, max);
                match seen.get_mut(&entity) {
                    None => {
                        crate::out!("{label}  appears");
                        seen.insert(entity, BossHp { current, max, lowest_tenth: tenth });
                    }
                    Some(known) => {
                        if current > known.current && current >= max {
                            crate::out!("{label}  back at full from {} ({:.1}%)", known.current, percent(known.current, known.max));
                        } else if current == 0 {
                            crate::out!("{label}  dead");
                        } else if tenth + DROP_STEP_TENTHS <= known.lowest_tenth {
                            crate::out!("{label}  falls");
                        }
                        known.lowest_tenth = if current > known.current { tenth } else { known.lowest_tenth.min(tenth) };
                        known.current = current;
                        known.max = max;
                    }
                }
            }

            for wipe in storage.take_boss_wipes() {
                crate::out!(
                    "{} entity {} WIPE ({:?}) hp left {}/{} ({:.1}%)",
                    clock::format_full(chunk.t),
                    wipe.target_id,
                    wipe.cause,
                    wipe.hp_left,
                    wipe.max_hp,
                    percent(wipe.hp_left, wipe.max_hp)
                );
                storage.forget_target(wipe.target_id);
            }
        }
    }
    Ok(())
}
