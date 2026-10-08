use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::Timelike;
use serde::Serialize;

use crate::args::Args;
use crate::capture::{Capture, Direction, Packet};
use crate::clock;

const MAP_EVENT_OPCODE: [u8; 2] = [0x2E, 0x92];
const ZONE_OPCODE: [u8; 2] = [0x21, 0x36];
const SERVER_TIME_OPCODE: [u8; 2] = [0x00, 0x36];
const STATE_BYTES: usize = 5;
const ZONE_MIN_BYTES: usize = 14;
const HOUR_FORMAT: &str = "%Y-%m-%d %H:00";
const ROTATION_GAP_SECS: f64 = 2.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapEvent {
    pub id: u32,
    pub key: String,
    pub state: [u8; STATE_BYTES],
    pub start_ms: i64,
    pub variables: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zone {
    pub sequence: u32,
    pub map_id: u32,
    pub instance: u32,
}

pub fn decode_map_event(data: &[u8]) -> Result<MapEvent, String> {
    if data.get(..2) != Some(&MAP_EVENT_OPCODE[..]) {
        return Err("not a 2E 92 frame".to_string());
    }
    let id = u32::from_le_bytes(data.get(2..6).ok_or("truncated before the event id")?.try_into().expect("slice of 4"));
    let key_len = usize::from(*data.get(6).ok_or("truncated before the key length")?);
    let key_end = 7 + key_len;
    let key_bytes = data.get(7..key_end).ok_or_else(|| format!("key of {key_len} byte(s) runs past the {} byte frame", data.len()))?;
    let key = std::str::from_utf8(key_bytes).map_err(|_| "key is not UTF-8".to_string())?;
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_graphic()) {
        return Err("key is not printable ASCII".to_string());
    }
    let state: [u8; STATE_BYTES] = data
        .get(key_end..key_end + STATE_BYTES)
        .ok_or("truncated before the state bytes")?
        .try_into()
        .expect("slice of STATE_BYTES");
    let start_at = key_end + STATE_BYTES;
    let start = data.get(start_at..start_at + 8).ok_or("truncated before the start time")?;
    let start_ms = i64::from_le_bytes(start.try_into().expect("slice of 8"));
    Ok(MapEvent { id, key: key.to_string(), state, start_ms, variables: data[start_at + 8..].to_vec() })
}

pub fn decode_zone(data: &[u8]) -> Result<Zone, String> {
    if data.get(..2) != Some(&ZONE_OPCODE[..]) {
        return Err("not a 21 36 frame".to_string());
    }
    if data.len() < ZONE_MIN_BYTES {
        return Err(format!("{} byte(s) is too short for a zone frame", data.len()));
    }
    let word = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().expect("slice of 4"));
    Ok(Zone { sequence: word(2), map_id: word(6), instance: word(10) })
}

fn decode_server_time(data: &[u8]) -> Option<i64> {
    if data.get(..2) != Some(&SERVER_TIME_OPCODE[..]) {
        return None;
    }
    Some(i64::from_le_bytes(data.get(2..10)?.try_into().ok()?))
}

fn unix_secs(ms: i64) -> f64 {
    ms as f64 / 1000.0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" ")
}

