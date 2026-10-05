//! When the fight summary goes up and comes down. Ported from `trackFight`
//! and `maybeShowSummary` in the Tauri overlay (`overlay/meter/main.js`).
//!
//! A kill is final. A boss fight that has gone quiet is provisional, because
//! the boss may only be between phases: if the fight resumes the summary comes
//! down and the fight can be summarised again, and a kill that follows gets
//! its own final summary.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const SHOWN_FOR: Duration = Duration::from_secs(60);
const MIN_FIGHT_SECS: f64 = 15.0;
const REMEMBERED_TARGETS: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryKind {
    Final,
    Provisional,
}

/// What the tracker needs to know about the current target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetFacts {
    pub id: u32,
    pub is_boss: bool,
    pub current_hp: Option<u32>,
    pub max_hp: Option<u32>,
    pub duration_secs: f64,
}

impl TargetFacts {
    pub(super) fn killed(&self) -> bool {
        self.current_hp == Some(0) && self.max_hp.is_some_and(|max| max > 0)
    }

    fn alive(&self) -> bool {
        self.current_hp.is_some_and(|hp| hp > 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShownSummary {
    pub target_id: u32,
    pub kind: SummaryKind,
    duration_secs: f64,
    hide_at: Instant,
    coexisting: Option<u32>,
}

pub struct FightTracker {
    enabled: bool,
    defeated: RecentTargets,
    quiet: RecentTargets,
    shown: Option<ShownSummary>,
}

impl FightTracker {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            defeated: RecentTargets::default(),
            quiet: RecentTargets::default(),
            shown: None,
        }
    }

    /// Switching summaries off takes the one that is up down with it.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.shown = None;
        }
    }

    pub fn shown(&self) -> Option<ShownSummary> {
        self.shown
    }

    /// The boss that took over from the one whose summary is up was already
    /// being fought when that one died, so it does not take the summary down.
    pub fn coexist_with(&mut self, boss: u32) {
        if let Some(shown) = &mut self.shown {
            shown.coexisting = Some(boss);
        }
    }

    /// A new snapshot. Returns whether the summary that is up changed.
    pub fn observe(&mut self, target: Option<TargetFacts>, now: Instant) -> bool {
        let before = self.key();
        let Some(target) = target else {
            self.shown = None;
            return before != self.key();
        };

        if let Some(shown) = self.shown {
            let same = target.id == shown.target_id;
            let resumed =
                same && target.alive() && target.duration_secs > shown.duration_secs + 1.0;
            if resumed {
                self.quiet.forget(target.id);
                self.shown = None;
            } else if !same && target.is_boss && shown.coexisting != Some(target.id) {
                self.shown = None;
            }
        }

        if self.enabled && target.is_boss && target.killed() {
            self.try_show(target, SummaryKind::Final, now);
        }
        before != self.key()
    }

    /// Time has passed. Returns whether the summary that is up changed.
    pub fn advance(&mut self, now: Instant) -> bool {
        let before = self.key();
        if self.shown.is_some_and(|shown| now >= shown.hide_at) {
            self.shown = None;
        }
        before != self.key()
    }

    /// The boss has been left alone for good: it was not killed, and may only
    /// be between phases. Returns whether the summary that is up changed.
    pub fn quiet(&mut self, target: TargetFacts, now: Instant) -> bool {
        let before = self.key();
        if self.enabled {
            self.try_show(target, SummaryKind::Provisional, now);
        }
        before != self.key()
    }

    pub fn dismiss(&mut self) -> bool {
        self.shown.take().is_some()
    }

    /// When `advance` next has something to do.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.shown.map(|shown| shown.hide_at)
    }

    fn try_show(&mut self, target: TargetFacts, kind: SummaryKind, now: Instant) {
        if self.defeated.contains(target.id)
            || (kind == SummaryKind::Provisional && self.quiet.contains(target.id))
            || target.duration_secs < MIN_FIGHT_SECS
        {
            return;
        }
        match kind {
            SummaryKind::Final => self.defeated.remember(target.id),
            SummaryKind::Provisional => self.quiet.remember(target.id),
        }
        // The most recent kill is the one to show: a boss dying while the
        // summary of an earlier kill is up takes its place.
        if kind == SummaryKind::Final || self.shown.is_none() {
            self.shown = Some(ShownSummary {
                target_id: target.id,
                kind,
                duration_secs: target.duration_secs,
                hide_at: now + SHOWN_FOR,
                coexisting: None,
            });
        }
    }

    fn key(&self) -> Option<(u32, SummaryKind)> {
        self.shown.map(|shown| (shown.target_id, shown.kind))
    }
}

#[derive(Default)]
struct RecentTargets(VecDeque<u32>);

impl RecentTargets {
    fn contains(&self, id: u32) -> bool {
        self.0.contains(&id)
    }

