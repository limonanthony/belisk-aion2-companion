//! Decoding views over packet bytes: text masking, hex lines, and the
//! per-offset field table. Offsets are payload offsets (the opcode is 0..2).

use std::ops::Range;

use meter_core::capture::parser::utils::read_varint;
use serde::Serialize;

use crate::clock;

const MIN_TEXT_CHARS: usize = 4;
const MIN_TEXT_LETTERS: usize = 3;
const PLAUSIBLE_UNIX_SECS: Range<f64> = 1_577_836_800.0..2_051_222_400.0;
const NEAR_CAPTURE_SECS: f64 = 90.0 * 86_400.0;

/// Byte ranges that read as text: four or more printable characters, at least
/// three of them letters. Numbers that happen to spell ASCII are masked too;
/// hiding a few false positives is the cheaper mistake.
pub fn text_runs(data: &[u8]) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let (mut start, mut chars, mut letters) = (0usize, 0usize, 0usize);
    let mut index = 0usize;
    let close = |runs: &mut Vec<Range<usize>>, start: usize, end: usize, chars: usize, letters: usize| {
        if chars >= MIN_TEXT_CHARS && letters >= MIN_TEXT_LETTERS {
            runs.push(start..end);
        }
    };
    while index < data.len() {
        let (width, is_letter) = match printable_char_at(data, index) {
            Some(found) => found,
            None => {
                close(&mut runs, start, index, chars, letters);
                (chars, letters) = (0, 0);
                index += 1;
                continue;
            }
        };
        if chars == 0 {
            start = index;
        }
        chars += 1;
        letters += usize::from(is_letter);
        index += width;
    }
    close(&mut runs, start, index, chars, letters);
    runs
}

fn printable_char_at(data: &[u8], index: usize) -> Option<(usize, bool)> {
    let byte = data[index];
    if (0x20..=0x7E).contains(&byte) {
        return Some((1, byte.is_ascii_alphabetic()));
    }
    let width = match byte {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return None,
    };
    let character = std::str::from_utf8(data.get(index..index + width)?).ok()?.chars().next()?;
    character.is_alphanumeric().then_some((width, character.is_alphabetic()))
}

pub fn text_mask(data: &[u8]) -> Vec<bool> {
    let mut mask = vec![false; data.len()];
    for run in text_runs(data) {
        mask[run].fill(true);
    }
    mask
}

#[derive(Serialize)]
pub struct MaskedRun {
    pub offset: usize,
    pub len: usize,
}

pub fn masked_runs(data: &[u8]) -> Vec<MaskedRun> {
    text_runs(data)
        .into_iter()
        .map(|run| MaskedRun { offset: run.start, len: run.len() })
        .collect()
}

pub fn hex_lines(data: &[u8], mask: Option<&[bool]>) -> Vec<String> {
    data.chunks(16)
        .enumerate()
        .map(|(row, bytes)| {
            let cells: Vec<String> = bytes
                .iter()
                .enumerate()
                .map(|(column, byte)| match mask {
                    Some(mask) if mask[row * 16 + column] => "##".to_string(),
                    _ => format!("{byte:02x}"),
                })
                .collect();
            format!("{:04x}  {}", row * 16, cells.join(" "))
        })
        .collect()
}

