//! Builds the replay fixture for the core's tests from a real recording, with
//! every player name replaced.
//!
//!     packet-explorer fixture <recording> --out <file.belisk>
//!
//! Only the frames the meter's fight tracking reads are kept (see `KEPT`), each
//! written as one plain frame (bundles unwrapped) in its own packet on a
//! synthetic connection, with the capture times moved to a fixed start. The
//! names are found the way the meter finds them: the recording runs through
//! the shipped `StreamAssembler`, and every name it stored for an actor or a
//! killer is replaced, wherever its bytes occur, by a placeholder of the same
//! byte length. In the frames that carry text (`TEXT_FRAMES`) every other text
//! run is filled with `x` too, so guild names and the like go as well.
//! Afterwards the tool replays the result and refuses to write unless the
//! actors' classes, servers, combat power and damage are unchanged and none of
//! the original names occurs anywhere in the output.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use meter_core::capture::assembler::StreamAssembler;
use meter_core::capture::capturer::CapturedPacket;
use meter_core::capture::recorder::PacketRecorder;
use meter_core::config::DpsMeterConfig;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;
use meter_core::mobs::catalog::MobCatalog;
use meter_core::storage::data_storage::DataStorage;

use crate::args::Args;
use crate::capture::{self, Direction, FlowRole, Packet};
use crate::fields::text_runs;

const KEPT: [[u8; 2]; 13] = [
    [0x00, 0x36],
    [0x33, 0x36],
    [0x45, 0x36],
    [0x56, 0x36],
    [0x41, 0x36],
    [0x42, 0x36],
    [0x04, 0x38],
    [0x05, 0x38],
    [0x2A, 0x38],
    [0x2B, 0x38],
    [0x2C, 0x38],
    [0x00, 0x8D],
    [0x04, 0x8D],
];
const TEXT_FRAMES: [[u8; 2]; 5] = [[0x33, 0x36], [0x45, 0x36], [0x41, 0x36], [0x42, 0x36], [0x04, 0x8D]];

const SERVER_IP: [u8; 4] = [203, 0, 113, 10];
const SERVER_PORT: u16 = 13_328;
const CLIENT_IP: [u8; 4] = [192, 0, 2, 10];
const CLIENT_PORT: u16 = 50_000;
const FIRST_SEQUENCE: u32 = 1_000;
const FIXTURE_START_SECS: f64 = 1_700_000_000.0;

struct DiscardEvents;

impl EventSink for DiscardEvents {
    fn emit(&self, _event: MeterEvent) {}
}

struct Frame {
    t: f64,
    payload: Vec<u8>,
}

impl Frame {
    fn opcode(&self) -> [u8; 2] {
        [self.payload[0], self.payload[1]]
    }

    fn on_the_wire(&self) -> Vec<u8> {
        let mut bytes = varint(self.payload.len() as u32 + 4);
        bytes.extend_from_slice(&self.payload);
        bytes
    }
}

fn varint(mut value: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let low = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            bytes.push(low);
            return bytes;
        }
        bytes.push(low | 0x80);
    }
}

/// What the core makes of a stream, compared before and after the names go.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    actors: BTreeMap<u32, (String, String, u64)>,
    damage_by_actor: BTreeMap<u32, u64>,
    names: BTreeMap<u32, String>,
    killer_names: BTreeSet<String>,
}

fn observe(frames: &[Frame]) -> Outcome {
    let log = std::env::temp_dir().join("packet-explorer").join("fixture.log");
    let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
    let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
    let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
    let storage = Arc::new(DataStorage::new(Arc::new(DiscardEvents), Arc::clone(&config), mobs));
    storage.set_fight_tracking(true);
    let mut assembler = StreamAssembler::new(Arc::clone(&storage), logger, "fixture".to_string(), config);
    for frame in frames {
        assembler.process_chunk(&frame.on_the_wire(), true, frame.t);
    }

    let names = storage.actor_id_name_snapshot();
    let classes = storage.actor_id_class_snapshot();
    let servers = storage.actor_id_server_snapshot();
    let powers = storage.actor_id_combat_power_snapshot();
    let actors = names
        .keys()
        .map(|id| {
            (
                *id,
                (
                    classes.get(id).cloned().unwrap_or_default(),
                    servers.get(id).cloned().unwrap_or_default(),
                    powers.get(id).copied().unwrap_or_default(),
                ),
            )
        })
        .collect();
    let mut damage_by_actor: BTreeMap<u32, u64> = BTreeMap::new();
    for by_actor in storage.get_dps_stats_snapshot().values() {
        for (actor, skills) in by_actor {
            *damage_by_actor.entry(*actor).or_default() += skills.values().map(|skill| skill.total_damage).sum::<u64>();
        }
    }
    let killer_names = storage
        .combat_events(f64::MIN, f64::MAX)
        .deaths
        .into_iter()
        .filter_map(|death| death.killer.name)
        .collect();
    Outcome { actors, damage_by_actor, names: names.into_iter().collect(), killer_names }
}