    fn remember(&mut self, id: u32) {
        self.0.push_back(id);
        if self.0.len() > REMEMBERED_TARGETS {
            self.0.pop_front();
        }
    }

    fn forget(&mut self, id: u32) {
        if let Some(index) = self.0.iter().position(|&known| known == id) {
            self.0.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boss(id: u32, hp: u32, secs: f64) -> Option<TargetFacts> {
        Some(TargetFacts {
            id,
            is_boss: true,
            current_hp: Some(hp),
            max_hp: Some(1_000),
            duration_secs: secs,
        })
    }

    fn kind(tracker: &FightTracker) -> Option<SummaryKind> {
        tracker.shown().map(|shown| shown.kind)
    }

    #[test]
    fn a_kill_after_fifteen_seconds_is_final() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        assert!(!tracker.observe(boss(7, 400, 30.0), now));
        assert!(tracker.observe(boss(7, 0, 31.0), now));
        assert_eq!(kind(&tracker), Some(SummaryKind::Final));
    }

    #[test]
    fn short_fights_and_other_targets_get_no_summary() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 0, 9.0), now);
        assert_eq!(kind(&tracker), None);

        let mob = TargetFacts { is_boss: false, ..boss(8, 0, 60.0).unwrap() };
        tracker.observe(Some(mob), now);
        assert_eq!(kind(&tracker), None);
        assert_eq!(tracker.next_deadline(), None);
    }

    #[test]
    fn a_boss_left_alone_is_provisional_and_a_kill_replaces_it() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 400, 40.0), now);
        assert_eq!(tracker.next_deadline(), None);
        assert!(tracker.quiet(boss(7, 400, 40.0).unwrap(), now));
        assert_eq!(kind(&tracker), Some(SummaryKind::Provisional));

        assert!(tracker.observe(boss(7, 0, 40.5), now + Duration::from_secs(16)));
        assert_eq!(kind(&tracker), Some(SummaryKind::Final));
    }

    #[test]
    fn a_resumed_fight_takes_the_summary_down_and_can_end_again() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.quiet(boss(7, 400, 40.0).unwrap(), now);
        assert!(tracker.observe(boss(7, 380, 42.0), now + Duration::from_secs(20)));
        assert_eq!(kind(&tracker), None);

        tracker.quiet(boss(7, 380, 42.0).unwrap(), now + Duration::from_secs(200));
        assert_eq!(kind(&tracker), Some(SummaryKind::Provisional));
    }

    #[test]
    fn a_quiet_fight_is_summarised_once_until_it_resumes() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.quiet(boss(7, 400, 40.0).unwrap(), now);
        tracker.dismiss();

        tracker.quiet(boss(7, 400, 40.0).unwrap(), now + Duration::from_secs(200));
        assert_eq!(kind(&tracker), None);
    }

    #[test]
    fn a_defeated_boss_is_never_summarised_again() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 0, 30.0), now);
        tracker.dismiss();
        tracker.observe(boss(7, 0, 30.0), now);
        tracker.quiet(boss(7, 0, 30.0).unwrap(), now);
        assert_eq!(kind(&tracker), None);
    }

    #[test]
    fn other_targets_do_not_take_a_summary_down() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 0, 30.0), now);
        let mob = TargetFacts { is_boss: false, ..boss(8, 300, 5.0).unwrap() };
        assert!(!tracker.observe(Some(mob), now));
        assert_eq!(kind(&tracker), Some(SummaryKind::Final));
    }


    #[test]
    fn a_later_kill_replaces_the_summary_of_an_earlier_one() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 0, 30.0), now);
        assert_eq!(tracker.shown().map(|shown| shown.target_id), Some(7));

        tracker.observe(boss(9, 0, 40.0), now);

        assert_eq!(tracker.shown().map(|shown| (shown.target_id, shown.kind)), Some((9, SummaryKind::Final)));
    }
    #[test]
    fn the_next_boss_or_no_target_takes_the_summary_down() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 0, 30.0), now);
        assert!(tracker.observe(boss(9, 1_000, 1.0), now));
        assert_eq!(kind(&tracker), None);

        tracker.observe(boss(9, 0, 30.0), now);
        assert!(tracker.observe(None, now));
        assert_eq!(kind(&tracker), None);
    }

    #[test]
    fn a_summary_comes_down_after_a_minute() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(true);
        tracker.observe(boss(7, 0, 30.0), now);
        assert_eq!(tracker.next_deadline(), Some(now + SHOWN_FOR));
        assert!(tracker.advance(now + SHOWN_FOR));
        assert_eq!(kind(&tracker), None);
        assert_eq!(tracker.next_deadline(), None);
    }

    #[test]
    fn summaries_turned_off_never_show() {
        let now = Instant::now();
        let mut tracker = FightTracker::new(false);
        tracker.observe(boss(7, 0, 30.0), now);
        tracker.quiet(boss(8, 400, 30.0).unwrap(), now);
        assert_eq!(kind(&tracker), None);
    }
}
