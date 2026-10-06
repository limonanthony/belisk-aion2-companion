use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::{limit_from, packet_line};
use crate::args::{describe_opcode, Args, DirectionFilter, Filter};
use crate::capture::{Capture, Packet};
use crate::fields::{self, hex_inline, plausible_time};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiffRun {
    a_offset: usize,
    b_offset: usize,
    len: usize,
    before: String,
    after: String,
    width: Option<usize>,
    before_value: Option<u64>,
    after_value: Option<u64>,
    delta: Option<i128>,
    notes: Vec<String>,
    text: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairDiff {
    a: usize,
    b: usize,
    a_len: usize,
    b_len: usize,
    common_prefix: usize,
    common_suffix: usize,
    aligned: Vec<DiffRun>,
    tail_aligned: Vec<DiffRun>,
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let a: Option<usize> = args.parsed("a")?;
    let b: Option<usize> = args.parsed("b")?;
    match (a, b) {
        (Some(a), Some(b)) => pair(capture, args, a, b),
        (None, None) if !args.values("op").is_empty() => consecutive(capture, args),
        _ => Err("diff needs --a <id> --b <id> (ids from timeline/dump), or --op <opcode> to compare consecutive packets".to_string()),
    }
}

fn packet_by_id(capture: &Capture, id: usize) -> Result<&Packet, String> {
    id.checked_sub(1)
        .and_then(|index| capture.packets.get(index))
        .ok_or_else(|| format!("no packet #{id}: the inputs hold {} packets", capture.packets.len()))
}

fn pair(capture: &Capture, args: &Args, a: usize, b: usize) -> Result<(), String> {
    let (first, second) = (packet_by_id(capture, a)?, packet_by_id(capture, b)?);
    let show_text = args.flag("show-text");
    let diff = diff_packets(first, second, show_text);

    if args.flag("json") {
        return crate::output::print_json(&diff);
    }
    crate::out!("A {}", packet_line(capture, first));
    crate::out!("B {}", packet_line(capture, second));
    if describe_opcode(first) != describe_opcode(second) {
        crate::out!("note: different opcodes; the comparison is positional only");
    }
    if diff.a_len == diff.b_len && diff.aligned.is_empty() {
        crate::out!("identical ({} bytes)", diff.a_len);
        return Ok(());
    }
    crate::out!("common prefix {} bytes, common suffix {} bytes", diff.common_prefix, diff.common_suffix);
    print_runs("changed (same offsets)", &diff.aligned);
    if diff.a_len != diff.b_len {
        crate::out!("lengths differ ({} vs {}); the tail of both packets compared end-aligned:", diff.a_len, diff.b_len);
        print_runs("changed (end-aligned)", &diff.tail_aligned);
    }
    Ok(())
}

fn print_runs(title: &str, runs: &[DiffRun]) {
    crate::out!("{title}: {} run(s)", runs.len());
    for run in runs {
        let place = if run.a_offset == run.b_offset {
            format!("@{:04x}", run.a_offset)
        } else {
            format!("a@{:04x} b@{:04x}", run.a_offset, run.b_offset)
        };
        let mut line = format!("  {place} len {}  {} -> {}", run.len, run.before, run.after);
        if let (Some(width), Some(before), Some(after), Some(delta)) =
            (run.width, run.before_value, run.after_value, run.delta)
        {
            line.push_str(&format!("   u{} {before} -> {after} (delta {delta:+})", width * 8));
        }
        if run.text {
            line.push_str("   [text]");
        }
        crate::out!("{line}");
        for note in &run.notes {
            crate::out!("      -> {note}");
        }
    }
}

fn diff_packets(a: &Packet, b: &Packet, show_text: bool) -> PairDiff {
    let (mask_a, mask_b) = (fields::text_mask(&a.data), fields::text_mask(&b.data));
    let compared = a.data.len().min(b.data.len());
    let common_prefix = a.data.iter().zip(&b.data).take_while(|(x, y)| x == y).count();
    let common_suffix = a
        .data
        .iter()
        .rev()
        .zip(b.data.iter().rev())
        .take(compared - common_prefix.min(compared))
        .take_while(|(x, y)| x == y)
        .count();
    let context = DiffContext { a, b, mask_a: &mask_a, mask_b: &mask_b, show_text };
    PairDiff {
        a: a.id,
        b: b.id,
        a_len: a.data.len(),
        b_len: b.data.len(),
        common_prefix,
        common_suffix,
        aligned: context.runs(0, 0, compared),
        tail_aligned: if a.data.len() == b.data.len() {
            Vec::new()
        } else {
            context.runs(a.data.len() - compared, b.data.len() - compared, compared)
        },
    }
}

struct DiffContext<'a> {
    a: &'a Packet,
    b: &'a Packet,
    mask_a: &'a [bool],
    mask_b: &'a [bool],
    show_text: bool,
}

