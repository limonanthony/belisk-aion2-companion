use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::engine::buff_groups::group_by_name;
use crate::models::combat::{BuffInterval, TargetInfo};
use crate::report::events::{CombatEvents, Heal, HealKind, Killer};

/// Players still die and get hit for a moment after the last blow on a boss
/// that was not killed: a wiped party is the typical case.
const END_GRACE_SECS: f64 = 5.0;

/// A boss fight on the meter's clock (seconds since the epoch): the first hit on
/// the boss to the last.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FightWindow {
    pub started_at: f64,
    pub ended_at: f64,
}

impl FightWindow {
    /// `None` until somebody has hit the target.
    pub fn of(target: &TargetInfo) -> Option<Self> {
        let started_at = target
            .target_start_time
            .values()
            .copied()
            .reduce(f64::min)?;
        let ended_at = target.target_last_time.values().copied().reduce(f64::max)?;
        Some(Self {
            started_at,
            ended_at: ended_at.max(started_at),
        })
    }

    pub fn duration_secs(&self) -> f64 {
        self.ended_at - self.started_at
    }

    /// The span in which the fight's events are looked for.
    pub(crate) fn event_range(&self) -> (f64, f64) {
        (self.started_at, self.ended_at + END_GRACE_SECS)
    }
}

/// What the party took, healed and lost during a boss fight, and how long each
/// buff was up. Saved with the fight; a fight saved before the meter kept this
/// has none, which is not the same as nothing happening.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FightLedger {
    pub window: FightWindow,
    /// Everyone in the fight: the players who hit the boss, and the ones who
    /// healed them. Each has an entry even when nothing was recorded for them.
    pub players: Vec<LedgerPlayer>,
    /// In the order they happened.
    pub deaths: Vec<LedgerDeath>,
    /// On the boss and on every player in `players`.
    pub buffs: Vec<BuffUptime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerPlayer {
    pub actor_id: u32,
    pub actor_name: String,
    pub actor_server_id: String,
    pub actor_class: String,
    /// What this player healed, per skill; most first.
    pub healing: Vec<HealSkill>,
    /// What monsters did to this player, per monster skill; most first.
    pub damage_taken: Vec<TakenSkill>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealSkill {
    pub skill_code: u32,
    pub kind: HealKind,
    pub heals: u32,
    pub crits: u32,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakenSkill {
    pub monster_code: Option<u32>,
    pub monster_name: Option<String>,
    pub skill_code: u32,
    pub hits: u32,
    pub largest: u64,
    pub amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KillerKind {
    Monster,
    Player,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerKiller {
    pub kind: KillerKind,
    pub name: Option<String>,
    pub monster_code: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerLastHit {
    pub monster_code: Option<u32>,
    pub monster_name: Option<String>,
    pub skill_code: u32,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerDeath {
    /// Seconds from the first hit on the boss; never negative.
    pub at_secs: f64,
    pub actor_id: u32,
    pub killer: LedgerKiller,
    /// The skill that killed, as the game's death record names it; 0 if none.
    pub skill_code: u32,
    /// The last hit the player took before dying, when one was seen.
    pub last_hit: Option<LedgerLastHit>,
}

/// How long a buff stayed up within the fight window, overlaps counted once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffUptime {
    pub target_id: u32,
    pub caster_id: u32,
    pub skill_code: u32,
    pub uptime_ms: u64,
}

pub(crate) struct ActorDirectory<'a> {
    pub names: &'a HashMap<u32, String>,
    pub servers: &'a HashMap<u32, String>,
    pub classes: &'a HashMap<u32, String>,
}

pub(crate) type BuffIntervals = HashMap<u32, HashMap<u32, HashMap<u32, VecDeque<BuffInterval>>>>;

pub(crate) struct LedgerSources<'a> {
    pub target_id: u32,
    pub window: FightWindow,
    /// The players who hit the target.
    pub damage_dealers: &'a [u32],
    /// The events inside `window.event_range()`.
    pub events: &'a CombatEvents,
    pub actors: ActorDirectory<'a>,
    pub mob_name: &'a dyn Fn(u32) -> Option<String>,
    pub buff_intervals: &'a BuffIntervals,
}

/// The damage dealers and the players who healed one of them.
pub(crate) fn fight_party(damage_dealers: &[u32], heals: &[Heal]) -> HashSet<u32> {
    let dealers: HashSet<u32> = damage_dealers.iter().copied().collect();
    let mut party = dealers.clone();
    for heal in heals {
        if dealers.contains(&heal.target_id) {
            party.insert(heal.healer_id);
        }
    }
    party
}

pub(crate) fn build_ledger(sources: &LedgerSources) -> FightLedger {
    let LedgerSources {
        target_id,
        window,
        damage_dealers,
        events,
        actors,
        mob_name,
        buff_intervals,
    } = sources;

    let party = fight_party(damage_dealers, &events.heals);

    let mut healing: HashMap<u32, BTreeMap<(u32, HealKind), HealSkill>> = HashMap::new();
    for heal in &events.heals {
        let healer = heal.healer_id;
        if !party.contains(&healer) || !party.contains(&heal.target_id) {
            continue;
        }
        let skill = healing
            .entry(healer)
            .or_default()
            .entry((heal.skill_code, heal.kind))
            .or_insert(HealSkill {
                skill_code: heal.skill_code,
                kind: heal.kind,
                heals: 0,
                crits: 0,
                amount: 0,
            });
        skill.heals += 1;
        skill.crits += u32::from(heal.critical);
        skill.amount += heal.amount;
    }

    let mut taken: HashMap<u32, BTreeMap<(Option<u32>, u32), TakenSkill>> = HashMap::new();
    for hit in &events.hits {
        if !party.contains(&hit.victim_id) {
            continue;
        }
        let skill = taken
            .entry(hit.victim_id)
            .or_default()
            .entry((hit.source_mob_code, hit.skill_code))
            .or_insert_with(|| TakenSkill {
                monster_code: hit.source_mob_code,
                monster_name: hit.source_mob_code.and_then(mob_name),
                skill_code: hit.skill_code,
                hits: 0,
                largest: 0,
                amount: 0,
            });
        skill.hits += 1;
        skill.largest = skill.largest.max(hit.amount);
        skill.amount += hit.amount;
    }

    let mut party_ids: Vec<u32> = party.iter().copied().collect();
    party_ids.sort_unstable();
    let players = party_ids
        .iter()
        .map(|&actor_id| {
            let mut healing: Vec<HealSkill> = healing
                .remove(&actor_id)
                .map(|skills| skills.into_values().collect())
                .unwrap_or_default();
            healing.sort_by(|a, b| {
                b.amount
                    .cmp(&a.amount)
                    .then(a.skill_code.cmp(&b.skill_code))
            });
            let mut damage_taken: Vec<TakenSkill> = taken
                .remove(&actor_id)
                .map(|skills| skills.into_values().collect())
                .unwrap_or_default();
            damage_taken.sort_by(|a, b| {
                b.amount
                    .cmp(&a.amount)
                    .then(a.skill_code.cmp(&b.skill_code))
            });
            LedgerPlayer {
                actor_id,
                actor_name: actors.names.get(&actor_id).cloned().unwrap_or_default(),
                actor_server_id: actors.servers.get(&actor_id).cloned().unwrap_or_default(),
                actor_class: actors.classes.get(&actor_id).cloned().unwrap_or_default(),
                healing,
                damage_taken,
            }
        })
        .collect();

    let deaths = events
        .deaths
        .iter()
        .filter(|death| party.contains(&death.victim_id))
        .map(|death| LedgerDeath {
            at_secs: (death.at - window.started_at).max(0.0),
            actor_id: death.victim_id,
            killer: ledger_killer(&death.killer, mob_name),
            skill_code: death.skill_code,
            last_hit: death.last_hit.as_ref().map(|hit| LedgerLastHit {
                monster_code: hit.source_mob_code,
                monster_name: hit.source_mob_code.and_then(mob_name),
                skill_code: hit.skill_code,
                amount: hit.amount,
            }),
        })
        .collect();

    let from_ms = seconds_to_ms(window.started_at);
    let to_ms = seconds_to_ms(window.ended_at);
    let mut buffs = Vec::new();
    for (&buffed, casters) in buff_intervals.iter() {
        if buffed != *target_id && !party.contains(&buffed) {
            continue;
        }
        for (&caster_id, skills) in casters {
            for group in group_by_name(skills, from_ms, to_ms) {
                let uptime_ms = group.covered_ms(from_ms, to_ms);
                if uptime_ms > 0 {
                    buffs.push(BuffUptime {
                        target_id: buffed,
                        caster_id,
                        skill_code: group.skill_code,
                        uptime_ms,
                    });
                }
            }
        }
    }
    buffs.sort_by_key(|buff| {
        (
            buff.target_id,
            std::cmp::Reverse(buff.uptime_ms),
            buff.skill_code,
            buff.caster_id,
        )
    });

    FightLedger {
        window: *window,
        players,
        deaths,
        buffs,
    }
}

fn ledger_killer(killer: &Killer, mob_name: &dyn Fn(u32) -> Option<String>) -> LedgerKiller {
    match (&killer.name, killer.mob_code) {
        (Some(name), _) => LedgerKiller {
            kind: KillerKind::Player,
            name: Some(name.clone()),
            monster_code: None,
        },
        (None, Some(code)) => LedgerKiller {
            kind: KillerKind::Monster,
            name: mob_name(code),
            monster_code: Some(code),
        },
        (None, None) => LedgerKiller {
            kind: KillerKind::Unknown,
            name: None,
            monster_code: None,
        },
    }
}

fn seconds_to_ms(seconds: f64) -> u64 {
    (seconds * 1000.0).max(0.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::events::{Death, IncomingHit, LastHit};

    const BOSS: u32 = 500;
    const TANK: u32 = 1;
    const CLERIC: u32 = 2;
    const STRANGER: u32 = 3;
    const BOSS_CODE: u32 = 2_300_810;

    fn window() -> FightWindow {
        FightWindow {
            started_at: 100.0,
            ended_at: 160.0,
        }
    }

    fn hit(at: f64, victim_id: u32, skill_code: u32, amount: u64) -> IncomingHit {
        IncomingHit {
            at,
            victim_id,
            source_id: 900,
            source_mob_code: Some(BOSS_CODE),
            owner_mob_code: None,
            skill_code,
            amount,
        }
    }

    fn heal(
        at: f64,
        healer_id: u32,
        target_id: u32,
        kind: HealKind,
        amount: u64,
        critical: bool,
    ) -> Heal {
        Heal {
            at,
            healer_id,
            target_id,
            skill_code: 17_100_000,
            kind,
            amount,
            critical,
        }
    }

    fn death(at: f64, victim_id: u32, killer: Killer, last_hit: Option<LastHit>) -> Death {
        Death {
            at,
            victim_id,
            skill_code: 1_607_630,
            killer,
            last_hit,
        }
    }

    fn monster_killer() -> Killer {
        Killer {
            id: 900,
            name: None,
            mob_code: Some(BOSS_CODE),
            owner_mob_code: None,
        }
    }

    struct Fixture {
        names: HashMap<u32, String>,
        servers: HashMap<u32, String>,
        classes: HashMap<u32, String>,
        buffs: BuffIntervals,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                names: HashMap::from([(TANK, "Tank".to_string()), (CLERIC, "Cleric".to_string())]),
                servers: HashMap::new(),
                classes: HashMap::from([(CLERIC, "CLERIC".to_string())]),
                buffs: HashMap::new(),
            }
        }

        fn ledger(&self, dealers: &[u32], events: &CombatEvents) -> FightLedger {
            build_ledger(&LedgerSources {
                target_id: BOSS,
                window: window(),
                damage_dealers: dealers,
                events,
                actors: ActorDirectory {
                    names: &self.names,
                    servers: &self.servers,
                    classes: &self.classes,
                },
                mob_name: &|code| (code == BOSS_CODE).then(|| "Tiere".to_string()),
                buff_intervals: &self.buffs,
            })
        }
    }

    fn player(ledger: &FightLedger, actor_id: u32) -> &LedgerPlayer {
        ledger
            .players
            .iter()
            .find(|player| player.actor_id == actor_id)
            .unwrap()
    }

    #[test]
    fn hits_are_summed_per_monster_skill_with_the_largest_kept() {
        let events = CombatEvents {
            hits: vec![
                hit(110.0, TANK, 1_607_630, 400),
                hit(111.0, TANK, 1_607_630, 900),
                hit(112.0, TANK, 1_607_640, 100),
            ],
            ..CombatEvents::default()
        };

        let ledger = Fixture::new().ledger(&[TANK], &events);

        let taken = &player(&ledger, TANK).damage_taken;
        assert_eq!(taken.len(), 2);
        assert_eq!(
            taken[0],
            TakenSkill {
                monster_code: Some(BOSS_CODE),
                monster_name: Some("Tiere".to_string()),
                skill_code: 1_607_630,
                hits: 2,
                largest: 900,
                amount: 1_300,
            }
        );
    }

    #[test]
    fn a_player_nobody_hit_has_an_entry_with_nothing_taken() {
        let ledger = Fixture::new().ledger(&[TANK], &CombatEvents::default());

        assert_eq!(player(&ledger, TANK).damage_taken, Vec::new());
        assert_eq!(player(&ledger, TANK).healing, Vec::new());
    }

    #[test]
    fn a_healer_who_never_hit_the_boss_is_in_the_fight_when_they_healed_someone_who_did() {
        let events = CombatEvents {
            heals: vec![
                heal(110.0, CLERIC, TANK, HealKind::Direct, 1_000, false),
                heal(120.0, CLERIC, TANK, HealKind::Direct, 3_000, true),
            ],
            ..CombatEvents::default()
        };

        let ledger = Fixture::new().ledger(&[TANK], &events);

        assert_eq!(ledger.players.len(), 2);
        let cleric = player(&ledger, CLERIC);
        assert_eq!(cleric.actor_name, "Cleric");
        assert_eq!(cleric.actor_class, "CLERIC");
        assert_eq!(
            cleric.healing,
            [HealSkill {
                skill_code: 17_100_000,
                kind: HealKind::Direct,
                heals: 2,
                crits: 1,
                amount: 4_000
            }]
        );
    }

    #[test]
    fn a_heal_of_someone_outside_the_fight_is_not_counted() {
        let events = CombatEvents {
            heals: vec![heal(
                110.0,
                CLERIC,
                STRANGER,
                HealKind::Direct,
                1_000,
                false,
            )],
            ..CombatEvents::default()
        };

        let ledger = Fixture::new().ledger(&[TANK], &events);

        assert_eq!(ledger.players.len(), 1);
    }

    #[test]
    fn heals_of_one_skill_are_kept_apart_by_kind() {
        let events = CombatEvents {
            heals: vec![
                heal(110.0, TANK, TANK, HealKind::Direct, 500, false),
                heal(111.0, TANK, TANK, HealKind::OverTime, 50, false),
            ],
            ..CombatEvents::default()
        };

        let ledger = Fixture::new().ledger(&[TANK], &events);

        assert_eq!(player(&ledger, TANK).healing.len(), 2);
    }

    #[test]
    fn a_death_names_its_killer_and_the_last_hit_and_counts_from_the_first_hit() {
        let events = CombatEvents {
            deaths: vec![death(
                142.5,
                TANK,
                monster_killer(),
                Some(LastHit {
                    source_id: 900,
                    source_mob_code: Some(BOSS_CODE),
                    owner_mob_code: None,
                    skill_code: 1_607_630,
                    amount: 52_300,
                }),
            )],
            ..CombatEvents::default()
        };

        let ledger = Fixture::new().ledger(&[TANK], &events);

        assert_eq!(
            ledger.deaths,
            [LedgerDeath {
                at_secs: 42.5,
                actor_id: TANK,
                killer: LedgerKiller {
                    kind: KillerKind::Monster,
                    name: Some("Tiere".to_string()),
                    monster_code: Some(BOSS_CODE),
                },
                skill_code: 1_607_630,
                last_hit: Some(LedgerLastHit {
                    monster_code: Some(BOSS_CODE),
                    monster_name: Some("Tiere".to_string()),
                    skill_code: 1_607_630,
                    amount: 52_300,
                }),
            }]
        );
    }

    #[test]
    fn a_player_killer_and_an_unknown_killer_are_told_apart() {
        let player_killer = Killer {
            id: 5,
            name: Some("Veyra".to_string()),
            mob_code: None,
            owner_mob_code: None,
        };
        let unknown = Killer {
            id: 0,
            name: None,
            mob_code: None,
            owner_mob_code: None,
        };

        let ledger = Fixture::new().ledger(
            &[TANK],
            &CombatEvents {
                deaths: vec![
                    death(110.0, TANK, player_killer, None),
                    death(111.0, TANK, unknown, None),
                ],
                ..CombatEvents::default()
            },
        );

        assert_eq!(ledger.deaths[0].killer.kind, KillerKind::Player);
        assert_eq!(ledger.deaths[0].killer.name.as_deref(), Some("Veyra"));
        assert_eq!(ledger.deaths[1].killer.kind, KillerKind::Unknown);
    }

    #[test]
    fn the_death_of_someone_outside_the_fight_is_left_out() {
        let events = CombatEvents {
            deaths: vec![death(110.0, STRANGER, monster_killer(), None)],
            ..CombatEvents::default()
        };

        assert!(Fixture::new().ledger(&[TANK], &events).deaths.is_empty());
    }

    #[test]
    fn buff_uptime_is_clipped_to_the_fight_window_and_overlaps_count_once() {
        let mut fixture = Fixture::new();
        let interval = |from_secs: u64, to_secs: u64| BuffInterval {
            start_ms: from_secs * 1000,
            end_ms: to_secs * 1000,
        };
        fixture.buffs.insert(
            TANK,
            HashMap::from([(
                CLERIC,
                HashMap::from([(
                    18_000_000,
                    VecDeque::from([interval(90, 110), interval(105, 120), interval(150, 200)]),
                )]),
            )]),
        );
        fixture.buffs.insert(
            STRANGER,
            HashMap::from([(
                CLERIC,
                HashMap::from([(18_000_000, VecDeque::from([interval(100, 160)]))]),
            )]),
        );

        let ledger = fixture.ledger(&[TANK], &CombatEvents::default());

        assert_eq!(
            ledger.buffs,
            [BuffUptime {
                target_id: TANK,
                caster_id: CLERIC,
                skill_code: 18_000_000,
                uptime_ms: 30_000
            }]
        );
    }

    #[test]
    fn skill_ids_with_one_name_are_one_buff_whose_uptime_is_the_union_per_caster() {
        let interval = |from_secs: u64, to_secs: u64| BuffInterval {
            start_ms: from_secs * 1000,
            end_ms: to_secs * 1000,
        };
        let spirits_descent_short = 167_300_011;
        let spirits_descent_long = 167_300_012;
        let mut fixture = Fixture::new();
        fixture.buffs.insert(
            TANK,
            HashMap::from([
                (
                    CLERIC,
                    HashMap::from([
                        (spirits_descent_short, VecDeque::from([interval(100, 110), interval(130, 140)])),
                        (spirits_descent_long, VecDeque::from([interval(105, 135)])),
                    ]),
                ),
                (TANK, HashMap::from([(spirits_descent_short, VecDeque::from([interval(100, 110)]))])),
            ]),
        );

        let ledger = fixture.ledger(&[TANK], &CombatEvents::default());

        assert_eq!(
            ledger.buffs,
            [
                BuffUptime { target_id: TANK, caster_id: CLERIC, skill_code: spirits_descent_long, uptime_ms: 40_000 },
                BuffUptime { target_id: TANK, caster_id: TANK, skill_code: spirits_descent_short, uptime_ms: 10_000 },
            ],
            "0-40 s once (not the 30 s longest, not the 50 s sum); the other caster stays apart"
        );
    }

    #[test]
    fn a_buff_on_the_boss_is_kept() {
        let mut fixture = Fixture::new();
        fixture.buffs.insert(
            BOSS,
            HashMap::from([(
                TANK,
                HashMap::from([(
                    11_000_000,
                    VecDeque::from([BuffInterval {
                        start_ms: 100_000,
                        end_ms: 130_000,
                    }]),
                )]),
            )]),
        );

        let ledger = fixture.ledger(&[TANK], &CombatEvents::default());

        assert_eq!(ledger.buffs[0].uptime_ms, 30_000);
    }

    #[test]
    fn the_window_comes_from_the_first_and_last_hit_on_the_target() {
        let target = TargetInfo {
            id: BOSS,
            target_mob_code: Some(BOSS_CODE),
            target_name: None,
            is_boss: true,
            current_hp: None,
            max_hp: None,
            target_start_time: HashMap::from([(TANK, 105.0), (CLERIC, 101.0)]),
            target_last_time: HashMap::from([(TANK, 150.0), (CLERIC, 158.0)]),
        };

        let window = FightWindow::of(&target).unwrap();

        assert_eq!(
            window,
            FightWindow {
                started_at: 101.0,
                ended_at: 158.0
            }
        );
        assert_eq!(window.event_range(), (101.0, 158.0 + END_GRACE_SECS));
    }
}
