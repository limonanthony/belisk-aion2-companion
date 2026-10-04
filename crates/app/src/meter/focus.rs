//! Which boss the meter has locked onto.
//!
//! The game's "current target" is whatever was hit last, so in a boss fight
//! with adds it jumps between the boss and trash. Once a boss is engaged the
//! Target board stays on that boss, whatever else is hit, until:
//!
//! - it dies (the final summary follows);
//! - another boss is engaged;
//! - it was left alone for [`QUIET_RELEASE`] (the provisional summary
//!   follows: it may only be between phases);
//! - it is gone from the snapshot, or the user ruled it is not a boss;
//! - the meter is reset, or the local player's actor id changes, which is how
//!   a zone change shows (ids are per-zone handles). The core parses no
//!   despawn packet, so nothing more precise is known.
//!
//! A boss that died is never engaged again. One released for being quiet is
//! engaged again once someone hits it.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::fight::TargetFacts;

/// Clearing the adds other players pulled can take long, so only this long
/// without a single hit on the boss lets go of it.
pub const QUIET_RELEASE: Duration = Duration::from_secs(120);

const REMEMBERED_BOSSES: usize = 50;
/// Hits are timestamped to the millisecond; this is well above that.
const NEW_HIT_SECS: f64 = 0.001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Died,
    DifferentBoss,
    Quiet,
    Gone,
    NoLongerBoss,
    ZoneChange,
    Reset,
}

impl Release {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Died => "the boss died",
            Self::DifferentBoss => "another boss was engaged",
            Self::Quiet => "no hit on the boss for the quiet limit",
            Self::Gone => "the boss left the snapshot",
            Self::NoLongerBoss => "the boss is no longer ruled a boss",
            Self::ZoneChange => "the local player's actor id changed (zone change)",
            Self::Reset => "the meter was reset",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Released {
    pub id: u32,
    pub reason: Release,
}

/// What one observation changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Change {
    pub released: Option<Released>,
    /// A boss was engaged; `released` may name the one it replaced.
    pub engaged: Option<u32>,
}

impl Change {
    pub fn any(&self) -> bool {
        self.released.is_some() || self.engaged.is_some()
    }
}

#[derive(Clone, Copy, Debug)]
struct Focused {
    id: u32,
    /// Fight length when the boss was last hit, to notice the next hit.
    duration_secs: f64,
    last_hit_at: Instant,
}

#[derive(Clone, Copy, Debug)]
struct Finished {
    id: u32,
    died: bool,
    duration_secs: f64,
}

#[derive(Default)]
pub struct BossFocus {
    focused: Option<Focused>,
    finished: VecDeque<Finished>,
}

impl BossFocus {
    pub fn focused(&self) -> Option<u32> {
        self.focused.map(|focused| focused.id)
    }

    /// A new snapshot. `current` is the game's current target; `facts_of`
    /// describes any target in the snapshot.
    pub fn observe(
        &mut self,
        current: Option<u32>,
        facts_of: impl Fn(u32) -> Option<TargetFacts>,
        now: Instant,
    ) -> Change {
        let mut change = Change::default();

        if let Some(focused) = &mut self.focused {
            let id = focused.id;
            let release = match facts_of(id) {
                None => Some(Release::Gone),
                Some(facts) if !facts.is_boss => Some(Release::NoLongerBoss),
                Some(facts) if facts.killed() => Some(Release::Died),
                Some(facts) => {
                    if facts.duration_secs > focused.duration_secs + NEW_HIT_SECS {
                        focused.duration_secs = facts.duration_secs;
                        focused.last_hit_at = now;
                    }
                    None
                }
            };
            if let Some(reason) = release {
                let duration_secs = facts_of(id).map_or(focused.duration_secs, |f| f.duration_secs);
                self.release(reason, duration_secs, &mut change);
            }
        }

        let candidate = current
            .and_then(&facts_of)
            .filter(|facts| facts.is_boss && !facts.killed() && Some(facts.id) != self.focused());
        if let Some(facts) = candidate.filter(|facts| self.may_engage(facts)) {
            if let Some(replaced) = self.focused {
                self.release(Release::DifferentBoss, replaced.duration_secs, &mut change);
            }
            self.focused = Some(Focused {
                id: facts.id,
                duration_secs: facts.duration_secs,
                last_hit_at: now,
            });
            change.engaged = Some(facts.id);
        }
        change
    }

    /// Time has passed with no snapshot: lets go of a boss nobody has hit for
    /// [`QUIET_RELEASE`].
    pub fn advance(&mut self, now: Instant) -> Change {
        let mut change = Change::default();
        if let Some(focused) = self.focused {
            if now >= focused.last_hit_at + QUIET_RELEASE {
                self.release(Release::Quiet, focused.duration_secs, &mut change);
            }
        }
        change
    }

