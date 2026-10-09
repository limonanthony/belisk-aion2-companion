use crate::models::session::SessionSummary;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillStats {
    pub counts: u32,
    pub total_damage: u64,
    pub min_damage: u64,
    pub max_damage: u64,
    #[serde(default)]
    pub special_counts: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvpWatchInfo {
    pub query_name: String,
    pub actor_id: Option<u32>,
    pub actor_name: Option<String>,
    pub server_id: Option<String>,
    pub actor_class: Option<String>,
    pub current_hp: Option<u32>,
    pub max_hp: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvpKnownPlayer {
    pub actor_id: u32,
    pub actor_name: String,
    pub server_id: Option<String>,
    pub actor_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvpWatchInfoResponse {
    pub watch_info: Vec<PvpWatchInfo>,
    pub known_players: Vec<PvpKnownPlayer>,
    pub last_dealt_player: Option<PvpWatchInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvpCombatStats {
    pub damage: u64,
    pub kills: u32,
    pub assists: u32,
    pub deaths: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvpCombatStatsRow {
    pub actor_name: String,
    pub server_id: String,
    pub damage: u64,
    pub kills: u32,
    pub assists: u32,
    pub deaths: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerHpInfo {
    pub actor_id: u32,
    pub current_hp: u32,
    pub max_observed_hp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffInterval {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffSummary {
    pub target_id: u32,
    pub actor_id: u32,
    pub skill_code: u32,
    pub coverage: f64,
    pub active: bool,
    pub last_start_ms: u64,
    pub last_end_ms: u64,
}

impl SkillStats {
    pub fn new() -> Self {
        Self {
            counts: 0,
            total_damage: 0,
            min_damage: u64::MAX,
            max_damage: 0,
            special_counts: HashMap::new(),
        }
    }
}

impl Default for SkillStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorInfo {
    pub id: u32,
    pub actor_name: Option<String>,
    pub actor_server_id: Option<String>,
    pub actor_class: Option<String>,
    #[serde(default)]
    pub combat_power: Option<u64>,
    pub actor_skill_spec: HashMap<u32, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub id: u32,
    pub target_mob_code: Option<u32>,
    pub target_name: Option<String>,
    pub is_boss: bool,
    pub current_hp: Option<u32>,
    pub max_hp: Option<u32>,
    pub target_start_time: HashMap<u32, f64>,
    pub target_last_time: HashMap<u32, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatInfos {
    pub actor_infos: HashMap<u32, ActorInfo>,
    pub target_infos: HashMap<u32, TargetInfo>,
    pub main_actor_id: Option<u32>,
    pub main_actor_name: Option<String>,
    pub last_target_by_main_actor: Option<u32>,
    pub last_target: Option<u32>,
    pub time_now: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerOverviewStat {
    pub actor_id: u32,
    pub actor_name: String,
    pub actor_server_id: String,
    pub actor_class: String,
    #[serde(default)]
    pub combat_power: Option<u64>,
    pub counts: u32,
    pub total_damage: u64,
    pub min_damage: u64,
    pub max_damage: u64,
    #[serde(default)]
    pub special_counts: HashMap<String, u32>,
    pub dps: f64,
    pub damage_share: f64,
    pub damage_contribution: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvpDamageOverviewStat {
    pub player_id: u32,
    pub player_name: String,
    pub player_server_id: String,
    pub player_class: String,
    pub counts: u32,
    pub total_damage: u64,
    pub min_damage: u64,
    pub max_damage: u64,
    pub dps: f64,
    pub damage_contribution: f64,
    pub hp_damage_ratio: Option<f64>,
    pub battle_start_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatSnapshot {
    pub total_damage: u64,
    pub by_target_player_skill_stats: HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>>,
    pub by_target_player_stats: HashMap<u32, HashMap<u32, PlayerOverviewStat>>,
    #[serde(default)]
    pub use_buffs_by_target: HashMap<u32, Vec<BuffSummary>>,
    pub combat_infos: CombatInfos,
    pub last_target_info: Option<TargetInfo>,
    #[serde(default)]
    pub last_target_all_players_overview_stats: Vec<PlayerOverviewStat>,
    #[serde(default)]
    pub main_actor_received_player_overview_stats: Vec<PlayerOverviewStat>,
    #[serde(default)]
    pub main_actor_dealt_player_overview_stats: Vec<PvpDamageOverviewStat>,
    #[serde(default)]
    pub session: SessionSummary,
}

impl CombatSnapshot {
    /// Whether `other` shows the overlay the same picture: every field equal,
    /// except the two that follow the wall clock and change with no new
    /// event. `combat_infos.time_now` is the moment the frame was built, and
    /// a buff's `coverage` is measured up to that moment. A change in either
    /// alone is not worth a frame; the next frame that differs for any other
    /// reason carries the fresh values. A buff starting, ending or expiring
    /// still differs (`active`, `last_start_ms`, `last_end_ms`).
    ///
    /// Every field is named, so a field added to the snapshot cannot be left
    /// out of the comparison without the compiler saying so.
    pub fn shows_the_same_as(&self, other: &Self) -> bool {
        let Self {
            total_damage,
            by_target_player_skill_stats,
            by_target_player_stats,
            use_buffs_by_target,
            combat_infos,
            last_target_info,
            last_target_all_players_overview_stats,
            main_actor_received_player_overview_stats,
            main_actor_dealt_player_overview_stats,
            session,
        } = self;
        *total_damage == other.total_damage
            && *by_target_player_stats == other.by_target_player_stats
            && *by_target_player_skill_stats == other.by_target_player_skill_stats
            && same_buffs(use_buffs_by_target, &other.use_buffs_by_target)
            && combat_infos.shows_the_same_as(&other.combat_infos)
            && *last_target_info == other.last_target_info
            && *last_target_all_players_overview_stats == other.last_target_all_players_overview_stats
            && *main_actor_received_player_overview_stats == other.main_actor_received_player_overview_stats
            && *main_actor_dealt_player_overview_stats == other.main_actor_dealt_player_overview_stats
            && *session == other.session
    }
}

impl CombatInfos {
    fn shows_the_same_as(&self, other: &Self) -> bool {
        let Self {
            actor_infos,
            target_infos,
            main_actor_id,
            main_actor_name,
            last_target_by_main_actor,
            last_target,
            time_now: _,
        } = self;
        *actor_infos == other.actor_infos
            && *target_infos == other.target_infos
            && *main_actor_id == other.main_actor_id
            && *main_actor_name == other.main_actor_name
            && *last_target_by_main_actor == other.last_target_by_main_actor
            && *last_target == other.last_target
    }
}

fn same_buffs(a: &HashMap<u32, Vec<BuffSummary>>, b: &HashMap<u32, Vec<BuffSummary>>) -> bool {
    a.len() == b.len()
        && a.iter().all(|(target, buffs)| {
            b.get(target).is_some_and(|other| {
                buffs.len() == other.len()
                    && buffs.iter().zip(other).all(|(buff, other)| {
                        let BuffSummary { target_id, actor_id, skill_code, coverage: _, active, last_start_ms, last_end_ms } = buff;
                        *target_id == other.target_id
                            && *actor_id == other.actor_id
                            && *skill_code == other.skill_code
                            && *active == other.active
                            && *last_start_ms == other.last_start_ms
                            && *last_end_ms == other.last_end_ms
                    })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> CombatSnapshot {
        CombatSnapshot {
            total_damage: 10,
            by_target_player_skill_stats: HashMap::new(),
            by_target_player_stats: HashMap::new(),
            use_buffs_by_target: HashMap::from([(
                1,
                vec![BuffSummary { target_id: 1, actor_id: 2, skill_code: 3, coverage: 0.5, active: true, last_start_ms: 100, last_end_ms: 900 }],
            )]),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: Some(2),
                main_actor_name: None,
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 1_000.0,
            },
            last_target_info: None,
            last_target_all_players_overview_stats: Vec::new(),
            main_actor_received_player_overview_stats: Vec::new(),
            main_actor_dealt_player_overview_stats: Vec::new(),
            session: SessionSummary::default(),
        }
    }

    #[test]
    fn the_clock_and_buff_coverage_alone_do_not_make_a_frame_new() {
        let before = frame();
        let mut later = frame();
        later.combat_infos.time_now += 30.0;
        later.use_buffs_by_target.get_mut(&1).unwrap()[0].coverage = 0.4;

        assert!(before.shows_the_same_as(&later));
    }

    #[test]
    fn a_buff_that_expired_or_a_name_that_arrived_makes_a_frame_new() {
        let before = frame();

        let mut expired = frame();
        expired.use_buffs_by_target.get_mut(&1).unwrap()[0].active = false;
        assert!(!before.shows_the_same_as(&expired));

        let mut named = frame();
        named.combat_infos.main_actor_name = Some("Rin".to_string());
        assert!(!before.shows_the_same_as(&named));

        let mut gained_a_buff = frame();
        gained_a_buff.use_buffs_by_target.insert(9, Vec::new());
        assert!(!before.shows_the_same_as(&gained_a_buff));
    }
}
