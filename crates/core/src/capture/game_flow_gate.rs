//! Which captured packets are worth queueing for the dispatcher.
//!
//! The capture threads share one bounded queue with the dispatcher. A download
//! on the same adapter would fill it and push game packets out, so packets
//! are kept only from a connection (port pair) that has carried the game's
//! heartbeat within the last [`CANDIDATE_STALE_AFTER`]; web ports never count.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::capture::capturer::CapturedPacket;
use crate::capture::dispatcher::{is_web_flow, MAGIC};

const CANDIDATE_STALE_AFTER: Duration = Duration::from_secs(30);
const CANDIDATE_CLEANUP_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) struct GameFlowGate {
    last_magic_at: HashMap<(u16, u16), Instant>,
    last_cleanup_at: Instant,
}

impl GameFlowGate {
    pub(crate) fn new(now: Instant) -> Self {
        Self { last_magic_at: HashMap::new(), last_cleanup_at: now }
    }

    /// Whether `packet` belongs on the dispatcher's queue. `newly_seen` is
    /// called with the port pair when this packet makes a connection a candidate.
    pub(crate) fn admits(
        &mut self,
        packet: &CapturedPacket,
        now: Instant,
        newly_seen: impl FnOnce((u16, u16)),
    ) -> bool {
        if is_web_flow(packet.src_port, packet.dst_port) {
            return false;
        }
        if now.saturating_duration_since(self.last_cleanup_at) >= CANDIDATE_CLEANUP_INTERVAL {
            self.remove_stale(now);
            self.last_cleanup_at = now;
        }

        let connection = normalized_connection(packet.src_port, packet.dst_port);
        if packet.data.windows(MAGIC.len()).any(|window| window == MAGIC) {
            if self.last_magic_at.insert(connection, now).is_none() {
                newly_seen(connection);
            }
            return true;
        }
        self.last_magic_at.contains_key(&connection)
    }

    fn remove_stale(&mut self, now: Instant) {
        self.last_magic_at
            .retain(|_, last_magic_at| now.saturating_duration_since(*last_magic_at) <= CANDIDATE_STALE_AFTER);
    }
}

fn normalized_connection(src_port: u16, dst_port: u16) -> (u16, u16) {
    if src_port <= dst_port {
        (src_port, dst_port)
    } else {
        (dst_port, src_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEARTBEAT: [u8; 5] = [0x01, 0x0E, 0x00, 0x36, 0x02];

    fn packet(src_port: u16, dst_port: u16, data: &[u8]) -> CapturedPacket {
        CapturedPacket {
            src_ip: [52, 1, 2, 3],
            src_port,
            dst_ip: [192, 168, 1, 20],
            dst_port,
            sequence: 0,
            data: data.to_vec(),
            captured_at: 0.0,
        }
    }

    fn admits(gate: &mut GameFlowGate, packet: &CapturedPacket, now: Instant) -> bool {
        gate.admits(packet, now, |_| {})
    }

    #[test]
    fn traffic_is_kept_from_the_heartbeat_on_in_both_directions_and_not_before() {
        let now = Instant::now();
        let mut gate = GameFlowGate::new(now);

        assert!(!admits(&mut gate, &packet(13_328, 50_000, b"before"), now));
        assert!(admits(&mut gate, &packet(13_328, 50_000, &HEARTBEAT), now));
        assert!(admits(&mut gate, &packet(13_328, 50_000, b"after"), now));
        assert!(admits(&mut gate, &packet(50_000, 13_328, b"reply"), now));
        assert!(!admits(&mut gate, &packet(60_000, 61_000, b"other flow"), now));
    }

    #[test]
    fn web_traffic_is_never_kept_even_with_the_heartbeat_bytes() {
        let now = Instant::now();
        let mut gate = GameFlowGate::new(now);

        assert!(!admits(&mut gate, &packet(443, 50_001, &HEARTBEAT), now));
        assert!(!admits(&mut gate, &packet(443, 50_001, b"download"), now));
        assert!(!admits(&mut gate, &packet(50_002, 80, &HEARTBEAT), now));
    }

    #[test]
    fn a_flow_that_stays_silent_for_over_thirty_seconds_is_forgotten() {
        let start = Instant::now();
        let mut gate = GameFlowGate::new(start);
        admits(&mut gate, &packet(13_328, 50_000, &HEARTBEAT), start);

        assert!(admits(&mut gate, &packet(13_328, 50_000, b"x"), start + Duration::from_secs(30)));
        assert!(!admits(&mut gate, &packet(13_328, 50_000, b"x"), start + Duration::from_secs(32)));
    }

    #[test]
    fn a_flow_is_announced_once() {
        let now = Instant::now();
        let mut gate = GameFlowGate::new(now);
        let mut announced = Vec::new();

        for _ in 0..3 {
            gate.admits(&packet(13_328, 50_000, &HEARTBEAT), now, |connection| announced.push(connection));
        }

        assert_eq!(announced, [(13_328, 50_000)]);
    }
}