/// One placeholder per name, as long as the name in bytes: `P` and then the
/// number of the name in base 36, so each is a valid, distinct nickname.
#[derive(Default)]
struct Placeholders {
    assigned: HashMap<Vec<u8>, Vec<u8>>,
    next_of_length: HashMap<usize, u64>,
}

impl Placeholders {
    fn for_name(&mut self, name: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(placeholder) = self.assigned.get(name) {
            return Ok(placeholder.clone());
        }
        let length = name.len();
        let number = self.next_of_length.entry(length).or_default();
        let digits = length.saturating_sub(1);
        let mut placeholder = vec![b'P'];
        let mut rest = vec![b'0'; digits];
        let mut remaining = *number;
        for slot in rest.iter_mut().rev() {
            *slot = b"0123456789abcdefghijklmnopqrstuvwxyz"[(remaining % 36) as usize];
            remaining /= 36;
        }
        if remaining != 0 || length < 2 {
            return Err(format!("no placeholder left for a name of {length} bytes"));
        }
        placeholder.extend(rest);
        *number += 1;
        self.assigned.insert(name.to_vec(), placeholder.clone());
        Ok(placeholder)
    }
}

fn replace_all(payload: &mut [u8], needle: &[u8], replacement: &[u8], spans: &mut Vec<(usize, usize)>) -> usize {
    let mut count = 0;
    let mut start = 0;
    while start + needle.len() <= payload.len() {
        if &payload[start..start + needle.len()] == needle {
            payload[start..start + needle.len()].copy_from_slice(replacement);
            spans.push((start, start + needle.len()));
            start += needle.len();
            count += 1;
        } else {
            start += 1;
        }
    }
    count
}

fn fill_text_runs(payload: &mut [u8], keep: &[(usize, usize)]) -> usize {
    let mut filled = 0;
    for run in text_runs(payload) {
        let mut index = run.start;
        while index < run.end {
            let width = match payload[index] {
                0xC2..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF4 => 4,
                _ => 1,
            };
            let kept = keep.iter().any(|(from, to)| index >= *from && index < *to);
            let is_letter = width > 1 || payload[index].is_ascii_alphabetic();
            if !kept && is_letter {
                payload[index..index + width].fill(b'x');
                filled += 1;
            }
            index += width;
        }
    }
    filled
}

fn anonymise(frames: &[Frame], names: &BTreeSet<String>) -> Result<(Vec<Frame>, usize, usize), String> {
    let mut by_length: Vec<&String> = names.iter().collect();
    by_length.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let mut placeholders = Placeholders::default();
    let (mut replaced, mut filled) = (0, 0);
    let mut output = Vec::with_capacity(frames.len());
    for frame in frames {
        let mut payload = frame.payload.clone();
        if TEXT_FRAMES.contains(&frame.opcode()) {
            let mut spans = Vec::new();
            for name in &by_length {
                let placeholder = placeholders.for_name(name.as_bytes())?;
                replaced += replace_all(&mut payload, name.as_bytes(), &placeholder, &mut spans);
            }
            filled += fill_text_runs(&mut payload, &spans);
        }
        output.push(Frame { t: frame.t, payload });
    }
    Ok((output, replaced, filled))
}

fn find_original_name(frames: &[Frame], names: &BTreeSet<String>) -> Option<String> {
    frames.iter().find_map(|frame| {
        names
            .iter()
            .find(|name| frame.payload.windows(name.len()).any(|window| window == name.as_bytes()))
            .cloned()
    })
}

fn write_fixture(frames: &[Frame], out: &Path) -> Result<(), String> {
    let folder = std::env::temp_dir().join(format!("packet-explorer-fixture-{}", std::process::id()));
    let recorder = PacketRecorder::new();
    let written = recorder.start(&folder)?;
    let first = frames.first().ok_or("no frames to write")?.t;
    let mut sequence = FIRST_SEQUENCE;
    let mut packets: Vec<(f64, Vec<u8>)> = Vec::new();
    for frame in frames {
        match packets.last_mut() {
            Some((t, data)) if *t == frame.t => data.extend(frame.on_the_wire()),
            _ => packets.push((frame.t, frame.on_the_wire())),
        }
    }
    for (t, data) in packets {
        let length = data.len() as u32;
        recorder.record(&CapturedPacket {
            src_ip: SERVER_IP,
            src_port: SERVER_PORT,
            dst_ip: CLIENT_IP,
            dst_port: CLIENT_PORT,
            sequence,
            data,
            captured_at: FIXTURE_START_SECS + (t - first),
        });
        sequence = sequence.wrapping_add(length);
    }
    recorder.stop()?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let result = std::fs::copy(&written, out).map_err(|error| format!("{}: {error}", out.display()));
    let _ = std::fs::remove_dir_all(&folder);
    result.map(|_| ())
}

