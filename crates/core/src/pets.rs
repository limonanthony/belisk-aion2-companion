//! What the game said about the local player's pet collection, tied to the
//! character it said it about, and the rule that turns souls into levels. The
//! events carry the game's numbers untouched; `PetCollection` applies them on
//! the game's Global rule and is shared by the app and packet-explorer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::capture::parser::pet_record::{CounterGain, PetSnapshot, PetUnlock, SoulGain};
use crate::character_binding::{CharacterBinder, CharacterReport};
use crate::pet_sources::SoulSource;

pub type PetReport = CharacterReport<PetContent>;
pub type PetBinder = CharacterBinder<PetContent>;

#[derive(Debug, Clone, PartialEq)]
pub enum PetContent {
    /// The whole collection, sent once per login.
    Snapshot(PetSnapshot),
    /// Souls gained, one entry per pet; each amount is a delta. `source` is
    /// what the traffic before the message proves about where they came from.
    SoulGains { gains: Vec<SoulGain>, source: SoulSource },
    /// A pet obtained for the first time.
    Unlock(PetUnlock),
    /// Gains of the six counters the snapshot lists. A pet already at its cap
    /// gets its souls reported only this way, with no pet id; `looted_mobs`
    /// are the mobs whose loot the player picked up just before it, each once
    /// (empty when the gain did not follow a pickup of a known mob's loot).
    CounterGains { gains: Vec<CounterGain>, looted_mobs: Vec<u32> },
}

/// Souls a pet needs to go from level `n` to level `n + 1`; the pet is at its
/// cap after the last one. Souls past a level carry over into the next.
pub const SOULS_PER_LEVEL: [u32; 3] = [5, 25, 75];
pub const MAX_LEVEL: u32 = SOULS_PER_LEVEL.len() as u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetState {
    pub level: u32,
    /// Souls toward the next level; always 0 at the cap.
    pub souls: u32,
}

impl PetState {
    /// A pet at `level` with `souls` collected since: the shape the game's
    /// snapshot lists. A level beyond the cap is the cap, and souls beyond what
    /// a level needs carry over.
    pub fn from_game(level: u32, souls: u32) -> Self {
        Self { level: level.min(MAX_LEVEL), souls: 0 }.gain(souls)
    }

    /// Adds souls: extra souls carry over into the next level, and souls past
    /// the cap are not counted (the game turns them into crystals).
    pub fn gain(self, amount: u32) -> Self {
        let mut level = self.level;
        let mut souls = self.souls.saturating_add(amount);
        while let Some(&needed) = SOULS_PER_LEVEL.get(level as usize) {
            if souls < needed {
                break;
            }
            souls -= needed;
            level += 1;
        }
        if level >= MAX_LEVEL {
            souls = 0;
        }
        Self { level, souls }
    }
}

/// One server's collection. A pet never obtained is absent: level 0, no souls.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PetCollection {
    /// When the last snapshot was captured, in seconds since the Unix epoch.
    /// None until one arrives; changes applied since are on top of it.
    pub snapshot_at: Option<f64>,
    pub pets: BTreeMap<u32, PetState>,
}

impl PetCollection {
    pub fn pet(&self, pet_id: u32) -> PetState {
        self.pets.get(&pet_id).copied().unwrap_or_default()
    }

    /// Replaces the collection with what the game listed.
    pub fn apply_snapshot(&mut self, snapshot: &PetSnapshot, captured_at: f64) {
        self.pets.clear();
        for entry in &snapshot.levels {
            self.pets.insert(entry.pet_id, PetState { level: entry.level.min(MAX_LEVEL), souls: 0 });
        }
        for entry in &snapshot.progress {
            let level = self.pet(entry.pet_id).level;
            self.pets.insert(entry.pet_id, PetState::from_game(level, entry.progress));
        }
        self.snapshot_at = Some(captured_at);
    }

    /// Adds the souls of one gain and returns the pet as it is now.
    pub fn apply_soul_gain(&mut self, gain: &SoulGain) -> PetState {
        let now = self.pet(gain.pet_id).gain(gain.amount);
        self.pets.insert(gain.pet_id, now);
        now
    }

