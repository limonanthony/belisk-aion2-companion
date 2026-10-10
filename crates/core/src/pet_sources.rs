//! Where a pet message's souls came from, read off the messages that precede
//! it: the player's own pickup of a loot object (`21 56`, tied through the
//! object's id and serial to the `1E 56` that dropped it from a world entity,
//! whose mob code the world knows), or the `95 56` the game sends with a
//! purchase. The game does not say it in the pet message itself.

use std::collections::VecDeque;

use serde::Serialize;

use crate::capture::parser::loot_record::{LootPickup, LootSpawn};

/// A pet message follows its cause by one capture tick; in the recordings the
/// gap is 39 to 72 ms. This leaves room for a missed tick without reaching
/// back to an earlier cause.
const CAUSE_PRECEDES_MESSAGE_BY_AT_MOST_SECONDS: f64 = 0.25;

/// The longest the recordings show between a loot object appearing and the
/// player's pickup is 0.65 s (508 pickups); a loot object is remembered 30 s,
/// and no more than `REMEMBERED_LOOT_OBJECTS` of them. The busiest minute seen
/// announced 113 objects, so 30 s of it is about 60 and the cap is a bound for
/// a clock that does not advance, not a limit anyone reaches.
const LOOT_OBJECT_MEMORY_SECONDS: f64 = 30.0;
const REMEMBERED_LOOT_OBJECTS: usize = 2048;

/// A pickup waits `CAUSE_PRECEDES_MESSAGE_BY_AT_MOST_SECONDS` for its message;
/// a batch of seven pets is the most one message has named.
const REMEMBERED_PICKUPS: usize = 64;

/// What the traffic proves about a soul gain; anything else is `Other` (a
/// cube, a chest, a quest reward...). `BlackMarket` is a purchase: the game's
/// `95 56` does not say which store it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SoulSource {
    Loot,
    BlackMarket,
    Other,
}

/// A loot object the player picked up.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedUpLoot {
    pub at_seconds: f64,
    /// The loot's announcement, when it was seen.
    pub spawn: Option<LootSpawn>,
    /// The mob the loot dropped from, when the world knew its entity.
    pub mob_code: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoulGainCause {
    pub source: SoulSource,
    pub pickups: Vec<PickedUpLoot>,
}

#[derive(Debug, Default)]
pub struct SoulSourceTracker {
    spawns: VecDeque<(f64, LootSpawn)>,
    pickups: Vec<PickedUpLoot>,
    purchase_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CounterGainCause {
    pub pickups: Vec<PickedUpLoot>,
    pub looted_mobs: Vec<u32>,
}

impl SoulSourceTracker {
    pub fn loot_spawned(&mut self, at: f64, spawn: LootSpawn) {
        while self.spawns.front().is_some_and(|(spawned_at, _)| at - spawned_at > LOOT_OBJECT_MEMORY_SECONDS) {
            self.spawns.pop_front();
        }
        if self.spawns.len() == REMEMBERED_LOOT_OBJECTS {
            self.spawns.pop_front();
        }
        self.spawns.push_back((at, spawn));
    }

    /// `mob_code_of` answers what mob a world entity is.
    pub fn picked_up(&mut self, at: f64, pickup: LootPickup, mob_code_of: impl Fn(u32) -> Option<u32>) {
        let spawn = self
            .spawns
            .iter()
            .rev()
            .map(|(_, spawn)| spawn)
            .find(|spawn| spawn.object_id == pickup.object_id && spawn.serial == pickup.serial)
            .copied();
        let mob_code = spawn.and_then(|spawn| mob_code_of(spawn.dropper_entity));
        self.pickups.retain(|waiting| is_recent(waiting.at_seconds, at));
        if self.pickups.len() == REMEMBERED_PICKUPS {
            self.pickups.remove(0);
        }
        self.pickups.push(PickedUpLoot { at_seconds: at, spawn, mob_code });
    }

    pub fn purchased(&mut self, at: f64) {
        self.purchase_at = Some(at);
    }

    /// The cause of a soul gain message at `at`, with the pickups that led up
    /// to it: consumed, so a later message cannot claim them again.
    pub fn soul_gain_cause(&mut self, at: f64) -> SoulGainCause {
        let pickups = self.take_pickups_before(at);
        let purchased = self.purchase_at.take().is_some_and(|purchased_at| is_recent(purchased_at, at));
        let source = if purchased {
            SoulSource::BlackMarket
        } else if !pickups.is_empty() {
            SoulSource::Loot
        } else {
            SoulSource::Other
        };
        SoulGainCause { source, pickups }
    }

