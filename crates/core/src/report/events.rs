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
}

/// The hit taken before a death.
#[derive(Debug, Clone, PartialEq)]
pub struct LastHit {
    pub source_mob_code: Option<u32>,
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
                source_mob_code: hit.source_mob_code,
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
            skill_code: 1_607_630,
            amount,
        }
    }

    fn unknown_killer() -> Killer {
        Killer {
            id: 0,
            name: None,
            mob_code: None,
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
                source_mob_code: Some(2_300_810),
                skill_code: 1_607_630,
                amount: 900
            })
        );
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
