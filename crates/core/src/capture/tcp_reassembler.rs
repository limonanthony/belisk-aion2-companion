use std::collections::HashMap;

const MAX_HELD_BYTES: usize = 2 * 1024 * 1024;
const MAX_GAP_WAIT_SECS: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TcpFlowKey {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub dst_ip: [u8; 4],
    pub dst_port: u16,
}

#[derive(Debug)]
struct PendingSegment {
    sequence: u32,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapSkip {
    pub missing_bytes: u32,
    pub waited_secs: f64,
    pub held_bytes: usize,
}

#[derive(Debug, Default, PartialEq)]
pub struct Reassembly {
    pub segments: Vec<Vec<u8>>,
    /// Set when the stream jumped over a hole: the bytes before `segments`
    /// are missing, so a consumer must drop any half-parsed frame.
    pub gap_skip: Option<GapSkip>,
}

#[derive(Debug, Default)]
pub struct TcpReassembler {
    next_sequence: Option<u32>,
    pending: HashMap<u32, Vec<u8>>,
    held_bytes: usize,
    retransmits: u64,
    gap_skips: u64,
    waiting_since: Option<f64>,
}

impl TcpReassembler {
    pub fn feed(&mut self, sequence: u32, data: Vec<u8>, captured_at: f64) -> Reassembly {
        if data.is_empty() {
            return Reassembly::default();
        }

        let Some(next_sequence) = self.next_sequence else {
            self.next_sequence = Some(sequence.wrapping_add(data.len() as u32));
            return Reassembly {
                segments: vec![data],
                gap_skip: None,
            };
        };

        let Some(segment) = trim_consumed_prefix(sequence, data, next_sequence) else {
            self.retransmits += 1;
            return Reassembly::default();
        };

        if segment.sequence == next_sequence {
            let mut emitted = vec![segment.data];
            self.advance_and_drain(&mut emitted);
            self.waiting_since = (!self.pending.is_empty()).then_some(captured_at);
            return Reassembly {
                segments: emitted,
                gap_skip: None,
            };
        }

        self.insert_pending(segment);
        let waiting_since = *self.waiting_since.get_or_insert(captured_at);
        let waited_secs = captured_at - waiting_since;
        if self.held_bytes > MAX_HELD_BYTES || waited_secs >= MAX_GAP_WAIT_SECS {
            return self.skip_gap(next_sequence, waited_secs, captured_at);
        }

        Reassembly::default()
    }

    fn skip_gap(&mut self, next_sequence: u32, waited_secs: f64, captured_at: f64) -> Reassembly {
        let Some(nearest_sequence) = self.nearest_pending_sequence(next_sequence) else {
            return Reassembly::default();
        };
        let gap_skip = GapSkip {
            missing_bytes: nearest_sequence.wrapping_sub(next_sequence),
            waited_secs,
            held_bytes: self.held_bytes,
        };
        self.gap_skips += 1;
        self.next_sequence = Some(nearest_sequence);
        let mut emitted = Vec::new();
        self.drain_contiguous(&mut emitted);
        self.waiting_since = (!self.pending.is_empty()).then_some(captured_at);
        Reassembly {
            segments: emitted,
            gap_skip: Some(gap_skip),
        }
    }

    pub fn retransmits(&self) -> u64 {
        self.retransmits
    }

    pub fn gap_skips(&self) -> u64 {
        self.gap_skips
    }

    pub fn next_sequence(&self) -> Option<u32> {
        self.next_sequence
    }

    pub fn pending_segments(&self) -> usize {
        self.pending.len()
    }

    pub fn held_bytes(&self) -> usize {
        self.held_bytes
    }

    fn advance_and_drain(&mut self, emitted: &mut Vec<Vec<u8>>) {
        if let Some(last) = emitted.last() {
            let next = self
                .next_sequence
                .unwrap_or_default()
                .wrapping_add(last.len() as u32);
            self.next_sequence = Some(next);
        }
        self.drain_contiguous(emitted);
    }

    fn drain_contiguous(&mut self, emitted: &mut Vec<Vec<u8>>) {
        loop {
            let next = self.next_sequence.unwrap_or_default();
            let Some((sequence, data)) = self.take_segment_covering(next) else {
                break;
            };
            self.held_bytes = self.held_bytes.saturating_sub(data.len());
            let consumed = next.wrapping_sub(sequence) as usize;
            if consumed >= data.len() {
                self.retransmits += 1;
                continue;
            }
            let remaining = if consumed == 0 {
                data
            } else {
                self.retransmits += 1;
                data[consumed..].to_vec()
            };
            self.next_sequence = Some(next.wrapping_add(remaining.len() as u32));
            emitted.push(remaining);
        }
    }

