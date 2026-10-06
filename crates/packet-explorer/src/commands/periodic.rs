use std::collections::BTreeMap;

use serde::Serialize;

use crate::args::{Args, DirectionFilter, Filter};
use crate::capture::labels::{format_opcode, parser_label};
use crate::capture::Capture;
use crate::clock;

const WALL_CLOCK_PERIODS_SECS: [f64; 11] = [
    60.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 10_800.0, 21_600.0, 43_200.0, 86_400.0,
];
const MIN_SHARE: f64 = 0.7;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Interval {
    median_gap_secs: f64,
    gaps_within_tolerance: usize,
    gaps: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WallClock {
    period_secs: f64,
    time_of_period: String,
    occurrences_on_it: usize,
    periods_seen: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PeriodicRow {
    opcode: String,
    parser: Option<&'static str>,
    bursts: usize,
    interval: Option<Interval>,
    wall_clock: Option<WallClock>,
    starts: Vec<String>,
}

/// Occurrences closer together than `merge_secs` are one burst, so a packet
/// sent several times in a row counts once.
fn burst_starts(times: &mut [f64], merge_secs: f64) -> Vec<f64> {
    times.sort_by(f64::total_cmp);
    let mut starts: Vec<f64> = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for &time in times.iter() {
        if time - last > merge_secs {
            starts.push(time);
        }
        last = time;
    }
    starts
}

fn circular_distance(a: f64, b: f64, period: f64) -> f64 {
    let delta = (a - b).rem_euclid(period);
    delta.min(period - delta)
}

fn wall_clock_fit(starts: &[f64], tolerance_secs: f64, min_bursts: usize) -> Option<WallClock> {
    let local: Vec<f64> = starts.iter().map(|&time| clock::local_seconds(time)).collect();
    WALL_CLOCK_PERIODS_SECS.iter().rev().find_map(|&period| {
        let residues: Vec<f64> = local.iter().map(|seconds| seconds.rem_euclid(period)).collect();
        let (anchor, members) = residues
            .iter()
            .map(|&candidate| {
                let members: Vec<usize> = (0..residues.len())
                    .filter(|&index| circular_distance(residues[index], candidate, period) <= tolerance_secs.min(period * 0.02))
                    .collect();
                (candidate, members)
            })
            .max_by_key(|(_, members)| members.len())?;
        let periods_seen = members
            .iter()
            .map(|&index| (local[index] / period).floor() as i64)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let enough = members.len() >= min_bursts && members.len() as f64 >= MIN_SHARE * starts.len() as f64;
        (enough && periods_seen >= 2).then(|| WallClock {
            period_secs: period,
            time_of_period: clock::format_clock(anchor.round() as u32),
            occurrences_on_it: members.len(),
            periods_seen,
        })
    })
}

fn interval_fit(starts_by_file: &BTreeMap<usize, Vec<f64>>, tolerance_secs: f64, min_bursts: usize) -> Option<Interval> {
    let mut gaps: Vec<f64> = starts_by_file
        .values()
        .flat_map(|starts| starts.windows(2).map(|pair| pair[1] - pair[0]))
        .collect();
    if gaps.len() + 1 < min_bursts {
        return None;
    }
    gaps.sort_by(f64::total_cmp);
    let median = gaps[gaps.len() / 2];
    let tolerance = tolerance_secs.max(median * 0.02);
    let consistent = gaps.iter().filter(|gap| (**gap - median).abs() <= tolerance).count();
    (consistent as f64 >= MIN_SHARE * gaps.len() as f64).then_some(Interval {
        median_gap_secs: median,
        gaps_within_tolerance: consistent,
        gaps: gaps.len(),
    })
}

pub fn run(capture: &Capture, args: &Args) -> Result<(), String> {
    let filter = Filter::from_args(args, DirectionFilter::ServerOnly)?;
    let bound = filter.bind(&capture.files);
    let min_bursts: usize = args.parsed_or("min", 3)?;
    let merge_secs: f64 = args.parsed_or("merge", 2.0)?;
    let tolerance_secs: f64 = args.parsed_or("tol", 10.0)?;

    let mut times: BTreeMap<[u8; 2], BTreeMap<usize, Vec<f64>>> = BTreeMap::new();
    for packet in capture.packets.iter().filter(|packet| bound.matches(packet)) {
        if let Some(opcode) = packet.opcode() {
            times.entry(opcode).or_default().entry(packet.file).or_default().push(packet.t);
        }
    }

    let mut rows = Vec::new();
    for (opcode, by_file) in times {
        let starts_by_file: BTreeMap<usize, Vec<f64>> = by_file
            .into_iter()
            .map(|(file, mut times)| (file, burst_starts(&mut times, merge_secs)))
            .collect();
        let mut all_starts: Vec<f64> = starts_by_file.values().flatten().copied().collect();
        all_starts.sort_by(f64::total_cmp);
        if all_starts.len() < min_bursts {
            continue;
        }
        let interval = interval_fit(&starts_by_file, tolerance_secs, min_bursts);
        let wall_clock = wall_clock_fit(&all_starts, tolerance_secs, min_bursts);
        if interval.is_none() && wall_clock.is_none() && !args.flag("all") {
            continue;
        }
        rows.push(PeriodicRow {
            opcode: format_opcode(opcode),
            parser: parser_label(opcode),
            bursts: all_starts.len(),
            interval,
            wall_clock,
            starts: all_starts.iter().take(8).map(|&time| clock::format_full(time)).collect(),
        });
    }
    rows.sort_by(|a, b| {
        let gap = |row: &PeriodicRow| row.interval.as_ref().map_or(0.0, |interval| interval.median_gap_secs);
        gap(b).total_cmp(&gap(a))
    });

    if args.flag("json") {
        return crate::output::print_json(&rows);
    }
    crate::out!(
        "bursts merge occurrences within {merge_secs}s; interval = >=70% of gaps (within a file) equal the median +/-max({tolerance_secs}s, 2%); wall clock = >=70% of bursts share a time within a period, seen in >=2 periods (tolerance {tolerance_secs}s, at most 2% of the period)"
    );
    for row in &rows {
        let interval = row.interval.as_ref().map_or("-".to_string(), |interval| {
            format!("every {} ({}/{} gaps)", clock::format_span(interval.median_gap_secs).trim_start_matches('+'), interval.gaps_within_tolerance, interval.gaps)
        });
        let wall = row.wall_clock.as_ref().map_or("-".to_string(), |wall| {
            format!(
                "every {} at {} into the period ({}/{} bursts, {} periods)",
                clock::format_span(wall.period_secs).trim_start_matches('+'),
                wall.time_of_period,
                wall.occurrences_on_it,
                row.bursts,
                wall.periods_seen
            )
        });
        crate::out!("{}  bursts {:<4} interval: {interval}  wall clock: {wall}  {}", row.opcode, row.bursts, row.parser.unwrap_or(""));
    }
    if rows.is_empty() {
        crate::out!("no opcode repeats on a fixed interval or at a round wall-clock time (try --min 2, or --all to see every opcode)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bursts_merge_close_occurrences() {
        let mut times = vec![100.0, 100.5, 101.0, 200.0, 200.2, 400.0];
        assert_eq!(burst_starts(&mut times, 2.0), vec![100.0, 200.0, 400.0]);
    }

    #[test]
    fn a_daily_moment_is_found_across_two_days() {
        let day_one = "2026-10-06 16:00:02".parse::<clock::TimeSpec>().unwrap().resolve(0.0);
        let day_two = "2026-10-07 16:00:00".parse::<clock::TimeSpec>().unwrap().resolve(0.0);
        let day_three = "2026-10-08 16:00:03".parse::<clock::TimeSpec>().unwrap().resolve(0.0);
        let fit = wall_clock_fit(&[day_one, day_two, day_three], 10.0, 3).unwrap();
        assert_eq!(fit.period_secs, 86_400.0);
        assert!(fit.time_of_period.starts_with("16:00:0"), "{}", fit.time_of_period);
    }
}
