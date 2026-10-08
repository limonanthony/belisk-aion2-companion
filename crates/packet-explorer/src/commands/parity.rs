//! Runs the real `StreamAssembler`/`StreamProcessor` over the same reassembled
//! chunks and compares its opcode census with this tool's framing. Whatever the
//! shipped meter never sees shows up as a difference.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use meter_core::capture::assembler::StreamAssembler;
use meter_core::capture::census;
use meter_core::config::DpsMeterConfig;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;
use meter_core::mobs::catalog::MobCatalog;
use meter_core::storage::data_storage::DataStorage;
use serde::Serialize;

use crate::args::Args;
use crate::capture::labels::format_opcode;
use crate::capture::{self, contains_heartbeat, Direction, RawFile};

struct DiscardEvents;

impl EventSink for DiscardEvents {
    fn emit(&self, _event: MeterEvent) {}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParityRow {
    opcode: String,
    explorer: u64,
    shipped: u64,
}

fn shipped_census(raws: &[RawFile]) -> BTreeMap<[u8; 2], u64> {
    census::reset();
    census::set_enabled(true);
    let log = std::env::temp_dir().join("packet-explorer").join("core.log");
    let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));

    for raw in raws {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = Arc::new(DataStorage::new(Arc::new(DiscardEvents), Arc::clone(&config), mobs));
        storage.set_fight_tracking(true);
        let mut assemblers: HashMap<usize, StreamAssembler> = HashMap::new();
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
            if let Some(assembler) = assemblers.get_mut(&chunk.flow) {
                assembler.process_chunk(&chunk.data, false, chunk.t);
            }
        }
    }

    let snapshot = census::snapshot();
    census::set_enabled(false);
    census::reset();
    snapshot
        .opcodes
        .into_iter()
        .filter_map(|stat| {
            let (first, second) = stat.opcode.split_once(',')?;
            let opcode = [u8::from_str_radix(first, 16).ok()?, u8::from_str_radix(second, 16).ok()?];
            Some((opcode, stat.count))
        })
        .collect()
}

pub fn run(files: &[PathBuf], args: &Args) -> Result<(), String> {
    let raws = files
        .iter()
        .map(|path| capture::read_raw(path).map_err(|error| format!("{}: {error}", path.display())))
        .collect::<Result<Vec<_>, _>>()?;
    let explorer = capture::load(files)?;

    let mut explorer_counts: BTreeMap<[u8; 2], u64> = BTreeMap::new();
    for packet in &explorer.packets {
        if let (Direction::ServerToClient, Some(opcode)) = (packet.dir, packet.opcode()) {
            *explorer_counts.entry(opcode).or_default() += 1;
        }
    }
    let shipped = shipped_census(&raws);

    let opcodes: std::collections::BTreeSet<[u8; 2]> =
        explorer_counts.keys().chain(shipped.keys()).copied().collect();
    let rows: Vec<ParityRow> = opcodes
        .into_iter()
        .map(|opcode| ParityRow {
            opcode: format_opcode(opcode),
            explorer: explorer_counts.get(&opcode).copied().unwrap_or(0),
            shipped: shipped.get(&opcode).copied().unwrap_or(0),
        })
        .filter(|row| args.flag("all") || row.explorer != row.shipped)
        .collect();

    if args.flag("json") {
        return crate::output::print_json(&rows);
    }
    let total = |pick: fn(&ParityRow) -> u64| rows.iter().map(pick).sum::<u64>();
    crate::out!(
        "{:<8} {:>9} {:>9}   (shipped = real StreamProcessor)",
        "opcode", "explorer", "shipped"
    );
    for row in &rows {
        crate::out!("{:<8} {:>9} {:>9}", row.opcode, row.explorer, row.shipped);
    }
    crate::out!(
        "{:<8} {:>9} {:>9}   (rows that differ{})",
        "total",
        total(|row| row.explorer),
        total(|row| row.shipped),
        if args.flag("all") { "; --all lists every opcode" } else { " only; --all lists every opcode" }
    );
    crate::out!("the shipped run is unpaced, so its time-based stall resync never fires; real captures can lose more");
    Ok(())
}
