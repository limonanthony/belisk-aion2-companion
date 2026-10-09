use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::engine::buff_groups::group_by_name;
use crate::models::combat::{
    ActorInfo, BuffInterval, BuffSummary, CombatInfos, CombatSnapshot, PlayerHpInfo,
    PlayerOverviewStat, PvpDamageOverviewStat, SkillStats, TargetInfo,
};
use crate::engine::session::{build_session_summary, SessionInputs};
use crate::report::ledger::{build_ledger, ActorDirectory, FightLedger, FightWindow, LedgerSources};
use crate::storage::data_storage::{infer_actor_class, DataStorage, FightLedgerInputs};

pub struct DpsCalculator {
    data_storage: Arc<DataStorage>,
}

impl DpsCalculator {
    pub fn new(data_storage: Arc<DataStorage>) -> Self {
        Self { data_storage }
    }

    /// What the party took, healed and lost in the fight against `target`, from
    /// the players in `damage_dealers`. `None` until somebody has hit it.
    pub fn fight_ledger(&self, target: &TargetInfo, damage_dealers: &[u32]) -> Option<FightLedger> {
        let window = FightWindow::of(target)?;
        let (from, to) = window.event_range();
        let FightLedgerInputs { events, names, servers, mut classes, buff_intervals } =
            self.data_storage.fight_ledger_inputs(target.id, damage_dealers, (from, to));
        for heal in &events.heals {
            if !classes.contains_key(&heal.healer_id) {
                if let Some(class) = infer_actor_class(heal.skill_code) {
                    classes.insert(heal.healer_id, class);
                }
            }
        }
        Some(build_ledger(&LedgerSources {
            target_id: target.id,
            window,
            damage_dealers,
            events: &events,
            actors: ActorDirectory { names: &names, servers: &servers, classes: &classes },
            mob_name: &|mob_code| self.data_storage.mob_name(mob_code),
            buff_intervals: &buff_intervals,
        }))
    }

    pub fn get_dps_snapshot(
        &self,
        target_damage_threshold: u64,
        hide_unknown_players: bool,
        max_player_count: usize,
    ) -> Option<CombatSnapshot> {
        build_combat_snapshot(
            &self.data_storage,
            target_damage_threshold,
            hide_unknown_players,
            max_player_count,
        )
    }
}

// =============================================================================
// Main snapshot builder
// =============================================================================

