//! The session board: every player's damage across all mobs since the last
//! reset, built from the per-target overview the snapshot has already computed.
//!
//! Combat time is the time a player was fighting, not the time since the
//! reset. On each target a player's fight window runs from their first hit to
//! the last hit anyone landed on that target, at least one second long: the
//! same span the per-target DPS divides by, so a session with a single target
//! reads the same as that target's board. A player's combat time is the length
//! of the union of their windows, so two mobs fought at once count once, and
//! the minutes spent walking between packs count not at all. The session clock
//! is the union of every target's window.

use std::collections::HashMap;

use crate::models::combat::{PlayerOverviewStat, SkillStats, TargetInfo};
use crate::models::session::{SessionPlayerStat, SessionSummary};

const MIN_WINDOW_SECS: f64 = 1.0;
const MIN_PLAYER_ROWS: usize = 5;
const MAX_PLAYER_ROWS: usize = 20;

pub struct SessionInputs<'a> {
    pub per_target_overview: &'a HashMap<u32, HashMap<u32, PlayerOverviewStat>>,
    pub target_infos: &'a HashMap<u32, TargetInfo>,
    pub main_actor_id: Option<u32>,
    pub main_actor_name: Option<&'a str>,
    pub hide_unknown_players: bool,
    pub max_player_count: usize,
}

#[derive(Clone, Copy)]
struct Window {
    start: f64,
    end: f64,
}

struct PlayerTotals<'a> {
    identity: &'a PlayerOverviewStat,
    damage: u64,
    windows: Vec<Window>,
}

/// `is_mob_target` keeps players out of the board: damage dealt to a player
/// (PvP, or a mob hitting you) is not part of the session.
pub fn build_session_summary(
    inputs: &SessionInputs,
    is_mob_target: impl Fn(u32) -> bool,
) -> SessionSummary {
    let mut players: HashMap<u32, PlayerTotals> = HashMap::new();
    let mut target_windows: Vec<Window> = Vec::new();
    let mut total_damage = 0u64;
    let mut target_count = 0u32;

    for (target_id, overview) in inputs.per_target_overview {
        if overview.is_empty() || !is_mob_target(*target_id) {
            continue;
        }
        let info = inputs.target_infos.get(target_id);
        let fight_end = info
            .map(|target| target.target_last_time.values().copied().fold(0.0, f64::max))
            .unwrap_or(0.0);

        let mut target_window: Option<Window> = None;
        for (player_id, stat) in overview {
            let start = info
                .and_then(|target| target.target_start_time.get(player_id).copied())
                .unwrap_or(fight_end);
            let window = Window {
                start,
                end: start + (fight_end - start).max(MIN_WINDOW_SECS),
            };
            target_window = Some(match target_window {
                Some(seen) => Window {
                    start: seen.start.min(window.start),
                    end: seen.end.max(window.end),
                },
                None => window,
            });

            total_damage += stat.total_damage;
            let totals = players.entry(*player_id).or_insert_with(|| PlayerTotals {
                identity: stat,
                damage: 0,
                windows: Vec::new(),
            });
            totals.damage += stat.total_damage;
            totals.windows.push(window);
        }
        target_windows.extend(target_window);
        target_count += 1;
    }

    let mut rows: Vec<PlayerTotals> = players.into_values().collect();
    rows.sort_by(|a, b| {
        b.damage
            .cmp(&a.damage)
            .then_with(|| a.identity.actor_id.cmp(&b.identity.actor_id))
    });
    if inputs.hide_unknown_players {
        rows.retain(|row| {
            !row.identity.actor_name.is_empty() || inputs.main_actor_id == Some(row.identity.actor_id)
        });
    }
    keep_within_row_limit(&mut rows, inputs.max_player_count, |player| {
        inputs.main_actor_id == Some(player.actor_id)
            || inputs
                .main_actor_name
                .is_some_and(|name| !name.is_empty() && name == player.actor_name)
    });

    SessionSummary {
        total_damage,
        target_count,
        combat_time_secs: union_length(&mut target_windows),
        players: rows
            .into_iter()
            .map(|mut row| {
                let combat_time_secs = union_length(&mut row.windows);
                SessionPlayerStat {
                    actor_id: row.identity.actor_id,
                    actor_name: row.identity.actor_name.clone(),
                    actor_server_id: row.identity.actor_server_id.clone(),
                    actor_class: row.identity.actor_class.clone(),
                    combat_power: row.identity.combat_power,
                    total_damage: row.damage,
                    combat_time_secs,
                    dps: if combat_time_secs > 0.0 {
                        row.damage as f64 / combat_time_secs
                    } else {
                        0.0
                    },
                    damage_share: if total_damage > 0 {
                        row.damage as f64 / total_damage as f64
                    } else {
                        0.0
                    },
                }
            })
            .collect(),
    }
}

