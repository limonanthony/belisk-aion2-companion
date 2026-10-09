//! The end-of-fight report of a boss fight: one value carrying every figure the
//! report window and the player details show.
//!
//! A report is built from a saved [`HistoryRecord`] or from the fight still on
//! the meter. Whatever a fight saved before the meter kept healing, damage taken
//! and deaths could not say is `None`, which is not the same as zero: "not
//! recorded" for the figure, the list or the tab.

use std::collections::HashMap;

use crate::history::HistoryRecord;
use crate::models::combat::{
    ActorInfo, BuffSummary, CombatInfos, CombatSnapshot, PlayerOverviewStat, SkillStats, TargetInfo,
};
use crate::report::events::HealKind;
use crate::report::ledger::{FightLedger, FightWindow, KillerKind, LedgerPlayer};

/// How the fight stands, from the boss's health when it was last seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightOutcome {
    Defeated,
    /// The boss lived. The fight may resume.
    Ended,
    /// The boss lived and the attempt was lost: its health went back to full,
    /// or it left the field. `hp_left` is the lowest the boss got.
    Wiped { hp_left: u32, max_hp: u32 },
}

/// How often each kind of hit landed, as counts. A rate is a count over `hits`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HitTypes {
    pub hits: u32,
    pub critical: u32,
    pub back: u32,
    pub front: u32,
    pub double: u32,
    pub perfect: u32,
    pub parry: u32,
    /// Hits that struck more than once.
    pub multi_hit: u32,
}

impl HitTypes {
    /// `count / hits`, or `None` without a hit.
    pub fn rate(&self, count: u32) -> Option<f64> {
        (self.hits > 0).then(|| f64::from(count) / f64::from(self.hits))
    }

    fn of(hits: u32, special_counts: &HashMap<String, u32>) -> Self {
        let count = |kind: &str| special_counts.get(kind).copied().unwrap_or(0);
        Self {
            hits,
            critical: count("CRITICAL"),
            back: count("BACK"),
            front: count("FRONT"),
            double: count("DOUBLE"),
            perfect: count("PERFECT"),
            parry: count("PARRY"),
            multi_hit: count("MULTIHIT"),
        }
    }
}

/// A skill's damage to the boss.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillReport {
    pub skill_code: u32,
    pub damage: u64,
    pub min_damage: u64,
    pub max_damage: u64,
    pub hit_types: HitTypes,
    pub multi_hit_damage: u64,
    /// Which of the five specialty slots are taken.
    pub spec: [bool; 5],
}

impl SkillReport {
    pub fn hits(&self) -> u32 {
        self.hit_types.hits
    }

    pub fn average_damage(&self) -> u64 {
        self.damage
            .checked_div(u64::from(self.hit_types.hits))
            .unwrap_or(0)
    }
}

/// A skill's healing. Amounts are what the game reports, overheal included.
#[derive(Debug, Clone, PartialEq)]
pub struct HealReport {
    pub skill_code: u32,
    pub kind: HealKind,
    pub heals: u32,
    pub crits: u32,
    pub amount: u64,
}

/// What one monster skill did to a player.
#[derive(Debug, Clone, PartialEq)]
pub struct TakenReport {
    pub monster_code: Option<u32>,
    pub monster_name: Option<String>,
    pub skill_code: u32,
    pub hits: u32,
    pub largest: u64,
    pub amount: u64,
}

/// The `caster_id` of a buff whose casters the report does not know.
pub const UNKNOWN_CASTER_ID: u32 = 0;

