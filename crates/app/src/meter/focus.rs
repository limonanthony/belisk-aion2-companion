//! Which boss the meter has locked onto.
//!
//! The game's "current target" is whatever was hit last, so in a boss fight
//! with adds it jumps between the boss and trash. Once a boss is engaged the
//! Target board stays on that boss, whatever else is hit, until:
//!
//! - it dies (the final summary follows);
//! - you move to another boss: it takes over only after you have hit only it
//!   for [`SWITCH_AFTER`] ([`Release::MovedToAnotherBoss`]);
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

/// Bosses fought together (two sub-bosses, or a sub-boss beside the main
/// boss) must not make the board flip: another boss takes over only after the
/// player has hit only it for this long.
pub const SWITCH_AFTER: Duration = Duration::from_secs(20);

const REMEMBERED_BOSSES: usize = 50;
/// Hits are timestamped to the millisecond; this is well above that.
const NEW_HIT_SECS: f64 = 0.001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Died,
    MovedToAnotherBoss,
    Quiet,
    Gone,
    NoLongerBoss,
    NotYourFight,
    ZoneChange,
    Reset,
    Wiped,
}

impl Release {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Died => "the boss died",
            Self::MovedToAnotherBoss => "you moved to another boss",
            Self::Quiet => "no hit on the boss for the quiet limit",
            Self::Gone => "the boss left the snapshot",
            Self::NoLongerBoss => "the boss is no longer ruled a boss",
            Self::NotYourFight => "not your fight: you neither hit it nor were hit by it",
            Self::ZoneChange => "the local player's actor id changed (zone change)",
            Self::Reset => "the meter was reset",
            Self::Wiped => "the attempt was wiped: the next pull on it is a new fight",
        }
    }
}

/// Why a boss is the local player's fight. Once the game has named the
/// player, only a boss they hit, or that hit them, qualifies: until they hit
/// something, the game's "current target" is anyone's last target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Involvement {
    YouHitIt,
    ItHitYou,
    /// The meter does not know who you are, so it cannot tell.
    Unidentified,
}

impl Involvement {
    pub fn describe(self) -> &'static str {
        match self {
            Self::YouHitIt => "you hit it",
            Self::ItHitYou => "it hit you",
            Self::Unidentified => "you are not identified yet, so anyone's target counts",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Released {
    pub id: u32,
    pub reason: Release,
}

/// What one observation changed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Change {
    pub released: Option<Released>,
    /// A boss was engaged; `released` may name the one it replaced.
    pub engaged: Option<u32>,
    pub engaged_because: Option<Involvement>,
    /// Set when the engagement was a move from another boss: how long (s) the
    /// player had left the old one alone, or the new one had been the target.
    pub moved_after_secs: Option<f64>,
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
    engaged_at: Instant,
    /// A wipe left the lock on this boss of the dungeon, which is not on the
    /// meter until it is hit again; the mob code tells its respawn.
    held_for_mob: Option<u32>,
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
    /// Before you are identified: the boss that has been the current target
    /// without interruption since this moment.
    pending_switch: Option<(u32, Instant)>,
}

impl BossFocus {
    pub fn focused(&self) -> Option<u32> {
        self.focused.map(|focused| focused.id)
    }

