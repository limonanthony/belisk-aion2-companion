use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

const INCOMING_HIT_CAPACITY: usize = 60_000;
const HEAL_CAPACITY: usize = 60_000;
const DEATH_CAPACITY: usize = 2_000;

/// A hit counts as the one that killed when it landed this long before the death
/// at the latest.
pub(crate) const LAST_HIT_WINDOW_SECS: f64 = 5.0;

/// Where a heal came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealKind {
    /// A healing skill's own hit (`04 38` of a skill in `healing_skill_code.json`).
    Direct,
    /// A tick of a heal over time (`05 38`, mode `0x0B`).
    OverTime,
    /// Life drained by a damaging hit (`04 38`, drain amount).
    Drain,
    /// Health regained on a damaging hit (`04 38`, regeneration amount).
    Regeneration,
}

impl HealKind {
    /// Whether the report counts it as healing: a healing skill's own heals and
    /// its heals over time. Drain and regeneration are flat recovery from stats
    /// on damaging hits, which every class has.
    pub fn counts_as_healing(self) -> bool {
        matches!(self, Self::Direct | Self::OverTime)
    }
}

/// A hit a monster landed on a player, by one of its own skills.
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingHit {
    pub at: f64,
    pub victim_id: u32,
    pub source_id: u32,
    pub source_mob_code: Option<u32>,
    /// The mob code of the monster at the top of the source's owner chain, when it has one.
    pub owner_mob_code: Option<u32>,
    pub skill_code: u32,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heal {
    pub at: f64,
    pub healer_id: u32,
    pub target_id: u32,
    pub skill_code: u32,
    pub kind: HealKind,
    pub amount: u64,
    pub critical: bool,
}

/// Who the game says killed a player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Killer {
    pub id: u32,
    /// Set when the killer is a player.
    pub name: Option<String>,
    /// Set when the killer is a monster.
    pub mob_code: Option<u32>,
    /// The mob code of the monster that owns a killer that is its summoned object.
    pub owner_mob_code: Option<u32>,
}

/// The hit taken before a death.
#[derive(Debug, Clone, PartialEq)]
pub struct LastHit {
    pub source_id: u32,
    pub source_mob_code: Option<u32>,
    pub owner_mob_code: Option<u32>,
    pub skill_code: u32,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Death {
    pub at: f64,
    pub victim_id: u32,
    pub skill_code: u32,
    pub killer: Killer,
    pub last_hit: Option<LastHit>,
}

/// The events of one stretch of time.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CombatEvents {
    pub hits: Vec<IncomingHit>,
    pub heals: Vec<Heal>,
    pub deaths: Vec<Death>,
}

impl CombatEvents {
    /// A monster's summoned objects (projectiles, zones, copies of the boss)
    /// land hits under their own mob codes. Credit them to the monster that
    /// owns them, unless the party fought the entity itself: that is a real
    /// add and stays its own source. `fought` says whether players damaged an
    /// entity.
    pub(crate) fn credit_unfought_sources_to_their_owners(&mut self, fought: impl Fn(u32) -> bool) {
        let credited = |entity_id: u32, own: Option<u32>, owner: Option<u32>| match owner {
            Some(owner) if !fought(entity_id) => Some(owner),
            _ => own,
        };
        for hit in &mut self.hits {
            hit.source_mob_code = credited(hit.source_id, hit.source_mob_code, hit.owner_mob_code);
        }
        for death in &mut self.deaths {
            let killer = &mut death.killer;
            if killer.name.is_none() {
                killer.mob_code = credited(killer.id, killer.mob_code, killer.owner_mob_code);
            }
            if let Some(last_hit) = &mut death.last_hit {
                last_hit.source_mob_code = credited(last_hit.source_id, last_hit.source_mob_code, last_hit.owner_mob_code);
            }
        }
    }
}

/// What happened around the fights that damage dealt does not say. Bounded:
/// the oldest events go first, so a fight in progress is never the one lost.
#[derive(Debug, Default)]
pub(crate) struct CombatEventLog {
    hits: VecDeque<IncomingHit>,
    heals: VecDeque<Heal>,
    deaths: VecDeque<Death>,
}