    fn take_segment_covering(&mut self, next_sequence: u32) -> Option<(u32, Vec<u8>)> {
        let fully_consumed: Vec<u32> = self
            .pending
            .iter()
            .filter_map(|(sequence, data)| {
                let relative = sequence.wrapping_sub(next_sequence) as i32;
                let consumed = next_sequence.wrapping_sub(*sequence) as usize;
                (relative < 0 && consumed >= data.len()).then_some(*sequence)
            })
            .collect();
        for sequence in fully_consumed {
            if let Some(data) = self.pending.remove(&sequence) {
                self.held_bytes = self.held_bytes.saturating_sub(data.len());
                self.retransmits += 1;
            }
        }

        let sequence = self
            .pending
            .iter()
            .filter_map(|(sequence, data)| {
                let relative = sequence.wrapping_sub(next_sequence) as i32;
                let distance = next_sequence.wrapping_sub(*sequence) as usize;
                if *sequence == next_sequence || (relative < 0 && distance < data.len()) {
                    Some((*sequence, distance))
                } else {
                    None
                }
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(sequence, _)| sequence)?;
        self.pending.remove(&sequence).map(|data| (sequence, data))
    }

    fn insert_pending(&mut self, segment: PendingSegment) {
        if let Some(previous) = self.pending.get(&segment.sequence) {
            if previous.len() >= segment.data.len() {
                self.retransmits += 1;
                return;
            }
            self.held_bytes = self.held_bytes.saturating_sub(previous.len());
        }

        self.held_bytes += segment.data.len();
        self.pending.insert(segment.sequence, segment.data);
    }

    fn nearest_pending_sequence(&self, next_sequence: u32) -> Option<u32> {
        self.pending
            .keys()
            .copied()
            .min_by_key(|sequence| sequence.wrapping_sub(next_sequence))
    }
}

fn trim_consumed_prefix(
    sequence: u32,
    data: Vec<u8>,
    next_sequence: u32,
) -> Option<PendingSegment> {
    let distance = sequence.wrapping_sub(next_sequence) as i32;
    if distance >= 0 {
        return Some(PendingSegment { sequence, data });
    }

    let consumed = next_sequence.wrapping_sub(sequence) as usize;
    if consumed >= data.len() {
        return None;
    }

    Some(PendingSegment {
        sequence: next_sequence,
        data: data[consumed..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::{GapSkip, TcpReassembler};

    fn feed(reassembler: &mut TcpReassembler, sequence: u32, data: Vec<u8>) -> Vec<Vec<u8>> {
        reassembler.feed(sequence, data, 0.0).segments
    }

    #[test]
    fn holds_out_of_order_data_until_gap_arrives() {
        let mut reassembler = TcpReassembler::default();
        assert_eq!(feed(&mut reassembler, 100, vec![1, 2]), vec![vec![1, 2]]);
        assert!(feed(&mut reassembler, 104, vec![5, 6]).is_empty());
        assert_eq!(
            feed(&mut reassembler, 102, vec![3, 4]),
            vec![vec![3, 4], vec![5, 6]]
        );
    }

    #[test]
    fn drops_retransmits_and_trims_partial_overlap() {
        let mut reassembler = TcpReassembler::default();
        assert_eq!(
            feed(&mut reassembler, 100, vec![1, 2, 3, 4]),
            vec![vec![1, 2, 3, 4]]
        );
        assert!(feed(&mut reassembler, 100, vec![1, 2, 3, 4]).is_empty());
        assert_eq!(
            feed(&mut reassembler, 102, vec![3, 4, 5, 6]),
            vec![vec![5, 6]]
        );
    }

    #[test]
    fn handles_sequence_wrap() {
        let mut reassembler = TcpReassembler::default();
        assert_eq!(
            feed(&mut reassembler, u32::MAX - 1, vec![1, 2]),
            vec![vec![1, 2]]
        );
        assert_eq!(feed(&mut reassembler, 0, vec![3, 4]), vec![vec![3, 4]]);
    }

    #[test]
    fn trims_overlap_from_a_previously_held_segment() {
        let mut reassembler = TcpReassembler::default();
        assert_eq!(feed(&mut reassembler, 100, vec![1, 2]), vec![vec![1, 2]]);
        assert!(feed(&mut reassembler, 104, vec![5, 6, 7, 8]).is_empty());
        assert_eq!(
            feed(&mut reassembler, 102, vec![3, 4, 5, 6]),
            vec![vec![3, 4, 5, 6], vec![7, 8]]
        );
    }

    #[test]
    fn removes_a_held_segment_fully_covered_by_later_data() {
        let mut reassembler = TcpReassembler::default();
        assert_eq!(feed(&mut reassembler, 100, vec![1, 2]), vec![vec![1, 2]]);
        assert!(feed(&mut reassembler, 106, vec![7, 8]).is_empty());
        assert!(feed(&mut reassembler, 104, vec![5, 6]).is_empty());
        assert_eq!(
            feed(&mut reassembler, 102, vec![3, 4, 5, 6, 7, 8]),
            vec![vec![3, 4, 5, 6, 7, 8]]
        );
        assert!(feed(&mut reassembler, 108, vec![9])
            .iter()
            .any(|part| part == &[9]));
    }

    #[test]
    fn resumes_after_a_lost_segment_once_the_capture_clock_passes_the_timeout() {
        let mut reassembler = TcpReassembler::default();
        assert_eq!(
            reassembler.feed(100, vec![1, 2], 10.0).segments,
            vec![vec![1, 2]]
        );
        assert!(reassembler.feed(104, vec![5, 6], 10.1).segments.is_empty());
        assert!(reassembler.feed(106, vec![7, 8], 12.9).segments.is_empty());

        let resumed = reassembler.feed(108, vec![9], 13.1);
        assert_eq!(resumed.segments, vec![vec![5, 6], vec![7, 8], vec![9]]);
        let gap_skip = resumed.gap_skip.expect("the hole is skipped");
        assert_eq!(gap_skip.missing_bytes, 2);
        assert!((gap_skip.waited_secs - 3.0).abs() < 1e-9);
        assert_eq!(reassembler.gap_skips(), 1);
        assert_eq!(reassembler.pending_segments(), 0);

        let next = reassembler.feed(109, vec![10], 13.2);
        assert_eq!(next.segments, vec![vec![10]]);
        assert_eq!(next.gap_skip, None);
    }

    #[test]
    fn reorders_segments_that_arrive_within_the_timeout() {
        let mut reassembler = TcpReassembler::default();
        reassembler.feed(100, vec![1, 2], 10.0);
        assert!(reassembler.feed(104, vec![5, 6], 10.1).segments.is_empty());
        assert!(reassembler.feed(106, vec![7, 8], 12.5).segments.is_empty());

        let filled = reassembler.feed(102, vec![3, 4], 12.9);
        assert_eq!(filled.segments, vec![vec![3, 4], vec![5, 6], vec![7, 8]]);
        assert_eq!(filled.gap_skip, None);
        assert_eq!(reassembler.gap_skips(), 0);
    }

    #[test]
    fn progress_restarts_the_wait_for_a_second_hole() {
        let mut reassembler = TcpReassembler::default();
        reassembler.feed(100, vec![1, 2], 10.0);
        assert!(reassembler.feed(104, vec![5, 6], 10.1).segments.is_empty());
        assert!(reassembler.feed(108, vec![9, 10], 11.0).segments.is_empty());

        let filled = reassembler.feed(102, vec![3, 4], 12.0);
        assert_eq!(filled.segments, vec![vec![3, 4], vec![5, 6]]);
        assert!(reassembler.feed(110, vec![11], 14.9).segments.is_empty());
        assert_eq!(reassembler.gap_skips(), 0);

        let skipped = reassembler.feed(111, vec![13], 15.1);
        assert_eq!(skipped.segments, vec![vec![9, 10], vec![11], vec![13]]);
        assert_eq!(skipped.gap_skip.map(|skip| skip.missing_bytes), Some(2));
    }

    #[test]
    fn still_skips_a_hole_once_held_bytes_pass_the_cap() {
        let mut reassembler = TcpReassembler::default();
        reassembler.feed(100, vec![1, 2], 10.0);
        assert!(reassembler
            .feed(104, vec![0; 1024 * 1024], 10.0)
            .segments
            .is_empty());
        let resumed = reassembler.feed(104 + 1024 * 1024, vec![0; 1024 * 1024 + 1], 10.0);
        assert_eq!(resumed.segments.len(), 2);
        assert!(matches!(
            resumed.gap_skip,
            Some(GapSkip { missing_bytes: 2, .. })
        ));
    }
}
