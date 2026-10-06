pub mod diff;
pub mod dump;
pub mod events;
pub mod flows;
pub mod opcodes;
pub mod parity;
pub mod periodic;
pub mod search;

use crate::args::{describe_opcode, Args};
use crate::capture::{self, Capture, Packet};
use crate::clock;

const USAGE: &str = "\
packet-explorer: developer-only explorer for AION 2 .aetherpc recordings (never shipped)

usage: packet-explorer <command> <recording>... [options]

  A recording is a path, a folder (all its .aetherpc files), or part of a file
  name in %APPDATA%/com.helveticxa.aether/recordings. Packet ids (#n) count
  across the recordings in the order given, so they are stable for one command line.

commands:
  opcodes   per-opcode histogram: direction, count, sizes, first/last, parser   [--sort count|opcode|first] [--c2s-sizes]
  flows     connections, which side is the game server, framing statistics
  timeline  one line per packet                                                  [--op 0be3,0143] [--from X] [--to Y] [--limit N]
  dump      hex + field view per packet                                          [--fields-max N (0 = no field table)]
  diff      changed offsets between two packets of one opcode                    --a <id> --b <id>   |   --op 0be3 (consecutive packets)
  search    a number in every encoding                                           --value 560 [--value ..] [--enc u16le,varint]   |   --pair 560,23 --within 16
  events    opcodes near, and rare away from, noted moments                      --notes notes.txt [--before 5] [--after 30] [--ratio 5]
  periodic  opcodes repeating on a fixed interval or at round wall-clock times   [--min 3] [--merge 2] [--tol 10] [--all]
  parity    the shipped StreamProcessor against this tool's framing

common options:
  --op       opcode selectors, comma separated: 0be3, '33 36', 8d (first byte), c2s
  --dir      s2c | c2s | both
  --from/--to  +SECONDS after recording start | HH:MM[:SS] local | 'YYYY-MM-DD HH:MM[:SS]' local
  --json     machine-readable output
  --show-text  print text runs unmasked (default: shown as ## / length only)

Sizes are payload bytes from the opcode on; offsets are payload offsets (opcode = 0..2).
Server-to-client packets are framed and bundle-unwrapped like the meter does, but
a packet that arrives in pieces is waited for however large. Client-to-server data
is encrypted: each reassembled chunk is listed as c2s with a size only.";

pub fn run(args: &Args) -> Result<(), String> {
    if args.flag("help") {
        crate::out!("{USAGE}");
        return Ok(());
    }
    match args.command.as_str() {
        "help" | "-h" => {
            crate::out!("{USAGE}");
            Ok(())
        }
        command => {
            let files = args.input_files()?;
            if command == "parity" {
                return parity::run(&files, args);
            }
            let capture = capture::load(&files)?;
            match command {
                "opcodes" => opcodes::run(&capture, args),
                "flows" => flows::run(&capture, args),
                "timeline" => dump::timeline(&capture, args),
                "dump" => dump::dump(&capture, args),
                "diff" => diff::run(&capture, args),
                "search" => search::run(&capture, args),
                "events" => events::run(&capture, args),
                "periodic" => periodic::run(&capture, args),
                other => Err(format!("unknown command '{other}' (try: packet-explorer help)")),
            }
        }
    }
}

pub fn relative_secs(capture: &Capture, packet: &Packet) -> f64 {
    packet.t - capture.files[packet.file].start
}

pub fn packet_line(capture: &Capture, packet: &Packet) -> String {
    format!(
        "#{} f{} +{:.3}s {}  {} {}  len={}{}",
        packet.id,
        packet.file,
        relative_secs(capture, packet),
        clock::format_full(packet.t),
        packet.dir.label(),
        describe_opcode(packet),
        packet.data.len(),
        if packet.in_bundle { "  bundled" } else { "" },
    )
}

pub fn limit_from(args: &Args, default: usize) -> Result<usize, String> {
    let limit: usize = args.parsed_or("limit", default)?;
    Ok(if limit == 0 { usize::MAX } else { limit })
}

pub fn median(sorted: &[usize]) -> usize {
    sorted.get(sorted.len() / 2).copied().unwrap_or_default()
}