/// A buff on a player or on the boss.
#[derive(Debug, Clone, PartialEq)]
pub struct BuffReport {
    pub skill_code: u32,
    pub caster_id: u32,
    /// `None` when the caster is not in the report.
    pub caster_name: Option<String>,
    pub caster_class: Option<String>,
    /// Time the buff was up within the fight. `None` in a fight saved before
    /// the meter kept it.
    pub uptime_ms: Option<u64>,
    /// `uptime_ms` over the fight time, 0 to 1.
    pub share: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KillerReport {
    pub kind: KillerKind,
    pub name: Option<String>,
    pub monster_code: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LastHitReport {
    pub monster_code: Option<u32>,
    pub monster_name: Option<String>,
    pub skill_code: u32,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeathReport {
    /// Seconds from the first hit on the boss.
    pub at_secs: f64,
    pub actor_id: u32,
    pub name: String,
    pub class: String,
    pub killer: KillerReport,
    /// The skill the game's death record names; 0 if none.
    pub skill_code: u32,
    pub last_hit: Option<LastHitReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerReport {
    pub actor_id: u32,
    pub name: String,
    pub server_id: String,
    pub class: String,
    pub combat_power: Option<u64>,
    /// The local player.
    pub is_you: bool,
    pub damage: u64,
    pub dps: f64,
    pub hit_types: HitTypes,
    /// Most damage first.
    pub skills: Vec<SkillReport>,
    /// Healing done: the heals in `heals`. `None`: not recorded.
    pub healing: Option<u64>,
    /// Damage taken from monsters. `None`: not recorded.
    pub damage_taken: Option<u64>,
    /// Times died. `None`: not recorded.
    pub deaths: Option<u32>,
    /// Per skill, most first; only what [`HealKind::counts_as_healing`], so
    /// the player's, the party's and the per-skill figures agree. `None`: not
    /// recorded.
    pub heals: Option<Vec<HealReport>>,
    /// Per monster skill, most first. `None`: not recorded.
    pub taken: Option<Vec<TakenReport>>,
    /// Buffs on this player, longest up first.
    pub buffs: Vec<BuffReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FightTotals {
    pub damage: u64,
    /// The players' DPS added up.
    pub party_dps: f64,
    pub healing: Option<u64>,
    pub damage_taken: Option<u64>,
    pub deaths: Option<u32>,
}

/// Everything the end-of-fight report of one boss fight shows.
#[derive(Debug, Clone, PartialEq)]
pub struct FightReport {
    pub target_id: u32,
    /// The mob code, for the place the boss belongs to.
    pub mob_code: Option<u32>,
    pub boss_name: Option<String>,
    pub outcome: FightOutcome,
    /// When the last blow landed, in milliseconds since the epoch.
    pub ended_at_ms: u64,
    /// From the first hit on the boss to the last.
    pub duration_secs: f64,
    pub totals: FightTotals,
    /// Most damage first, then the players who only healed.
    pub players: Vec<PlayerReport>,
    /// In the order they happened. `None`: not recorded.
    pub deaths: Option<Vec<DeathReport>>,
    /// Buffs on the boss, longest up first.
    pub boss_buffs: Vec<BuffReport>,
}

struct Parts<'a> {
    target: &'a TargetInfo,
    combat_infos: &'a CombatInfos,
    player_stats: &'a HashMap<u32, PlayerOverviewStat>,
    skill_stats: &'a HashMap<u32, HashMap<u32, SkillStats>>,
    buff_summaries: &'a HashMap<u32, Vec<BuffSummary>>,
    ledger: Option<&'a FightLedger>,
    saved_at_ms: u64,
    wiped: bool,
}

impl FightReport {
    /// The report of a saved boss fight. `None` for any other fight.
    pub fn from_record(record: &HistoryRecord) -> Option<Self> {
        let target = record
            .target_info
            .as_ref()
            .filter(|target| target.is_boss)?;
        Some(Self::build(&Parts {
            target,
            combat_infos: &record.combat_infos,
            player_stats: &record.player_stats,
            skill_stats: &record.player_skill_stats,
            buff_summaries: &record.use_buffs_by_target,
            ledger: record.ledger.as_ref(),
            saved_at_ms: record.created_at,
            wiped: record.wiped,
        }))
    }

    /// The report of a boss fight still on the meter, with the fight's `ledger`
    /// (`DpsCalculator::fight_ledger`; `None` reports what it holds as not
    /// recorded). `DpsMeter::fight_report` does both. `None` when `target_id` is
    /// not a boss somebody has hit.
    pub fn from_snapshot(
        snapshot: &CombatSnapshot,
        target_id: u32,
        ledger: Option<&FightLedger>,
    ) -> Option<Self> {
        let target = snapshot
            .combat_infos
            .target_infos
            .get(&target_id)
            .filter(|target| target.is_boss)?;
        let player_stats = snapshot.by_target_player_stats.get(&target_id)?;
        let no_skills = HashMap::new();
        Some(Self::build(&Parts {
            target,
            combat_infos: &snapshot.combat_infos,
            player_stats,
            skill_stats: snapshot
                .by_target_player_skill_stats
                .get(&target_id)
                .unwrap_or(&no_skills),
            buff_summaries: &snapshot.use_buffs_by_target,
            ledger,
            saved_at_ms: now_ms(),
            wiped: false,
        }))
    }

    /// One player's lines of the report.
    pub fn player(&self, actor_id: u32) -> Option<&PlayerReport> {
        self.players
            .iter()
            .find(|player| player.actor_id == actor_id)
    }

    /// The deaths of one player; `None` when deaths were not recorded.
    pub fn deaths_of(&self, actor_id: u32) -> Option<Vec<&DeathReport>> {
        self.deaths.as_ref().map(|deaths| {
            deaths
                .iter()
                .filter(|death| death.actor_id == actor_id)
                .collect()
        })
    }

    fn build(parts: &Parts) -> Self {
        let Parts {
            target,
            combat_infos,
            player_stats,
            skill_stats,
            buff_summaries,
            ledger,
            saved_at_ms,
            wiped,
        } = *parts;
        let window = FightWindow::of(target);
        let duration_secs = window.map_or(0.0, |window| window.duration_secs());
        let ended_at_ms = window.map_or(saved_at_ms, |window| {
            (window.ended_at * 1000.0).max(0.0) as u64
        });

        let mut ids: Vec<u32> = player_stats.keys().copied().collect();
        if let Some(ledger) = ledger {
            ids.extend(
                ledger
                    .players
                    .iter()
                    .map(|player| player.actor_id)
                    .filter(|id| !player_stats.contains_key(id)),
            );
        }
        let you = find_you(
            combat_infos,
            ids.iter().map(|id| {
                let name = player_stats.get(id).map_or_else(
                    || ledger_player(ledger, *id).map_or("", |player| player.actor_name.as_str()),
                    |stat| stat.actor_name.as_str(),
                );
                (*id, name)
            }),
        );

        let mut players: Vec<PlayerReport> = ids
            .iter()
            .map(|&actor_id| {
                player_report(
                    actor_id,
                    player_stats.get(&actor_id),
                    skill_stats.get(&actor_id),
                    combat_infos.actor_infos.get(&actor_id),
                    ledger,
                    you == Some(actor_id),
                )
            })
            .collect();
        players.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.actor_id.cmp(&b.actor_id)));

        let identity: HashMap<u32, (String, String)> = players
            .iter()
            .map(|player| (player.actor_id, (player.name.clone(), player.class.clone())))
            .collect();
        let caster = |caster_id: u32| identity.get(&caster_id).cloned();

        let deaths = ledger.map(|ledger| {
            ledger
                .deaths
                .iter()
                .map(|death| {
                    let (name, class) = caster(death.actor_id).unwrap_or_default();
                    DeathReport {
                        at_secs: death.at_secs,
                        actor_id: death.actor_id,
                        name,
                        class,
                        killer: KillerReport {
                            kind: death.killer.kind,
                            name: death.killer.name.clone(),
                            monster_code: death.killer.monster_code,
                        },
                        skill_code: death.skill_code,
                        last_hit: death.last_hit.as_ref().map(|hit| LastHitReport {
                            monster_code: hit.monster_code,
                            monster_name: hit.monster_name.clone(),
                            skill_code: hit.skill_code,
                            amount: hit.amount,
                        }),
                    }
                })
                .collect::<Vec<_>>()
        });

        for player in &mut players {
            player.deaths = deaths.as_ref().map(|deaths| {
                deaths
                    .iter()
                    .filter(|death| death.actor_id == player.actor_id)
                    .count() as u32
            });
        }
        let boss_buffs = buff_reports(ledger, buff_summaries, target.id, duration_secs, &caster);
        for player in &mut players {
            player.buffs = buff_reports(
                ledger,
                buff_summaries,
                player.actor_id,
                duration_secs,
                &caster,
            );
        }

        let totals = FightTotals {
            damage: players.iter().map(|player| player.damage).sum(),
            party_dps: players.iter().map(|player| player.dps.max(0.0)).sum(),
            healing: ledger.map(|_| players.iter().filter_map(|player| player.healing).sum()),
            damage_taken: ledger.map(|_| {
                players
                    .iter()
                    .filter_map(|player| player.damage_taken)
                    .sum()
            }),
            deaths: deaths.as_ref().map(|deaths| deaths.len() as u32),
        };

        Self {
            target_id: target.id,
            mob_code: target.target_mob_code,
            boss_name: target.target_name.clone().filter(|name| !name.is_empty()),
            outcome: outcome_of(target, wiped),
            ended_at_ms,
            duration_secs,
            totals,
            players,
            deaths,
            boss_buffs,
        }
    }
}

fn outcome_of(target: &TargetInfo, wiped: bool) -> FightOutcome {
    if wiped {
        return FightOutcome::Wiped {
            hp_left: target.current_hp.unwrap_or(0),
            max_hp: target.max_hp.unwrap_or(0),
        };
    }
    let killed = target.current_hp == Some(0) && target.max_hp.is_some_and(|max| max > 0);
    if killed {
        FightOutcome::Defeated
    } else {
        FightOutcome::Ended
    }
}

fn ledger_player(ledger: Option<&FightLedger>, actor_id: u32) -> Option<&LedgerPlayer> {
    ledger?
        .players
        .iter()
        .find(|player| player.actor_id == actor_id)
}

/// The name wins: ids are entity handles that change between zones, so an id is
/// trusted only on a row nobody has named.
fn find_you<'a>(
    infos: &CombatInfos,
    mut players: impl Iterator<Item = (u32, &'a str)> + Clone,
) -> Option<u32> {
    if let Some(name) = infos
        .main_actor_name
        .as_deref()
        .filter(|name| !name.is_empty())
    {
        if let Some((id, _)) = players.clone().find(|(_, row_name)| *row_name == name) {
            return Some(id);
        }
    }
    let id = infos.main_actor_id?;
    players
        .find(|(row_id, row_name)| *row_id == id && row_name.is_empty())
        .map(|(id, _)| id)
}

fn player_report(
    actor_id: u32,
    stat: Option<&PlayerOverviewStat>,
    skills: Option<&HashMap<u32, SkillStats>>,
    actor: Option<&ActorInfo>,
    ledger: Option<&FightLedger>,
    is_you: bool,
) -> PlayerReport {
    let from_ledger = ledger_player(ledger, actor_id);

    let mut skill_rows: Vec<SkillReport> = skills
        .into_iter()
        .flatten()
        .map(|(&skill_code, stats)| SkillReport {
            skill_code,
            damage: stats.total_damage,
            min_damage: if stats.min_damage == u64::MAX {
                0
            } else {
                stats.min_damage
            },
            max_damage: stats.max_damage,
            hit_types: HitTypes::of(stats.counts, &stats.special_counts),
            multi_hit_damage: u64::from(
                stats
                    .special_counts
                    .get("MULTIHITDMG")
                    .copied()
                    .unwrap_or(0),
            ),
            spec: spec_slots(actor, skill_code),
        })
        .collect();
    skill_rows.sort_by(|a, b| {
        b.damage
            .cmp(&a.damage)
            .then(a.skill_code.cmp(&b.skill_code))
    });

    let heals = from_ledger.map(|player| {
        player
            .healing
            .iter()
            .filter(|skill| skill.kind.counts_as_healing())
            .map(|skill| HealReport {
                skill_code: skill.skill_code,
                kind: skill.kind,
                heals: skill.heals,
                crits: skill.crits,
                amount: skill.amount,
            })
            .collect::<Vec<_>>()
    });
    let taken = from_ledger.map(|player| {
        player
            .damage_taken
            .iter()
            .map(|skill| TakenReport {
                monster_code: skill.monster_code,
                monster_name: skill.monster_name.clone(),
                skill_code: skill.skill_code,
                hits: skill.hits,
                largest: skill.largest,
                amount: skill.amount,
            })
            .collect::<Vec<_>>()
    });

    let hit_types = stat.map_or_else(HitTypes::default, |stat| {
        HitTypes::of(stat.counts, &stat.special_counts)
    });
    PlayerReport {
        actor_id,
        name: stat
            .map(|stat| stat.actor_name.clone())
            .or_else(|| from_ledger.map(|player| player.actor_name.clone()))
            .unwrap_or_default(),
        server_id: stat
            .map(|stat| stat.actor_server_id.clone())
            .or_else(|| from_ledger.map(|player| player.actor_server_id.clone()))
            .unwrap_or_default(),
        class: stat
            .map(|stat| stat.actor_class.clone())
            .or_else(|| from_ledger.map(|player| player.actor_class.clone()))
            .unwrap_or_default(),
        combat_power: stat.and_then(|stat| stat.combat_power),
        is_you,
        damage: stat.map_or(0, |stat| stat.total_damage),
        dps: stat.map_or(0.0, |stat| stat.dps),
        hit_types,
        skills: skill_rows,
        healing: heals
            .as_ref()
            .map(|heals| heals.iter().map(|heal| heal.amount).sum()),
        damage_taken: taken
            .as_ref()
            .map(|taken| taken.iter().map(|skill| skill.amount).sum()),
        deaths: None,
        heals,
        taken,
        buffs: Vec::new(),
    }
}

fn spec_slots(actor: Option<&ActorInfo>, skill_code: u32) -> [bool; 5] {
    let mut spec = [false; 5];
    for slot in actor
        .and_then(|actor| actor.actor_skill_spec.get(&skill_code))
        .into_iter()
        .flatten()
    {
        if (1..=5).contains(slot) {
            spec[*slot as usize - 1] = true;
        }
    }
    spec
}

/// The buffs on `target_id`: with their uptime from the ledger, or, for a fight
/// saved without one, only which buffs were seen. Casters the report does not
/// know (summons the meter could not tie to an owner, mobs, strangers, and the
/// one row per summon that fights saved before owner resolution hold) are one
/// `UNKNOWN_CASTER_ID` row per buff, with the longest uptime among them.
fn buff_reports(
    ledger: Option<&FightLedger>,
    buff_summaries: &HashMap<u32, Vec<BuffSummary>>,
    target_id: u32,
    duration_secs: f64,
    caster: &dyn Fn(u32) -> Option<(String, String)>,
) -> Vec<BuffReport> {
    let seen: Vec<(u32, u32, Option<u64>)> = match ledger {
        Some(ledger) => ledger
            .buffs
            .iter()
            .filter(|buff| buff.target_id == target_id)
            .map(|buff| (buff.skill_code, buff.caster_id, Some(buff.uptime_ms)))
            .collect(),
        None => buff_summaries
            .get(&target_id)
            .into_iter()
            .flatten()
            .map(|buff| (buff.skill_code, buff.actor_id, None))
            .collect(),
    };

    let mut rows: Vec<(u32, u32, Option<u64>, Option<(String, String)>)> = Vec::with_capacity(seen.len());
    let mut row_of: HashMap<(u32, u32), usize> = HashMap::new();
    for (skill_code, caster_id, uptime_ms) in seen {
        let identity = caster(caster_id);
        let caster_id = if identity.is_some() { caster_id } else { UNKNOWN_CASTER_ID };
        match row_of.get(&(skill_code, caster_id)) {
            Some(&index) => rows[index].2 = rows[index].2.max(uptime_ms),
            None => {
                row_of.insert((skill_code, caster_id), rows.len());
                rows.push((skill_code, caster_id, uptime_ms, identity));
            }
        }
    }
    if ledger.is_some() {
        rows.sort_by_key(|&(skill_code, caster_id, uptime_ms, _)| {
            (std::cmp::Reverse(uptime_ms), skill_code, caster_id)
        });
    }

    rows.into_iter()
        .map(|(skill_code, caster_id, uptime_ms, identity)| BuffReport {
            skill_code,
            caster_id,
            caster_name: identity.as_ref().map(|(name, _)| name.clone()),
            caster_class: identity.map(|(_, class)| class),
            uptime_ms,
            share: uptime_ms.and_then(|ms| {
                (duration_secs > 0.0).then(|| (ms as f64 / 1000.0 / duration_secs).clamp(0.0, 1.0))
            }),
        })
        .collect()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ledger::{
        BuffUptime, HealSkill, LedgerDeath, LedgerKiller, LedgerLastHit, TakenSkill,
    };

    const BOSS: u32 = 56_499;
    const YSMERA: u32 = 1_845;
    const SAEBRIN: u32 = 1_846;
    const MIRELLE: u32 = 1_847;

    fn record_saved_before_the_ledger() -> HistoryRecord {
        serde_json::from_str::<Vec<HistoryRecord>>(include_str!(
            "fixtures/head-history-record.json"
        ))
        .unwrap()
        .remove(0)
    }

    fn ledger() -> FightLedger {
        let player = |actor_id: u32,
                      name: &str,
                      healing: Vec<HealSkill>,
                      damage_taken: Vec<TakenSkill>| LedgerPlayer {
            actor_id,
            actor_name: name.to_string(),
            actor_server_id: "1302".to_string(),
            actor_class: "CLERIC".to_string(),
            healing,
            damage_taken,
        };
        FightLedger {
            window: FightWindow {
                started_at: 1_791_000_000.0,
                ended_at: 1_791_000_060.0,
            },
            players: vec![
                player(
                    YSMERA,
                    "Ysmera",
                    Vec::new(),
                    vec![TakenSkill {
                        monster_code: Some(2_980_153),
                        monster_name: Some("Furious Feruk".to_string()),
                        skill_code: 1_607_630,
                        hits: 6,
                        largest: 1_130,
                        amount: 6_474,
                    }],
                ),
                player(
                    SAEBRIN,
                    "Saebrin",
                    vec![HealSkill {
                        skill_code: 17_800_000,
                        kind: HealKind::Direct,
                        heals: 3,
                        crits: 1,
                        amount: 3_000,
                    }],
                    Vec::new(),
                ),
                player(
                    MIRELLE,
                    "Mirelle",
                    vec![HealSkill {
                        skill_code: 17_100_000,
                        kind: HealKind::Direct,
                        heals: 2,
                        crits: 0,
                        amount: 900,
                    }],
                    Vec::new(),
                ),
            ],
            deaths: vec![LedgerDeath {
                at_secs: 42.5,
                actor_id: SAEBRIN,
                killer: LedgerKiller {
                    kind: KillerKind::Monster,
                    name: Some("Furious Feruk".to_string()),
                    monster_code: Some(2_980_153),
                },
                skill_code: 1_607_630,
                last_hit: Some(LedgerLastHit {
                    monster_code: Some(2_980_153),
                    monster_name: Some("Furious Feruk".to_string()),
                    skill_code: 1_607_630,
                    amount: 1_130,
                }),
            }],
            buffs: vec![
                BuffUptime {
                    target_id: YSMERA,
                    caster_id: SAEBRIN,
                    skill_code: 18_000_000,
                    uptime_ms: 30_000,
                },
                BuffUptime {
                    target_id: BOSS,
                    caster_id: YSMERA,
                    skill_code: 16_300_008,
                    uptime_ms: 45_000,
                },
            ],
        }
    }

    fn record_with_the_ledger() -> HistoryRecord {
        HistoryRecord {
            ledger: Some(ledger()),
            ..record_saved_before_the_ledger()
        }
    }

    #[test]
    fn a_fight_saved_before_the_ledger_opens_with_healing_taken_and_deaths_not_recorded() {
        let report = FightReport::from_record(&record_saved_before_the_ledger()).unwrap();

        assert_eq!(report.totals.healing, None);
        assert_eq!(report.totals.damage_taken, None);
        assert_eq!(report.totals.deaths, None);
        assert_eq!(report.deaths, None);
        for player in &report.players {
            assert_eq!(
                (player.healing, player.damage_taken, player.deaths),
                (None, None, None)
            );
            assert_eq!((&player.heals, &player.taken), (&None, &None));
        }
    }

    #[test]
    fn a_fight_saved_before_the_ledger_still_shows_damage_skills_and_the_buffs_seen() {
        let report = FightReport::from_record(&record_saved_before_the_ledger()).unwrap();

        assert_eq!(report.boss_name.as_deref(), Some("Furious Feruk"));
        assert_eq!(report.outcome, FightOutcome::Defeated);
        assert_eq!(report.duration_secs, 60.0);
        assert_eq!(report.ended_at_ms, 1_791_000_060_000);
        assert_eq!(report.totals.damage, 1_680_000);
        let ysmera = report.player(YSMERA).unwrap();
        assert!(ysmera.is_you);
        assert_eq!(ysmera.skills[0].hit_types.critical, 40);
        assert_eq!(ysmera.skills[0].spec, [false, true, false, true, false]);
        assert_eq!(ysmera.hit_types.critical, 364);
        assert_eq!(ysmera.hit_types.rate(364), Some(364.0 / 811.0));
        assert_eq!(report.boss_buffs.len(), 1);
        assert_eq!(report.boss_buffs[0].caster_name.as_deref(), Some("Ysmera"));
        assert_eq!(
            (report.boss_buffs[0].uptime_ms, report.boss_buffs[0].share),
            (None, None)
        );
    }

    #[test]
    fn a_fight_with_a_ledger_reports_what_the_party_took_healed_and_lost() {
        let report = FightReport::from_record(&record_with_the_ledger()).unwrap();

        assert_eq!(report.totals.healing, Some(3_900));
        assert_eq!(report.totals.damage_taken, Some(6_474));
        assert_eq!(report.totals.deaths, Some(1));
        assert_eq!(report.player(YSMERA).unwrap().damage_taken, Some(6_474));
        assert_eq!(report.player(YSMERA).unwrap().healing, Some(0));
        assert_eq!(report.player(SAEBRIN).unwrap().healing, Some(3_000));
        assert_eq!(report.player(SAEBRIN).unwrap().deaths, Some(1));
        assert_eq!(report.player(YSMERA).unwrap().deaths, Some(0));
        let deaths = report.deaths.as_ref().unwrap();
        assert_eq!(
            (deaths[0].name.as_str(), deaths[0].at_secs),
            ("Saebrin", 42.5)
        );
        assert_eq!(deaths[0].last_hit.as_ref().unwrap().amount, 1_130);
        assert_eq!(report.deaths_of(SAEBRIN).unwrap().len(), 1);
        assert_eq!(report.deaths_of(YSMERA).unwrap().len(), 0);
    }

    #[test]
    fn a_player_who_only_healed_is_in_the_report_after_the_damage_dealers() {
        let report = FightReport::from_record(&record_with_the_ledger()).unwrap();

        let last = report.players.last().unwrap();
        assert_eq!(
            (last.actor_id, last.name.as_str(), last.damage),
            (MIRELLE, "Mirelle", 0)
        );
        assert_eq!(last.healing, Some(900));
        assert_eq!(report.players.len(), 3);
    }

    #[test]
    fn life_drain_and_regeneration_on_hits_are_not_counted_as_healing() {
        let mut record = record_with_the_ledger();
        let saebrin = record
            .ledger
            .as_mut()
            .unwrap()
            .players
            .iter_mut()
            .find(|player| player.actor_id == SAEBRIN)
            .unwrap();
        for (kind, amount) in [(HealKind::Drain, 700), (HealKind::Regeneration, 5_000), (HealKind::OverTime, 400)] {
            saebrin.healing.push(HealSkill {
                skill_code: 13_000_001,
                kind,
                heals: 10,
                crits: 0,
                amount,
            });
        }

        let report = FightReport::from_record(&record).unwrap();

        let saebrin = report.player(SAEBRIN).unwrap();
        assert_eq!(saebrin.healing, Some(3_400));
        let kinds: Vec<HealKind> = saebrin.heals.as_ref().unwrap().iter().map(|heal| heal.kind).collect();
        assert_eq!(kinds, vec![HealKind::Direct, HealKind::OverTime]);
        assert_eq!(report.totals.healing, Some(4_300));
    }

    #[test]
    fn buff_uptime_is_shown_with_its_share_of_the_fight() {
        let report = FightReport::from_record(&record_with_the_ledger()).unwrap();

        let on_player = &report.player(YSMERA).unwrap().buffs;
        assert_eq!(on_player.len(), 1);
        assert_eq!(on_player[0].uptime_ms, Some(30_000));
        assert_eq!(on_player[0].share, Some(0.5));
        assert_eq!(on_player[0].caster_name.as_deref(), Some("Saebrin"));
        assert_eq!(report.boss_buffs[0].share, Some(0.75));
    }

    #[test]
    fn the_casters_the_report_does_not_know_are_one_row_per_buff() {
        let mut record = record_with_the_ledger();
        let ledger = record.ledger.as_mut().unwrap();
        for (summon, uptime_ms) in [(40_001, 4_000), (40_002, 6_000), (40_003, 2_000)] {
            ledger.buffs.push(BuffUptime {
                target_id: YSMERA,
                caster_id: summon,
                skill_code: 118_000_071,
                uptime_ms,
            });
        }
        ledger.buffs.push(BuffUptime {
            target_id: YSMERA,
            caster_id: 40_001,
            skill_code: 174_100_111,
            uptime_ms: 3_000,
        });

        let report = FightReport::from_record(&record).unwrap();

        let rows: Vec<(u32, u32, Option<u64>)> = report
            .player(YSMERA)
            .unwrap()
            .buffs
            .iter()
            .map(|buff| (buff.skill_code, buff.caster_id, buff.uptime_ms))
            .collect();
        assert_eq!(
            rows,
            [
                (18_000_000, SAEBRIN, Some(30_000)),
                (118_000_071, UNKNOWN_CASTER_ID, Some(6_000)),
                (174_100_111, UNKNOWN_CASTER_ID, Some(3_000)),
            ]
        );
    }

    #[test]
    fn a_buff_a_known_caster_and_a_stranger_cast_keeps_both_rows() {
        let mut record = record_with_the_ledger();
        record.ledger.as_mut().unwrap().buffs.push(BuffUptime {
            target_id: YSMERA,
            caster_id: 40_001,
            skill_code: 18_000_000,
            uptime_ms: 5_000,
        });

        let report = FightReport::from_record(&record).unwrap();

        let casters: Vec<u32> = report.player(YSMERA).unwrap().buffs.iter().map(|buff| buff.caster_id).collect();
        assert_eq!(casters, [SAEBRIN, UNKNOWN_CASTER_ID]);
    }

    #[test]
    fn a_fight_that_is_not_a_bosss_has_no_report() {
        let mut record = record_with_the_ledger();
        record.target_info.as_mut().unwrap().is_boss = false;

        assert_eq!(FightReport::from_record(&record), None);
    }

    #[test]
    fn a_boss_that_lived_is_a_fight_that_ended() {
        let mut record = record_with_the_ledger();
        record.target_info.as_mut().unwrap().current_hp = Some(400);

        assert_eq!(
            FightReport::from_record(&record).unwrap().outcome,
            FightOutcome::Ended
        );
    }

    #[test]
    fn a_wiped_attempt_is_a_wipe_with_the_health_the_boss_had_left() {
        let mut record = record_with_the_ledger();
        record.wiped = true;
        {
            let target = record.target_info.as_mut().unwrap();
            target.current_hp = Some(300);
            target.max_hp = Some(1_000);
        }

        assert_eq!(
            FightReport::from_record(&record).unwrap().outcome,
            FightOutcome::Wiped { hp_left: 300, max_hp: 1_000 }
        );
    }

    #[test]
    fn the_party_dps_adds_the_players_up() {
        let report = FightReport::from_record(&record_saved_before_the_ledger()).unwrap();

        let expected: f64 = report.players.iter().map(|player| player.dps).sum();
        assert_eq!(report.totals.party_dps, expected);
        assert!(expected > 0.0);
    }
}
