//! Packets the meter lost before they were read, and how that reaches the log.
//!
//! Two places can drop a packet: the queue between capture and the dispatcher
//! (full because the dispatcher fell behind) and the recording queues (full
//! because a disk stalled). Both are counted where they happen; this module
//! adds them up and turns them into at most one log line per period.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::capture::capturer::CapturedPacket;
use crate::capture::channel::Channel;
use crate::capture::recording_queue::RecordingQueue;

/// Between two summary lines in the log.
const REPORT_PERIOD: Duration = Duration::from_secs(30);

/// Totals since the meter was created.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketDrops {
    /// Packets capture handed over while the dispatcher's queue was full.
    /// They are missing from the game stream, which can break reassembly.
    pub capture_queue: u64,
    /// Packets left out of a recording because the recording queue was full.
    /// The meter read them; only the file lacks them.
    pub recording_queue: u64,
}

impl PacketDrops {
    fn since(self, earlier: PacketDrops) -> PacketDrops {
        PacketDrops {
            capture_queue: self.capture_queue.saturating_sub(earlier.capture_queue),
            recording_queue: self.recording_queue.saturating_sub(earlier.recording_queue),
        }
    }

    fn is_none(self) -> bool {
        self == PacketDrops::default()
    }
}

#[derive(Clone)]
pub struct PacketDropCounters {
    capture: Channel<CapturedPacket>,
    recordings: Vec<Arc<RecordingQueue>>,
}

impl PacketDropCounters {
    pub fn new(capture: Channel<CapturedPacket>, recordings: Vec<Arc<RecordingQueue>>) -> Self {
        Self { capture, recordings }
    }

    pub fn totals(&self) -> PacketDrops {
        PacketDrops {
            capture_queue: self.capture.rejected_when_full(),
            recording_queue: self.recordings.iter().map(|recording| recording.dropped()).sum(),
        }
    }
}

/// Decides when the drops so far deserve a log line: the first time they are
/// seen, then no more often than once a period, and never when nothing new
/// was dropped.
#[derive(Default)]
pub struct DropReporter {
    reported: PacketDrops,
    last_line_at: Option<Instant>,
}

impl DropReporter {
    /// The line to log now, if there is one. `force` skips the period, for the
    /// last word when the meter stops.
    pub fn line(&mut self, totals: PacketDrops, now: Instant, force: bool) -> Option<String> {
        let new = totals.since(self.reported);
        if new.is_none() {
            return None;
        }
        let waiting = self
            .last_line_at
            .is_some_and(|at| now.saturating_duration_since(at) < REPORT_PERIOD);
        if waiting && !force {
            return None;
        }

        self.reported = totals;
        self.last_line_at = Some(now);
        Some(format!(
            "packets dropped: {} before the dispatcher read them (queue full), {} left out of recordings (queue full); totals {} and {}",
            new.capture_queue, new.recording_queue, totals.capture_queue, totals.recording_queue
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::recording_queue::test_support::{packet, GatedSink};

    fn drops(capture_queue: u64, recording_queue: u64) -> PacketDrops {
        PacketDrops { capture_queue, recording_queue }
    }

    #[test]
    fn logs_nothing_while_nothing_was_dropped() {
        let mut reporter = DropReporter::default();

        assert_eq!(reporter.line(drops(0, 0), Instant::now(), false), None);
        assert_eq!(reporter.line(drops(0, 0), Instant::now(), true), None);
    }

    #[test]
    fn logs_one_line_per_period_however_many_packets_dropped() {
        let mut reporter = DropReporter::default();
        let start = Instant::now();

        let first = reporter.line(drops(3, 0), start, false).expect("first drops are logged");
        assert!(first.contains("3 before the dispatcher"), "{first}");
        assert_eq!(reporter.line(drops(900, 40), start + Duration::from_secs(1), false), None);
        assert_eq!(reporter.line(drops(901, 40), start + Duration::from_secs(29), false), None);

        let second = reporter
            .line(drops(902, 40), start + Duration::from_secs(31), false)
            .expect("a period later the drops since the first line are logged");
        assert!(second.contains("899 before the dispatcher"), "{second}");
        assert!(second.contains("40 left out of recordings"), "{second}");
        assert!(second.contains("totals 902 and 40"), "{second}");
    }

    #[test]
    fn a_forced_line_ignores_the_period_but_not_the_absence_of_new_drops() {
        let mut reporter = DropReporter::default();
        let start = Instant::now();
        reporter.line(drops(1, 0), start, false).expect("first line");

        assert_eq!(reporter.line(drops(1, 0), start, true), None);
        assert!(reporter.line(drops(2, 0), start, true).is_some());
    }

    #[test]
    fn totals_add_the_capture_queue_and_every_recording_queue() {
        let capture = Channel::new(1);
        assert!(capture.try_send(packet(0)));
        assert!(!capture.try_send(packet(1)));
        assert!(!capture.try_send(packet(2)));
        let stalled = GatedSink::blocked();
        let recording = Arc::new(RecordingQueue::new(stalled.clone(), 1));
        recording.offer(&packet(0));
        stalled.wait_for_write_to_start();
        for sequence in 1..=4 {
            recording.offer(&packet(sequence));
        }
        let counters = PacketDropCounters::new(capture, vec![Arc::clone(&recording)]);

        let totals = counters.totals();
        stalled.release();

        assert_eq!(totals, drops(2, 3));
    }
}