pub fn run(files: &[PathBuf], args: &Args) -> Result<(), String> {
    let [file] = files else {
        return Err("fixture takes exactly one recording".to_string());
    };
    let out = PathBuf::from(args.value("out").ok_or("fixture needs --out <file.belisk>")?);
    let capture = capture::load(files)?;
    let servers: Vec<usize> = capture
        .flows
        .iter()
        .enumerate()
        .filter(|(_, flow)| flow.role == FlowRole::GameServer)
        .map(|(index, _)| index)
        .collect();
    if servers.len() != 1 {
        return Err(format!("{} has {} game server flows; the fixture needs exactly one", file.display(), servers.len()));
    }

    let frames: Vec<Frame> = capture
        .packets
        .iter()
        .filter(|packet: &&Packet| packet.dir == Direction::ServerToClient)
        .filter_map(|packet| {
            let opcode = [*packet.data.first()?, *packet.data.get(1)?];
            KEPT.contains(&opcode).then(|| Frame { t: packet.t, payload: packet.data.clone() })
        })
        .collect();

    let original = observe(&frames);
    let names: BTreeSet<String> =
        original.names.values().cloned().chain(original.killer_names.iter().cloned()).collect();
    if original.names.is_empty() || original.damage_by_actor.is_empty() {
        return Err("the recording names no actor or holds no damage; pick a recording of a fight with your own character in it".to_string());
    }

    let (anonymous, replaced, filled) = anonymise(&frames, &names)?;
    if let Some(name) = find_original_name(&anonymous, &names) {
        return Err(format!("a name ({} bytes) is still in the frames after replacing it; not writing", name.len()));
    }

    let after = observe(&anonymous);
    let same_shape = after.actors == original.actors
        && after.damage_by_actor == original.damage_by_actor
        && after.names.keys().eq(original.names.keys())
        && after.names.iter().all(|(id, name)| name.len() == original.names[id].len())
        && after.killer_names.len() == original.killer_names.len();
    if !same_shape {
        return Err("the core reads the anonymised frames differently (actors, classes, servers, combat power or damage); not writing".to_string());
    }

    write_fixture(&anonymous, &out)?;
    let size = std::fs::metadata(&out).map_err(|error| error.to_string())?.len();
    crate::out!(
        "{}: {} frames kept of {} server frames, {} names ({} killers) replaced at {replaced} places, {filled} other text characters filled, {size} bytes",
        out.display(),
        anonymous.len(),
        capture.packets.iter().filter(|packet| packet.dir == Direction::ServerToClient).count(),
        names.len(),
        original.killer_names.len(),
    );
    crate::out!(
        "players hit for {} damage in total; review the text left in the identity frames:",
        original.damage_by_actor.values().sum::<u64>()
    );
    let mut left: BTreeMap<String, usize> = BTreeMap::new();
    for frame in anonymous.iter().filter(|frame| TEXT_FRAMES.contains(&frame.opcode())) {
        for run in text_runs(&frame.payload) {
            *left.entry(String::from_utf8_lossy(&frame.payload[run]).into_owned()).or_default() += 1;
        }
    }
    for (text, count) in &left {
        crate::out!("  {count:>4} x {text}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(payload: &[u8]) -> Frame {
        Frame { t: 0.0, payload: payload.to_vec() }
    }

    fn names(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn placeholders_are_distinct_and_as_long_as_the_name_in_bytes() {
        let mut placeholders = Placeholders::default();

        let first = placeholders.for_name("Quillo".as_bytes()).unwrap();
        let second = placeholders.for_name("Brenna".as_bytes()).unwrap();
        let accented = placeholders.for_name("Élodie".as_bytes()).unwrap();
        let again = placeholders.for_name("Quillo".as_bytes()).unwrap();

        assert_eq!((first.len(), second.len(), accented.len()), (6, 6, 7));
        assert_ne!(first, second);
        assert_eq!(first, again);
        assert!(first.starts_with(b"P"));
    }

    #[test]
    fn a_name_goes_wherever_it_occurs_in_an_identity_frame_along_with_other_text() {
        let mut payload = vec![0x33, 0x36, 7, 6];
        payload.extend_from_slice(b"Quillo");
        payload.extend_from_slice(&[0x16, 0x05, 0x01]);
        payload.extend_from_slice(b"Guildhall of Quillo");
        let original_len = payload.len();

        let (anonymous, replaced, filled) = anonymise(&[frame(&payload)], &names(&["Quillo"])).unwrap();

        let text = String::from_utf8_lossy(&anonymous[0].payload).into_owned();
        assert_eq!(anonymous[0].payload.len(), original_len);
        assert!(!text.contains("Quillo") && !text.contains("Guildhall"), "{text}");
        assert_eq!(replaced, 2);
        assert!(filled > 0);
        assert_eq!(&anonymous[0].payload[..4], &[0x33, 0x36, 7, 6], "the bytes around the text are not touched");
    }

    #[test]
    fn a_frame_without_text_keeps_its_bytes_even_when_they_spell_letters() {
        let payload = vec![0x04, 0x38, b'a', b'b', b'c', b'd', b'e', b'f'];

        let (anonymous, replaced, filled) = anonymise(&[frame(&payload)], &names(&["abcdef"])).unwrap();

        assert_eq!(anonymous[0].payload, payload);
        assert_eq!((replaced, filled), (0, 0));
        assert_eq!(find_original_name(&anonymous, &names(&["abcdef"])).as_deref(), Some("abcdef"), "and the check refuses it");
    }
}