pub fn hex_inline(data: &[u8], mask: Option<&[bool]>, offset: usize) -> String {
    data.iter()
        .enumerate()
        .map(|(index, byte)| match mask {
            Some(mask) if mask[offset + index] => "##".to_string(),
            _ => format!("{byte:02x}"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A number that could be a wall-clock time, rendered in local time. Close to
/// the capture time it also shows the distance from it.
pub fn plausible_time(unix_secs: f64, captured_at: f64) -> Option<String> {
    if !PLAUSIBLE_UNIX_SECS.contains(&unix_secs) {
        return None;
    }
    let delta = unix_secs - captured_at;
    let mut text = clock::format_with_weekday(unix_secs);
    if delta.abs() < NEAR_CAPTURE_SECS {
        text.push_str(&format!(" ({} from capture)", clock::format_span(delta)));
    }
    Some(text)
}

fn time_notes(row: &FieldRow, captured_at: f64) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(value) = row.u64 {
        if let Some(text) = plausible_time(value as f64 / 1000.0, captured_at) {
            notes.push(format!("u64 ms = {text}"));
        }
        if let Some(text) = plausible_time(value as f64, captured_at) {
            notes.push(format!("u64 s = {text}"));
        }
    }
    if let Some(text) = row.u32.and_then(|value| plausible_time(f64::from(value), captured_at)) {
        notes.push(format!("u32 s = {text}"));
    }
    notes
}

#[derive(Serialize)]
pub struct FieldRow {
    pub offset: usize,
    pub u8: Option<u8>,
    pub u16: Option<u16>,
    pub u32: Option<u32>,
    pub u64: Option<u64>,
    pub varint: Option<i64>,
    pub varint_len: Option<usize>,
    pub f32: Option<f32>,
    pub notes: Vec<String>,
    pub masked: bool,
}

pub fn plausible_f32(bits: u32) -> Option<f32> {
    let value = f32::from_bits(bits);
    let magnitude = value.abs();
    (value.is_finite() && (1e-3..=1e8).contains(&magnitude) && bits & 0xFFF == 0).then_some(value)
}

pub fn field_rows(data: &[u8], captured_at: f64, mask: &[bool], limit: usize) -> Vec<FieldRow> {
    (0..data.len().min(limit))
        .map(|offset| {
            if mask[offset] {
                return FieldRow {
                    offset,
                    u8: None,
                    u16: None,
                    u32: None,
                    u64: None,
                    varint: None,
                    varint_len: None,
                    f32: None,
                    notes: Vec::new(),
                    masked: true,
                };
            }
            let window = |len: usize| {
                let bytes = data.get(offset..offset + len)?;
                (!mask[offset..offset + len].contains(&true)).then_some(bytes)
            };
            let varint = read_varint(data, offset);
            let usable_varint = varint.is_valid()
                && varint.length > 1
                && !mask[offset..offset + varint.length].contains(&true);
            let mut row = FieldRow {
                offset,
                u8: Some(data[offset]),
                u16: window(2).map(|b| u16::from_le_bytes([b[0], b[1]])),
                u32: window(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
                u64: window(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())),
                varint: usable_varint.then_some(varint.value),
                varint_len: usable_varint.then_some(varint.length),
                f32: None,
                notes: Vec::new(),
                masked: false,
            };
            row.f32 = row.u32.and_then(plausible_f32);
            row.notes = time_notes(&row, captured_at);
            row
        })
        .collect()
}

fn is_zero_row(row: &FieldRow) -> bool {
    !row.masked
        && row.u8 == Some(0)
        && row.u16.unwrap_or(0) == 0
        && row.u32.unwrap_or(0) == 0
        && row.u64.unwrap_or(0) == 0
}

/// One text line per offset; a run of all-zero offsets collapses to one line.
pub fn format_field_rows(rows: &[FieldRow]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut index = 0;
    while index < rows.len() {
        let run = rows[index..].iter().take_while(|row| is_zero_row(row)).count();
        if run >= 3 {
            lines.push(format!("{:04x}..{:04x}  zeros", rows[index].offset, rows[index + run - 1].offset));
            index += run;
        } else {
            lines.push(format_field_row(&rows[index]));
            index += 1;
        }
    }
    lines
}

fn format_field_row(row: &FieldRow) -> String {
    if row.masked {
        return format!("{:04x}  [text]", row.offset);
    }
    let mut cells = vec![format!("u8={}", row.u8.unwrap_or_default())];
    if let Some(value) = row.u16 {
        cells.push(format!("u16={value}"));
    }
    if let Some(value) = row.u32 {
        cells.push(format!("u32={value}"));
    }
    if let Some(value) = row.u64 {
        cells.push(format!("u64={value}"));
    }
    if let (Some(value), Some(len)) = (row.varint, row.varint_len) {
        cells.push(format!("varint={value}({len}B)"));
    }
    if let Some(value) = row.f32 {
        cells.push(format!("f32={value}"));
    }
    let mut line = format!("{:04x}  {}", row.offset, cells.join("  "));
    for note in &row.notes {
        line.push_str(&format!("\n          -> {note}"));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_masked_but_plain_numbers_and_short_runs_are_not() {
        let mut data = vec![0x00, 0x01, 0x30, 0x02];
        data.extend_from_slice(b"\x07Abcdefg\x00\x00");
        data.extend_from_slice(b"12345\x00ab\x00");
        let runs = text_runs(&data);
        assert_eq!(runs, vec![5..12]);
    }

    #[test]
    fn multibyte_letters_count_as_text() {
        let data = "\u{00e9}t\u{00e9}s\u{00e9}".as_bytes();
        assert_eq!(text_runs(data), vec![0..data.len()]);
    }

    #[test]
    fn hex_lines_hide_masked_bytes() {
        let data = b"\x01Abcdef\x02";
        let mask = text_mask(data);
        assert_eq!(hex_lines(data, Some(&mask)), vec!["0000  01 ## ## ## ## ## ## 02".to_string()]);
    }

    #[test]
    fn only_round_floats_are_reported() {
        assert_eq!(plausible_f32(1.5f32.to_bits()), Some(1.5));
        assert_eq!(plausible_f32(560), None);
        assert_eq!(plausible_f32(0x3E99_999A), None);
    }

    #[test]
    fn millisecond_timestamps_near_capture_show_their_distance() {
        let captured = 1_791_300_000.0;
        let rendered = plausible_time(captured + 3600.0, captured).unwrap();
        assert!(rendered.contains("+1h00m from capture"), "{rendered}");
        assert_eq!(plausible_time(560.0, captured), None);
    }
}