fn push_bounded<T>(queue: &mut VecDeque<T>, item: T, capacity: usize) {
    if queue.len() == capacity {
        queue.pop_front();
    }
    queue.push_back(item);
}

impl CombatEventLog {
    pub(crate) fn record_hit(&mut self, hit: IncomingHit) {
        push_bounded(&mut self.hits, hit, INCOMING_HIT_CAPACITY);
    }

    pub(crate) fn record_heal(&mut self, heal: Heal) {
        push_bounded(&mut self.heals, heal, HEAL_CAPACITY);
    }

    /// Ties the death to the last hit its victim took, if one landed just before.
    pub(crate) fn record_death(
        &mut self,
        at: f64,
        victim_id: u32,
        skill_code: u32,
        killer: Killer,
    ) {
        let last_hit = self
            .hits
            .iter()
            .rev()
            .take_while(|hit| hit.at >= at - LAST_HIT_WINDOW_SECS)
            .find(|hit| hit.victim_id == victim_id && hit.at <= at)
            .map(|hit| LastHit {
                source_id: hit.source_id,
                source_mob_code: hit.source_mob_code,
                owner_mob_code: hit.owner_mob_code,
                skill_code: hit.skill_code,
                amount: hit.amount,
            });
        push_bounded(
            &mut self.deaths,
            Death {
                at,
                victim_id,
                skill_code,
                killer,
                last_hit,
            },
            DEATH_CAPACITY,
        );
    }

