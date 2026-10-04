use std::time::{Duration, Instant};

/// Bars grow and shrink, and rows trade places, over 240 ms (tokens.md).
pub const TWEEN: Duration = Duration::from_millis(240);

/// Changes smaller than this land at once rather than starting a tween, so a
/// bar that barely moved does not cost a quarter second of frames.
const SETTLE: f32 = 0.002;

/// A value easing out towards its latest target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween {
    from: f32,
    to: f32,
    started: Instant,
    duration: Duration,
}

impl Tween {
    pub fn at(value: f32, now: Instant) -> Self {
        Self::lasting(TWEEN, value, now)
    }

    /// A tween with its own duration, for motion tokens other than 240 ms.
    pub fn lasting(duration: Duration, value: f32, now: Instant) -> Self {
        Self { from: value, to: value, started: now, duration }
    }

    /// Heads for `to` from wherever the value is now. With `animate` off, or
    /// for a negligible change, the value lands immediately.
    pub fn retarget(&mut self, to: f32, now: Instant, animate: bool) {
        if to == self.to {
            return;
        }
        let current = self.value(now);
        let from = if animate && (to - current).abs() >= SETTLE { current } else { to };
        *self = Self { from, to, started: now, duration: self.duration };
    }

    pub fn value(&self, now: Instant) -> f32 {
        let progress = self.progress(now);
        if progress >= 1.0 {
            return self.to;
        }
        let eased = 1.0 - (1.0 - progress).powi(3);
        self.from + (self.to - self.from) * eased
    }

    pub fn is_running(&self, now: Instant) -> bool {
        self.progress(now) < 1.0
    }

    fn progress(&self, now: Instant) -> f32 {
        if self.from == self.to {
            return 1.0;
        }
        now.saturating_duration_since(self.started).as_secs_f32() / self.duration.as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tween_eases_out_to_its_target_and_stops() {
        let start = Instant::now();
        let mut tween = Tween::at(0.2, start);
        assert!(!tween.is_running(start));
        tween.retarget(0.8, start, true);
        assert_eq!(tween.value(start), 0.2);
        let halfway = tween.value(start + TWEEN / 2);
        assert!(halfway > 0.5 && halfway < 0.8, "ease-out is past the midpoint: {halfway}");
        assert!(tween.is_running(start + TWEEN / 2));
        assert_eq!(tween.value(start + TWEEN), 0.8);
        assert!(!tween.is_running(start + TWEEN));
    }

    #[test]
    fn retargeting_mid_flight_starts_from_the_current_value() {
        let start = Instant::now();
        let mut tween = Tween::at(0.0, start);
        tween.retarget(1.0, start, true);
        let mid = start + TWEEN / 2;
        let current = tween.value(mid);
        tween.retarget(0.5, mid, true);
        assert_eq!(tween.value(mid), current);
        assert_eq!(tween.value(mid + TWEEN), 0.5);
    }

    #[test]
    fn small_changes_and_reduced_motion_land_at_once() {
        let start = Instant::now();
        let mut tween = Tween::at(0.5, start);
        tween.retarget(0.501, start, true);
        assert!(!tween.is_running(start));
        tween.retarget(0.9, start, false);
        assert_eq!(tween.value(start), 0.9);
        assert!(!tween.is_running(start));
    }
}
