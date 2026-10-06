use serde::Serialize;

use super::{limit_from, packet_line, relative_secs};
use crate::args::{describe_opcode, Args, DirectionFilter, Filter};
use crate::capture::labels::parser_label;
use crate::capture::{Capture, Direction, Packet};
use crate::clock;
use crate::fields::{self, FieldRow, MaskedRun};

const DEFAULT_FIELDS_MAX: usize = 32;

#[derive(Serialize)]
struct TextRun {
    offset: usize,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DumpRow {
    id: usize,
    file: usize,
    time: String,
    unix: f64,
    rel_secs: f64,
    direction: Direction,
    opcode: String,
    parser: Option<&'static str>,
    len: usize,
    in_bundle: bool,
    hex: Vec<String>,
    fields: Vec<FieldRow>,
    masked_text: Vec<MaskedRun>,
    text: Vec<TextRun>,
}

fn selected<'a>(capture: &'a Capture, args: &Args) -> Result<Vec<&'a Packet>, String> {
    let filter = Filter::from_args(args, DirectionFilter::ServerOnly)?;
    let bound = filter.bind(&capture.files);
    Ok(capture.packets.iter().filter(|packet| bound.matches(packet)).collect())
}

pub fn timeline(capture: &Capture, args: &Args) -> Result<(), String> {
    let packets = selected(capture, args)?;
    let limit = limit_from(args, 500)?;
    if args.flag("json") {
        let rows: Vec<_> = packets.iter().take(limit).map(|packet| dump_row(capture, packet, false, 0)).collect();
        return crate::output::print_json(&rows);
    }
    for packet in packets.iter().take(limit) {
        let parser = packet.opcode().and_then(parser_label).unwrap_or("");
        crate::out!("{}  {parser}", packet_line(capture, packet));
    }
    announce_truncation(packets.len(), limit);
    Ok(())
}

pub fn dump(capture: &Capture, args: &Args) -> Result<(), String> {
    let packets = selected(capture, args)?;
    let limit = limit_from(args, 50)?;
    let show_text = args.flag("show-text");
    let fields_max = args.parsed_or("fields-max", DEFAULT_FIELDS_MAX)?;
    let rows: Vec<DumpRow> = packets
        .iter()
        .take(limit)
        .map(|packet| dump_row(capture, packet, show_text, fields_max))
        .collect();

    if args.flag("json") {
        return crate::output::print_json(&rows);
    }
    for (row, packet) in rows.iter().zip(&packets) {
        crate::out!("{}  {}", packet_line(capture, packet), row.parser.unwrap_or(""));
        for line in &row.hex {
            crate::out!("  {line}");
        }
        if !row.masked_text.is_empty() && !show_text {
            let runs: Vec<String> = row.masked_text.iter().map(|run| format!("@{:x} len {}", run.offset, run.len)).collect();
            crate::out!("  text masked: {} (use --show-text)", runs.join(", "));
        }
        for run in &row.text {
            crate::out!("  text @{:x}: {}", run.offset, run.text);
        }
        for line in fields::format_field_rows(&row.fields) {
            crate::out!("  {line}");
        }
        if packet.data.len() > fields_max && fields_max > 0 && packet.dir == Direction::ServerToClient {
            crate::out!("  ... fields shown for the first {fields_max} of {} bytes (--fields-max)", packet.data.len());
        }
        crate::out!();
    }
    announce_truncation(packets.len(), limit);
    Ok(())
}

fn announce_truncation(total: usize, limit: usize) {
    if total > limit {
        crate::out!("({} more; raise --limit, 0 = all)", total - limit);
    }
}

fn dump_row(capture: &Capture, packet: &Packet, show_text: bool, fields_max: usize) -> DumpRow {
    let mask = fields::text_mask(&packet.data);
    let mask_view = (!show_text).then_some(mask.as_slice());
    let server_side = packet.dir == Direction::ServerToClient;
    DumpRow {
        id: packet.id,
        file: packet.file,
        time: clock::format_full(packet.t),
        unix: packet.t,
        rel_secs: relative_secs(capture, packet),
        direction: packet.dir,
        opcode: describe_opcode(packet),
        parser: packet.opcode().and_then(parser_label),
        len: packet.data.len(),
        in_bundle: packet.in_bundle,
        hex: fields::hex_lines(&packet.data, mask_view),
        fields: if server_side {
            let effective_mask = if show_text { vec![false; packet.data.len()] } else { mask.clone() };
            fields::field_rows(&packet.data, packet.t, &effective_mask, fields_max)
        } else {
            Vec::new()
        },
        masked_text: if show_text { Vec::new() } else { fields::masked_runs(&packet.data) },
        text: if show_text {
            fields::text_runs(&packet.data)
                .into_iter()
                .map(|run| TextRun {
                    offset: run.start,
                    text: String::from_utf8_lossy(&packet.data[run]).into_owned(),
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}
