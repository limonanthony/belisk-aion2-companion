use std::collections::{BTreeMap, BTreeSet};

use meter_core::capture::parser::ticket_record::{
    decode_snapshot, decode_update, TicketReason, TicketRecord, SNAPSHOT_OPCODE, UPDATE_OPCODE,
};
use meter_core::capture::parser::utils::read_varint;
use serde::Serialize;

use crate::args::Args;
use crate::capture::{Capture, Direction, Packet};
use crate::clock;

const IDENTITY_OPCODE: [u8; 2] = [0x33, 0x36];
const IDENTITY_AFTER_SNAPSHOT_SECS: f64 = 15.0;
const IDENTITY_BEFORE_SNAPSHOT_SECS: f64 = 2.0;
const MAX_NAME_BYTES: usize = 36;

const TICKET_NAMES: &[(u32, &str)] = &[
    (1, "Shugo Festival entries (server)"),
    (3, "Abyss time (base)"),
    (4, "Nightmare"),
    (6, "Rift time"),
    (7, "Shugo Festival key (character)"),
    (8, "Dimensional Invasion keys (server)"),
    (9, "Abyss time (Reshanta B)"),
    (10, "test ticket 10"),
    (11, "test ticket 11"),
    (12, "test ticket 12"),
    (13, "Abyss time (Reshanta D)"),
    (14, "Mentor dungeon reward (character)"),
    (15, "Mentor dungeon reward (server)"),
    (101, "Daily dungeon #1 (server)"),
    (102, "Daily dungeon #2 (Balaur/Fafnium?)"),
    (103, "Daily dungeon #3 (Balaur/Fafnium?)"),
    (201, "Growth dungeon time 1"),
    (202, "Growth dungeon time 2"),
    (203, "Growth dungeon time 3"),
    (204, "Growth dungeon time 4"),
    (205, "Growth dungeon time 5"),
    (206, "Growth dungeon time 6"),
    (10000001, "Abyss Corridor time 001"),
    (10000002, "Abyss Corridor time 002"),
    (10000003, "Abyss Corridor time 003"),
    (10000004, "Abyss Corridor time 004"),
    (10000005, "Abyss Corridor time 005"),
    (10000006, "Abyss Corridor time 006"),
    (10000007, "Abyss Corridor time 101"),
    (10000008, "Abyss Corridor time 102"),
    (10000009, "Abyss Corridor time 103"),
    (10000010, "Abyss Corridor time 201"),
    (10000011, "Abyss Corridor time 202"),
    (10000012, "Abyss Corridor time 203"),
    (60000001, "ODYLE ENERGY"),
    (60000002, "Conquest Hard/Challenge entries (shared)"),
    (60000003, "Conquest tier-1 entries"),
    (60000004, "Conquest tier-2 entries"),
    (60000005, "Conquest tier-3 entries"),
    (60000006, "Conquest Normal entries (shared)"),
    (60000007, "Conquest final-boss kills"),
    (60000008, "Transcendence final-boss kills"),
    (60000009, "Unclaimed-reward entries (server)"),
    (60000010, "Unclaimed-reward entries (character)"),
    (60000101, "Krao Cave, Conquest"),
    (60000201, "Krao Cave, Exploration"),
    (60001101, "Urugugu Canyon, Exploration"),
    (60001201, "Urugugu Canyon, Conquest"),
    (60002101, "Fire Temple, Exploration"),
    (60002201, "Fire Temple, Conquest"),
    (60003101, "Draupnir, Exploration"),
    (60003201, "Draupnir, Conquest"),
    (60004101, "Dying Dramata's Nest, Exploration"),
    (60004201, "Dying Dramata's Nest, Conquest"),
    (60005301, "Deus Research Base (Grade)"),
    (60006301, "Shattered Arkanis (Grade)"),
    (60007101, "Vakron Sky Island, Exploration"),
    (60007201, "Vakron Sky Island, Conquest"),
    (60009101, "Ferocious Horn Den, Exploration"),
    (60009201, "Ferocious Horn Den, Conquest"),
    (60012101, "Cradle of Nihility, Exploration"),
    (60012201, "Cradle of Nihility, Conquest"),
    (60013101, "Hall of Illusion, Exploration"),
    (60014101, "Azure Breath Island, Exploration"),
    (60015101, "Ordeal / Trial clears (probable)"),
    (70000001, "Subjugation"),
    (80000001, "Ascension Trial"),
    (90000001, "Abyssal Forge attempts"),
    (90000002, "Abyssal Forge final boss"),
    (90000003, "Corroded Facility attempts"),
    (90000004, "Corroded Facility final boss"),
    (90000005, "Chalice of Muspel attempts"),
    (90000006, "Chalice of Muspel final boss"),
];