fn shown_key(key: &str, show_text: bool) -> String {
    if show_text {
        key.to_string()
    } else {
        format!("##({})", key.chars().count())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum Entry {
    Zone {
        packet: usize,
        time: String,
        map_id: u32,
        instance: u32,
    },
    Event {
        packet: usize,
        time: String,
        id: u32,
        key: String,
        state: String,
        start: String,
        start_minus_packet_secs: f64,
        variables: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HourEvent {
    key: String,
    ids: Vec<u32>,
    states: Vec<String>,
    frames: usize,
    first_seen: String,
    last_seen: String,
    starts: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HourMap {
    from: String,
    map_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Hour {
    hour: String,
    covered: Vec<String>,
    maps: Vec<HourMap>,
    events: Vec<HourEvent>,
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
    map_events: usize,
    zone_changes: usize,
    server_time_offset_secs: Option<f64>,
    entries: Vec<Entry>,
    hours: Vec<Hour>,
    rejected: Vec<Rejected>,
}

struct Options {
    show_text: bool,
    key_filter: Option<String>,
}

impl Options {
    fn wants(&self, key: &str) -> bool {
        self.key_filter.as_deref().is_none_or(|filter| key.contains(filter))
    }
}

struct HourAccumulator {
    interval_start: f64,
    covered: Vec<(f64, f64)>,
    events: BTreeMap<String, EventAccumulator>,
}

#[derive(Default)]
struct EventAccumulator {
    ids: BTreeSet<u32>,
    states: BTreeSet<String>,
    frames: usize,
    first_seen: Option<f64>,
    last_seen: f64,
    starts: BTreeSet<i64>,
}

fn hour_of(unix: f64) -> String {
    clock::local_datetime(unix).format(HOUR_FORMAT).to_string()
}

fn hour_start(unix: f64) -> f64 {
    let moment = clock::local_datetime(unix);
    unix - f64::from(moment.minute() * 60 + moment.second()) - f64::from(moment.nanosecond()) / 1e9
}

fn merged_intervals(capture: &Capture) -> Vec<(f64, f64)> {
    let mut intervals: Vec<(f64, f64)> = capture.files.iter().map(|file| (file.start, file.end)).collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in intervals {
        match merged.last_mut() {
            Some(last) if start <= last.1 + ROTATION_GAP_SECS => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

fn minute_second(unix: f64, hour_start: f64) -> String {
    let offset = (unix - hour_start).max(0.0).round() as u64;
    format!("{:02}:{:02}", offset / 60, offset % 60)
}

fn hour_maps(zone_marks: &[(f64, u32)], interval_start: f64, first_covered: f64, base: f64) -> Vec<HourMap> {
    let hour_end = hour_start(first_covered) + 3600.0;
    let carried = zone_marks
        .iter()
        .rev()
        .find(|(at, _)| *at >= interval_start && *at <= first_covered)
        .map(|(_, map_id)| HourMap { from: minute_second(first_covered, base), map_id: *map_id });
    let changes = zone_marks
        .iter()
        .filter(|(at, _)| *at > first_covered && *at < hour_end)
        .map(|(at, map_id)| HourMap { from: minute_second(*at, base), map_id: *map_id });
    carried.into_iter().chain(changes).collect()
}

fn analyze(capture: &Capture, options: &Options) -> Report {
    let mut report = Report {
        map_events: 0,
        zone_changes: 0,
        server_time_offset_secs: None,
        entries: Vec::new(),
        hours: Vec::new(),
        rejected: Vec::new(),
    };
    let mut hours: BTreeMap<String, HourAccumulator> = BTreeMap::new();
    for (start, end) in merged_intervals(capture) {
        let mut cursor = hour_start(start);
        while cursor < end {
            let accumulator = hours.entry(hour_of(cursor + 1.0)).or_insert_with(|| HourAccumulator {
                interval_start: start,
                covered: Vec::new(),
                events: BTreeMap::new(),
            });
            accumulator.covered.push((start.max(cursor), end.min(cursor + 3600.0)));
            cursor += 3600.0;
        }
    }

    let mut zone_marks: Vec<(f64, u32)> = Vec::new();
    let mut seen_frames: HashSet<(usize, u64)> = HashSet::new();
    for packet in capture.packets.iter().filter(|packet| packet.dir == Direction::ServerToClient) {
        let Some(opcode) = packet.opcode() else { continue };
        if report.server_time_offset_secs.is_none() {
            report.server_time_offset_secs = decode_server_time(&packet.data).map(|ms| unix_secs(ms) - packet.t);
        }
        let time = clock::format_full(packet.t);
        match opcode {
            ZONE_OPCODE => match decode_zone(&packet.data) {
                Ok(zone) => {
                    if !seen_frames.insert((packet.data.len(), packet.t.to_bits())) {
                        continue;
                    }
                    report.zone_changes += 1;
                    zone_marks.push((packet.t, zone.map_id));
                    report.entries.push(Entry::Zone { packet: packet.id, time, map_id: zone.map_id, instance: zone.instance });
                }
                Err(why) => report.rejected.push(rejection(packet, &time, opcode, why)),
            },
            MAP_EVENT_OPCODE => match decode_map_event(&packet.data) {
                Ok(event) => {
                    if !seen_frames.insert((packet.data.len(), packet.t.to_bits())) {
                        continue;
                    }
                    report.map_events += 1;
                    if !options.wants(&event.key) {
                        continue;
                    }
                    let start_secs = unix_secs(event.start_ms);
                    if let Some(hour) = hours.get_mut(&hour_of(packet.t)) {
                        let seen = hour.events.entry(event.key.clone()).or_default();
                        seen.ids.insert(event.id);
                        seen.states.insert(hex(&event.state));
                        seen.frames += 1;
                        seen.first_seen.get_or_insert(packet.t);
                        seen.last_seen = packet.t;
                        seen.starts.insert(event.start_ms);
                    }
                    report.entries.push(Entry::Event {
                        packet: packet.id,
                        time,
                        id: event.id,
                        key: shown_key(&event.key, options.show_text),
                        state: hex(&event.state),
                        start: clock::format_full(start_secs),
                        start_minus_packet_secs: start_secs - packet.t,
                        variables: hex(&event.variables),
                    });
                }
                Err(why) => report.rejected.push(rejection(packet, &time, opcode, why)),
            },
            _ => {}
        }
    }
    zone_marks.sort_by(|a, b| a.0.total_cmp(&b.0));

    report.hours = hours
        .into_iter()
        .map(|(hour, accumulator)| {
            let base = accumulator.covered.first().map_or(0.0, |(start, _)| hour_start(*start));
            Hour {
                covered: accumulator
                    .covered
                    .iter()
                    .map(|(start, end)| format!("{}-{}", minute_second(*start, base), minute_second(*end, base)))
                    .collect(),
                maps: hour_maps(&zone_marks, accumulator.interval_start, accumulator.covered.first().map_or(0.0, |(start, _)| *start), base),
                events: accumulator
                    .events
                    .into_iter()
                    .map(|(key, seen)| {
                        let first_seen = seen.first_seen.unwrap_or_default();
                        HourEvent {
                            key: shown_key(&key, options.show_text),
                            ids: seen.ids.into_iter().collect(),
                            states: seen.states.into_iter().collect(),
                            frames: seen.frames,
                            first_seen: minute_second(first_seen, base),
                            last_seen: minute_second(seen.last_seen, base),
                            starts: seen.starts.into_iter().map(|ms| clock::format_full(unix_secs(ms))).collect(),
                        }
                    })
                    .collect(),
                hour,
            }
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

fn print_text(report: &Report) {
    crate::out!(
        "{} map event(s) (2E 92), {} zone frame(s) (21 36), {} rejected frame(s)",
        report.map_events,
        report.zone_changes,
        report.rejected.len()
    );
    if let Some(offset) = report.server_time_offset_secs {
        crate::out!("server clock (00 36) minus capture clock at the first server-time frame: {offset:+.1}s");
    }
    crate::out!();
    for entry in &report.entries {
        match entry {
            Entry::Zone { packet, time, map_id, instance } => {
                crate::out!("ZONE  #{packet} {time}  map {map_id}  instance {instance}");
            }
            Entry::Event { packet, time, id, key, state, start, start_minus_packet_secs, variables } => {
                crate::out!(
                    "EVENT #{packet} {time}  id {id}  {key}  state [{state}]  start {start} ({start_minus_packet_secs:+.1}s)  vars [{variables}]"
                );
            }
        }
    }
    crate::out!();
    crate::out!("PER HOUR (local clock; minute:second inside the hour; covered = capture present)");
    for hour in &report.hours {
        let maps: Vec<String> = hour.maps.iter().map(|map| format!("{} from {}", map.map_id, map.from)).collect();
        crate::out!("{}  covered {}  map [{}]", hour.hour, hour.covered.join(", "), maps.join(", "));
        if hour.events.is_empty() {
            crate::out!("    (no map event)");
        }
        for event in &hour.events {
            let ids: Vec<String> = event.ids.iter().map(u32::to_string).collect();
            crate::out!(
                "    {}  ids [{}]  states [{}]  {} frame(s)  seen {}-{}  starts [{}]",
                event.key,
                ids.join(" "),
                event.states.join(" | "),
                event.frames,
                event.first_seen,
                event.last_seen,
                event.starts.join(", ")
            );
        }
    }
    for rejected in &report.rejected {
        crate::out!();
        crate::out!("REJECTED #{} {} {} len {}: {}", rejected.packet, rejected.time, rejected.opcode, rejected.len, rejected.why);
    }
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let options = Options { show_text: args.flag("show-text"), key_filter: args.value("key").map(str::to_string) };
    let report = analyze(capture, &options);
    if args.flag("json") {
        return crate::output::print_json(&report);
    }
    print_text(&report);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_event(id: u32, key: &str, state: [u8; STATE_BYTES], start_ms: i64, variables: &[u8]) -> Vec<u8> {
        let mut bytes = MAP_EVENT_OPCODE.to_vec();
        bytes.extend(id.to_le_bytes());
        bytes.push(key.len() as u8);
        bytes.extend(key.as_bytes());
        bytes.extend(state);
        bytes.extend(start_ms.to_le_bytes());
        bytes.extend(variables);
        bytes
    }

    #[test]
    fn a_map_event_carries_id_key_state_start_time_and_variables() {
        let data = map_event(151_374, "BossChallenge_MapEvent", [2, 1, 1, 1, 0], 1_791_398_256_895, &[8, 0, 0, 0]);
        let event = decode_map_event(&data).expect("valid map event");
        assert_eq!(event.id, 151_374);
        assert_eq!(event.key, "BossChallenge_MapEvent");
        assert_eq!(event.state, [2, 1, 1, 1, 0]);
        assert_eq!(event.start_ms, 1_791_398_256_895);
        assert_eq!(event.variables, vec![8, 0, 0, 0]);
    }

    #[test]
    fn a_map_event_with_a_key_past_the_frame_or_a_binary_key_is_rejected() {
        let mut truncated = map_event(1, "Key_MapEvent", [0; STATE_BYTES], 0, &[]);
        truncated[6] = 200;
        assert!(decode_map_event(&truncated).is_err());
        let binary = map_event(1, "\u{1}\u{2}", [0; STATE_BYTES], 0, &[]);
        assert!(decode_map_event(&binary).is_err());
        assert!(decode_map_event(&[0x2E, 0x92, 0x91, 0x80, 0x9A, 0x0C, 1, 2, 3, 4]).is_err());
    }

    #[test]
    fn a_zone_frame_gives_the_sequence_map_id_and_instance() {
        let mut data = ZONE_OPCODE.to_vec();
        data.extend(2u32.to_le_bytes());
        data.extend(3395u32.to_le_bytes());
        data.extend(9_000_000u32.to_le_bytes());
        data.extend([0; 34]);
        assert_eq!(decode_zone(&data), Ok(Zone { sequence: 2, map_id: 3395, instance: 9_000_000 }));
        assert!(decode_zone(&[0x21, 0x36, 1, 2, 3, 4, 5, 6]).is_err());
    }

    #[test]
    fn server_time_is_a_little_endian_millisecond_stamp() {
        let mut data = SERVER_TIME_OPCODE.to_vec();
        data.extend(1_791_398_256_895i64.to_le_bytes());
        assert_eq!(decode_server_time(&data), Some(1_791_398_256_895));
        assert_eq!(decode_server_time(&[0x00, 0x36, 1]), None);
    }

    #[test]
    fn keys_are_masked_unless_text_is_shown() {
        assert_eq!(shown_key("SF_JumpJump_01_L_MapEvent", false), "##(25)");
        assert_eq!(shown_key("SF_JumpJump_01_L_MapEvent", true), "SF_JumpJump_01_L_MapEvent");
    }

    #[test]
    fn minute_second_counts_from_the_start_of_the_hour() {
        assert_eq!(minute_second(1_800.0 + 12.4, 0.0), "30:12");
        assert_eq!(minute_second(5.0, 10.0), "00:00");
    }

    #[test]
    fn an_hour_starts_in_the_zone_carried_over_and_lists_changes_inside_it() {
        let base = hour_start(1_800_000_000.0 + 1234.0);
        let marks = [(base - 5000.0, 1110), (base - 100.0, 1010), (base + 600.0, 600072), (base + 4000.0, 999)];
        let maps = hour_maps(&marks, base - 200.0, base + 10.0, base);
        let shown: Vec<(&str, u32)> = maps.iter().map(|map| (map.from.as_str(), map.map_id)).collect();
        assert_eq!(shown, vec![("00:10", 1010), ("10:00", 600072)]);
    }
}
