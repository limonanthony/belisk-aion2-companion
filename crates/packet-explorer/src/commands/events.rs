use std::collections::BTreeMap;

use serde::Serialize;

use crate::args::{Args, DirectionFilter, Filter};
use crate::capture::labels::parser_label;
use crate::capture::Capture;
use crate::clock::{self, TimeSpec};

struct Note {
    spec: TimeSpec,
    label: String,
}

fn parse_notes(text: &str) -> Result<Vec<Note>, String> {
    let mut notes = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_whitespace();
        let first = tokens.next().unwrap_or_default();
        let spec_text = if first.contains('-') {
            format!("{first} {}", tokens.next().unwrap_or_default())
        } else {
            first.to_string()
        };
        let label = tokens.collect::<Vec<_>>().join(" ");
        let spec: TimeSpec = spec_text
            .parse()
            .map_err(|error| format!("notes line {}: {error}", index + 1))?;
        if matches!(spec, TimeSpec::Offset(_)) {
            return Err(format!("notes line {}: use a local clock time, not an offset", index + 1));
        }
        notes.push(Note { spec, label });
    }
    if notes.is_empty() {
        return Err("the notes file has no 'HH:MM[:SS] label' lines".to_string());
    }
    Ok(notes)
}


struct Window {
    file: usize,
    note: usize,
    at: f64,
    from: f64,
    to: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    opcode: String,
    parser: Option<&'static str>,
    in_window: usize,
    outside_windows: usize,
    ratio: Option<f64>,
    offsets_secs: Vec<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventReport {
    label: String,
    at: String,
    file: usize,
    candidates: Vec<Candidate>,
}

fn union_length(windows: &mut Vec<(f64, f64)>) -> f64 {
    windows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut total = 0.0;
    let mut reach = f64::NEG_INFINITY;
    for &(from, to) in windows.iter() {
        let from = from.max(reach);
        if to > from {
            total += to - from;
        }
        reach = reach.max(to);
    }
    total
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let notes_path = args.value("notes").ok_or("events needs --notes <file> with 'HH:MM[:SS] label' lines (local time)")?;
    let notes = parse_notes(&std::fs::read_to_string(notes_path).map_err(|error| format!("cannot read {notes_path}: {error}"))?)?;
    let before: f64 = args.parsed_or("before", 5.0)?;
    let after: f64 = args.parsed_or("after", 30.0)?;
    let min_ratio: f64 = args.parsed_or("ratio", 5.0)?;
    let filter = Filter::from_args(args, DirectionFilter::ServerOnly)?;
    let bound = filter.bind(&capture.files);

    let mut windows = Vec::new();
    for (file_index, file) in capture.files.iter().enumerate() {
        for (note_index, note) in notes.iter().enumerate() {
            for at in note.spec.occurrences(file.start - after, file.end + before) {
                windows.push(Window { file: file_index, note: note_index, at, from: at - before, to: at + after });
            }
        }
    }
    if windows.is_empty() {
        return Err("no noted moment falls inside the recordings".to_string());
    }

    let mut outside_seconds = 0.0;
    for (file_index, file) in capture.files.iter().enumerate() {
        let mut spans: Vec<(f64, f64)> = windows
            .iter()
            .filter(|window| window.file == file_index)
            .map(|window| (window.from.max(file.start), window.to.min(file.end)))
            .collect();
        outside_seconds += (file.end - file.start - union_length(&mut spans)).max(0.0);
    }

    let mut outside_counts: BTreeMap<[u8; 2], usize> = BTreeMap::new();
    let mut inside: Vec<BTreeMap<[u8; 2], Vec<f64>>> = windows.iter().map(|_| BTreeMap::new()).collect();
    for packet in capture.packets.iter().filter(|packet| bound.matches(packet)) {
        let Some(opcode) = packet.opcode() else { continue };
        let mut in_any = false;
        for (index, window) in windows.iter().enumerate() {
            if window.file == packet.file && (window.from..=window.to).contains(&packet.t) {
                inside[index].entry(opcode).or_default().push(packet.t - window.at);
                in_any = true;
            }
        }
        if !in_any {
            *outside_counts.entry(opcode).or_default() += 1;
        }
    }

    let window_secs = before + after;
    let reports: Vec<EventReport> = windows
        .iter()
        .zip(inside)
        .map(|(window, seen)| {
            let mut candidates: Vec<Candidate> = seen
                .into_iter()
                .filter_map(|(opcode, offsets)| {
                    let outside = outside_counts.get(&opcode).copied().unwrap_or(0);
                    let ratio = (outside > 0 && outside_seconds > 0.0).then(|| {
                        (offsets.len() as f64 / window_secs) / (outside as f64 / outside_seconds)
                    });
                    if ratio.is_some_and(|ratio| ratio < min_ratio) {
                        return None;
                    }
                    Some(Candidate {
                        opcode: crate::capture::labels::format_opcode(opcode),
                        parser: parser_label(opcode),
                        in_window: offsets.len(),
                        outside_windows: outside,
                        ratio,
                        offsets_secs: offsets.into_iter().take(6).collect(),
                    })
                })
                .collect();
            candidates.sort_by(|a, b| {
                b.ratio.unwrap_or(f64::INFINITY).total_cmp(&a.ratio.unwrap_or(f64::INFINITY))
                    .then(b.in_window.cmp(&a.in_window))
            });
            EventReport {
                label: notes[window.note].label.clone(),
                at: clock::format_full(window.at),
                file: window.file,
                candidates,
            }
        })
        .collect();

    if args.flag("json") {
        return crate::output::print_json(&reports);
    }
    crate::out!(
        "windows: {before}s before .. {after}s after each noted moment; baseline = {outside_seconds:.0}s outside every window; keep ratio >= {min_ratio} (none = never seen outside)"
    );
    for report in &reports {
        crate::out!();
        crate::out!("{}  f{}  \"{}\"", report.at, report.file, report.label);
        if report.candidates.is_empty() {
            crate::out!("  nothing rare in this window");
        }
        for candidate in &report.candidates {
            let offsets: Vec<String> = candidate.offsets_secs.iter().map(|secs| format!("{secs:+.1}s")).collect();
            crate::out!(
                "  {}  in-window {}  outside {}  ratio {}  at {}  {}",
                candidate.opcode,
                candidate.in_window,
                candidate.outside_windows,
                candidate.ratio.map_or("never-outside".to_string(), |ratio| format!("{ratio:.1}")),
                offsets.join(" "),
                candidate.parser.unwrap_or(""),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_accept_time_only_and_dated_lines() {
        let notes = parse_notes("# comment\n16:00 shugo started\n2026-10-07 17:10:05 key refill\n").unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].label, "shugo started");
        assert_eq!(notes[0].spec, TimeSpec::TimeOfDay(16 * 3600));
        assert_eq!(notes[1].label, "key refill");
        assert!(matches!(notes[1].spec, TimeSpec::Absolute(_)));
    }

    #[test]
    fn overlapping_windows_are_counted_once() {
        let mut spans = vec![(0.0, 10.0), (5.0, 12.0), (20.0, 25.0)];
        assert_eq!(union_length(&mut spans), 17.0);
    }
}