    /// A pet obtained for the first time is at least level 1. The game sends
    /// this either right after the soul gain that crossed the first threshold,
    /// when the pet is already level 1 and nothing changes, or alone for a
    /// purchase of 5 souls, where the pet's souls stay as they were: the 5
    /// bought pay the first threshold. A pet already further is not lowered.
    pub fn apply_unlock(&mut self, unlock: &PetUnlock) -> PetState {
        let known = self.pet(unlock.pet_id);
        let now = if known.level == 0 { PetState { level: 1, souls: known.souls } } else { known };
        self.pets.insert(unlock.pet_id, now);
        now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::parser::pet_record::{PetLevel, PetProgress};
    use crate::identity::KnownCharacter;

    fn character(name: &str) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id: 1302, class: "RANGER".to_string(), key: None }
    }

    fn snapshot_content() -> PetContent {
        PetContent::Snapshot(PetSnapshot { levels: Vec::new(), progress: Vec::new(), counters: Vec::new() })
    }

    fn gain_content(pet_id: u32) -> PetContent {
        PetContent::SoulGains { gains: vec![SoulGain { pet_id, amount: 1 }], source: SoulSource::Other }
    }

    #[test]
    fn a_snapshot_in_the_same_moment_as_the_name_is_tied_to_that_character() {
        let mut binder = PetBinder::default();
        binder.character_named(character("Alpha"), 100.0);

        let report = binder.snapshot(snapshot_content(), 100.0).expect("bound at once");

        assert_eq!((report.character.name.as_str(), report.character.server_id), ("Alpha", 1302));
        assert_eq!(report.captured_at, 100.0);
    }

    #[test]
    fn soul_gains_before_any_name_wait_and_keep_their_capture_time() {
        let mut binder = PetBinder::default();
        assert!(binder.change(gain_content(1086), 100.0).is_none());

        let reports = binder.character_named(character("Alpha"), 104.0);

        assert_eq!(reports.len(), 1);
        assert_eq!((reports[0].captured_at, &reports[0].content), (100.0, &gain_content(1086)));
    }

    #[test]
    fn a_change_before_any_name_is_bound_to_the_assumed_server_at_once() {
        let mut binder = PetBinder::default();
        assert!(binder.assume_server(character("Alpha")));

        let report = binder.change(gain_content(1086), 100.0).expect("bound at once");

        assert_eq!((report.character.server_id, report.captured_at), (1302, 100.0));
        assert_eq!(binder.assumed_server(), Some((1302, 1)));
        assert!(binder.assumed().is_none(), "no character was assumed, only the server");
    }

    #[test]
    fn a_snapshot_is_not_bound_to_the_assumed_server_and_waits_for_the_name() {
        let mut binder = PetBinder::default();
        binder.assume_server(character("Alpha"));

        assert!(binder.snapshot(snapshot_content(), 100.0).is_none());

        let reports = binder.character_named(character("Beta"), 104.0);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].character.name, "Beta");
    }

    #[test]
    fn the_games_name_replaces_the_assumed_server_and_one_already_given_is_never_replaced() {
        let mut binder = PetBinder::default();
        binder.assume_server(character("Alpha"));
        binder.character_named(character("Beta"), 100.0);

        assert_eq!(binder.assumed_server(), None);
        assert!(!binder.assume_server(character("Alpha")));
        let report = binder.change(gain_content(1086), 101.0).expect("bound");
        assert_eq!(report.character.name, "Beta");
    }

    fn state(level: u32, souls: u32) -> PetState {
        PetState { level, souls }
    }

    #[test]
    fn souls_below_the_threshold_stay_on_the_level() {
        assert_eq!(state(0, 0).gain(4), state(0, 4));
        assert_eq!(state(1, 24).gain(0), state(1, 24));
        assert_eq!(state(2, 74).gain(0), state(2, 74));
    }

    #[test]
    fn the_thresholds_are_five_twenty_five_and_seventy_five_per_level() {
        assert_eq!(state(0, 4).gain(1), state(1, 0));
        assert_eq!(state(1, 24).gain(1), state(2, 0));
        assert_eq!(state(2, 74).gain(1), state(3, 0));
    }

    #[test]
    fn extra_souls_carry_over_into_the_next_level() {
        assert_eq!(state(1, 21).gain(5), state(2, 1));
        assert_eq!(state(1, 24).gain(5), state(2, 4));
        assert_eq!(state(2, 71).gain(4), state(3, 0));
    }

    #[test]
    fn souls_carry_across_several_levels_at_once() {
        assert_eq!(state(0, 1).gain(41), state(2, 12));
        assert_eq!(state(1, 13).gain(39), state(2, 27));
        assert_eq!(state(0, 0).gain(105), state(3, 0));
    }

    #[test]
    fn souls_past_the_cap_are_not_counted() {
        assert_eq!(state(3, 0).gain(5), state(3, 0));
        assert_eq!(state(2, 70).gain(500), state(3, 0));
    }

    #[test]
    fn a_pet_the_game_lists_keeps_its_level_and_progress_even_when_the_progress_is_odd() {
        assert_eq!(PetState::from_game(2, 27), state(2, 27));
        assert_eq!(PetState::from_game(3, 0), state(3, 0));
        assert_eq!(PetState::from_game(1, 30), state(2, 5));
        assert_eq!(PetState::from_game(9, 4), state(3, 0));
    }

    fn snapshot(levels: &[(u32, u32)], progress: &[(u32, u32)]) -> PetSnapshot {
        PetSnapshot {
            levels: levels.iter().map(|&(pet_id, level)| PetLevel { pet_id, level }).collect(),
            progress: progress.iter().map(|&(pet_id, progress)| PetProgress { pet_id, progress }).collect(),
            counters: Vec::new(),
        }
    }

    #[test]
    fn a_snapshot_replaces_the_collection_and_records_when_it_was_captured() {
        let mut collection = PetCollection::default();
        collection.apply_soul_gain(&SoulGain { pet_id: 1500, amount: 3 });

        collection.apply_snapshot(&snapshot(&[(1086, 2), (1214, 3), (1010, 1)], &[(1086, 27), (1119, 4)]), 100.0);

        assert_eq!(collection.snapshot_at, Some(100.0));
        assert_eq!(collection.pet(1086), state(2, 27));
        assert_eq!(collection.pet(1214), state(3, 0));
        assert_eq!(collection.pet(1010), state(1, 0), "a levelled pet without progress has none yet");
        assert_eq!(collection.pet(1119), state(0, 4), "progress alone is a pet below level 1");
        assert_eq!(collection.pet(1500), PetState::default(), "a pet the snapshot does not list is gone");
    }

    #[test]
    fn a_gain_to_a_pet_the_collection_does_not_know_starts_from_level_zero() {
        let mut collection = PetCollection::default();

        let now = collection.apply_soul_gain(&SoulGain { pet_id: 1182, amount: 5 });

        assert_eq!(now, state(1, 0));
        assert_eq!(collection.pet(1182), state(1, 0));
        assert_eq!(collection.snapshot_at, None);
    }

    #[test]
    fn a_recorded_gain_on_a_snapshotted_pet_adds_to_its_progress() {
        let mut collection = PetCollection::default();
        collection.apply_snapshot(&snapshot(&[(1086, 2)], &[(1086, 27)]), 100.0);

        let now = collection.apply_soul_gain(&SoulGain { pet_id: 1086, amount: 1 });

        assert_eq!(now, state(2, 28));
    }

    #[test]
    fn an_unlock_alone_makes_a_level_zero_pet_level_one_and_keeps_the_souls_it_had() {
        let mut collection = PetCollection::default();
        collection.apply_snapshot(&snapshot(&[], &[(1192, 3)]), 100.0);

        assert_eq!(collection.apply_unlock(&PetUnlock { pet_id: 1192 }), state(1, 3));
        assert_eq!(collection.apply_unlock(&PetUnlock { pet_id: 1206 }), state(1, 0), "a pet the snapshot did not list");
        assert_eq!(collection.pet(1206), state(1, 0));
    }

    #[test]
    fn an_unlock_after_the_gain_that_crossed_the_threshold_adds_nothing() {
        let mut collection = PetCollection::default();
        collection.apply_snapshot(&snapshot(&[], &[(1108, 4)]), 100.0);

        collection.apply_soul_gain(&SoulGain { pet_id: 1108, amount: 1 });
        let now = collection.apply_unlock(&PetUnlock { pet_id: 1108 });

        assert_eq!(now, state(1, 0));
    }

    #[test]
    fn an_unlock_never_lowers_a_pet_that_is_already_further() {
        let mut collection = PetCollection::default();
        collection.apply_snapshot(&snapshot(&[(1086, 2)], &[(1086, 27)]), 100.0);

        assert_eq!(collection.apply_unlock(&PetUnlock { pet_id: 1086 }), state(2, 27));
    }
}