fn build_combat_snapshot(
    data_storage: &DataStorage,
    target_damage_threshold: u64,
    hide_unknown_players: bool,
    max_player_count: usize,
) -> Option<CombatSnapshot> {
    if data_storage.start_time().is_none() {
        return None;
    }

    // 1. Collect raw data
    let raw_stats = data_storage.get_dps_stats_snapshot();
    let summon_owner_map = data_storage.summon_owner_snapshot();

    // 2. Merge summon damage into their owners
    let merged = merge_summon_stats(&raw_stats, &summon_owner_map);

    // 3. Calculate total damage per target, filter by threshold
    let target_damage: HashMap<u32, u64> = merged
        .iter()
        .map(|(tid, actors)| {
            let dmg: u64 = actors
                .values()
                .flat_map(|skills| skills.values())
                .map(|s| s.total_damage)
                .sum();
            (*tid, dmg)
        })
        .collect();

    let kept_targets: Vec<u32> = target_damage
        .iter()
        .filter(|(_, dmg)| **dmg > target_damage_threshold)
        .map(|(tid, _)| *tid)
        .collect();

    let total_damage: u64 = kept_targets
        .iter()
        .filter_map(|tid| target_damage.get(tid))
        .sum();

    // 5. Filter stats to kept targets only
    let filtered_skill_stats = filter_nested_map(&merged, &kept_targets);

    // 6. Aggregate: per-target per-actor SkillStats (sum across skills)
    let aggregated = aggregate_target_actor_stats(&filtered_skill_stats);

    // 7. Build info maps
    let target_infos = build_target_infos(data_storage, &kept_targets, &summon_owner_map);
    let actor_infos = build_actor_infos(data_storage, &filtered_skill_stats, &summon_owner_map);

    // 8. Build per-target per-actor overview stats (with DPS, share, name, etc.)
    let per_target_overview =
        build_per_target_overview_stats(&aggregated, &target_infos, &actor_infos);

    // 9. Extract last-target overview for frontend convenience (already sorted by damage desc)
    let last_target_id = data_storage
        .last_target_by_main_actor()
        .or_else(|| data_storage.last_target());
    let own_actor_id = data_storage.main_actor_id();
    let (last_target_info, last_target_overview) = last_target_id
        .and_then(|tid| per_target_overview.get(&tid))
        .map(|stats| {
            let mut sorted: Vec<_> = stats.values().cloned().collect();
            sorted.sort_by(|a, b| b.total_damage.cmp(&a.total_damage).then_with(|| a.actor_id.cmp(&b.actor_id)));
            // Filter unknown players if configured
            let filtered: Vec<_> = if hide_unknown_players {
                sorted
                    .into_iter()
                    .filter(|p| !p.actor_name.is_empty() || Some(p.actor_id) == own_actor_id)
                    .collect()
            } else {
                sorted
            };
            // Truncate to max player count
            let truncated: Vec<_> = filtered
                .into_iter()
                .take(max_player_count.max(5).min(20))
                .collect();
            (
                target_infos.get(&last_target_id.unwrap()).cloned(),
                truncated,
            )
        })
        .unwrap_or((None, Vec::new()));

    let main_actor_received_overview = data_storage
        .main_actor_id()
        .and_then(|main_actor_id| {
            per_target_overview
                .get(&main_actor_id)
                .map(|stats| (main_actor_id, stats))
        })
        .map(|(main_actor_id, stats)| {
            let mut sorted: Vec<_> = stats.values().cloned().collect();
            sorted.retain(|p| p.actor_id != main_actor_id);
            let start_times = target_infos
                .get(&main_actor_id)
                .map(|target| &target.target_start_time);
            sorted.sort_by(|a, b| {
                let a_start = start_times
                    .and_then(|times| times.get(&a.actor_id))
                    .copied()
                    .unwrap_or(f64::MAX);
                let b_start = start_times
                    .and_then(|times| times.get(&b.actor_id))
                    .copied()
                    .unwrap_or(f64::MAX);
                a_start
                    .total_cmp(&b_start)
                    .then_with(|| b.total_damage.cmp(&a.total_damage))
                    .then_with(|| a.actor_id.cmp(&b.actor_id))
            });
            if hide_unknown_players {
                sorted
                    .into_iter()
                    .filter(|p| !p.actor_name.is_empty())
                    .collect()
            } else {
                sorted
            }
        })
        .unwrap_or_default();

    let main_actor_dealt_overview = build_main_actor_dealt_player_overview(
        data_storage,
        &per_target_overview,
        &target_infos,
        hide_unknown_players,
        max_player_count,
    );
    let own_actor_ids = data_storage.own_actor_ids();
    let session = build_session_summary(
        &SessionInputs {
            per_target_overview: &per_target_overview,
            target_infos: &target_infos,
            main_actor_id: data_storage.main_actor_id(),
            local_actor_ids: &own_actor_ids,
            hide_unknown_players,
            max_player_count,
        },
        |target_id| !data_storage.is_player(target_id),
    );

    Some(CombatSnapshot {
        total_damage,
        by_target_player_skill_stats: filtered_skill_stats,
        by_target_player_stats: per_target_overview,
        use_buffs_by_target: build_buff_summaries(data_storage, &kept_targets, &actor_infos),
        combat_infos: CombatInfos {
            actor_infos,
            target_infos,
            main_actor_id: data_storage.main_actor_id(),
            main_actor_name: data_storage.main_actor_name(),
            last_target_by_main_actor: data_storage.last_target_by_main_actor(),
            last_target: data_storage.last_target(),
            time_now: current_timestamp_seconds(),
        },
        last_target_info,
        last_target_all_players_overview_stats: last_target_overview,
        main_actor_received_player_overview_stats: main_actor_received_overview,
        main_actor_dealt_player_overview_stats: main_actor_dealt_overview,
        session,
    })
}

