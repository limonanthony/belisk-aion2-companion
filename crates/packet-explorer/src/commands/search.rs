use std::collections::BTreeMap;

use serde::Serialize;

use super::{limit_from, packet_line};
use crate::args::{describe_opcode, Args, DirectionFilter, Filter};
use crate::capture::Capture;
use crate::fields;

const CONTEXT_BYTES: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Number {
    Int(u64),
    Float(f32),
}

fn parse_number(text: &str) -> Result<Number, String> {
    let text = text.trim();
    let invalid = || format!("'{text}' is not a number (decimal, 0x hex, or a decimal with a point for f32)");
    if let Some(hex) = text.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).map(Number::Int).map_err(|_| invalid());
    }
    if text.contains('.') {
        return text.parse().map(Number::Float).map_err(|_| invalid());
    }
    text.parse().map(Number::Int).map_err(|_| invalid())
}

fn leb128(mut value: u64) -> Vec<u8> {
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

/// Every byte sequence the number can take, labelled by encoding. Varints are
/// the game's 7-bit little-endian groups, readable up to 32 bits.
fn encodings(number: Number) -> Vec<(&'static str, Vec<u8>)> {
    let mut found: Vec<(&'static str, Vec<u8>)> = Vec::new();
    match number {
        Number::Int(value) => {
            if let Ok(byte) = u8::try_from(value) {
                found.push(("u8", vec![byte]));
            }
            if let Ok(short) = u16::try_from(value) {
                found.push(("u16le", short.to_le_bytes().to_vec()));
                found.push(("u16be", short.to_be_bytes().to_vec()));
            }
            if let Ok(word) = u32::try_from(value) {
                found.push(("u32le", word.to_le_bytes().to_vec()));
                found.push(("u32be", word.to_be_bytes().to_vec()));
                found.push(("varint", leb128(value)));
            }
            found.push(("u64le", value.to_le_bytes().to_vec()));
            found.push(("u64be", value.to_be_bytes().to_vec()));
            if value <= 1 << 24 {
                let float = value as f32;
                found.push(("f32le", float.to_le_bytes().to_vec()));
                found.push(("f32be", float.to_be_bytes().to_vec()));
            }
        }
        Number::Float(value) => {
            found.push(("f32le", value.to_le_bytes().to_vec()));
            found.push(("f32be", value.to_be_bytes().to_vec()));
        }
    }
    found
}

#[derive(Clone, Serialize)]
struct Hit {
    offset: usize,
    len: usize,
    encodings: Vec<&'static str>,
}

fn find_hits(data: &[u8], number: Number, only: &[String]) -> Vec<Hit> {
    let mut by_place: BTreeMap<(usize, usize), Vec<&'static str>> = BTreeMap::new();
    for (label, needle) in encodings(number) {
        if !only.is_empty() && !only.iter().any(|wanted| wanted == label) {
            continue;
        }
        for (offset, window) in data.windows(needle.len()).enumerate() {
            if window == needle.as_slice() {
                by_place.entry((offset, needle.len())).or_default().push(label);
            }
        }
    }
    by_place
        .into_iter()
        .map(|((offset, len), encodings)| Hit { offset, len, encodings })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairHit {
    a: Hit,
    b: Hit,
    distance: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRow {
    id: usize,
    time: String,
    opcode: String,
    len: usize,
    hits: Vec<Hit>,
    pairs: Vec<PairHit>,
    context: Vec<String>,
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let filter = Filter::from_args(args, DirectionFilter::ServerOnly)?;
    let bound = filter.bind(&capture.files);
    let only: Vec<String> = args
        .values("enc")
        .iter()
        .flat_map(|list| list.split(','))
        .map(|label| label.trim().to_lowercase())
        .collect();
    let values = args.values("value").iter().map(|text| parse_number(text)).collect::<Result<Vec<_>, _>>()?;
    let pair = match args.value("pair") {
        Some(text) => {
            let (a, b) = text.split_once(',').ok_or("--pair takes two values: --pair 560,23")?;
            Some((parse_number(a)?, parse_number(b)?))
        }
        None => None,
    };
    if values.is_empty() && pair.is_none() {
        return Err("search needs --value <number> (repeatable) or --pair A,B --within N".to_string());
    }
    let within: usize = args.parsed_or("within", 16)?;
    let limit = limit_from(args, 100)?;
    let show_text = args.flag("show-text");

    let mut rows = Vec::new();
    for packet in capture.packets.iter().filter(|packet| bound.matches(packet)) {
        let mut hits = Vec::new();
        for &value in &values {
            hits.extend(find_hits(&packet.data, value, &only));
        }
        let mut pairs: Vec<PairHit> = Vec::new();
        if let Some((a, b)) = pair {
            let (hits_a, hits_b) = (find_hits(&packet.data, a, &only), find_hits(&packet.data, b, &only));
            let mut merged: BTreeMap<(usize, usize), PairHit> = BTreeMap::new();
            for first in &hits_a {
                for second in &hits_b {
                    let disjoint = first.offset + first.len <= second.offset || second.offset + second.len <= first.offset;
                    let distance = first.offset.abs_diff(second.offset);
                    if !disjoint || distance > within {
                        continue;
                    }
                    let entry = merged.entry((first.offset, second.offset)).or_insert_with(|| PairHit {
                        a: Hit { encodings: Vec::new(), ..first.clone() },
                        b: Hit { encodings: Vec::new(), ..second.clone() },
                        distance,
                    });
                    for label in &first.encodings {
                        if !entry.a.encodings.contains(label) {
                            entry.a.encodings.push(label);
                        }
                    }
                    for label in &second.encodings {
                        if !entry.b.encodings.contains(label) {
                            entry.b.encodings.push(label);
                        }
                    }
                }
            }
            pairs = merged.into_values().collect();
            if pairs.is_empty() && values.is_empty() {
                continue;
            }
        }
        if hits.is_empty() && pairs.is_empty() {
            continue;
        }
        let mask = fields::text_mask(&packet.data);
        let view = (!show_text).then_some(mask.as_slice());
        let mut anchors: Vec<(usize, usize)> = hits.iter().map(|hit| (hit.offset, hit.len)).collect();
        anchors.extend(pairs.iter().flat_map(|pair| [(pair.a.offset, pair.a.len), (pair.b.offset, pair.b.len)]));
        anchors.sort_unstable();
        anchors.dedup();
        let context = anchors
            .iter()
            .map(|&(offset, len)| {
                let from = offset.saturating_sub(CONTEXT_BYTES);
                let to = (offset + len + CONTEXT_BYTES).min(packet.data.len());
                format!("@{offset:x}: {}", fields::hex_inline(&packet.data[from..to], view, from))
            })
            .collect();
        rows.push(SearchRow {
            id: packet.id,
            time: crate::clock::format_full(packet.t),
            opcode: describe_opcode(packet),
            len: packet.data.len(),
            hits,
            pairs,
            context,
        });
    }

    if args.flag("json") {
        rows.truncate(limit);
        return crate::output::print_json(&rows);
    }
    for row in rows.iter().take(limit) {
        let packet = &capture.packets[row.id - 1];
        crate::out!("{}", packet_line(capture, packet));
        for hit in &row.hits {
            crate::out!("  @{:04x} len {} [{}]", hit.offset, hit.len, hit.encodings.join(","));
        }
        for pair in &row.pairs {
            crate::out!(
                "  pair A@{:04x}[{}] B@{:04x}[{}] distance {}",
                pair.a.offset, pair.a.encodings.join(","), pair.b.offset, pair.b.encodings.join(","), pair.distance
            );
        }
        for context in &row.context {
            crate::out!("    {context}");
        }
    }
    crate::out!("{} packet(s) matched{}", rows.len(), if rows.len() > limit { format!(" (first {limit} shown, --limit 0 = all)") } else { String::new() });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(number: Number) -> Vec<&'static str> {
        encodings(number).into_iter().map(|(label, _)| label).collect()
    }

    #[test]
    fn a_counter_value_is_looked_up_in_every_width_and_as_a_varint() {
        let found: BTreeMap<_, _> = encodings(Number::Int(560)).into_iter().collect();
        assert_eq!(found["u16le"], vec![0x30, 0x02]);
        assert_eq!(found["u16be"], vec![0x02, 0x30]);
        assert_eq!(found["varint"], vec![0xB0, 0x04]);
        assert_eq!(found["f32le"], 560f32.to_le_bytes().to_vec());
        assert!(!found.contains_key("u8"));
    }

    #[test]
    fn floats_only_search_as_f32() {
        assert_eq!(labels(Number::Float(1.5)), vec!["f32le", "f32be"]);
    }

    #[test]
    fn one_offset_reports_every_encoding_that_matches_there() {
        let hits = find_hits(&[9, 0x30, 0x02, 0, 0, 7], Number::Int(560), &[]);
        let at_one: Vec<_> = hits.iter().filter(|hit| hit.offset == 1).flat_map(|hit| hit.encodings.clone()).collect();
        assert!(at_one.contains(&"u16le") && at_one.contains(&"u32le"));
    }

    #[test]
    fn numbers_parse_as_decimal_hex_or_float() {
        assert_eq!(parse_number("560"), Ok(Number::Int(560)));
        assert_eq!(parse_number("0x230"), Ok(Number::Int(560)));
        assert_eq!(parse_number("2.5"), Ok(Number::Float(2.5)));
        assert!(parse_number("abc").is_err());
    }
}