    /// The pickups that led up to a counter gain message at `at`, and the
    /// mobs they dropped from, each once, in pickup order; consumed. The mobs
    /// are empty when the gain did not follow a pickup, or the dropper was
    /// not a mob the world knew.
    pub fn counter_gain_cause(&mut self, at: f64) -> CounterGainCause {
        let pickups = self.take_pickups_before(at);
        let mut looted_mobs = Vec::new();
        for mob_code in pickups.iter().filter_map(|pickup| pickup.mob_code) {
            if !looted_mobs.contains(&mob_code) {
                looted_mobs.push(mob_code);
            }
        }
        CounterGainCause { pickups, looted_mobs }
    }

    fn take_pickups_before(&mut self, at: f64) -> Vec<PickedUpLoot> {
        let pickups = std::mem::take(&mut self.pickups);
        pickups.into_iter().filter(|pickup| is_recent(pickup.at_seconds, at)).collect()
    }
}

fn is_recent(cause_at: f64, message_at: f64) -> bool {
    let elapsed = message_at - cause_at;
    (0.0..=CAUSE_PRECEDES_MESSAGE_BY_AT_MOST_SECONDS).contains(&elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FUNGEN_MOB: u32 = 2_100_456;
    const ARMADON_MOB: u32 = 2_100_453;

    fn spawn(dropper_entity: u32, object_id: u32, serial: u32) -> LootSpawn {
        LootSpawn { dropper_entity, object_id, serial, item_id: 238_906 }
    }

    fn other_item(object_id: u32) -> LootSpawn {
        LootSpawn { dropper_entity: 1, object_id, serial: object_id, item_id: 217_155 }
    }

    fn pickup(object_id: u32, serial: u32) -> LootPickup {
        LootPickup { object_id, serial }
    }

    fn world(entity: u32) -> Option<u32> {
        match entity {
            48_710 => Some(FUNGEN_MOB),
            37_308 => Some(ARMADON_MOB),
            _ => None,
        }
    }

    #[test]
    fn a_soul_gain_right_after_the_players_pickup_is_loot() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(99.5, spawn(48_710, 0x580c, 7));
        tracker.picked_up(100.0, pickup(0x580c, 7), world);

        let cause = tracker.soul_gain_cause(100.048);

        assert_eq!(cause.source, SoulSource::Loot);
        assert_eq!(cause.pickups.len(), 1);
        assert_eq!(cause.pickups[0].mob_code, Some(FUNGEN_MOB));
    }

    #[test]
    fn a_soul_gain_with_the_purchase_message_before_it_is_a_black_market_buy() {
        let mut tracker = SoulSourceTracker::default();
        tracker.purchased(200.0);

        assert_eq!(tracker.soul_gain_cause(200.05).source, SoulSource::BlackMarket);
    }

    #[test]
    fn a_soul_gain_with_no_recent_pickup_or_purchase_is_other() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(99.5, spawn(48_710, 1, 1));
        tracker.picked_up(100.0, pickup(1, 1), world);
        tracker.purchased(100.0);

        assert_eq!(tracker.soul_gain_cause(100.6).source, SoulSource::Other);
        assert_eq!(tracker.soul_gain_cause(100.65).source, SoulSource::Other, "nothing is left to claim");
    }

    #[test]
    fn a_pickup_is_claimed_by_one_message_only() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(99.5, spawn(48_710, 1, 1));
        tracker.picked_up(100.0, pickup(1, 1), world);

