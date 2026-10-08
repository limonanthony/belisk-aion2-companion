//! What the player detail window shows, worked out from a live snapshot or a
//! history record. Rules ported from `overlay/detail/main.js` of the Tauri app.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use meter_core::engine::session::session_skill_totals;
use meter_core::history::HistoryRecord;
use meter_core::models::combat::{
    ActorInfo, BuffSummary, CombatSnapshot, PlayerOverviewStat, SkillStats, TargetInfo,
};

use crate::game_data::{self, PlayerClass};
use crate::meter::OverlaySettings;

/// What the window was opened on.
#[derive(Clone, Debug)]
pub enum DetailSource {
    /// A player's skills on one target. `target_id: None` follows the target
    /// you hit last, as Aether did.
    Target { actor_id: u32, target_id: Option<u32> },
    /// A player's skills across every mob since the last reset, the Session
    /// board's view of them.
    Session { actor_id: u32 },
    /// A player in a saved fight; frozen.
    History { actor_id: u32, record: Arc<HistoryRecord> },
}

impl DetailSource {
    pub fn actor_id(&self) -> u32 {
        match self {
            Self::Target { actor_id, .. } | Self::Session { actor_id } | Self::History { actor_id, .. } => *actor_id,
        }
    }

    /// Whether new snapshots change what this source shows.
    pub fn is_live(&self) -> bool {
        !matches!(self, Self::History { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailMode {
    Live,
    Session,
    History,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Identity {
    /// Masked when the meter masks nicknames; `ID:<n>` while the game has not
    /// named the player.
    pub name: String,
    pub class: Option<PlayerClass>,
    /// Short server name, empty when unknown.
    pub server: String,
    pub combat_power: Option<u64>,
}

/// Rates are fractions of the player's hits (0 to 1), `None` with no hits.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub damage: u64,
    pub dps: f64,
    /// Zero when unknown.
    pub fight_secs: f64,
    pub hits: u32,
    pub critical: Option<f64>,
    pub back: Option<f64>,
    pub front: Option<f64>,
    pub double: Option<f64>,
    pub perfect: Option<f64>,
    pub parry: Option<f64>,
    pub multi: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MultiHit {
    /// How many hits one use of the skill landed ("2 hits").
    pub hits: u32,
    pub count: u32,
    /// Of the skill's uses.
    pub share: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillRow {
    pub code: u32,
    pub name: String,
    /// Asset path, when the game data has an icon.
    pub icon: Option<String>,
    /// Which of the five spec slots are invested in this skill.
    pub spec: [bool; 5],
    pub hits: u32,
    pub total: u64,
    /// Of the player's damage in the table.
    pub share: f64,
    pub min: u64,
    pub max: u64,
    pub average: u64,
    pub critical: Option<f64>,
    pub perfect: Option<f64>,
    pub double: Option<f64>,
    pub front: Option<f64>,
    pub back: Option<f64>,
    pub parry: Option<f64>,
    pub multi: Option<f64>,
    pub multi_damage: u64,
    pub multi_breakdown: Vec<MultiHit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuffRow {
    pub code: u32,
    pub name: String,
    pub icon: Option<String>,
    pub caster: String,
    pub caster_id: u32,
    pub caster_class: Option<PlayerClass>,
    /// Share of the fight the buff was up, 0 to 1.
    pub coverage: f64,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DetailData {
    pub mode: DetailMode,
    pub actor_id: u32,
    pub identity: Identity,
    /// The target the figures are about; none on the Session board.
    pub target_name: Option<String>,
    pub target_is_boss: bool,
    pub summary: Summary,
    pub skills: Vec<SkillRow>,
    pub player_buffs: Vec<BuffRow>,
    pub target_buffs: Vec<BuffRow>,
}

/// Everything one build reads, whichever kind of source it came from.
struct Facts<'a> {
    mode: DetailMode,
    actor: Option<&'a ActorInfo>,
    overview: Option<&'a PlayerOverviewStat>,
    skills: Cow<'a, HashMap<u32, SkillStats>>,
    target: Option<&'a TargetInfo>,
    target_id: Option<u32>,
    actor_infos: &'a HashMap<u32, ActorInfo>,
    buffs: &'a HashMap<u32, Vec<BuffSummary>>,
    /// Session only: the board's own row for the player.
    session_row: Option<SessionFigures<'a>>,
    main_actor_id: Option<u32>,
}

struct SessionFigures<'a> {
    name: &'a str,
    server: &'a str,
    class: &'a str,
    combat_power: Option<u64>,
    dps: f64,
    combat_secs: f64,
}

impl DetailData {
    /// `None` when there is nothing to show: no snapshot yet, or the player is
    /// in none of the tables.
    pub fn build(source: &DetailSource, snapshot: Option<&CombatSnapshot>, settings: &OverlaySettings) -> Option<Self> {
        let facts = match source {
            DetailSource::History { actor_id, record } => from_record(*actor_id, record),
            DetailSource::Target { actor_id, target_id } => from_target(*actor_id, *target_id, snapshot?),
            DetailSource::Session { actor_id } => from_session(*actor_id, snapshot?),
        };
        facts.finish(source.actor_id(), settings)
    }
}

fn from_record(actor_id: u32, record: &HistoryRecord) -> Facts<'_> {
    let infos = &record.combat_infos;
    Facts {
        mode: DetailMode::History,
        actor: infos.actor_infos.get(&actor_id),
        overview: record.player_stats.get(&actor_id),
        skills: Cow::Borrowed(record.player_skill_stats.get(&actor_id).unwrap_or(&NO_SKILLS)),
        target: record.target_info.as_ref(),
        target_id: Some(record.target_id),
        actor_infos: &infos.actor_infos,
        buffs: &record.use_buffs_by_target,
        session_row: None,
        main_actor_id: infos.main_actor_id,
    }
}

fn from_target(actor_id: u32, target_id: Option<u32>, snapshot: &CombatSnapshot) -> Facts<'_> {
    let infos = &snapshot.combat_infos;
    let target_id = target_id.or(infos.last_target_by_main_actor);
    let per_actor = target_id.and_then(|id| snapshot.by_target_player_skill_stats.get(&id));
    Facts {
        mode: DetailMode::Live,
        actor: infos.actor_infos.get(&actor_id),
        overview: target_id.and_then(|id| snapshot.by_target_player_stats.get(&id)?.get(&actor_id)),
        skills: Cow::Borrowed(per_actor.and_then(|actors| actors.get(&actor_id)).unwrap_or(&NO_SKILLS)),
        target: target_id
            .and_then(|id| infos.target_infos.get(&id))
            .or(snapshot.last_target_info.as_ref()),
        target_id,
        actor_infos: &infos.actor_infos,
        buffs: &snapshot.use_buffs_by_target,
        session_row: None,
        main_actor_id: infos.main_actor_id,
    }
}

fn from_session(actor_id: u32, snapshot: &CombatSnapshot) -> Facts<'_> {
    let infos = &snapshot.combat_infos;
    let is_mob = |target_id: u32| {
        let described_as_mob = infos.target_infos.get(&target_id).is_some_and(|info| info.target_mob_code.is_some());
        let is_player = infos
            .actor_infos
            .get(&target_id)
            .is_some_and(|actor| actor.actor_name.as_deref().is_some_and(|name| !name.is_empty()));
        described_as_mob || !is_player
    };
    let row = snapshot.session.players.iter().find(|player| player.actor_id == actor_id);
    let row_actor_ids: Vec<u32> = row.map_or_else(
        || vec![actor_id],
        |row| std::iter::once(row.actor_id).chain(row.earlier_actor_ids.iter().copied()).collect(),
    );
    Facts {
        mode: DetailMode::Session,
        actor: infos.actor_infos.get(&actor_id),
        overview: None,
        skills: Cow::Owned(session_skill_totals(&snapshot.by_target_player_skill_stats, &row_actor_ids, is_mob)),
        target: None,
        target_id: None,
        actor_infos: &infos.actor_infos,
        buffs: &snapshot.use_buffs_by_target,
        main_actor_id: infos.main_actor_id,
        session_row: row.map(|row| SessionFigures {
            name: &row.actor_name,
            server: &row.actor_server_id,
            class: &row.actor_class,
            combat_power: row.combat_power,
            dps: row.dps,
            combat_secs: row.combat_time_secs,
        }),
    }
}

static NO_SKILLS: std::sync::LazyLock<HashMap<u32, SkillStats>> = std::sync::LazyLock::new(HashMap::new);

impl Facts<'_> {
    fn finish(self, actor_id: u32, settings: &OverlaySettings) -> Option<DetailData> {
        if self.actor.is_none() && self.overview.is_none() && self.session_row.is_none() && self.skills.is_empty() {
            return None;
        }
        let identity = self.identity(actor_id, settings);
        let mut skills: Vec<SkillRow> = self
            .skills
            .iter()
            .map(|(code, stats)| skill_row(*code, stats, self.actor))
            .collect();
        skills.sort_by(|a, b| b.total.cmp(&a.total).then(a.code.cmp(&b.code)));
        let damage: u64 = skills.iter().map(|skill| skill.total).sum();
        for skill in &mut skills {
            skill.share = if damage > 0 { skill.total as f64 / damage as f64 } else { 0.0 };
        }

        let summary = self.summary(damage);
        let player_buffs = buff_rows(self.buffs.get(&actor_id), self.actor_infos, settings);
        let target_buffs = self
            .target_id
            .filter(|_| self.mode != DetailMode::Session)
            .map(|id| buff_rows(self.buffs.get(&id), self.actor_infos, settings))
            .unwrap_or_default();
        Some(DetailData {
            mode: self.mode,
            actor_id,
            identity,
            target_name: self.target.map(crate::meter::target_name),
            target_is_boss: self.target.is_some_and(|target| target.is_boss),
            summary,
            skills,
            player_buffs,
            target_buffs,
        })
    }

    fn identity(&self, actor_id: u32, settings: &OverlaySettings) -> Identity {
        let named = |name: &str| (!name.is_empty()).then(|| name.to_string());
        let name = self
            .session_row
            .as_ref()
            .and_then(|row| named(row.name))
            .or_else(|| self.actor.and_then(|actor| actor.actor_name.as_deref().and_then(named)))
            .or_else(|| self.overview.and_then(|overview| named(&overview.actor_name)))
            .map_or_else(
                || crate::format::unnamed_player(actor_id, self.main_actor_id == Some(actor_id)),
                |name| settings.display_name(&name),
            );
        let class = [
            self.session_row.as_ref().map(|row| row.class),
            self.actor.and_then(|actor| actor.actor_class.as_deref()),
            self.overview.map(|overview| overview.actor_class.as_str()),
        ]
        .into_iter()
        .flatten()
        .find_map(PlayerClass::parse);
        let server = [
            self.session_row.as_ref().map(|row| row.server),
            self.actor.and_then(|actor| actor.actor_server_id.as_deref()),
            self.overview.map(|overview| overview.actor_server_id.as_str()),
        ]
        .into_iter()
        .flatten()
        .find(|server| !server.is_empty())
        .map(game_data::server_name)
        .unwrap_or_default();
        let combat_power = self
            .session_row
            .as_ref()
            .and_then(|row| row.combat_power)
            .or(self.actor.and_then(|actor| actor.combat_power))
            .or(self.overview.and_then(|overview| overview.combat_power))
            .filter(|power| *power > 0);
        Identity { name, class, server, combat_power }
    }

    fn summary(&self, damage: u64) -> Summary {
        let table_hits: u32 = self.skills.values().map(|stats| stats.counts).sum();
        let hits = self.overview.map(|overview| overview.counts).filter(|hits| *hits > 0).unwrap_or(table_hits);
        let special = |kind: &str| -> u32 { self.skills.values().map(|stats| stats.special_counts.get(kind).copied().unwrap_or(0)).sum() };
        let fraction = |kind: &str| rate(special(kind), hits);

        let (fight_secs, dps) = match &self.session_row {
            Some(row) => (row.combat_secs, row.dps),
            None => {
                let secs = self.actor_fight_secs().max(1.0);
                (secs, damage as f64 / secs)
            }
        };
        Summary {
            damage,
            dps,
            fight_secs,
            hits,
            critical: fraction("CRITICAL"),
            back: fraction("BACK"),
            front: fraction("FRONT"),
            double: fraction("DOUBLE"),
            perfect: fraction("PERFECT"),
            parry: fraction("PARRY"),
            multi: fraction("MULTIHIT"),
        }
    }

    /// From the player's first hit on the target to their last.
    fn actor_fight_secs(&self) -> f64 {
        let (Some(target), Some(actor)) = (self.target, self.actor.map(|actor| actor.id).or(self.overview.map(|o| o.actor_id)))
        else {
            return 0.0;
        };
        match (target.target_start_time.get(&actor), target.target_last_time.get(&actor)) {
            (Some(start), Some(end)) => (end - start).max(0.0),
            _ => 0.0,
        }
    }
}

fn rate(count: u32, hits: u32) -> Option<f64> {
    (hits > 0).then(|| f64::from(count) / f64::from(hits))
}

fn skill_row(code: u32, stats: &SkillStats, actor: Option<&ActorInfo>) -> SkillRow {
    let special = |kind: &str| stats.special_counts.get(kind).copied().unwrap_or(0);
    let hits = stats.counts;
    let multi_damage = u64::from(special("MULTIHITDMG"));
    let mut breakdown: Vec<MultiHit> = stats
        .special_counts
        .iter()
        .filter_map(|(kind, count)| {
            let hits_per_use: u32 = kind.strip_prefix("MULTIHIT")?.parse().ok()?;
            (*count > 0).then(|| MultiHit { hits: hits_per_use, count: *count, share: f64::from(*count) / f64::from(hits.max(1)) })
        })
        .collect();
    breakdown.sort_by_key(|multi| multi.hits);

    let mut spec = [false; 5];
    for slot in actor.and_then(|actor| actor.actor_skill_spec.get(&code)).into_iter().flatten() {
        if (1..=5).contains(slot) {
            spec[*slot as usize - 1] = true;
        }
    }
    SkillRow {
        code,
        name: game_data::skill_name(code),
        icon: game_data::skill_icon(code),
        spec,
        hits,
        total: stats.total_damage,
        share: 0.0,
        min: if stats.min_damage == u64::MAX { 0 } else { stats.min_damage },
        max: stats.max_damage,
        average: if hits > 0 { stats.total_damage / u64::from(hits) } else { 0 },
        critical: rate(special("CRITICAL"), hits),
        perfect: rate(special("PERFECT"), hits),
        double: rate(special("DOUBLE"), hits),
        front: rate(special("FRONT"), hits),
        back: rate(special("BACK"), hits),
        parry: rate(special("PARRY"), hits),
        multi: rate(special("MULTIHIT"), hits),
        multi_damage,
        multi_breakdown: breakdown,
    }
}

/// Active buffs first, then the ones that were up the longest.
fn buff_rows(
    buffs: Option<&Vec<BuffSummary>>,
    actor_infos: &HashMap<u32, ActorInfo>,
    settings: &OverlaySettings,
) -> Vec<BuffRow> {
    let mut rows: Vec<BuffRow> = buffs
        .into_iter()
        .flatten()
        .map(|buff| {
            let caster = actor_infos.get(&buff.actor_id);
            BuffRow {
                code: buff.skill_code,
                name: game_data::skill_name(buff.skill_code),
                icon: game_data::skill_icon(buff.skill_code),
                caster: caster
                    .and_then(|caster| caster.actor_name.as_deref())
                    .filter(|name| !name.is_empty())
                    .map_or_else(|| format!("#{}", buff.actor_id), |name| settings.display_name(name)),
                caster_id: buff.actor_id,
                caster_class: caster.and_then(|caster| caster.actor_class.as_deref()).and_then(PlayerClass::parse),
                coverage: buff.coverage.clamp(0.0, 1.0),
                active: buff.active,
            }
        })
        .collect();
    rows.sort_by(|a, b| b.active.cmp(&a.active).then(b.coverage.total_cmp(&a.coverage)));
    rows
}

#[cfg(test)]
mod tests {
    use meter_core::models::combat::CombatInfos;
    use meter_core::models::session::{SessionPlayerStat, SessionSummary};

    use super::*;

    const ME: u32 = 7;
    const BOSS: u32 = 100;
    const ADD: u32 = 101;
    const TARGET_PLAYER: u32 = 900;

    fn stats(counts: u32, total: u64, special: &[(&str, u32)]) -> SkillStats {
        SkillStats {
            counts,
            total_damage: total,
            min_damage: 10,
            max_damage: total / u64::from(counts.max(1)) * 2,
            special_counts: special.iter().map(|(kind, count)| (kind.to_string(), *count)).collect(),
        }
    }

    fn actor(id: u32, name: &str, class: &str) -> ActorInfo {
        ActorInfo {
            id,
            actor_name: Some(name.to_string()),
            actor_server_id: Some("1001".to_string()),
            actor_class: Some(class.to_string()),
            combat_power: Some(41_000),
            actor_skill_spec: HashMap::from([(100_001, vec![1, 3, 9])]),
        }
    }

    fn target(id: u32, code: Option<u32>, start: f64, end: f64) -> TargetInfo {
        TargetInfo {
            id,
            target_mob_code: code,
            target_name: Some(format!("Mob {id}")),
            is_boss: id == BOSS,
            current_hp: Some(10),
            max_hp: Some(100),
            target_start_time: HashMap::from([(ME, start)]),
            target_last_time: HashMap::from([(ME, end)]),
        }
    }

    fn snapshot() -> CombatSnapshot {
        let skills = |target: u32, entries: &[(u32, SkillStats)]| {
            (target, HashMap::from([(ME, entries.iter().cloned().collect::<HashMap<_, _>>())]))
        };
        CombatSnapshot {
            total_damage: 0,
            by_target_player_skill_stats: HashMap::from([
                skills(
                    BOSS,
                    &[
                        (100_001, stats(10, 6_000, &[("CRITICAL", 4), ("BACK", 5), ("MULTIHIT", 3), ("MULTIHIT2", 2), ("MULTIHIT3", 1), ("MULTIHITDMG", 900)])),
                        (100_002, stats(5, 1_000, &[])),
                    ],
                ),
                skills(ADD, &[(100_001, stats(4, 400, &[("CRITICAL", 1)]))]),
                skills(TARGET_PLAYER, &[(100_001, stats(2, 9_000, &[]))]),
            ]),
            by_target_player_stats: HashMap::new(),
            use_buffs_by_target: HashMap::from([
                (
                    ME,
                    vec![
                        BuffSummary { target_id: ME, actor_id: 8, skill_code: 100_002, coverage: 0.4, active: false, last_start_ms: 0, last_end_ms: 0 },
                        BuffSummary { target_id: ME, actor_id: 8, skill_code: 100_001, coverage: 0.2, active: true, last_start_ms: 0, last_end_ms: 0 },
                    ],
                ),
                (
                    BOSS,
                    vec![BuffSummary { target_id: BOSS, actor_id: ME, skill_code: 100_001, coverage: 1.5, active: true, last_start_ms: 0, last_end_ms: 0 }],
                ),
            ]),
            combat_infos: CombatInfos {
                actor_infos: HashMap::from([(ME, actor(ME, "Mirelle", "Ranger")), (8, actor(8, "Ilyrae", "Chanter")), (TARGET_PLAYER, actor(TARGET_PLAYER, "Victim", "Cleric"))]),
                target_infos: HashMap::from([
                    (BOSS, target(BOSS, Some(5), 1_000.0, 1_050.0)),
                    (ADD, target(ADD, Some(6), 1_060.0, 1_070.0)),
                    (TARGET_PLAYER, target(TARGET_PLAYER, None, 1_080.0, 1_090.0)),
                ]),
                main_actor_id: Some(ME),
                main_actor_name: Some("Mirelle".to_string()),
                last_target_by_main_actor: Some(ADD),
                last_target: Some(ADD),
                time_now: 1_090.0,
            },
            last_target_info: None,
            last_target_all_players_overview_stats: Vec::new(),
            main_actor_received_player_overview_stats: Vec::new(),
            main_actor_dealt_player_overview_stats: Vec::new(),
            session: SessionSummary {
                total_damage: 7_400,
                target_count: 2,
                combat_time_secs: 70.0,
                players: vec![SessionPlayerStat {
                    actor_id: ME,
                    earlier_actor_ids: Vec::new(),
                    actor_name: "Mirelle".to_string(),
                    actor_server_id: "1001".to_string(),
                    actor_class: "Ranger".to_string(),
                    combat_power: Some(41_000),
                    total_damage: 7_400,
                    combat_time_secs: 60.0,
                    dps: 123.0,
                    damage_share: 1.0,
                }],
            },
        }
    }

    fn build(source: DetailSource, snapshot: &CombatSnapshot) -> DetailData {
        DetailData::build(&source, Some(snapshot), &OverlaySettings::default()).unwrap()
    }

    #[test]
    fn a_target_board_click_shows_that_targets_skills_best_first() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: Some(BOSS) }, &snapshot());