    /// When [`advance`](Self::advance) next has something to do.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.focused.map(|focused| focused.last_hit_at + QUIET_RELEASE)
    }

    /// The meter was reset, or a zone change made every boss unreachable.
    pub fn release_all(&mut self, reason: Release) -> Change {
        let mut change = Change::default();
        if let Some(focused) = self.focused {
            self.release(reason, focused.duration_secs, &mut change);
        }
        if reason == Release::Reset {
            self.finished.clear();
        }
        change
    }

    fn may_engage(&self, facts: &TargetFacts) -> bool {
        self.finished
            .iter()
            .find(|finished| finished.id == facts.id)
            .map_or(true, |finished| {
                !finished.died && facts.duration_secs > finished.duration_secs + NEW_HIT_SECS
            })
    }

    fn release(&mut self, reason: Release, duration_secs: f64, change: &mut Change) {
        let Some(focused) = self.focused.take() else {
            return;
        };
        self.finished.retain(|finished| finished.id != focused.id);
        self.finished.push_back(Finished {
            id: focused.id,
            died: reason == Release::Died,
            duration_secs,
        });
        if self.finished.len() > REMEMBERED_BOSSES {
            self.finished.pop_front();
        }
        change.released = Some(Released { id: focused.id, reason });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    const BOSS: u32 = 10;
    const OTHER_BOSS: u32 = 11;
    const TRASH: u32 = 1;

    #[derive(Default)]
    struct World(HashMap<u32, TargetFacts>);

    impl World {
        fn set(&mut self, id: u32, is_boss: bool, hp: u32, secs: f64) {
            self.0.insert(
                id,
                TargetFacts { id, is_boss, current_hp: Some(hp), max_hp: Some(1_000), duration_secs: secs },
            );
        }

        fn observe(&self, focus: &mut BossFocus, current: u32, now: Instant) -> Change {
            focus.observe(Some(current), |id| self.0.get(&id).copied(), now)
        }
    }

    fn at(start: Instant, secs: u64) -> Instant {
        start + Duration::from_secs(secs)
    }

    fn engaged_on_boss(start: Instant) -> (BossFocus, World) {
        let mut world = World::default();
        world.set(TRASH, false, 100, 5.0);
        world.set(BOSS, true, 900, 3.0);
        let mut focus = BossFocus::default();
        assert_eq!(world.observe(&mut focus, BOSS, start).engaged, Some(BOSS));
        (focus, world)
    }

    #[test]
    fn trash_is_not_a_boss_to_engage() {
        let mut world = World::default();
        world.set(TRASH, false, 100, 5.0);
        let mut focus = BossFocus::default();

        let change = world.observe(&mut focus, TRASH, Instant::now());

        assert!(!change.any());
        assert_eq!(focus.focused(), None);
    }

    #[test]
    fn hitting_trash_meanwhile_keeps_the_boss() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        for second in 1..=60 {
            world.set(TRASH, false, 100, 5.0 + second as f64);
            let change = world.observe(&mut focus, TRASH, at(start, second));
            assert!(!change.any());
        }

        assert_eq!(focus.focused(), Some(BOSS));
        assert!(focus.advance(at(start, 60)).released.is_none());
    }

    #[test]
    fn two_minutes_without_a_hit_on_the_boss_release_it() {
        let start = Instant::now();
        let (mut focus, _) = engaged_on_boss(start);

        assert!(focus.advance(at(start, 119)).released.is_none());
        assert_eq!(focus.next_deadline(), Some(start + QUIET_RELEASE));

        let change = focus.advance(at(start, 120));
        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::Quiet }));
        assert_eq!(focus.focused(), None);
    }

    #[test]
    fn a_hit_on_the_boss_restarts_the_quiet_count() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        world.set(BOSS, true, 800, 90.0);
        world.observe(&mut focus, TRASH, at(start, 100));

        assert!(focus.advance(at(start, 150)).released.is_none());
        assert!(focus.advance(at(start, 220)).released.is_some());
    }

    #[test]
    fn a_quiet_boss_is_engaged_again_when_someone_hits_it() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);
        focus.advance(at(start, 120));

        assert!(!world.observe(&mut focus, BOSS, at(start, 121)).any(), "no new hit yet");

        world.set(BOSS, true, 700, 130.0);
        assert_eq!(world.observe(&mut focus, BOSS, at(start, 130)).engaged, Some(BOSS));
    }

    #[test]
    fn a_dead_boss_is_released_and_never_engaged_again() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        world.set(BOSS, true, 0, 40.0);
        let change = world.observe(&mut focus, BOSS, at(start, 40));
        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::Died }));
        assert_eq!(change.engaged, None);
        assert_eq!(focus.focused(), None);

        assert!(!world.observe(&mut focus, BOSS, at(start, 41)).any());
        world.set(BOSS, true, 0, 50.0);
        assert!(!world.observe(&mut focus, BOSS, at(start, 50)).any());
    }

    #[test]
    fn a_boss_already_dead_when_first_seen_is_not_engaged() {
        let mut world = World::default();
        world.set(BOSS, true, 0, 40.0);
        let mut focus = BossFocus::default();

        assert!(!world.observe(&mut focus, BOSS, Instant::now()).any());
    }

    #[test]
    fn a_second_boss_takes_over() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        world.set(OTHER_BOSS, true, 1_000, 1.0);
        let change = world.observe(&mut focus, OTHER_BOSS, at(start, 30));

        assert_eq!(change.engaged, Some(OTHER_BOSS));
        assert_eq!(focus.focused(), Some(OTHER_BOSS));
    }

    #[test]
    fn a_boss_missing_from_the_snapshot_is_released() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        world.0.remove(&BOSS);
        let change = world.observe(&mut focus, TRASH, at(start, 5));

        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::Gone }));
    }

    #[test]
    fn a_boss_the_user_rules_out_is_released() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        world.set(BOSS, false, 900, 4.0);
        let change = world.observe(&mut focus, BOSS, at(start, 5));

        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::NoLongerBoss }));
        assert_eq!(focus.focused(), None);
    }

    #[test]
    fn a_reset_or_a_zone_change_releases_the_boss_and_a_reset_forgets_the_dead() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);
        let change = focus.release_all(Release::ZoneChange);
        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::ZoneChange }));

        world.set(BOSS, true, 0, 40.0);
        world.observe(&mut focus, BOSS, start);
        focus.release_all(Release::Reset);
        world.set(BOSS, true, 900, 1.0);
        assert_eq!(world.observe(&mut focus, BOSS, start).engaged, Some(BOSS));
    }
}