/// The last row makes way for the local player when they fall below the cut,
/// so you always see yourself. Actor ids change between zones while the
/// character's name does not, so either one identifies you.
fn keep_within_row_limit(
    rows: &mut Vec<PlayerTotals>,
    max_player_count: usize,
    is_local_player: impl Fn(&PlayerOverviewStat) -> bool,
) {
    let limit = max_player_count.clamp(MIN_PLAYER_ROWS, MAX_PLAYER_ROWS);
    if rows.len() <= limit {
        return;
    }
    let own_row = rows
        .iter()
        .position(|row| is_local_player(row.identity))
        .filter(|position| *position >= limit);
    match own_row {
        Some(position) => {
            rows.swap(limit - 1, position);
            rows.truncate(limit);
        }
        None => rows.truncate(limit),
    }
}

fn union_length(windows: &mut [Window]) -> f64 {
    windows.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut length = 0.0;
    let mut open: Option<Window> = None;
    for window in windows.iter().copied() {
        open = match open {
            Some(current) if window.start <= current.end => Some(Window {
                start: current.start,
                end: current.end.max(window.end),
            }),
            Some(current) => {
                length += current.end - current.start;
                Some(window)
            }
            None => Some(window),
        };
    }
    length + open.map_or(0.0, |last| last.end - last.start)
}