        assert_eq!(tracker.soul_gain_cause(100.05).source, SoulSource::Loot);
        assert_eq!(tracker.soul_gain_cause(100.10).source, SoulSource::Other);
    }

    #[test]
    fn a_message_before_the_cause_does_not_claim_it() {
        let mut tracker = SoulSourceTracker::default();
        tracker.picked_up(100.0, pickup(1, 1), world);

        assert_eq!(tracker.soul_gain_cause(99.9).source, SoulSource::Other);
    }

    #[test]
    fn a_pickup_whose_announcement_was_not_seen_is_still_loot_with_no_mob() {
        let mut tracker = SoulSourceTracker::default();
        tracker.picked_up(100.0, pickup(9, 9), world);

        let cause = tracker.soul_gain_cause(100.05);

        assert_eq!(cause.source, SoulSource::Loot);
        assert_eq!(cause.pickups[0].spawn, None);
        assert_eq!(cause.pickups[0].mob_code, None);
    }

    #[test]
    fn the_pickup_matches_its_object_by_id_and_serial() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(99.0, spawn(37_308, 0x5442, 1));
        tracker.loot_spawned(99.0, spawn(48_710, 0x5442, 2));
        tracker.picked_up(100.0, pickup(0x5442, 2), world);

        assert_eq!(tracker.counter_gain_cause(100.05).looted_mobs, vec![FUNGEN_MOB]);
    }

    #[test]
    fn looted_mobs_lists_each_mob_once_in_pickup_order_and_skips_unknown_ones() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(99.0, spawn(37_308, 1, 1));
        tracker.loot_spawned(99.0, spawn(48_710, 2, 2));
        tracker.loot_spawned(99.0, spawn(48_710, 3, 3));
        tracker.loot_spawned(99.0, spawn(1, 4, 4));
        tracker.picked_up(100.0, pickup(1, 1), world);
        tracker.picked_up(100.0, pickup(2, 2), world);
        tracker.picked_up(100.0, pickup(3, 3), world);
        tracker.picked_up(100.0, pickup(4, 4), world);

        assert_eq!(tracker.counter_gain_cause(100.05).looted_mobs, vec![ARMADON_MOB, FUNGEN_MOB]);
        assert!(tracker.counter_gain_cause(100.06).looted_mobs.is_empty(), "consumed");
    }

    #[test]
    fn a_soul_spawn_survives_a_busy_minute_of_other_loot_and_is_found_by_its_pickup() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(100.0, spawn(48_710, 1, 1));
        for object_id in 2..=113 {
            tracker.loot_spawned(100.0 + f64::from(object_id) / 5.0, other_item(object_id));
        }
        tracker.picked_up(123.0, pickup(1, 1), world);

        assert_eq!(tracker.counter_gain_cause(123.05).looted_mobs, vec![FUNGEN_MOB]);
    }

    #[test]
    fn loot_objects_older_than_the_memory_are_forgotten_as_new_ones_arrive() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(100.0, spawn(48_710, 1, 1));
        tracker.loot_spawned(100.0 + LOOT_OBJECT_MEMORY_SECONDS + 1.0, other_item(2));

        assert_eq!(tracker.spawns.len(), 1);
        tracker.picked_up(131.5, pickup(1, 1), world);
        assert_eq!(tracker.soul_gain_cause(131.55).pickups[0].spawn, None);
    }

    #[test]
    fn a_flood_of_loot_objects_in_one_instant_stays_at_the_cap_and_keeps_the_newest() {
        let mut tracker = SoulSourceTracker::default();
        tracker.loot_spawned(100.0, spawn(48_710, 1, 1));
        for object_id in 2..=REMEMBERED_LOOT_OBJECTS as u32 + 1 {
            tracker.loot_spawned(100.0, other_item(object_id));
        }

        assert_eq!(tracker.spawns.len(), REMEMBERED_LOOT_OBJECTS);
        tracker.picked_up(100.5, pickup(1, 1), world);
        assert_eq!(tracker.soul_gain_cause(100.55).pickups[0].spawn, None, "the oldest went first");
    }

    #[test]
    fn pickups_with_no_pet_message_are_dropped_once_past_the_claim_window() {
        let mut tracker = SoulSourceTracker::default();
        for second in 0..1000 {
            tracker.picked_up(f64::from(second), pickup(second, second), world);
        }

        assert_eq!(tracker.pickups.len(), 1);
        assert_eq!(tracker.soul_gain_cause(999.05).source, SoulSource::Loot, "the latest pickup still counts");
    }

    #[test]
    fn a_flood_of_pickups_in_one_instant_stays_at_the_cap() {
        let mut tracker = SoulSourceTracker::default();
        for object_id in 0..10_000 {
            tracker.picked_up(100.0, pickup(object_id, object_id), world);
        }

        assert_eq!(tracker.pickups.len(), REMEMBERED_PICKUPS);
    }
}
