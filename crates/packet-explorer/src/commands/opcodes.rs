use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::median;
use crate::args::{Args, DirectionFilter, Filter};
use crate::capture::labels::{format_opcode, parser_label};
use crate::capture::{Capture, Direction, Packet};
use crate::clock;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpcodeRow {
    direction: Direction,
    opcode: String,
    parser: Option<&'static str>,
    count: usize,
    bundled: usize,
    min_size: usize,
    median_size: usize,
    max_size: usize,
    first: String,
    last: String,
    first_unix: f64,
    last_unix: f64,
    files: usize,
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let filter = Filter::from_args(args, DirectionFilter::Both)?;
    let bound = filter.bind(&capture.files);
    let split_c2s_by_size = args.flag("c2s-sizes");

    let mut groups: BTreeMap<(u8, [u8; 2], usize), Vec<&Packet>> = BTreeMap::new();
    for packet in capture.packets.iter().filter(|packet| bound.matches(packet)) {
        let key = match packet.opcode() {
            Some(opcode) => (0, opcode, 0),
            None => (1, [0, 0], if split_c2s_by_size { packet.data.len() } else { 0 }),
        };
        groups.entry(key).or_default().push(packet);
    }

    let mut rows: Vec<OpcodeRow> = groups
        .into_iter()
        .map(|((_, opcode, c2s_size), packets)| {
            let direction = packets[0].dir;
            let mut sizes: Vec<usize> = packets.iter().map(|packet| packet.data.len()).collect();
            sizes.sort_unstable();
            let first = packets.iter().map(|packet| packet.t).fold(f64::INFINITY, f64::min);
            let last = packets.iter().map(|packet| packet.t).fold(f64::NEG_INFINITY, f64::max);
            let files: BTreeSet<usize> = packets.iter().map(|packet| packet.file).collect();
            OpcodeRow {
                direction,
                opcode: match direction {
                    Direction::ServerToClient => format_opcode(opcode),
                    Direction::ClientToServer if split_c2s_by_size => format!("c2s/{c2s_size}"),
                    Direction::ClientToServer => "c2s".to_string(),
                },
                parser: (direction == Direction::ServerToClient).then(|| parser_label(opcode)).flatten(),
                count: packets.len(),
                bundled: packets.iter().filter(|packet| packet.in_bundle).count(),
                min_size: sizes[0],
                median_size: median(&sizes),
                max_size: sizes[sizes.len() - 1],
                first: clock::format_short(first),
                last: clock::format_short(last),
                first_unix: first,
                last_unix: last,
                files: files.len(),
            }
        })
        .collect();

    match args.value("sort") {
        None | Some("opcode") => {}
        Some("count") => rows.sort_by(|a, b| b.count.cmp(&a.count)),
        Some("first") => rows.sort_by(|a, b| a.first_unix.total_cmp(&b.first_unix)),
        Some(other) => return Err(format!("--sort: '{other}' is not count, opcode or first")),
    }

    if args.flag("json") {
        return crate::output::print_json(&rows);
    }
    crate::out!(
        "{:<4} {:<9} {:>6} {:>7} {:>6} {:>6} {:>6}  {:<14} {:<14} {:>5}  parser",
        "dir", "opcode", "count", "bundled", "min", "median", "max", "first", "last", "files"
    );
    for row in &rows {
        crate::out!(
            "{:<4} {:<9} {:>6} {:>7} {:>6} {:>6} {:>6}  {:<14} {:<14} {:>5}  {}",
            row.direction.label(),
            row.opcode,
            row.count,
            row.bundled,
            row.min_size,
            row.median_size,
            row.max_size,
            row.first,
            row.last,
            row.files,
            row.parser.unwrap_or("-"),
        );
    }
    crate::out!("sizes are payload bytes from the opcode on; times are local");
    Ok(())
}
