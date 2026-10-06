use serde::Serialize;

use crate::args::Args;
use crate::capture::{Capture, FlowInfo, FlowRole};
use crate::clock;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileRow<'a> {
    index: usize,
    name: &'a str,
    records: u64,
    start: String,
    end: String,
    seconds: f64,
}

#[derive(Serialize)]
struct FlowsReport<'a> {
    files: Vec<FileRow<'a>>,
    flows: Vec<&'a FlowInfo>,
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let files: Vec<FileRow> = capture
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| FileRow {
            index,
            name: &file.name,
            records: file.records,
            start: clock::format_full(file.start),
            end: clock::format_full(file.end),
            seconds: file.end - file.start,
        })
        .collect();
    let flows: Vec<&FlowInfo> = capture.flows.iter().filter(|flow| flow.role != FlowRole::Other || args.flag("all")).collect();

    if args.flag("json") {
        return crate::output::print_json(&FlowsReport { files, flows });
    }
    for file in &files {
        crate::out!(
            "f{} {}  records={}  {} .. {}  ({:.1}s)",
            file.index, file.name, file.records, file.start, file.end, file.seconds
        );
    }
    crate::out!();
    crate::out!("game flows (the heartbeat 0e 00 36 marks the server side; --all adds other connections)");
    for flow in flows {
        crate::out!(
            "f{} {:?} {} -> {}  chunks={} bytes={} before-heartbeat={} frames={} bundles ok/failed={}/{} largest={} over-8K={} skipped-bytes={} stall-skips={} leftover={}",
            flow.file,
            flow.role,
            flow.source,
            flow.destination,
            flow.chunks,
            flow.bytes,
            flow.bytes_before_first_heartbeat,
            flow.stats.frames,
            flow.stats.bundles_unwrapped,
            flow.stats.bundles_failed,
            flow.stats.largest_frame,
            flow.stats.frames_over_8k,
            flow.stats.skipped_bytes,
            flow.stats.stall_skips,
            flow.leftover_bytes,
        );
    }
    Ok(())
}