        assert_eq!(detail.mode, DetailMode::Live);
        assert_eq!(detail.skills.iter().map(|skill| skill.code).collect::<Vec<_>>(), vec![100_001, 100_002]);
        assert_eq!(detail.summary.damage, 7_000);
        assert_eq!(detail.summary.hits, 15);
        assert_eq!(detail.target_name.as_deref(), Some("Mob 100"));
        assert!((detail.skills[0].share - 6.0 / 7.0).abs() < 1e-9);
    }

    #[test]
    fn the_target_and_skill_names_are_built_in_the_language_in_force() {
        const PUNCHING_BAG: u32 = 2_000_000;
        let mut snapshot = snapshot();
        snapshot.combat_infos.target_infos.get_mut(&BOSS).unwrap().target_mob_code = Some(PUNCHING_BAG);
        let source = || DetailSource::Target { actor_id: ME, target_id: Some(BOSS) };

        crate::i18n::set_language(crate::i18n::Language::English);
        let english = build(source(), &snapshot);
        assert_eq!(english.target_name.as_deref(), Some("Mob 100"));
        assert_eq!(english.skills[0].name, "Basic Attack");

        crate::i18n::set_language(crate::i18n::Language::French);
        let french = build(source(), &snapshot);
        assert_eq!(french.target_name.as_deref(), Some("Mannequin"));
        assert_eq!(french.skills[0].name, "Attaque de base");
    }

    #[test]
    fn fight_time_and_dps_come_from_the_players_own_window_on_the_target() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: Some(BOSS) }, &snapshot());

        assert_eq!(detail.summary.fight_secs, 50.0);
        assert_eq!(detail.summary.dps, 140.0);
    }

    #[test]
    fn without_a_chosen_target_the_one_you_hit_last_is_shown() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: None }, &snapshot());

        assert_eq!(detail.target_name.as_deref(), Some("Mob 101"));
        assert_eq!(detail.summary.damage, 400);
    }

    #[test]
    fn hit_rates_and_skill_figures_follow_the_special_counts() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: Some(BOSS) }, &snapshot());
        let strike = &detail.skills[0];

        assert_eq!(strike.critical, Some(0.4));
        assert_eq!(strike.back, Some(0.5));
        assert_eq!(strike.front, Some(0.0));
        assert_eq!((strike.min, strike.max, strike.average), (10, 1_200, 600));
        assert_eq!(strike.multi_damage, 900);
        assert_eq!(
            strike.multi_breakdown,
            vec![MultiHit { hits: 2, count: 2, share: 0.2 }, MultiHit { hits: 3, count: 1, share: 0.1 }]
        );
        assert!((detail.summary.critical.unwrap() - 4.0 / 15.0).abs() < 1e-9);
    }

    #[test]
    fn spec_slots_outside_one_to_five_are_ignored() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: Some(BOSS) }, &snapshot());

        assert_eq!(detail.skills[0].spec, [true, false, true, false, false]);
        assert_eq!(detail.skills[1].spec, [false; 5]);
    }

    #[test]
    fn the_session_board_sums_skills_across_mobs_but_not_players() {
        let detail = build(DetailSource::Session { actor_id: ME }, &snapshot());

        assert_eq!(detail.mode, DetailMode::Session);
        let strike = detail.skills.iter().find(|skill| skill.code == 100_001).unwrap();
        assert_eq!((strike.hits, strike.total), (14, 6_400));
        assert_eq!(detail.summary.damage, 7_400);
        assert_eq!(detail.target_name, None);
        assert!(detail.target_buffs.is_empty());
    }

    #[test]
    fn the_session_board_takes_time_and_dps_from_its_own_row() {
        let detail = build(DetailSource::Session { actor_id: ME }, &snapshot());

        assert_eq!(detail.summary.fight_secs, 60.0);
        assert_eq!(detail.summary.dps, 123.0);
    }

    #[test]
    fn buffs_list_the_active_first_then_the_longest_up_with_coverage_capped() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: Some(BOSS) }, &snapshot());

        assert_eq!(detail.player_buffs.iter().map(|buff| buff.code).collect::<Vec<_>>(), vec![100_001, 100_002]);
        assert_eq!(detail.player_buffs[0].caster, "Ilyrae");
        assert_eq!(detail.player_buffs[0].caster_class, Some(PlayerClass::Chanter));
        assert_eq!(detail.target_buffs[0].coverage, 1.0);
    }

    #[test]
    fn identity_carries_class_server_and_power() {
        let detail = build(DetailSource::Target { actor_id: ME, target_id: Some(BOSS) }, &snapshot());

        assert_eq!(detail.identity.name, "Mirelle");
        assert_eq!(detail.identity.class, Some(PlayerClass::Ranger));
        assert_eq!(detail.identity.server, "SIE");
        assert_eq!(detail.identity.combat_power, Some(41_000));
    }

    #[test]
    fn masking_covers_the_player_and_the_casters_but_not_the_mob() {
        let settings = OverlaySettings { mask_nicknames: true, ..OverlaySettings::default() };
        let source = DetailSource::Target { actor_id: ME, target_id: Some(BOSS) };
        let snapshot = snapshot();

        let detail = DetailData::build(&source, Some(&snapshot), &settings).unwrap();

        assert_eq!(detail.identity.name, "M*****e");
        assert_eq!(detail.player_buffs[0].caster, "I****e");
        let boss = crate::meter::target_name(&snapshot.combat_infos.target_infos[&BOSS]);
        assert!(boss.starts_with("Mob "), "{boss}");
        assert_eq!(detail.target_name.as_deref(), Some(boss.as_str()));
    }

    #[test]
    fn an_unnamed_player_shows_their_id_and_you_when_it_is_you() {
        let mut anonymous = snapshot();
        anonymous.combat_infos.actor_infos.get_mut(&ME).unwrap().actor_name = None;
        let source = DetailSource::Target { actor_id: ME, target_id: Some(BOSS) };

        assert_eq!(build(source.clone(), &anonymous).identity.name, "You");

        anonymous.combat_infos.main_actor_id = None;
        assert_eq!(build(source, &anonymous).identity.name, "ID:7");
    }

    #[test]
    fn nothing_is_shown_before_a_snapshot_or_for_a_stranger() {
        let source = DetailSource::Target { actor_id: ME, target_id: Some(BOSS) };

        assert!(DetailData::build(&source, None, &OverlaySettings::default()).is_none());
        assert!(DetailData::build(&DetailSource::Target { actor_id: 12_345, target_id: Some(BOSS) }, Some(&snapshot()), &OverlaySettings::default()).is_none());
    }

    #[test]
    fn a_history_record_is_shown_from_the_record_alone() {
        let live = snapshot();
        let record = HistoryRecord {
            id: "100-1".to_string(),
            target_id: BOSS,
            total_damage: 7_000,
            target_info: live.combat_infos.target_infos.get(&BOSS).cloned(),
            combat_infos: live.combat_infos.clone(),
            player_skill_stats: HashMap::from([(ME, live.by_target_player_skill_stats[&BOSS][&ME].clone())]),
            player_stats: HashMap::new(),
            use_buffs_by_target: live.use_buffs_by_target.clone(),
            ledger: None,
            created_at: 0,
        };

        let detail = DetailData::build(
            &DetailSource::History { actor_id: ME, record: Arc::new(record) },
            None,
            &OverlaySettings::default(),
        )
        .unwrap();

        assert_eq!(detail.mode, DetailMode::History);
        assert_eq!(detail.summary.damage, 7_000);
        assert_eq!(detail.summary.fight_secs, 50.0);
        assert_eq!(detail.target_name.as_deref(), Some("Mob 100"));
    }
}