/// One player's skills summed per skill code across every mob target, for the
/// player detail window on the Session board. The snapshot already holds every
/// target's skill table (the core keeps at most 512 targets), so this is one
/// pass over that player's own tables: no new bookkeeping in the hot path, and
/// the caller runs it only while that window is open.
pub fn session_skill_totals(
    skills_by_target: &HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>>,
    actor_id: u32,
    is_mob_target: impl Fn(u32) -> bool,
) -> HashMap<u32, SkillStats> {
    let mut totals: HashMap<u32, SkillStats> = HashMap::new();
    for (target_id, actors) in skills_by_target {
        let Some(skills) = actors.get(&actor_id).filter(|_| is_mob_target(*target_id)) else {
            continue;
        };
        for (code, stats) in skills {
            let total = totals.entry(*code).or_default();
            total.counts += stats.counts;
            total.total_damage += stats.total_damage;
            total.min_damage = total.min_damage.min(stats.min_damage);
            total.max_damage = total.max_damage.max(stats.max_damage);
            for (kind, count) in &stats.special_counts {
                *total.special_counts.entry(kind.clone()).or_insert(0) += count;
            }
        }
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: u32 = 1;
    const ALLY: u32 = 2;
    const STRANGER: u32 = 3;
    const PACK_A: u32 = 100;
    const PACK_B: u32 = 101;
    const PLAYER_TARGET: u32 = 900;

    fn stat(actor_id: u32, name: &str, damage: u64) -> PlayerOverviewStat {
        PlayerOverviewStat {
            actor_id,
            actor_name: name.to_string(),
            actor_server_id: String::new(),
            actor_class: String::new(),
            combat_power: None,
            counts: 1,
            total_damage: damage,
            min_damage: 0,
            max_damage: damage,
            special_counts: HashMap::new(),
            dps: 0.0,
            damage_share: 0.0,
            damage_contribution: 0.0,
        }
    }

    fn target(id: u32, starts: &[(u32, f64)], ends: &[(u32, f64)]) -> TargetInfo {
        TargetInfo {
            id,
            target_mob_code: Some(1),
            target_name: None,
            is_boss: false,
            current_hp: None,
            max_hp: None,
            target_start_time: starts.iter().copied().collect(),
            target_last_time: ends.iter().copied().collect(),
        }
    }

    struct Fight {
        overview: HashMap<u32, HashMap<u32, PlayerOverviewStat>>,
        infos: HashMap<u32, TargetInfo>,
    }

    impl Fight {
        fn new() -> Self {
            Self {
                overview: HashMap::new(),
                infos: HashMap::new(),
            }
        }

        /// `hits`: (player, name, damage, first hit, last hit) on this target.
        fn target(mut self, id: u32, hits: &[(u32, &str, u64, f64, f64)]) -> Self {
            let starts: Vec<_> = hits.iter().map(|hit| (hit.0, hit.3)).collect();
            let ends: Vec<_> = hits.iter().map(|hit| (hit.0, hit.4)).collect();
            self.infos.insert(id, target(id, &starts, &ends));
            self.overview.insert(
                id,
                hits.iter()
                    .map(|hit| (hit.0, stat(hit.0, hit.1, hit.2)))
                    .collect(),
            );
            self
        }

        fn summary(&self, hide_unknown_players: bool, max_player_count: usize) -> SessionSummary {
            build_session_summary(
                &SessionInputs {
                    per_target_overview: &self.overview,
                    target_infos: &self.infos,
                    main_actor_id: Some(ME),
                    main_actor_name: None,
                    hide_unknown_players,
                    max_player_count,
                },
                |id| id != PLAYER_TARGET,
            )
        }
    }

    fn row(summary: &SessionSummary, actor_id: u32) -> &SessionPlayerStat {
        summary
            .players
            .iter()
            .find(|player| player.actor_id == actor_id)
            .expect("player row")
    }

    #[test]
    fn damage_adds_up_across_targets_and_sorts_descending() {
        let fight = Fight::new()
            .target(PACK_A, &[(ME, "Me", 600, 0.0, 10.0), (ALLY, "Ally", 900, 0.0, 10.0)])
            .target(PACK_B, &[(ME, "Me", 700, 20.0, 30.0)]);

        let summary = fight.summary(false, 10);

        assert_eq!(summary.total_damage, 2_200);
        assert_eq!(summary.target_count, 2);
        let order: Vec<_> = summary.players.iter().map(|p| p.actor_id).collect();
        assert_eq!(order, vec![ME, ALLY]);
        assert_eq!(row(&summary, ME).total_damage, 1_300);
        assert!((row(&summary, ME).damage_share - 1_300.0 / 2_200.0).abs() < 1e-9);
    }

    #[test]
    fn combat_time_skips_the_gap_between_targets() {
        let fight = Fight::new()
            .target(PACK_A, &[(ME, "Me", 500, 0.0, 10.0)])
            .target(PACK_B, &[(ME, "Me", 500, 110.0, 120.0)]);

        let summary = fight.summary(false, 10);

        assert_eq!(row(&summary, ME).combat_time_secs, 20.0);
        assert_eq!(row(&summary, ME).dps, 50.0);
        assert_eq!(summary.combat_time_secs, 20.0);
    }

    #[test]
    fn hiding_unknown_players_keeps_your_own_row_when_the_game_has_not_named_you() {
        let fight = Fight::new().target(PACK_A, &[(ME, "", 600, 0.0, 10.0), (ALLY, "", 900, 0.0, 10.0)]);

        let summary = fight.summary(true, 10);

        let order: Vec<_> = summary.players.iter().map(|p| p.actor_id).collect();
        assert_eq!(order, vec![ME]);
    }

    #[test]
    fn fighting_two_targets_at_once_counts_the_overlap_once() {
        let fight = Fight::new()
            .target(PACK_A, &[(ME, "Me", 500, 0.0, 10.0)])
            .target(PACK_B, &[(ME, "Me", 500, 5.0, 15.0)]);

        let summary = fight.summary(false, 10);

        assert_eq!(row(&summary, ME).combat_time_secs, 15.0);
        assert_eq!(summary.combat_time_secs, 15.0);
    }

    #[test]
    fn a_player_joining_late_has_a_shorter_window_than_the_target() {
        let fight = Fight::new().target(
            PACK_A,
            &[(ME, "Me", 1_000, 0.0, 10.0), (ALLY, "Ally", 300, 6.0, 10.0)],
        );

        let summary = fight.summary(false, 10);

        assert_eq!(row(&summary, ME).combat_time_secs, 10.0);
        assert_eq!(row(&summary, ALLY).combat_time_secs, 4.0);
        assert_eq!(row(&summary, ALLY).dps, 75.0);
        assert_eq!(summary.combat_time_secs, 10.0);
    }

    #[test]
    fn a_one_hit_target_counts_for_at_least_a_second() {
        let fight = Fight::new().target(PACK_A, &[(ME, "Me", 80, 4.0, 4.0)]);

        assert_eq!(row(&fight.summary(false, 10), ME).combat_time_secs, 1.0);
    }

    #[test]
    fn damage_dealt_to_players_is_not_part_of_the_session() {
        let fight = Fight::new()
            .target(PACK_A, &[(ME, "Me", 500, 0.0, 10.0)])
            .target(PLAYER_TARGET, &[(ALLY, "Ally", 9_000, 0.0, 10.0)]);

        let summary = fight.summary(false, 10);

        assert_eq!(summary.total_damage, 500);
        assert_eq!(summary.target_count, 1);
        assert_eq!(summary.players.len(), 1);
    }

    #[test]
    fn hiding_unknown_players_keeps_their_damage_in_the_shares() {
        let fight = Fight::new().target(
            PACK_A,
            &[(ME, "Me", 600, 0.0, 10.0), (STRANGER, "", 400, 0.0, 10.0)],
        );

        let summary = fight.summary(true, 10);

        assert_eq!(summary.players.len(), 1);
        assert_eq!(summary.total_damage, 1_000);
        assert!((summary.players[0].damage_share - 0.6).abs() < 1e-9);
    }

    #[test]
    fn the_row_limit_keeps_the_local_player() {
        let names = ["", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8"];
        let hits: Vec<_> = (1..=8u32)
            .map(|id| {
                let damage = if id == ME { 1 } else { 1_000 - u64::from(id) * 10 };
                (id, names[id as usize], damage, 0.0, 10.0)
            })
            .collect();
        let fight = Fight::new().target(PACK_A, &hits);

        let summary = fight.summary(false, 5);

        assert_eq!(summary.players.len(), 5);
        assert_eq!(summary.players.last().unwrap().actor_id, ME);
        assert!(summary
            .players
            .windows(2)
            .all(|pair| pair[0].total_damage >= pair[1].total_damage));
    }

    #[test]
    fn the_local_player_is_also_found_by_name_when_their_id_changed() {
        let names = ["", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8"];
        let hits: Vec<_> = (1..=8u32)
            .map(|id| (id, names[id as usize], 1_000 - u64::from(id) * 10, 0.0, 10.0))
            .collect();
        let fight = Fight::new().target(PACK_A, &hits);

        let summary = build_session_summary(
            &SessionInputs {
                per_target_overview: &fight.overview,
                target_infos: &fight.infos,
                main_actor_id: Some(999),
                main_actor_name: Some("P8"),
                hide_unknown_players: false,
                max_player_count: 5,
            },
            |_| true,
        );

        assert_eq!(summary.players.last().unwrap().actor_name, "P8");
    }

    #[test]
    fn an_empty_meter_has_an_empty_session() {
        let summary = Fight::new().summary(false, 10);

        assert_eq!(summary, SessionSummary::default());
    }

    fn skill(counts: u32, total: u64, min: u64, max: u64, kinds: &[(&str, u32)]) -> SkillStats {
        SkillStats {
            counts,
            total_damage: total,
            min_damage: min,
            max_damage: max,
            special_counts: kinds.iter().map(|(kind, count)| (kind.to_string(), *count)).collect(),
        }
    }

    fn tables(
        entries: &[(u32, u32, u32, SkillStats)],
    ) -> HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>> {
        let mut tables: HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>> = HashMap::new();
        for (target, actor, code, stats) in entries {
            tables.entry(*target).or_default().entry(*actor).or_default().insert(*code, stats.clone());
        }
        tables
    }

    #[test]
    fn session_skills_are_summed_per_skill_code_across_targets() {
        let tables = tables(&[
            (PACK_A, ME, 7, skill(10, 1_000, 50, 200, &[("CRITICAL", 4)])),
            (PACK_B, ME, 7, skill(5, 800, 30, 300, &[("CRITICAL", 1), ("BACK", 2)])),
            (PACK_B, ME, 8, skill(2, 90, 40, 50, &[])),
            (PACK_A, ALLY, 7, skill(99, 9_999, 1, 999, &[])),
        ]);

        let totals = session_skill_totals(&tables, ME, |_| true);

        assert_eq!(totals.len(), 2);
        let summed = &totals[&7];
        assert_eq!((summed.counts, summed.total_damage), (15, 1_800));
        assert_eq!((summed.min_damage, summed.max_damage), (30, 300));
        assert_eq!(summed.special_counts["CRITICAL"], 5);
        assert_eq!(summed.special_counts["BACK"], 2);
        assert_eq!(totals[&8].total_damage, 90);
    }

    #[test]
    fn session_skills_leave_out_damage_dealt_to_players() {
        let tables = tables(&[
            (PACK_A, ME, 7, skill(1, 100, 100, 100, &[])),
            (PLAYER_TARGET, ME, 7, skill(1, 5_000, 5_000, 5_000, &[])),
        ]);

        let totals = session_skill_totals(&tables, ME, |target| target != PLAYER_TARGET);

        assert_eq!(totals[&7].total_damage, 100);
    }

    #[test]
    fn a_player_who_hit_nothing_has_no_session_skills() {
        let tables = tables(&[(PACK_A, ALLY, 7, skill(1, 100, 100, 100, &[]))]);

        assert!(session_skill_totals(&tables, ME, |_| true).is_empty());
    }
}
