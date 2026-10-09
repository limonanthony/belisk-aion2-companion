//! A boss attempt that ends without a kill: the boss's health goes back to
//! full after the party hurt it, or the boss leaves the field alive.
//!
//! The game sends no event for a wipe, and players cannot be counted on to be
//! dead together (stones revive on the spot), so the boss is the witness. In a
//! dungeon the boss entity is removed with flag 7 and a new one spawns at full
//! health; elsewhere its health may climb back.

use std::collections::HashMap;

/// Health that fell below this share of the maximum is a lost attempt when it
/// returns to full. A boss that only regenerated what a few stray hits took
/// (99.8%, seen on a 160M-health world boss) is not.
const WIPE_BELOW_PERCENT: u64 = 98;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeCause {
    HealthRestored,
    LeftTheField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossWipe {
    pub target_id: u32,
    /// The lowest the boss's health got in the attempt.
    pub hp_left: u64,
    pub max_hp: u64,
    pub cause: WipeCause,
}

/// Follows each boss's lowest health in the attempt and holds the wipes the
/// meter has not closed yet.
#[derive(Debug, Default)]
pub(crate) struct WipeWatch {
    /// The lowest health of the attempt and the maximum it was read against.
    lowest_hp: HashMap<u32, (u64, u64)>,
    pending: Vec<BossWipe>,
}

impl WipeWatch {
    pub(crate) fn note_health(&mut self, target_id: u32, current: u64, max: u64) {
        if current == 0 {
            self.forget(target_id);
            return;
        }
        // At the start of a pull the game sends a short maximum and then fills
        // it up (87% or 95.2% back to 100% within 150 ms): a changed maximum
        // restarts the watch and that reading is not a loss.
        if self.lowest_hp.get(&target_id).is_some_and(|&(_, known_max)| known_max != max) {
            self.lowest_hp.remove(&target_id);
            return;
        }
        let (lowest, _) = self.lowest_hp.entry(target_id).or_insert((current, max));
        *lowest = (*lowest).min(current);
        let lowest = *lowest;
        if max > 0 && current >= max && lowest * 100 <= max * WIPE_BELOW_PERCENT {
            self.lowest_hp.remove(&target_id);
            self.push(BossWipe { target_id, hp_left: lowest, max_hp: max, cause: WipeCause::HealthRestored });
        }
    }

    pub(crate) fn note_left_the_field(&mut self, target_id: u32, current: u64, max: u64) {
        if current == 0 {
            return;
        }
        let lowest = self.lowest_hp.remove(&target_id).map_or(current, |(lowest, _)| lowest.min(current));
        self.push(BossWipe { target_id, hp_left: lowest, max_hp: max, cause: WipeCause::LeftTheField });
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(crate) fn take_pending(&mut self) -> Vec<BossWipe> {
        std::mem::take(&mut self.pending)
    }

    /// The boss died or its attempt was taken off: nothing is left to close.
    pub(crate) fn forget(&mut self, target_id: u32) {
        self.lowest_hp.remove(&target_id);
        self.pending.retain(|wipe| wipe.target_id != target_id);
    }

    fn push(&mut self, wipe: BossWipe) {
        if !self.pending.iter().any(|pending| pending.target_id == wipe.target_id) {
            self.pending.push(wipe);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOSS: u32 = 7;
    const MAX: u64 = 1_000;

    fn wipes_after(health: &[u64]) -> Vec<BossWipe> {
        let mut watch = WipeWatch::default();
        for &current in health {
            watch.note_health(BOSS, current, MAX);
        }
        watch.take_pending()
    }

    #[test]
    fn health_back_to_full_after_a_real_loss_is_a_wipe_with_the_lowest_health_left() {
        let wipes = wipes_after(&[900, 500, 300, 320, 1_000]);

        assert_eq!(
            wipes,
            vec![BossWipe { target_id: BOSS, hp_left: 300, max_hp: MAX, cause: WipeCause::HealthRestored }]
        );
    }

    #[test]
    fn healing_part_of_the_way_back_is_not_a_wipe() {
        assert!(wipes_after(&[900, 500, 700, 840]).is_empty());
    }

    #[test]
    fn regenerating_a_scratch_is_not_a_wipe() {
        assert!(wipes_after(&[999, 998, 1_000]).is_empty());
    }

    #[test]
    fn a_boss_that_dies_is_not_a_wipe() {
        assert!(wipes_after(&[500, 100, 0, 1_000]).is_empty());
    }

    #[test]
    fn the_attempt_starts_over_after_a_wipe() {
        let mut watch = WipeWatch::default();
        for current in [400, 1_000] {
            watch.note_health(BOSS, current, MAX);
        }
        assert_eq!(watch.take_pending().len(), 1);

        for current in [990, 1_000] {
            watch.note_health(BOSS, current, MAX);
        }
        assert!(watch.take_pending().is_empty(), "the scratch of the new attempt is not a wipe");
    }

    #[test]
    fn leaving_the_field_alive_is_a_wipe_and_dead_is_not() {
        let mut watch = WipeWatch::default();
        watch.note_health(BOSS, 300, MAX);
        watch.note_left_the_field(BOSS, 320, MAX);
        watch.note_left_the_field(8, 0, MAX);

        assert_eq!(
            watch.take_pending(),
            vec![BossWipe { target_id: BOSS, hp_left: 300, max_hp: MAX, cause: WipeCause::LeftTheField }]
        );
    }

    #[test]
    fn a_wipe_is_held_once_until_taken() {
        let mut watch = WipeWatch::default();
        watch.note_left_the_field(BOSS, 300, MAX);
        watch.note_left_the_field(BOSS, 300, MAX);

        assert!(watch.has_pending());
        assert_eq!(watch.take_pending().len(), 1);
        assert!(!watch.has_pending());
    }

    #[test]
    fn a_death_cancels_a_wipe_not_yet_closed_in_either_order() {
        let mut left = WipeWatch::default();
        left.note_left_the_field(BOSS, 300, MAX);
        left.forget(BOSS);
        assert!(left.take_pending().is_empty());

        let mut restored = WipeWatch::default();
        restored.note_health(BOSS, 300, MAX);
        restored.note_health(BOSS, MAX, MAX);
        restored.note_health(BOSS, 0, MAX);
        assert!(restored.take_pending().is_empty());
    }

    #[test]
    fn the_maximum_filling_up_at_the_start_of_a_pull_is_not_a_wipe() {
        let mut watch = WipeWatch::default();
        watch.note_health(BOSS, 10_540_000, 10_540_000);
        watch.note_health(BOSS, 10_540_000, 11_067_000);
        watch.note_health(BOSS, 11_067_000, 11_067_000);

        assert!(watch.take_pending().is_empty());

        watch.note_health(BOSS, 1_757_191, 11_067_000);
        watch.note_health(BOSS, 11_067_000, 11_067_000);
        assert_eq!(watch.take_pending().len(), 1, "a real loss after the fill-up still counts");
    }
}