    /// A new snapshot. `current` is the game's current target; `bosses` are
    /// the bosses in the snapshot; `facts_of` describes any target;
    /// `involvement_of` says why a boss is the local player's fight, or `None`
    /// if it is not; `my_hit_age_of` is the seconds since the local player's
    /// last hit on a target, `None` if they have not hit it or are unknown.
    pub fn observe(
        &mut self,
        current: Option<u32>,
        bosses: &[u32],
        facts_of: impl Fn(u32) -> Option<TargetFacts>,
        involvement_of: impl Fn(u32) -> Option<Involvement>,
        my_hit_age_of: impl Fn(u32) -> Option<f64>,
        now: Instant,
    ) -> Change {
        let mut change = Change::default();

        if let Some(focused) = &mut self.focused {
            let id = focused.id;
            let release = match facts_of(id) {
                None if focused.held_for_mob.is_some() => None,
                None => Some(Release::Gone),
                Some(facts) if !facts.is_boss => Some(Release::NoLongerBoss),
                Some(facts) if facts.killed() => Some(Release::Died),
                Some(_) if involvement_of(id).is_none() => Some(Release::NotYourFight),
                Some(facts) => {
                    focused.held_for_mob = None;
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

        let finished = &self.finished;
        let usable = |id: u32| {
            let facts = facts_of(id)?;
            (facts.is_boss && !facts.killed() && may_engage(finished, &facts))
                .then(|| involvement_of(id).map(|why| (facts, why)))
                .flatten()
        };

        match self.focused {
            Some(focused) => {
                let switch = switch_target(
                    focused,
                    &mut self.pending_switch,
                    current,
                    bosses,
                    &usable,
                    &my_hit_age_of,
                    now,
                );
                if let Some((facts, why, idle_secs)) = switch {
                    self.release(Release::MovedToAnotherBoss, focused.duration_secs, &mut change);
                    self.engage(&facts, why, now, &mut change);
                    change.moved_after_secs = Some(idle_secs);
                }
            }
            None => {
                let current_boss = current.and_then(&usable);
                let recent_boss = || {
                    bosses
                        .iter()
                        .filter_map(|&id| {
                            let (facts, why) = usable(id)?;
                            let age = my_hit_age_of(id).filter(|age| *age < SWITCH_AFTER.as_secs_f64())?;
                            (why == Involvement::YouHitIt).then_some((facts, why, age))
                        })
                        .min_by(|a, b| a.2.total_cmp(&b.2))
                        .map(|(facts, why, _)| (facts, why))
                };
                if let Some((facts, why)) = current_boss.or_else(recent_boss) {
                    self.engage(&facts, why, now, &mut change);
                }
            }
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

    /// The attempt on `id` was wiped. The next pull on it is a new fight, so
    /// nothing about this one is remembered. In a dungeon (`hold_for_mob`, the
    /// boss's mob code) the lock stays on the boss, as it does until a kill;
    /// otherwise it is let go.
    pub fn wiped(&mut self, id: u32, hold_for_mob: Option<u32>, now: Instant) -> Change {
        let mut change = Change::default();
        self.finished.retain(|finished| finished.id != id);
        if let Some(focused) = self.focused.as_mut().filter(|focused| focused.id == id) {
            if hold_for_mob.is_some() {
                focused.held_for_mob = hold_for_mob;
                focused.duration_secs = 0.0;
                focused.last_hit_at = now;
            } else {
                let duration_secs = focused.duration_secs;
                self.release(Release::Wiped, duration_secs, &mut change);
            }
        }
        change
    }

    /// The mob code of the boss the lock is held for after a wipe, until it
    /// is on the meter again.
    pub fn held_for_mob(&self) -> Option<u32> {
        self.focused.and_then(|focused| focused.held_for_mob)
    }

    /// The boss of a wiped dungeon attempt came back as a new entity: the
    /// lock moves to it.
    pub fn relock(&mut self, facts: &TargetFacts, why: Involvement, now: Instant) -> Change {
        let mut change = Change::default();
        if let Some(focused) = self.focused {
            self.release(Release::Wiped, focused.duration_secs, &mut change);
        }
        self.engage(facts, why, now, &mut change);
        change
    }


    fn release(&mut self, reason: Release, duration_secs: f64, change: &mut Change) {
        let Some(focused) = self.focused.take() else {
            return;
        };
        // Not remembered: it was never the player's fight, the player only
        // walked away from it, or its attempt was wiped; any of them may
        // become their fight again.
        if !matches!(reason, Release::NotYourFight | Release::MovedToAnotherBoss | Release::Wiped) {
            self.finished.retain(|finished| finished.id != focused.id);
            self.finished.push_back(Finished {
                id: focused.id,
                died: reason == Release::Died,
                duration_secs,
            });
            if self.finished.len() > REMEMBERED_BOSSES {
                self.finished.pop_front();
            }
        }
        change.released = Some(Released { id: focused.id, reason });
    }

    fn engage(&mut self, facts: &TargetFacts, why: Involvement, now: Instant, change: &mut Change) {
        self.focused = Some(Focused {
            id: facts.id,
            duration_secs: facts.duration_secs,
            last_hit_at: now,
            engaged_at: now,
            held_for_mob: None,
        });
        self.pending_switch = None;
        change.engaged = Some(facts.id);
        change.engaged_because = Some(why);
    }

}

/// Another boss takes over from `focused` once you have hit only it for
/// [`SWITCH_AFTER`]. Returns it with how long you had left the focused one
/// alone.
fn switch_target(
    focused: Focused,
    pending_switch: &mut Option<(u32, Instant)>,
    current: Option<u32>,
    bosses: &[u32],
    usable: &impl Fn(u32) -> Option<(TargetFacts, Involvement)>,
    my_hit_age_of: &impl Fn(u32) -> Option<f64>,
    now: Instant,
) -> Option<(TargetFacts, Involvement, f64)> {
    let switch_after = SWITCH_AFTER.as_secs_f64();
    let engaged_for = now.saturating_duration_since(focused.engaged_at).as_secs_f64();
    let idle_on_focused = my_hit_age_of(focused.id).unwrap_or(f64::INFINITY).min(engaged_for);

    let mut unidentified_candidate = None;
    let mut best: Option<(TargetFacts, Involvement, f64)> = None;
    for &id in bosses.iter().filter(|&&id| id != focused.id) {
        let Some((facts, why)) = usable(id) else {
            continue;
        };
        if why == Involvement::Unidentified {
            if current == Some(id) {
                unidentified_candidate = Some((facts, why));
            }
            continue;
        }
        let Some(age) = my_hit_age_of(id) else {
            continue;
        };
        let leads = age < idle_on_focused && age < switch_after;
        if leads && idle_on_focused >= switch_after && best.as_ref().map_or(true, |(_, _, seen)| age < *seen) {
            best = Some((facts, why, age));
        }
    }
    if let Some((facts, why, _)) = best {
        return Some((facts, why, idle_on_focused));
    }

    // Nothing is known about your own hits: B must stay the current target,
    // uninterrupted, for the same time.
    match unidentified_candidate {
        Some((facts, why)) => {
            let since = match *pending_switch {
                Some((id, since)) if id == facts.id => since,
                _ => {
                    *pending_switch = Some((facts.id, now));
                    now
                }
            };
            let held = now.saturating_duration_since(since);
            (held >= SWITCH_AFTER).then_some((facts, why, held.as_secs_f64()))
        }
        None => {
            *pending_switch = None;
            None
        }
    }
}

fn may_engage(finished: &VecDeque<Finished>, facts: &TargetFacts) -> bool {
    finished
        .iter()
        .find(|finished| finished.id == facts.id)
        .map_or(true, |finished| {
            !finished.died && facts.duration_secs > finished.duration_secs + NEW_HIT_SECS
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    const BOSS: u32 = 10;
    const OTHER_BOSS: u32 = 11;
    const TRASH: u32 = 1;

    #[derive(Default)]
    struct World {
        facts: HashMap<u32, TargetFacts>,
        /// Seconds since the local player's last hit on each target.
        ages: HashMap<u32, f64>,
    }

    impl World {
        fn set(&mut self, id: u32, is_boss: bool, hp: u32, secs: f64) {
            self.facts.insert(
                id,
                TargetFacts { id, is_boss, current_hp: Some(hp), max_hp: Some(1_000), duration_secs: secs },
            );
        }

        fn observe(&self, focus: &mut BossFocus, current: u32, now: Instant) -> Change {
            self.observe_involved(focus, current, Some(Involvement::YouHitIt), now)
        }

        fn observe_involved(
            &self,
            focus: &mut BossFocus,
            current: u32,
            involved: Option<Involvement>,
            now: Instant,
        ) -> Change {
            let bosses: Vec<u32> = self.facts.values().filter(|f| f.is_boss).map(|f| f.id).collect();
            focus.observe(
                Some(current),
                &bosses,
                |id| self.facts.get(&id).copied(),
                |_| involved,
                |id| self.ages.get(&id).copied(),
                now,
            )
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

    fn two_bosses() -> (BossFocus, World, Instant) {
        let start = Instant::now();
        let (focus, mut world) = engaged_on_boss(start);
        world.set(OTHER_BOSS, true, 1_000, 1.0);
        (focus, world, start)
    }

    #[test]
    fn alternating_hits_between_two_bosses_never_switch() {
        let (mut focus, mut world, start) = two_bosses();

        for second in 1..=90u64 {
            let on_boss = second % 2 == 0;
            world.ages.insert(BOSS, if on_boss { 0.0 } else { 3.0 });
            world.ages.insert(OTHER_BOSS, if on_boss { 3.0 } else { 0.0 });
            let current = if on_boss { BOSS } else { OTHER_BOSS };
            let change = world.observe(&mut focus, current, at(start, second));
            assert!(!change.any(), "switched at {second}s");
        }

        assert_eq!(focus.focused(), Some(BOSS));
    }

    #[test]
    fn twenty_seconds_of_hitting_only_the_other_boss_moves_the_board() {
        let (mut focus, mut world, start) = two_bosses();
        world.ages.insert(BOSS, 19.0);
        world.ages.insert(OTHER_BOSS, 1.0);
        assert!(!world.observe(&mut focus, OTHER_BOSS, at(start, 19)).any());

        world.ages.insert(BOSS, 20.0);
        let change = world.observe(&mut focus, OTHER_BOSS, at(start, 20));

        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::MovedToAnotherBoss }));
        assert_eq!(change.engaged, Some(OTHER_BOSS));
        assert_eq!(change.moved_after_secs, Some(20.0));
        assert_eq!(focus.focused(), Some(OTHER_BOSS));
    }

    #[test]
    fn a_hit_on_the_focused_boss_restarts_the_count() {
        let (mut focus, mut world, start) = two_bosses();
        world.ages.insert(BOSS, 19.0);
        world.ages.insert(OTHER_BOSS, 1.0);
        world.observe(&mut focus, OTHER_BOSS, at(start, 19));

        // A hit on the first boss at 19 s.
        world.ages.insert(BOSS, 5.0);
        world.ages.insert(OTHER_BOSS, 6.0);
        assert!(!world.observe(&mut focus, BOSS, at(start, 24)).any());

        world.ages.insert(BOSS, 21.0);
        world.ages.insert(OTHER_BOSS, 1.0);
        assert_eq!(world.observe(&mut focus, OTHER_BOSS, at(start, 40)).engaged, Some(OTHER_BOSS));
    }

    #[test]
    fn a_boss_that_only_hit_you_counts_idle_from_its_engagement() {
        let (mut focus, mut world, start) = two_bosses();
        world.ages.insert(OTHER_BOSS, 1.0);

        assert!(!world.observe(&mut focus, OTHER_BOSS, at(start, 10)).any());
        assert!(world.observe(&mut focus, OTHER_BOSS, at(start, 20)).engaged.is_some());
    }

    #[test]
    fn the_boss_dying_hands_over_to_the_other_at_once() {
        let (mut focus, mut world, start) = two_bosses();
        world.ages.insert(OTHER_BOSS, 2.0);

        world.set(BOSS, true, 0, 40.0);
        let change = world.observe(&mut focus, OTHER_BOSS, at(start, 40));

        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::Died }));
        assert_eq!(change.engaged, Some(OTHER_BOSS));
    }

    #[test]
    fn after_the_death_the_boss_you_hit_last_engages_even_if_the_target_is_trash() {
        let (mut focus, mut world, start) = two_bosses();
        world.ages.insert(OTHER_BOSS, 2.0);

        world.set(BOSS, true, 0, 40.0);
        let change = world.observe(&mut focus, TRASH, at(start, 40));

        assert_eq!(change.engaged, Some(OTHER_BOSS));
    }

    #[test]
    fn a_boss_walked_away_from_is_not_finished_and_can_be_moved_back_to() {
        let (mut focus, mut world, start) = two_bosses();
        world.ages.insert(BOSS, 25.0);
        world.ages.insert(OTHER_BOSS, 1.0);
        world.observe(&mut focus, OTHER_BOSS, at(start, 25));
        assert_eq!(focus.focused(), Some(OTHER_BOSS));

        world.ages.insert(BOSS, 1.0);
        world.ages.insert(OTHER_BOSS, 30.0);
        let change = world.observe(&mut focus, BOSS, at(start, 60));

        assert_eq!(change.engaged, Some(BOSS));
        assert_eq!(
            change.released,
            Some(Released { id: OTHER_BOSS, reason: Release::MovedToAnotherBoss })
        );
    }

    #[test]
    fn unidentified_the_other_boss_must_stay_the_current_target_for_the_whole_time() {
        let (mut focus, world, start) = two_bosses();
        let unidentified = Some(Involvement::Unidentified);

        assert!(!world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 1)).any());
        assert!(!world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 20)).any());
        // The first boss is the target again: the count starts over.
        assert!(!world.observe_involved(&mut focus, BOSS, unidentified, at(start, 21)).any());
        assert!(!world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 22)).any());
        assert!(!world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 41)).any());

        let change = world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 42));
        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::MovedToAnotherBoss }));
        assert_eq!(change.engaged, Some(OTHER_BOSS));
        assert_eq!(change.moved_after_secs, Some(20.0));
    }

    #[test]
    fn unidentified_a_trash_target_in_between_restarts_the_count() {
        let (mut focus, mut world, start) = two_bosses();
        world.set(TRASH, false, 100, 5.0);
        let unidentified = Some(Involvement::Unidentified);

        world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 1));
        world.observe_involved(&mut focus, TRASH, unidentified, at(start, 10));

        assert!(!world.observe_involved(&mut focus, OTHER_BOSS, unidentified, at(start, 25)).any());
    }

    #[test]
    fn a_boss_missing_from_the_snapshot_is_released() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        world.facts.remove(&BOSS);
        let change = world.observe(&mut focus, TRASH, at(start, 5));

        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::Gone }));
    }

    #[test]
    fn a_wiped_boss_is_released_and_its_next_pull_is_a_new_fight_even_when_shorter() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);
        world.set(BOSS, true, 900, 120.0);
        world.observe(&mut focus, BOSS, at(start, 1));

        let change = focus.wiped(BOSS, None, at(start, 2));
        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::Wiped }));
        assert_eq!(focus.focused(), None);

        world.set(BOSS, true, 900, 4.0);
        assert_eq!(world.observe(&mut focus, BOSS, at(start, 30)).engaged, Some(BOSS));
    }

    #[test]
    fn in_a_dungeon_a_wipe_keeps_the_lock_through_trash_and_a_respawned_boss_takes_it() {
        let start = Instant::now();
        let (mut focus, mut world) = engaged_on_boss(start);

        let change = focus.wiped(BOSS, Some(2_300_243), at(start, 2));
        assert!(change.released.is_none());
        assert_eq!(focus.focused(), Some(BOSS));
        world.facts.remove(&BOSS);
        world.set(TRASH, false, 100, 8.0);
        let change = world.observe(&mut focus, TRASH, at(start, 5));
        assert!(!change.any(), "trash between attempts does not take the lock");
        assert_eq!(focus.focused(), Some(BOSS));
        assert_eq!(focus.held_for_mob(), Some(2_300_243));

        world.set(OTHER_BOSS, true, 1_000, 2.0);
        let facts = world.facts[&OTHER_BOSS];
        let change = focus.relock(&facts, Involvement::YouHitIt, at(start, 40));
        assert_eq!(change.engaged, Some(OTHER_BOSS));
        assert_eq!(focus.focused(), Some(OTHER_BOSS));
        assert_eq!(focus.held_for_mob(), None);
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

    #[test]
    fn a_boss_nobody_involved_you_in_is_not_engaged() {
        let mut world = World::default();
        world.set(BOSS, true, 900, 3.0);
        let mut focus = BossFocus::default();

        let change = world.observe_involved(&mut focus, BOSS, None, Instant::now());

        assert!(!change.any());
    }

    #[test]
    fn the_reason_for_engaging_is_reported() {
        for why in [Involvement::YouHitIt, Involvement::ItHitYou, Involvement::Unidentified] {
            let mut world = World::default();
            world.set(BOSS, true, 900, 3.0);
            let mut focus = BossFocus::default();

            let change = world.observe_involved(&mut focus, BOSS, Some(why), Instant::now());

            assert_eq!(change.engaged_because, Some(why));
        }
    }

    #[test]
    fn losing_your_involvement_releases_the_boss_and_a_later_hit_engages_it_again() {
        let start = Instant::now();
        let mut world = World::default();
        world.set(BOSS, true, 900, 3.0);
        let mut focus = BossFocus::default();
        world.observe_involved(&mut focus, BOSS, Some(Involvement::Unidentified), start);

        let change = world.observe_involved(&mut focus, BOSS, None, start);
        assert_eq!(change.released, Some(Released { id: BOSS, reason: Release::NotYourFight }));

        let change = world.observe_involved(&mut focus, BOSS, Some(Involvement::YouHitIt), start);
        assert_eq!(change.engaged, Some(BOSS), "not remembered as finished");
    }
}