impl DiffContext<'_> {
    fn runs(&self, a_start: usize, b_start: usize, len: usize) -> Vec<DiffRun> {
        let mut runs = Vec::new();
        let mut index = 0;
        while index < len {
            if self.a.data[a_start + index] == self.b.data[b_start + index] {
                index += 1;
                continue;
            }
            let start = index;
            while index < len && self.a.data[a_start + index] != self.b.data[b_start + index] {
                index += 1;
            }
            runs.push(self.describe(a_start + start, b_start + start, index - start));
        }
        runs
    }

    fn describe(&self, a_offset: usize, b_offset: usize, len: usize) -> DiffRun {
        let touches_text = |mask: &[bool], offset: usize| mask[offset..offset + len].contains(&true);
        let text = !self.show_text && (touches_text(self.mask_a, a_offset) || touches_text(self.mask_b, b_offset));
        let view_a = (!self.show_text).then_some(self.mask_a);
        let view_b = (!self.show_text).then_some(self.mask_b);
        let mut run = DiffRun {
            a_offset,
            b_offset,
            len,
            before: hex_inline(&self.a.data[a_offset..a_offset + len], view_a, a_offset),
            after: hex_inline(&self.b.data[b_offset..b_offset + len], view_b, b_offset),
            width: None,
            before_value: None,
            after_value: None,
            delta: None,
            notes: Vec::new(),
            text,
        };
        if text || len > 8 {
            return run;
        }
        let Some(width) = [1usize, 2, 4, 8].into_iter().find(|&width| width >= len) else {
            return run;
        };
        let read = |data: &[u8], offset: usize| {
            let bytes = data.get(offset..offset + width)?;
            let mut padded = [0u8; 8];
            padded[..width].copy_from_slice(bytes);
            Some(u64::from_le_bytes(padded))
        };
        if let (Some(before), Some(after)) = (read(&self.a.data, a_offset), read(&self.b.data, b_offset)) {
            run.width = Some(width);
            run.before_value = Some(before);
            run.after_value = Some(after);
            run.delta = Some(i128::from(after) - i128::from(before));
            for (label, value) in [("before", before), ("after", after)] {
                let seconds = match width {
                    8 => Some(value as f64 / 1000.0),
                    4 => Some(value as f64),
                    _ => None,
                };
                if let Some(rendered) = seconds.and_then(|secs| plausible_time(secs, self.a.t)) {
                    run.notes.push(format!("{label} as time: {rendered}"));
                }
            }
        }
        run
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangedSpan {
    offset: usize,
    len: usize,
    pairs_changed: usize,
    distinct_values: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsecutiveSummary {
    opcode: String,
    packets: usize,
    same_length_pairs: usize,
    length_changes: usize,
    changed_spans: Vec<ChangedSpan>,
    first_pairs: Vec<PairDiff>,
}

fn consecutive(capture: &Capture, args: &Args) -> Result<(), String> {
    let filter = Filter::from_args(args, DirectionFilter::ServerOnly)?;
    let bound = filter.bind(&capture.files);
    let show_text = args.flag("show-text");
    let shown_pairs = limit_from(args, 10)?;

    let mut by_opcode: BTreeMap<String, Vec<&Packet>> = BTreeMap::new();
    for packet in capture.packets.iter().filter(|packet| bound.matches(packet)) {
        by_opcode.entry(describe_opcode(packet)).or_default().push(packet);
    }

    let mut summaries = Vec::new();
    for (opcode, packets) in by_opcode {
        let mut changes_at: BTreeMap<usize, usize> = BTreeMap::new();
        let mut values_at: BTreeMap<usize, BTreeSet<u8>> = BTreeMap::new();
        let (mut same_length, mut length_changes) = (0, 0);
        let mut first_pairs = Vec::new();
        for pair in packets.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.data.len() != b.data.len() {
                length_changes += 1;
                continue;
            }
            same_length += 1;
            for (offset, (x, y)) in a.data.iter().zip(&b.data).enumerate() {
                values_at.entry(offset).or_default().extend([*x, *y]);
                if x != y {
                    *changes_at.entry(offset).or_default() += 1;
                }
            }
            if first_pairs.len() < shown_pairs {
                first_pairs.push(diff_packets(a, b, show_text));
            }
        }
        summaries.push(ConsecutiveSummary {
            opcode,
            packets: packets.len(),
            same_length_pairs: same_length,
            length_changes,
            changed_spans: spans(&changes_at, &values_at),
            first_pairs,
        });
    }

    if args.flag("json") {
        return crate::output::print_json(&summaries);
    }
    for summary in &summaries {
        crate::out!(
            "{}: {} packets, {} same-length consecutive pairs, {} length changes",
            summary.opcode, summary.packets, summary.same_length_pairs, summary.length_changes
        );
        crate::out!("  offsets that ever change between consecutive packets:");
        for span in &summary.changed_spans {
            crate::out!(
                "    @{:04x} len {:<3} changed in {} pair(s), {} distinct byte value(s)",
                span.offset, span.len, span.pairs_changed, span.distinct_values
            );
        }
        for pair in &summary.first_pairs {
            let spans: Vec<String> = pair.aligned.iter().map(|run| format!("@{:x}+{}", run.a_offset, run.len)).collect();
            crate::out!("  #{} -> #{}: {}", pair.a, pair.b, if spans.is_empty() { "identical".to_string() } else { spans.join(" ") });
        }
    }
    Ok(())
}

fn spans(changes_at: &BTreeMap<usize, usize>, values_at: &BTreeMap<usize, BTreeSet<u8>>) -> Vec<ChangedSpan> {
    let mut spans: Vec<ChangedSpan> = Vec::new();
    for (&offset, &changes) in changes_at {
        let distinct = values_at.get(&offset).map_or(0, BTreeSet::len);
        match spans.last_mut() {
            Some(last) if last.offset + last.len == offset => {
                last.len += 1;
                last.pairs_changed = last.pairs_changed.max(changes);
                last.distinct_values = last.distinct_values.max(distinct);
            }
            _ => spans.push(ChangedSpan { offset, len: 1, pairs_changed: changes, distinct_values: distinct }),
        }
    }
    spans
}
