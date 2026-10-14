use std::sync::Mutex;

const MARKER: [u8; 4] = [0x03, 0x36, 0x00, 0x00];
const DOTNET_EPOCH_OFFSET_MS: i64 = 62_135_596_800_000;
const MIN_PING_RS_BYTES: usize = 12;
const MAX_PING_MS: f64 = 1000.0;

#[derive(Debug, Default)]
pub struct PingTracker {
    last_ping_ms: Mutex<Option<f64>>,
}

impl PingTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_packet(&self, data: &[u8], captured_at_seconds: f64) {
        if data.len() < MIN_PING_RS_BYTES {
            return;
        }

        let mut start = 0usize;
        while start + MIN_PING_RS_BYTES <= data.len() {
            let Some(relative_idx) = data[start..]
                .windows(MARKER.len())
                .position(|window| window == MARKER)
            else {
                break;
            };

            let idx = start + relative_idx;
            if idx + MIN_PING_RS_BYTES > data.len() {
                break;
            }

            let client_sent_raw = i64::from_le_bytes([
                data[idx + 4],
                data[idx + 5],
                data[idx + 6],
                data[idx + 7],
                data[idx + 8],
                data[idx + 9],
                data[idx + 10],
                data[idx + 11],
            ]);
            let client_sent_unix_ms = client_sent_raw - DOTNET_EPOCH_OFFSET_MS;
            let arrival_ms = captured_at_seconds * 1000.0;
            let ping_ms = (arrival_ms - client_sent_unix_ms as f64).round();

            if (10.0..=MAX_PING_MS).contains(&ping_ms) {
                *self.last_ping_ms.lock().unwrap() = Some(ping_ms);
            }

            start = idx + MIN_PING_RS_BYTES;
        }
    }

    pub fn current_ping_ms(&self) -> Option<f64> {
        *self.last_ping_ms.lock().unwrap()
    }

    pub fn reset(&self) {
        *self.last_ping_ms.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARRIVED_AT_SECONDS: f64 = 1_700_000_000.0;

    fn ping_reply_sent_ms_before_arrival(ms: i64) -> Vec<u8> {
        let sent_dotnet_ms = (ARRIVED_AT_SECONDS * 1000.0) as i64 - ms + DOTNET_EPOCH_OFFSET_MS;
        let mut data = MARKER.to_vec();
        data.extend_from_slice(&sent_dotnet_ms.to_le_bytes());
        data
    }

    #[test]
    fn the_ping_is_the_time_between_the_game_sending_the_reply_and_its_capture() {
        let tracker = PingTracker::new();

        tracker.on_packet(&ping_reply_sent_ms_before_arrival(42), ARRIVED_AT_SECONDS);

        assert_eq!(tracker.current_ping_ms(), Some(42.0));
    }

    #[test]
    fn a_reset_forgets_the_ping_of_the_fight_before() {
        let tracker = PingTracker::new();
        tracker.on_packet(&ping_reply_sent_ms_before_arrival(42), ARRIVED_AT_SECONDS);

        tracker.reset();

        assert_eq!(tracker.current_ping_ms(), None);
    }
}
