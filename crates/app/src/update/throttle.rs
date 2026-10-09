use std::time::{Duration, SystemTime};

/// The download never goes faster than this, fight or not (1 MB/s).
pub const MAX_BYTES_PER_SECOND: u64 = 1_000_000;

/// The longest one wait is allowed to be, so a clock that jumps cannot stall
/// the download.
const LONGEST_PAUSE: Duration = Duration::from_secs(1);

/// Spaces reads so the average rate stays within a limit. Time spent paused
/// earns no credit: after a pause the download resumes at the limit, not in a
/// burst.
#[derive(Debug)]
pub struct Pacer {
    bytes_per_second: u64,
    free_at: SystemTime,
}

impl Pacer {
    pub fn new(bytes_per_second: u64, now: SystemTime) -> Self {
        Self { bytes_per_second, free_at: now }
    }

    /// `bytes` were just read: how long to wait before reading again.
    pub fn after_read(&mut self, bytes: usize, now: SystemTime) -> Duration {
        let taken = Duration::from_secs_f64(bytes as f64 / self.bytes_per_second as f64);
        self.free_at = self.free_at.max(now) + taken;
        self.free_at.duration_since(now).unwrap_or(Duration::ZERO).min(LONGEST_PAUSE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(millis: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_000_000 + millis)
    }

    /// Reads `chunk` bytes at a time for `total` bytes, sleeping as told; the
    /// seconds it took.
    fn seconds_to_read(total: usize, chunk: usize, pause_after: Option<(usize, Duration)>) -> f64 {
        let start = at(0);
        let mut now = start;
        let mut pacer = Pacer::new(MAX_BYTES_PER_SECOND, now);
        let mut read = 0;
        while read < total {
            read += chunk;
            now += pacer.after_read(chunk, now);
            if let Some((when, how_long)) = pause_after {
                if read == when {
                    now += how_long;
                }
            }
        }
        now.duration_since(start).unwrap().as_secs_f64()
    }

    #[test]
    fn four_megabytes_take_at_least_four_seconds() {
        let seconds = seconds_to_read(4_000_000, 64 * 1024, None);
        assert!(seconds >= 4.0 * 0.98, "{seconds}");
        assert!(seconds < 4.5, "{seconds}");
    }

    #[test]
    fn the_rate_never_exceeds_the_limit_over_any_prefix() {
        let mut now = at(0);
        let start = now;
        let mut pacer = Pacer::new(MAX_BYTES_PER_SECOND, now);
        let mut read = 0u64;
        for _ in 0..200 {
            read += 16 * 1024;
            now += pacer.after_read(16 * 1024, now);
            let elapsed = now.duration_since(start).unwrap().as_secs_f64();
            assert!(read as f64 <= MAX_BYTES_PER_SECOND as f64 * elapsed + 1.0, "{read} bytes by {elapsed}s");
        }
    }

    #[test]
    fn a_pause_earns_no_credit_for_a_burst_afterwards() {
        let steady = seconds_to_read(2_000_000, 64 * 1024, None);
        let paused = seconds_to_read(2_000_000, 64 * 1024, Some((64 * 1024 * 10, Duration::from_secs(120))));

        assert!(paused >= steady + 119.0, "the pause itself is on top: {steady} vs {paused}");
        assert!(paused <= steady + 121.0, "and nothing is made up after it: {steady} vs {paused}");
    }

    #[test]
    fn a_clock_that_jumps_back_cannot_stall_the_download_for_long() {
        let mut pacer = Pacer::new(MAX_BYTES_PER_SECOND, at(0));
        let wait = pacer.after_read(64 * 1024, at(0) - Duration::from_secs(10_000));
        assert!(wait <= LONGEST_PAUSE);
    }
}
