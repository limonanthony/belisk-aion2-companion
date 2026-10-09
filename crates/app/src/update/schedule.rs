use std::time::{Duration, SystemTime};

/// GitHub allows 60 unauthenticated requests an hour per address: one check an
/// hour from each PC leaves room for every PC behind the same address.
pub const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);
/// "Check now" does nothing again until this has passed since the last check.
pub const CHECK_NOW_EVERY: Duration = Duration::from_secs(60);
/// A wait GitHub asks for is kept within these bounds.
const SHORTEST_WAIT: Duration = Duration::from_secs(60);
const LONGEST_WAIT: Duration = Duration::from_secs(24 * 60 * 60);

/// When the next check may happen. It is kept as wall-clock times and looked at
/// every few seconds by the worker, so a PC that slept through the due time
/// checks right after it wakes.
#[derive(Debug)]
pub struct Schedule {
    next_automatic: SystemTime,
    last_attempt: Option<SystemTime>,
    held_until: Option<SystemTime>,
}

impl Schedule {
    /// The first automatic check is due at once: that is the check at launch.
    pub fn at_launch(now: SystemTime) -> Self {
        Self { next_automatic: now, last_attempt: None, held_until: None }
    }

    /// True when the hourly check is due. A clock set back far enough that the
    /// next check would be over a day away is taken as a new hour from now.
    pub fn automatic_due(&mut self, now: SystemTime) -> bool {
        if self.next_automatic > now + LONGEST_WAIT {
            self.next_automatic = now + CHECK_EVERY;
        }
        now >= self.next_automatic && self.held_until.is_none_or(|until| now >= until)
    }

    /// When the next automatic check is expected.
    pub fn next_check_at(&self) -> SystemTime {
        self.held_until.map_or(self.next_automatic, |until| until.max(self.next_automatic))
    }

    /// How long until the next automatic check may be due, at most `limit`.
    pub fn wait(&self, now: SystemTime, limit: Duration) -> Duration {
        self.next_check_at().duration_since(now).unwrap_or(Duration::ZERO).min(limit)
    }

    pub fn can_check_now(&self, now: SystemTime) -> bool {
        now >= self.check_now_available_at()
    }

    pub fn check_now_available_at(&self) -> SystemTime {
        let throttled = self.last_attempt.map(|attempt| attempt + CHECK_NOW_EVERY);
        [throttled, self.held_until].into_iter().flatten().max().unwrap_or(SystemTime::UNIX_EPOCH)
    }

    /// A check ran (whatever it found): the next automatic one is an hour away.
    pub fn attempted(&mut self, now: SystemTime) {
        self.last_attempt = Some(now);
        self.next_automatic = now + CHECK_EVERY;
        self.held_until = self.held_until.filter(|until| *until > now);
    }

    /// GitHub asked to wait until `until`; nothing is asked before then, however
    /// the check was started. Returns the time actually waited for.
    pub fn hold_until(&mut self, now: SystemTime, until: Option<SystemTime>) -> SystemTime {
        let until = until.unwrap_or(now + CHECK_EVERY).clamp(now + SHORTEST_WAIT, now + LONGEST_WAIT);
        self.held_until = Some(until);
        self.next_automatic = self.next_automatic.max(until);
        until
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + seconds)
    }

    #[test]
    fn the_first_check_is_due_at_launch_and_the_next_one_an_hour_later() {
        let mut schedule = Schedule::at_launch(at(0));
        assert!(schedule.automatic_due(at(0)));

        schedule.attempted(at(2));

        assert!(!schedule.automatic_due(at(3_601)));
        assert!(schedule.automatic_due(at(2 + 3_600)));
    }

    #[test]
    fn a_pc_that_slept_past_the_due_time_checks_as_soon_as_it_is_looked_at_again() {
        let mut schedule = Schedule::at_launch(at(0));
        schedule.attempted(at(0));

        assert!(!schedule.automatic_due(at(1_800)));
        assert!(schedule.automatic_due(at(10 * 3_600)), "woke ten hours later");
    }

    #[test]
    fn the_worker_never_waits_longer_than_the_limit_and_never_negative() {
        let mut schedule = Schedule::at_launch(at(0));
        schedule.attempted(at(0));
        let limit = Duration::from_secs(30);

        assert_eq!(schedule.wait(at(0), limit), limit);
        assert_eq!(schedule.wait(at(3_590), limit), Duration::from_secs(10));
        assert_eq!(schedule.wait(at(9_999), limit), Duration::ZERO);
    }

    #[test]
    fn check_now_waits_a_minute_after_any_check() {
        let mut schedule = Schedule::at_launch(at(0));
        assert!(schedule.can_check_now(at(0)), "nothing checked yet");

        schedule.attempted(at(100));

        assert!(!schedule.can_check_now(at(159)));
        assert!(schedule.can_check_now(at(160)));
        assert_eq!(schedule.check_now_available_at(), at(160));
    }

    #[test]
    fn a_rate_limit_holds_automatic_and_manual_checks_until_the_time_github_gave() {
        let mut schedule = Schedule::at_launch(at(0));
        schedule.attempted(at(0));

        let until = schedule.hold_until(at(10), Some(at(5_000)));

        assert_eq!(until, at(5_000));
        assert!(!schedule.can_check_now(at(4_999)));
        assert!(schedule.can_check_now(at(5_000)));
        assert!(!schedule.automatic_due(at(4_999)));
        assert!(schedule.automatic_due(at(5_000)), "later than the hourly time, so it waits for the reset");
    }

    #[test]
    fn a_rate_limit_without_a_usable_time_waits_an_hour_and_a_silly_one_is_bounded() {
        let mut schedule = Schedule::at_launch(at(0));
        assert_eq!(schedule.hold_until(at(0), None), at(3_600));
        assert_eq!(schedule.hold_until(at(0), Some(at(0))), at(60), "a reset already past still waits a minute");
        assert_eq!(schedule.hold_until(at(0), Some(at(10_000_000))), at(86_400));
    }

    #[test]
    fn a_clock_set_far_back_does_not_postpone_checks_for_days() {
        let mut schedule = Schedule::at_launch(at(0));
        schedule.attempted(at(10 * 86_400));

        assert!(!schedule.automatic_due(at(0)));
        assert!(schedule.automatic_due(at(3_600)));
    }
}