    /// The events that happened from `from` to `to`, both included.
    pub(crate) fn between(&self, from: f64, to: f64) -> CombatEvents {
        let within = |at: f64| at >= from && at <= to;
        CombatEvents {
            hits: self
                .hits
                .iter()
                .filter(|hit| within(hit.at))
                .cloned()
                .collect(),
            heals: self
                .heals
                .iter()
                .filter(|heal| within(heal.at))
                .cloned()
                .collect(),
            deaths: self
                .deaths
                .iter()
                .filter(|death| within(death.at))
                .cloned()
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER: u32 = 7;
    const OTHER_PLAYER: u32 = 8;

    fn hit(at: f64, victim_id: u32, amount: u64) -> IncomingHit {
        IncomingHit {
            at,
            victim_id,
            source_id: 900,
            source_mob_code: Some(2_300_810),
            owner_mob_code: None,
            skill_code: 1_607_630,
            amount,
        }
    }

    fn unknown_killer() -> Killer {
        Killer {
            id: 0,
            name: None,
            mob_code: None,
            owner_mob_code: None,
        }
    }

    #[test]
    fn a_death_carries_the_last_hit_its_victim_took() {
        let mut log = CombatEventLog::default();
        log.record_hit(hit(10.0, PLAYER, 400));
        log.record_hit(hit(11.0, PLAYER, 900));
        log.record_hit(hit(11.5, OTHER_PLAYER, 5_000));

        log.record_death(12.0, PLAYER, 0, unknown_killer());

        let death = &log.between(0.0, 20.0).deaths[0];
        assert_eq!(
            death.last_hit,
            Some(LastHit {
                source_id: 900,
                source_mob_code: Some(2_300_810),
                owner_mob_code: None,
                skill_code: 1_607_630,
                amount: 900
            })
        );
    }

    const TALISRA: u32 = 2_350_709;
    const TALISRA_ENTITY: u32 = 18_376;
    const OBJECT_CODE: u32 = 2_920_564;
    const OBJECT_ENTITY: u32 = 20_890;
    const ADD_CODE: u32 = 2_920_540;
    const ADD_ENTITY: u32 = 21_001;

    fn hit_by(source_id: u32, source_mob_code: u32, owner_mob_code: Option<u32>) -> IncomingHit {
        IncomingHit { source_id, source_mob_code: Some(source_mob_code), owner_mob_code, ..hit(10.0, PLAYER, 500) }
    }

    fn events_with(hits: Vec<IncomingHit>, deaths: Vec<Death>) -> CombatEvents {
        CombatEvents { hits, deaths, ..CombatEvents::default() }
    }

    #[test]
    fn an_object_nobody_fought_is_credited_to_the_monster_that_owns_it() {
        let mut events = events_with(vec![hit_by(OBJECT_ENTITY, OBJECT_CODE, Some(TALISRA))], Vec::new());

        events.credit_unfought_sources_to_their_owners(|_| false);

        assert_eq!(events.hits[0].source_mob_code, Some(TALISRA));
    }

    #[test]
    fn an_add_the_party_fought_stays_its_own_source() {
        let mut events = events_with(vec![hit_by(ADD_ENTITY, ADD_CODE, Some(TALISRA))], Vec::new());

        events.credit_unfought_sources_to_their_owners(|entity_id| entity_id == ADD_ENTITY);

        assert_eq!(events.hits[0].source_mob_code, Some(ADD_CODE));
    }

    #[test]
    fn a_monster_nobody_owns_keeps_its_code() {
        let mut events = events_with(vec![hit_by(TALISRA_ENTITY, TALISRA, None)], Vec::new());

        events.credit_unfought_sources_to_their_owners(|_| false);

        assert_eq!(events.hits[0].source_mob_code, Some(TALISRA));
    }

    #[test]
    fn a_death_by_an_owned_object_names_the_owner_as_killer_and_last_hit() {
        let mut log = CombatEventLog::default();
        log.record_hit(hit_by(OBJECT_ENTITY, OBJECT_CODE, Some(TALISRA)));
        let killer = Killer { id: OBJECT_ENTITY, name: None, mob_code: Some(OBJECT_CODE), owner_mob_code: Some(TALISRA) };
        log.record_death(11.0, PLAYER, 1_801_777, killer);
        let mut events = log.between(0.0, 20.0);

        events.credit_unfought_sources_to_their_owners(|_| false);

        let death = &events.deaths[0];
        assert_eq!(death.killer.mob_code, Some(TALISRA));
        assert_eq!(death.last_hit.as_ref().unwrap().source_mob_code, Some(TALISRA));
    }

    #[test]
    fn a_player_killer_is_never_credited_to_a_monster() {
        let mut log = CombatEventLog::default();
        let killer = Killer { id: 5, name: Some("Veyra".to_string()), mob_code: None, owner_mob_code: Some(TALISRA) };
        log.record_death(11.0, PLAYER, 0, killer);
        let mut events = log.between(0.0, 20.0);

        events.credit_unfought_sources_to_their_owners(|_| false);

        assert_eq!(events.deaths[0].killer.mob_code, None);
    }

    #[test]
    fn a_hit_too_long_before_the_death_is_not_its_last_hit() {
        let mut log = CombatEventLog::default();
        log.record_hit(hit(1.0, PLAYER, 900));

        log.record_death(
            1.0 + LAST_HIT_WINDOW_SECS + 0.5,
            PLAYER,
            0,
            unknown_killer(),
        );

        assert_eq!(log.between(0.0, 20.0).deaths[0].last_hit, None);
    }

    #[test]
    fn events_are_selected_by_time_with_both_ends_included() {
        let mut log = CombatEventLog::default();
        log.record_hit(hit(1.0, PLAYER, 1));
        log.record_hit(hit(2.0, PLAYER, 2));
        log.record_hit(hit(3.0, PLAYER, 3));

        let amounts: Vec<u64> = log
            .between(1.0, 2.0)
            .hits
            .iter()
            .map(|hit| hit.amount)
            .collect();

        assert_eq!(amounts, [1, 2]);
    }

    #[test]
    fn the_oldest_events_are_dropped_past_capacity() {
        let mut log = CombatEventLog::default();
        for index in 0..=INCOMING_HIT_CAPACITY {
            log.record_hit(hit(index as f64, PLAYER, index as u64));
        }

        let kept = log.between(0.0, f64::MAX).hits;

        assert_eq!(kept.len(), INCOMING_HIT_CAPACITY);
        assert_eq!(kept[0].amount, 1);
    }
}