fn ticket_name(id: u32) -> &'static str {
    TICKET_NAMES.iter().find(|(known, _)| *known == id).map_or("(not in the notes' table)", |(_, name)| name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct Value {
    count: u32,
    charge: u32,
}

impl Value {
    fn of(record: &TicketRecord) -> Value {
        Value { count: record.count, charge: record.charge }
    }

    fn text(self) -> String {
        format!("count={} charge={}", self.count, self.charge)
    }
}

fn reason_label(reason: TicketReason) -> String {
    match reason {
        TicketReason::Gain => "gain".to_string(),
        TicketReason::TimedRefill => "refill".to_string(),
        TicketReason::Spend => "spend".to_string(),
        TicketReason::Unknown(code) => format!("reason {code}"),
    }
}

fn parse_identity(data: &[u8]) -> Option<(String, u32)> {
    if data.get(..2) != Some(&IDENTITY_OPCODE[..]) {
        return None;
    }
    let actor = read_varint(data, 2);
    if !actor.is_valid() || actor.value <= 0 {
        return None;
    }
    let structure_start = 2 + actor.length;
    let name_len = usize::from(*data.get(structure_start + 5)?);
    if !(1..=MAX_NAME_BYTES).contains(&name_len) {
        return None;
    }
    let name_start = structure_start + 6;
    let name_end = name_start + name_len;
    let server = data.get(name_end..name_end + 2)?;
    let server_id = u32::from(u16::from_le_bytes([server[0], server[1]]));
    let name = std::str::from_utf8(&data[name_start..name_end]).ok()?;
    if server_id == 0 || name.chars().any(char::is_control) {
        return None;
    }
    Some((name.to_string(), server_id))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Character {
    index: usize,
    name: String,
    server_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TicketRow {
    id: u32,
    name: &'static str,
    record: TicketRecord,
    previous: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum Event {
    Snapshot {
        packet: usize,
        time: String,
        character: Option<usize>,
        tickets: usize,
        first_for_character: bool,
        hidden_rows: usize,
        rows: Vec<TicketRow>,
    },
    Update {
        packet: usize,
        time: String,
        character: Option<usize>,
        outer: u8,
        reason: String,
        delta: Option<u32>,
        row: TicketRow,
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
struct Change {
    time: String,
    kind: String,
    from: Option<Value>,
    to: Value,
    delta: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TicketSummary {
    id: u32,
    name: &'static str,
    first: Value,
    last: Value,
    gains: usize,
    refills: usize,
    spends: usize,
    changes: Vec<Change>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CharacterSummary {
    character: Option<usize>,
    unchanged_tickets: usize,
    tickets: Vec<TicketSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    snapshots: usize,
    updates: usize,
    characters: Vec<Character>,
    events: Vec<Event>,
    rejected: Vec<Rejected>,
    summaries: Vec<CharacterSummary>,
}

struct Options<'a> {
    ids: &'a [u32],
    changes_only: bool,
    show_text: bool,
}

impl Options<'_> {
    fn wants(&self, id: u32) -> bool {
        self.ids.is_empty() || self.ids.contains(&id)
    }
}

struct Identity {
    position: usize,
    character: usize,
}

struct Observed {
    first: Value,
    last: Value,
    changes: Vec<Change>,
}

fn register_character(characters: &mut Vec<(String, u32)>, identity: (String, u32)) -> usize {
    characters.iter().position(|known| *known == identity).unwrap_or_else(|| {
        characters.push(identity);
        characters.len() - 1
    })
}

fn bind_snapshot(packets: &[Packet], position: usize, identities: &[Identity]) -> Option<usize> {
    let snapshot = &packets[position];
    let close = |identity: &&Identity| packets[identity.position].file == snapshot.file;
    let after = identities.iter().filter(close).find(|identity| {
        identity.position > position && packets[identity.position].t - snapshot.t <= IDENTITY_AFTER_SNAPSHOT_SECS
    });
    let before = identities.iter().filter(close).rev().find(|identity| {
        identity.position < position && snapshot.t - packets[identity.position].t <= IDENTITY_BEFORE_SNAPSHOT_SECS
    });
    after.or(before).map(|identity| identity.character)
}

fn remember(observed: &mut BTreeMap<u32, Observed>, id: u32, value: Value, change: Option<Change>) {
    match observed.get_mut(&id) {
        Some(entry) => {
            entry.last = value;
            entry.changes.extend(change);
        }
        None => {
            observed.insert(id, Observed { first: value, last: value, changes: change.into_iter().collect() });
        }
    }
}

fn analyze(packets: &[Packet], options: &Options) -> Report {
    let mut known: Vec<(String, u32)> = Vec::new();
    let mut identities = Vec::new();
    for (position, packet) in packets.iter().enumerate() {
        if packet.dir != Direction::ServerToClient {
            continue;
        }
        if let Some(identity) = parse_identity(&packet.data) {
            identities.push(Identity { position, character: register_character(&mut known, identity) });
        }
    }

    let mut report = Report {
        snapshots: 0,
        updates: 0,
        characters: Vec::new(),
        events: Vec::new(),
        rejected: Vec::new(),
        summaries: Vec::new(),
    };
    let mut state: BTreeMap<Option<usize>, BTreeMap<u32, Value>> = BTreeMap::new();
    let mut observed: BTreeMap<Option<usize>, BTreeMap<u32, Observed>> = BTreeMap::new();
    let mut snapshotted: BTreeSet<Option<usize>> = BTreeSet::new();
    let mut current: Option<usize> = None;

    for (position, packet) in packets.iter().enumerate() {
        if packet.dir != Direction::ServerToClient {
            continue;
        }
        let Some(opcode) = packet.opcode() else { continue };
        let time = clock::format_full(packet.t);
        match opcode {
            IDENTITY_OPCODE => {
                if let Some(identity) = identities.iter().find(|identity| identity.position == position) {
                    current = Some(identity.character);
                }
            }
            SNAPSHOT_OPCODE => match decode_snapshot(&packet.data) {
                Ok(records) => {
                    report.snapshots += 1;
                    current = bind_snapshot(packets, position, &identities);
                    let previous_values = state.entry(current).or_default();
                    let observed_here = observed.entry(current).or_default();
                    let first_for_character = !snapshotted.contains(&current);
                    let mut rows = Vec::new();
                    let mut hidden_rows = 0;
                    for record in &records {
                        let value = Value::of(record);
                        let previous = previous_values.get(&record.id).copied();
                        let changed = previous.is_some_and(|previous| previous != value);
                        let change = changed.then(|| Change {
                            time: time.clone(),
                            kind: "snapshot".to_string(),
                            from: previous,
                            to: value,
                            delta: None,
                        });
                        remember(observed_here, record.id, value, change);
                        previous_values.insert(record.id, value);
                        let shown = options.wants(record.id) && (!options.changes_only || changed);
                        if shown {
                            rows.push(TicketRow { id: record.id, name: ticket_name(record.id), record: *record, previous });
                        } else {
                            hidden_rows += 1;
                        }
                    }
                    snapshotted.insert(current);
                    report.events.push(Event::Snapshot {
                        packet: packet.id,
                        time,
                        character: current,
                        tickets: records.len(),
                        first_for_character,
                        hidden_rows,
                        rows,
                    });
                }
                Err(why) => report.rejected.push(rejection(packet, &time, opcode, why)),
            },
            UPDATE_OPCODE => match decode_update(&packet.data) {
                Ok(update) => {
                    report.updates += 1;
                    let record = update.record;
                    let value = Value::of(&record);
                    let previous = state.entry(current).or_default().insert(record.id, value);
                    let reason = reason_label(update.reason);
                    let change = Change { time: time.clone(), kind: reason.clone(), from: previous, to: value, delta: update.delta };
                    remember(observed.entry(current).or_default(), record.id, value, Some(change));
                    if options.wants(record.id) {
                        report.events.push(Event::Update {
                            packet: packet.id,
                            time,
                            character: current,
                            outer: update.outer,
                            reason,
                            delta: update.delta,
                            row: TicketRow { id: record.id, name: ticket_name(record.id), record, previous },
                        });
                    }
                }
                Err(why) => report.rejected.push(rejection(packet, &time, opcode, why)),
            },
            _ => {}
        }
    }

    report.characters = known
        .into_iter()
        .enumerate()
        .map(|(index, (name, server_id))| Character {
            index,
            name: if options.show_text { name } else { format!("##({})", name.chars().count()) },
            server_id,
        })
        .collect();
    report.summaries = observed
        .into_iter()
        .map(|(character, tickets)| {
            let mut unchanged_tickets = 0;
            let mut summaries = Vec::new();
            for (id, entry) in tickets {
                if !options.wants(id) {
                    continue;
                }
                if entry.changes.is_empty() && entry.first == entry.last {
                    unchanged_tickets += 1;
                    continue;
                }
                let count_kind = |kind: &str| entry.changes.iter().filter(|change| change.kind == kind).count();
                summaries.push(TicketSummary {
                    id,
                    name: ticket_name(id),
                    first: entry.first,
                    last: entry.last,
                    gains: count_kind("gain"),
                    refills: count_kind("refill"),
                    spends: count_kind("spend"),
                    changes: entry.changes,
                });
            }
            CharacterSummary { character, unchanged_tickets, tickets: summaries }
        })
        .collect();
    report
}

fn rejection(packet: &Packet, time: &str, opcode: [u8; 2], why: String) -> Rejected {
    Rejected {
        packet: packet.id,
        time: time.to_string(),
        opcode: format!("{:02x}{:02x}", opcode[0], opcode[1]),
        len: packet.data.len(),
        why,
    }
}

fn character_label(report: &Report, character: Option<usize>) -> String {
    match character.and_then(|index| report.characters.get(index)) {
        Some(found) => format!("[{}] {} (server {})", found.index + 1, found.name, found.server_id),
        None => "[?] unknown character".to_string(),
    }
}

fn hours_minutes(ms: i64) -> String {
    let minutes = ms / 60_000;
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

fn delta_text(from: u32, to: u32) -> String {
    format!("{from}->{to} ({:+})", i64::from(to) - i64::from(from))
}

fn change_text(from: Option<Value>, to: Value) -> String {
    match from {
        None => format!("new: {}", to.text()),
        Some(from) if from == to => "unchanged".to_string(),
        Some(from) => {
            let mut parts = Vec::new();
            if from.count != to.count {
                parts.push(format!("count {}", delta_text(from.count, to.count)));
            }
            if from.charge != to.charge {
                parts.push(format!("charge {}", delta_text(from.charge, to.charge)));
            }
            parts.join(", ")
        }
    }
}

fn row_line(row: &TicketRow, show_change: bool) -> String {
    let record = &row.record;
    let remaining = record.remaining_ms.map_or_else(String::new, hours_minutes);
    let change = match row.previous {
        Some(previous) if show_change && previous != Value::of(record) => format!("  <- {}", change_text(Some(previous), Value::of(record))),
        _ => String::new(),
    };
    format!(
        "  {:>9}  {:<42} {:>6} {:>6} {:>6}  {:04b}{change}",
        row.id,
        row.name,
        record.count,
        record.charge,
        remaining,
        record.flags
    )
}

fn print_text(report: &Report) {
    crate::out!(
        "{} snapshot(s) (0B 61), {} update(s) (0C 61), {} rejected frame(s), {} character(s)",
        report.snapshots,
        report.updates,
        report.rejected.len(),
        report.characters.len()
    );
    for character in &report.characters {
        crate::out!("  [{}] {} (server {})", character.index + 1, character.name, character.server_id);
    }
    for event in &report.events {
        crate::out!();
        match event {
            Event::Snapshot { packet, time, character, tickets, first_for_character, hidden_rows, rows } => {
                crate::out!(
                    "SNAPSHOT #{packet} {time}  {}  {tickets} tickets{}",
                    character_label(report, *character),
                    if *first_for_character { "  (first snapshot of this character)" } else { "" }
                );
                crate::out!("  {:>9}  {:<42} {:>6} {:>6} {:>6}  flags", "id", "content", "count", "charge", "left");
                for row in rows {
                    crate::out!("{}", row_line(row, true));
                }
                if *hidden_rows > 0 {
                    crate::out!("  ({hidden_rows} row(s) hidden by --id/--changes-only)");
                }
            }
            Event::Update { packet, time, character, outer, reason, delta, row } => {
                let delta = delta.map_or_else(String::new, |delta| format!("  delta {delta}"));
                crate::out!(
                    "UPDATE   #{packet} {time}  {}  {} {} [{reason}, outer {outer}]{delta}",
                    character_label(report, *character),
                    row.id,
                    row.name
                );
                crate::out!("  now {}   {}", Value::of(&row.record).text(), change_text(row.previous, Value::of(&row.record)));
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
    for summary in &report.summaries {
        crate::out!();
        crate::out!("SUMMARY {}", character_label(report, summary.character));
        for ticket in &summary.tickets {
            crate::out!(
                "  {} {}: first {} -> last {}  (gains {}, refills {}, spends {})",
                ticket.id,
                ticket.name,
                ticket.first.text(),
                ticket.last.text(),
                ticket.gains,
                ticket.refills,
                ticket.spends
            );
            for change in &ticket.changes {
                let delta = change.delta.map_or_else(String::new, |delta| format!("  delta {delta}"));
                crate::out!("      {}  {:<8} {}{delta}", change.time, change.kind, change_text(change.from, change.to));
            }
        }
        if summary.unchanged_tickets > 0 {
            crate::out!("  ({} ticket(s) never changed, not listed)", summary.unchanged_tickets);
        }
    }
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let ids = args
        .values("id")
        .iter()
        .flat_map(|list| list.split(','))
        .map(|text| text.trim().parse::<u32>().map_err(|_| format!("--id: cannot read '{text}' as a ticket id")))
        .collect::<Result<Vec<_>, _>>()?;
    let options = Options { ids: &ids, changes_only: args.flag("changes-only"), show_text: args.flag("show-text") };
    let report = analyze(&capture.packets, &options);
    if args.flag("json") {
        return crate::output::print_json(&report);
    }
    print_text(&report);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn record(id: u32, remaining_ms: Option<i64>, count: Option<u32>, charge: Option<u32>) -> Vec<u8> {
        let flags = u8::from(remaining_ms.is_some()) | (u8::from(count.is_some()) << 2) | (u8::from(charge.is_some()) << 3);
        let mut bytes = vec![flags];
        bytes.extend(id.to_le_bytes());
        if let Some(ms) = remaining_ms {
            bytes.extend(ms.to_le_bytes());
        }
        if let Some(count) = count {
            bytes.extend(varint(count));
        }
        if let Some(charge) = charge {
            bytes.extend(varint(charge));
        }
        bytes
    }

    fn snapshot(records: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = SNAPSHOT_OPCODE.to_vec();
        bytes.extend(varint(records.len() as u32));
        records.iter().for_each(|record| bytes.extend(record));
        bytes
    }

    fn update(outer: u8, record: Vec<u8>, reason: u8, delta: Option<u32>) -> Vec<u8> {
        let mut bytes = UPDATE_OPCODE.to_vec();
        bytes.push(outer);
        bytes.extend(record);
        bytes.push(reason);
        if let Some(delta) = delta {
            bytes.extend(delta.to_le_bytes());
        }
        bytes
    }

    fn identity(name: &str, server_id: u16) -> Vec<u8> {
        let mut bytes = IDENTITY_OPCODE.to_vec();
        bytes.extend(varint(1234));
        bytes.extend([0; 5]);
        bytes.push(name.len() as u8);
        bytes.extend(name.as_bytes());
        bytes.extend(server_id.to_le_bytes());
        bytes.push(1);
        bytes
    }

    fn packet(id: usize, t: f64, data: Vec<u8>) -> Packet {
        Packet { id, file: 0, t, dir: Direction::ServerToClient, in_bundle: false, data }
    }

    fn options() -> Options<'static> {
        Options { ids: &[], changes_only: false, show_text: true }
    }

    #[test]
    fn a_snapshot_belongs_to_the_identity_that_follows_it() {
        let packets = vec![
            packet(1, 100.0, snapshot(&[record(4, None, Some(2), None)])),
            packet(2, 104.0, identity("Alpha", 1001)),
        ];
        let report = analyze(&packets, &options());
        let Event::Snapshot { character, .. } = &report.events[0] else { panic!("snapshot event") };
        assert_eq!(*character, Some(0));
        assert_eq!(report.characters[0].name, "Alpha");
    }

    #[test]
    fn an_identity_more_than_fifteen_seconds_after_leaves_the_snapshot_unbound() {
        let packets = vec![
            packet(1, 100.0, snapshot(&[record(4, None, Some(2), None)])),
            packet(2, 116.0, identity("Alpha", 1001)),
        ];
        let report = analyze(&packets, &options());
        let Event::Snapshot { character, .. } = &report.events[0] else { panic!("snapshot event") };
        assert_eq!(*character, None);
    }

    #[test]
    fn updates_report_the_change_against_the_previous_value_and_feed_the_summary() {
        let packets = vec![
            packet(1, 100.0, snapshot(&[record(60000001, None, Some(15), Some(125))])),
            packet(2, 104.0, identity("Alpha", 1001)),
            packet(3, 200.0, update(0, record(60000001, None, None, Some(60)), 3, None)),
            packet(4, 300.0, update(0, record(60000001, None, Some(15), Some(60)), 2, None)),
        ];
        let report = analyze(&packets, &options());
        let Event::Update { row, reason, .. } = &report.events[1] else { panic!("update event") };
        assert_eq!(reason, "spend");
        assert_eq!(row.previous, Some(Value { count: 15, charge: 125 }));
        assert_eq!(Value::of(&row.record), Value { count: 0, charge: 60 });
        let summary = &report.summaries[0].tickets[0];
        assert_eq!((summary.spends, summary.refills, summary.gains), (1, 1, 0));
        assert_eq!(summary.first, Value { count: 15, charge: 125 });
        assert_eq!(summary.last, Value { count: 15, charge: 60 });
    }

    #[test]
    fn changes_only_hides_unchanged_snapshot_rows_and_the_first_snapshot() {
        let first = snapshot(&[record(4, None, Some(2), None), record(8, None, Some(2), None)]);
        let second = snapshot(&[record(4, None, Some(2), None), record(8, None, Some(1), None)]);
        let packets = vec![
            packet(1, 100.0, first),
            packet(2, 104.0, identity("Alpha", 1001)),
            packet(3, 500.0, second),
            packet(4, 504.0, identity("Alpha", 1001)),
        ];
        let report = analyze(&packets, &Options { ids: &[], changes_only: true, show_text: true });
        let rows: Vec<usize> = report
            .events
            .iter()
            .map(|event| match event {
                Event::Snapshot { rows, .. } => rows.len(),
                Event::Update { .. } => 0,
            })
            .collect();
        assert_eq!(rows, vec![0, 1]);
    }

    #[test]
    fn the_id_filter_limits_rows_and_summaries() {
        let data = snapshot(&[record(4, None, Some(2), None), record(8, None, Some(2), None)]);
        let packets = vec![packet(1, 100.0, data)];
        let report = analyze(&packets, &Options { ids: &[8], changes_only: false, show_text: true });
        let Event::Snapshot { rows, .. } = &report.events[0] else { panic!("snapshot event") };
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![8]);
    }

    #[test]
    fn look_alike_frames_are_reported_as_rejected_not_decoded() {
        let mut filler = SNAPSHOT_OPCODE.to_vec();
        filler.extend([0x91, 0x80, 0x9A, 0x0C, 1, 2, 3, 4]);
        let report = analyze(&[packet(1, 100.0, filler)], &options());
        assert_eq!((report.snapshots, report.rejected.len()), (0, 1));
    }

    #[test]
    fn remaining_time_prints_as_hours_and_minutes() {
        assert_eq!(hours_minutes(25_200_000), "7:00");
        assert_eq!(hours_minutes(23_714_000), "6:35");
    }
}