fn build_main_actor_dealt_player_overview(
    data_storage: &DataStorage,
    per_target_overview: &HashMap<u32, HashMap<u32, PlayerOverviewStat>>,
    target_infos: &HashMap<u32, TargetInfo>,
    hide_unknown_players: bool,
    max_player_count: usize,
) -> Vec<PvpDamageOverviewStat> {
    let actor_names = data_storage.actor_id_name_snapshot();
    let actor_servers = data_storage.actor_id_server_snapshot();
    let actor_classes = data_storage.actor_id_class_snapshot();
    let player_hp = data_storage.player_hp_snapshot();
    let mob_codes = data_storage.mob_id_code_snapshot();

    aggregate_main_actor_dealt_player_overview(
        data_storage.main_actor_id(),
        per_target_overview,
        target_infos,
        &actor_names,
        &actor_servers,
        &actor_classes,
        &player_hp,
        &mob_codes,
        hide_unknown_players,
        max_player_count,
    )
}

#[allow(clippy::too_many_arguments)]
fn aggregate_main_actor_dealt_player_overview(
    main_actor_id: Option<u32>,
    per_target_overview: &HashMap<u32, HashMap<u32, PlayerOverviewStat>>,
    target_infos: &HashMap<u32, TargetInfo>,
    actor_names: &HashMap<u32, String>,
    actor_servers: &HashMap<u32, String>,
    actor_classes: &HashMap<u32, String>,
    player_hp: &HashMap<u32, PlayerHpInfo>,
    mob_codes: &HashMap<u32, u32>,
    hide_unknown_players: bool,
    max_player_count: usize,
) -> Vec<PvpDamageOverviewStat> {
    let Some(main_actor_id) = main_actor_id else {
        return Vec::new();
    };

    let mut rows: Vec<_> = per_target_overview
        .iter()
        .filter_map(|(target_id, actors)| {
            if *target_id == main_actor_id
                || mob_codes.contains_key(target_id)
                || !actor_names.contains_key(target_id)
            {
                return None;
            }

            let player_name = actor_names.get(target_id).cloned().unwrap_or_default();
            if hide_unknown_players && player_name.is_empty() {
                return None;
            }

            let stats = actors.get(&main_actor_id)?;
            let max_hp = player_hp.get(target_id).map(|hp| hp.max_observed_hp);
            let hp_damage_ratio = max_hp
                .filter(|value| *value > 0)
                .map(|value| stats.total_damage as f64 / value as f64);
            let battle_start_time = target_infos
                .get(target_id)
                .and_then(|target| target.target_start_time.get(&main_actor_id))
                .copied()
                .unwrap_or(f64::MAX);

            Some(PvpDamageOverviewStat {
                player_id: *target_id,
                player_name,
                player_server_id: actor_servers.get(target_id).cloned().unwrap_or_default(),
                player_class: actor_classes.get(target_id).cloned().unwrap_or_default(),
                counts: stats.counts,
                total_damage: stats.total_damage,
                min_damage: stats.min_damage,
                max_damage: stats.max_damage,
                dps: stats.dps,
                damage_contribution: stats.damage_share,
                hp_damage_ratio,
                battle_start_time,
            })
        })
        .collect();

    rows.sort_by(|a, b| {
        a.battle_start_time
            .total_cmp(&b.battle_start_time)
            .then_with(|| b.total_damage.cmp(&a.total_damage))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    rows.truncate(max_player_count.max(5).min(20));
    rows
}

// =============================================================================
// Target infos
// =============================================================================

fn build_target_infos(
    data_storage: &DataStorage,
    kept_targets: &[u32],
    summon_owner_map: &HashMap<u32, u32>,
) -> HashMap<u32, TargetInfo> {
    let mob_id_code = data_storage.mob_id_code_snapshot();
    let mob_id_hp = data_storage.mob_id_hp_snapshot();
    let start_times = merge_time_map_min(
        &data_storage.start_time_by_target_snapshot(),
        summon_owner_map,
    );
    let last_times = merge_time_map_max(
        &data_storage.last_time_by_target_snapshot(),
        summon_owner_map,
    );

    kept_targets
        .iter()
        .map(|tid| {
            let mob_code = mob_id_code.get(tid).copied();
            let is_boss = mob_code.map(|c| data_storage.is_boss(c)).unwrap_or(false);
            // A mob the catalogue does not name is labelled by its code rather
            // than left blank, so two unnamed targets stay distinguishable.
            let name = mob_code.map(|c| {
                data_storage
                    .mob_name(c)
                    .unwrap_or_else(|| format!("{} {c}", if is_boss { "Boss" } else { "Mob" }))
            });
            let (cur_hp, max_hp) = mob_id_hp
                .get(tid)
                .map(|(c, m)| (Some(display_hp(*c)), Some(display_hp(*m))))
                .unwrap_or((None, None));

            (
                *tid,
                TargetInfo {
                    id: *tid,
                    target_mob_code: mob_code,
                    target_name: name,
                    is_boss,
                    current_hp: cur_hp,
                    max_hp,
                    target_start_time: start_times.get(tid).cloned().unwrap_or_default(),
                    target_last_time: last_times.get(tid).cloned().unwrap_or_default(),
                },
            )
        })
        .collect()
}

// =============================================================================
// Actor infos
// =============================================================================

fn build_actor_infos(
    data_storage: &DataStorage,
    stats: &HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>>,
    summon_owner_map: &HashMap<u32, u32>,
) -> HashMap<u32, ActorInfo> {
    let id_name = data_storage.actor_id_name_snapshot();
    let id_server = data_storage.actor_id_server_snapshot();
    let id_class = data_storage.actor_id_class_snapshot();
    let id_combat_power = data_storage.actor_id_combat_power_snapshot();
    let main_actor_id = data_storage.main_actor_id();
    let main_actor_combat_power = data_storage.main_actor_combat_power();
    let id_skill_spec =
        merge_actor_skill_specs(&data_storage.actor_skill_spec_snapshot(), summon_owner_map);

    let mut actor_ids: Vec<u32> = stats
        .values()
        .flat_map(|actors| actors.keys())
        .copied()
        .collect();
    actor_ids.sort_unstable();
    actor_ids.dedup();

    actor_ids
        .into_iter()
        .map(|aid| {
            (
                aid,
                ActorInfo {
                    id: aid,
                    actor_name: id_name.get(&aid).cloned(),
                    actor_server_id: id_server.get(&aid).cloned(),
                    actor_class: id_class.get(&aid).cloned(),
                    combat_power: if Some(aid) == main_actor_id {
                        main_actor_combat_power.or_else(|| id_combat_power.get(&aid).copied())
                    } else {
                        id_combat_power.get(&aid).copied()
                    },
                    actor_skill_spec: id_skill_spec.get(&aid).cloned().unwrap_or_default(),
                },
            )
        })
        .collect()
}

// =============================================================================
// Aggregate: per-target per-actor SkillStats (collapse skill dimension)
// =============================================================================

fn aggregate_target_actor_stats(
    stats: &HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>>,
) -> HashMap<u32, HashMap<u32, SkillStats>> {
    stats
        .iter()
        .map(|(tid, actors)| {
            let aggregated = actors
                .iter()
                .map(|(aid, skills)| (*aid, aggregate_skills(skills)))
                .collect();
            (*tid, aggregated)
        })
        .collect()
}

fn aggregate_skills(skills: &HashMap<u32, SkillStats>) -> SkillStats {
    let mut result = SkillStats::new();
    for s in skills.values() {
        result.counts += s.counts;
        result.total_damage += s.total_damage;
        result.max_damage = result.max_damage.max(s.max_damage);
        result.min_damage = result.min_damage.min(s.min_damage);
        for (k, v) in &s.special_counts {
            *result.special_counts.entry(k.clone()).or_insert(0) += v;
        }
    }
    if result.min_damage == u64::MAX {
        result.min_damage = 0;
    }
    result
}

// =============================================================================
// Per-target per-actor overview stats (with DPS, share, name, class)
// =============================================================================

fn build_per_target_overview_stats(
    aggregated: &HashMap<u32, HashMap<u32, SkillStats>>,
    target_infos: &HashMap<u32, TargetInfo>,
    actor_infos: &HashMap<u32, ActorInfo>,
) -> HashMap<u32, HashMap<u32, PlayerOverviewStat>> {
    aggregated
        .iter()
        .map(|(tid, player_stats)| {
            let target_info = target_infos.get(tid);
            let max_hp = target_info.and_then(|t| t.max_hp).unwrap_or(0) as f64;
            let max_last_time = target_info
                .map(|t| t.target_last_time.values().copied().fold(0.0, f64::max))
                .unwrap_or(0.0);
            let target_total: u64 = player_stats.values().map(|s| s.total_damage).sum();

            let overview: HashMap<u32, PlayerOverviewStat> = player_stats
                .iter()
                .map(|(pid, stats)| {
                    let start = target_info
                        .and_then(|t| t.target_start_time.get(pid).copied())
                        .unwrap_or(max_last_time);
                    let duration = (max_last_time - start).max(1.0);
                    let dps = stats.total_damage as f64 / duration;
                    let damage_share = if target_total > 0 {
                        stats.total_damage as f64 / target_total as f64
                    } else {
                        0.0
                    };
                    let damage_contribution = if max_hp > 0.0 {
                        stats.total_damage as f64 / max_hp
                    } else {
                        0.0
                    };

                    let info = actor_infos.get(pid);
                    (
                        *pid,
                        PlayerOverviewStat {
                            actor_id: *pid,
                            actor_name: info.and_then(|a| a.actor_name.clone()).unwrap_or_default(),
                            actor_server_id: info
                                .and_then(|a| a.actor_server_id.clone())
                                .unwrap_or_default(),
                            actor_class: info
                                .and_then(|a| a.actor_class.clone())
                                .unwrap_or_default(),
                            combat_power: info.and_then(|a| a.combat_power),
                            counts: stats.counts,
                            total_damage: stats.total_damage,
                            min_damage: stats.min_damage,
                            max_damage: stats.max_damage,
                            special_counts: stats.special_counts.clone(),
                            dps,
                            damage_share,
                            damage_contribution,
                        },
                    )
                })
                .collect();

            (*tid, overview)
        })
        .collect()
}

// =============================================================================
// Summon merging
// =============================================================================

/// `TargetInfo` carries 32-bit HP; larger values show as the largest it holds.
fn display_hp(hp: u64) -> u32 {
    u32::try_from(hp).unwrap_or(u32::MAX)
}

fn resolve_owner(actor_id: u32, summon_owner_map: &HashMap<u32, u32>) -> u32 {
    let mut current = actor_id;
    for _ in 0..8 {
        match summon_owner_map.get(&current) {
            Some(owner) if *owner != current => current = *owner,
            _ => break,
        }
    }
    current
}

fn merge_summon_stats(
    raw: &HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>>,
    summon_owner_map: &HashMap<u32, u32>,
) -> HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>> {
    let mut merged: HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>> = HashMap::new();
    for (tid, actors) in raw {
        let target = merged.entry(*tid).or_default();
        for (aid, skills) in actors {
            let owner = resolve_owner(*aid, summon_owner_map);
            let actor = target.entry(owner).or_default();
            for (code, stats) in skills {
                let entry = actor.entry(*code).or_insert_with(SkillStats::new);
                entry.counts += stats.counts;
                entry.total_damage += stats.total_damage;
                entry.max_damage = entry.max_damage.max(stats.max_damage);
                entry.min_damage = entry.min_damage.min(stats.min_damage);
                for (k, v) in &stats.special_counts {
                    *entry.special_counts.entry(k.clone()).or_insert(0) += v;
                }
            }
        }
    }
    merged
}

// =============================================================================
// Time map merging (min / max)
// =============================================================================

fn merge_time_map_min(
    raw: &HashMap<u32, HashMap<u32, f64>>,
    summon_owner_map: &HashMap<u32, u32>,
) -> HashMap<u32, HashMap<u32, f64>> {
    merge_time_map(raw, summon_owner_map, |a, b| if a < b { a } else { b })
}

fn merge_time_map_max(
    raw: &HashMap<u32, HashMap<u32, f64>>,
    summon_owner_map: &HashMap<u32, u32>,
) -> HashMap<u32, HashMap<u32, f64>> {
    merge_time_map(raw, summon_owner_map, |a, b| if a > b { a } else { b })
}

fn merge_time_map(
    raw: &HashMap<u32, HashMap<u32, f64>>,
    summon_owner_map: &HashMap<u32, u32>,
    pick: fn(f64, f64) -> f64,
) -> HashMap<u32, HashMap<u32, f64>> {
    let mut merged: HashMap<u32, HashMap<u32, f64>> = HashMap::new();
    for (tid, actors) in raw {
        let target: &mut HashMap<u32, f64> = merged.entry(*tid).or_default();
        for (aid, &time) in actors {
            let owner = resolve_owner(*aid, summon_owner_map);
            target
                .entry(owner)
                .and_modify(|t| *t = pick(*t, time))
                .or_insert(time);
        }
    }
    merged
}

// =============================================================================
// Skill spec merging
// =============================================================================

fn merge_actor_skill_specs(
    raw: &HashMap<u32, HashMap<u32, Vec<u32>>>,
    summon_owner_map: &HashMap<u32, u32>,
) -> HashMap<u32, HashMap<u32, Vec<u32>>> {
    let mut merged = HashMap::new();
    for (aid, skills) in raw {
        let owner = resolve_owner(*aid, summon_owner_map);
        let entry = merged.entry(owner).or_insert_with(HashMap::new);
        for (code, slots) in skills {
            entry.entry(*code).or_insert_with(|| slots.clone());
        }
    }
    merged
}

// =============================================================================
// Buff summaries
// =============================================================================

fn build_buff_summaries(
    data_storage: &DataStorage,
    kept_targets: &[u32],
    actor_infos: &HashMap<u32, ActorInfo>,
) -> HashMap<u32, Vec<BuffSummary>> {
    let intervals_by_target = data_storage.buff_intervals_snapshot();
    let target_start_times = data_storage.start_time_by_target_snapshot();
    let global_start_time = data_storage.start_time();
    let now_ms = current_timestamp_millis();
    let allowed_targets: HashSet<u32> = kept_targets
        .iter()
        .copied()
        .chain(actor_infos.keys().copied())
        .collect();
    let mut result = HashMap::new();

    for (target_id, actor_map) in intervals_by_target {
        if !allowed_targets.contains(&target_id) {
            continue;
        }

        let battle_start_ms =
            target_battle_start_ms(&target_start_times, global_start_time, target_id, now_ms);
        let summaries = target_buff_summaries(target_id, &actor_map, battle_start_ms, now_ms);
        if !summaries.is_empty() {
            result.insert(target_id, summaries);
        }
    }

    result
}

/// The buffs on one target, one summary per caster and buff name: the
/// coverage is the union of the intervals of the skill ids that show that name.
fn target_buff_summaries(
    target_id: u32,
    actor_map: &HashMap<u32, HashMap<u32, VecDeque<BuffInterval>>>,
    battle_start_ms: u64,
    now_ms: u64,
) -> Vec<BuffSummary> {
    let battle_duration_ms = now_ms.saturating_sub(battle_start_ms).max(1);
    let mut summaries = Vec::new();
    for (actor_id, skill_map) in actor_map {
        for group in group_by_name(skill_map, battle_start_ms, now_ms) {
            let Some(last_interval) = group.last_interval() else {
                continue;
            };
            let covered_ms = group.covered_ms(battle_start_ms, now_ms);
            summaries.push(BuffSummary {
                target_id,
                actor_id: *actor_id,
                skill_code: group.skill_code,
                coverage: (covered_ms as f64 / battle_duration_ms as f64).clamp(0.0, 1.0),
                active: last_interval.end_ms > now_ms,
                last_start_ms: last_interval.start_ms,
                last_end_ms: last_interval.end_ms,
            });
        }
    }
    summaries.sort_by(|a, b| a.skill_code.cmp(&b.skill_code).then_with(|| a.actor_id.cmp(&b.actor_id)));
    summaries
}

fn target_battle_start_ms(
    target_start_times: &HashMap<u32, HashMap<u32, f64>>,
    global_start_time: Option<f64>,
    target_id: u32,
    now_ms: u64,
) -> u64 {
    target_start_times
        .get(&target_id)
        .and_then(|by_actor| by_actor.values().copied().reduce(f64::min))
        .or(global_start_time)
        .map(|seconds| (seconds * 1000.0).max(0.0) as u64)
        .unwrap_or(now_ms)
}

// =============================================================================
// Helpers
// =============================================================================

fn filter_nested_map<T: Clone>(data: &HashMap<u32, T>, kept: &[u32]) -> HashMap<u32, T> {
    kept.iter()
        .filter_map(|tid| data.get(tid).map(|v| (*tid, v.clone())))
        .collect()
}

fn current_timestamp_seconds() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or_default()
}

fn current_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPIRITS_DESCENT_SHORT: u32 = 167_300_011;
    const SPIRITS_DESCENT_LONG: u32 = 167_300_012;
    const ELEMENT_UNIFICATION: u32 = 167_800_011;
    const ELEMENT_UNIFICATION_PULSE: u32 = 167_800_021;
    const YSMERA: u32 = 1;
    const OTHER: u32 = 2;
    const TARGET: u32 = 10;

    fn secs(from: u64, to: u64) -> BuffInterval {
        BuffInterval { start_ms: from * 1_000, end_ms: to * 1_000 }
    }

    fn by_skill(entries: &[(u32, &[BuffInterval])]) -> HashMap<u32, VecDeque<BuffInterval>> {
        entries.iter().map(|&(code, intervals)| (code, intervals.iter().cloned().collect())).collect()
    }

    #[test]
    fn a_live_summary_covers_the_union_of_the_ids_that_show_one_name() {
        let actors = HashMap::from([
            (
                YSMERA,
                by_skill(&[
                    (SPIRITS_DESCENT_SHORT, &[secs(0, 10), secs(20, 30)]),
                    (SPIRITS_DESCENT_LONG, &[secs(5, 35)]),
                    (ELEMENT_UNIFICATION, &[secs(0, 10)]),
                    (ELEMENT_UNIFICATION_PULSE, &[secs(40, 41)]),
                ]),
            ),
            (OTHER, by_skill(&[(SPIRITS_DESCENT_SHORT, &[secs(0, 20)])])),
        ]);

        let summaries = target_buff_summaries(TARGET, &actors, 0, 100_000);

        let rows: Vec<(u32, u32, f64, bool)> = summaries
            .iter()
            .map(|summary| (summary.actor_id, summary.skill_code, summary.coverage, summary.active))
            .collect();
        assert_eq!(
            rows,
            [
                (OTHER, SPIRITS_DESCENT_SHORT, 0.2, false),
                (YSMERA, SPIRITS_DESCENT_LONG, 0.35, false),
                (YSMERA, ELEMENT_UNIFICATION, 0.11, false),
            ],
            "0-35 s of Spirit's Descent once (not the 30 s longest, not the 50 s sum), the two keys of \
             Element Unification as one, the other caster apart"
        );
    }

    #[test]
    fn a_merged_buff_is_active_while_any_of_its_ids_is_and_shows_its_latest_interval() {
        let actors = HashMap::from([(
            YSMERA,
            by_skill(&[(SPIRITS_DESCENT_SHORT, &[secs(0, 10)]), (SPIRITS_DESCENT_LONG, &[secs(20, 130)])]),
        )]);

        let summaries = target_buff_summaries(TARGET, &actors, 0, 100_000);

        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].active);
        assert_eq!((summaries[0].last_start_ms, summaries[0].last_end_ms), (20_000, 130_000));
        assert_eq!(summaries[0].coverage, 0.9);
    }
}
