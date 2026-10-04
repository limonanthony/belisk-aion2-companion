//! Synthetic snapshots for `--capture --fixtures`, mirroring the approved
//! mockup's sample fight (docs/gpui-mockups/approved/kit.js). Every player
//! and boss name here is invented.

use std::collections::HashMap;

use aether_core::models::combat::{
    CombatInfos, CombatSnapshot, PlayerOverviewStat, SkillStats, TargetInfo,
};
use aether_core::models::session::{SessionPlayerStat, SessionSummary};

pub const YOU: &str = "Mirelle";
/// A real boss id from dungeons.json, so the summary names its dungeon.
pub const BOSS_MOB: u32 = 2300243;
const BOSS_ID: u32 = 90_001;
const YOUR_ID: u32 = 2;

const ROSTER: [(&str, &str, &str, u64, f64); 8] = [
    ("Vaelith", "Sorcerer", "1001", 41_820, 61_240.0),
    ("Mirelle", "Ranger", "1002", 38_904, 48_210.0),
    ("Korrinth", "Gladiator", "1001", 40_112, 39_875.0),
    ("Saebrin", "Assassin", "1003", 36_455, 34_602.0),
    ("Tobiah", "Elementalist", "1002", 35_020, 28_940.0),
    ("Quenna", "Templar", "1001", 39_670, 22_318.0),
    ("Ilyrae", "Chanter", "1003", 34_210, 17_455.0),
    ("Dovrek", "Cleric", "1002", 33_880, 12_936.0),
];

const SKILLS: [(u32, u64); 4] = [
    (14_020_000, 3_708_000),
    (14_110_000, 2_631_000),
    (14_030_000, 1_674_000),
    (13_000_001, 1_950_000),
];

/// The session since the last reset: 18:42 of combat over 37 mobs, as in
/// the approved mockup. Brannoc and Ferelith are passers-by.
const SESSION_SECS: f64 = 1_122.0;
const SESSION_TARGETS: u32 = 37;
const SESSION: [(u32, &str, &str, &str, u64, f64, f64); 8] = [
    (48_213, "Vaelith", "Sorcerer", "1001", 41_820, 17.84e6, 41_240.0),
    (51_907, "Mirelle", "Ranger", "1002", 38_904, 14.62e6, 38_420.0),
    (47_730, "Korrinth", "Gladiator", "1001", 40_112, 11.05e6, 29_610.0),
    (52_816, "Ilyrae", "Chanter", "1003", 34_210, 4.31e6, 12_870.0),
    (49_964, "Brannoc", "Templar", "1001", 37_450, 2.18e6, 24_730.0),
    (50_381, "Ferelith", "Elementalist", "1003", 35_980, 0.96e6, 31_020.0),
    (46_102, "Hask", "Fighter", "1002", 32_610, 0.41e6, 15_360.0),
    (53_340, "Nalla", "Cleric", "1002", 30_420, 0.18e6, 6_120.0),
];
pub const LONG_BOSS: &str = "Ashen Tide Colossus, Warden of the Sunken Vault";
const MOB_ID: u32 = 90_002;
const MOB_CODE: u32 = 1_100_017;

fn session() -> SessionSummary {
    let total: f64 = SESSION.iter().map(|player| player.5).sum();
    SessionSummary {
        total_damage: total as u64,
        target_count: SESSION_TARGETS,
        combat_time_secs: SESSION_SECS,
        players: SESSION
            .iter()
            .map(|&(id, name, class, server, power, damage, dps)| SessionPlayerStat {
                actor_id: id,
                actor_name: name.to_string(),
                actor_server_id: server.to_string(),
                actor_class: class.to_string(),
                combat_power: Some(power),
                total_damage: damage as u64,
                combat_time_secs: damage / dps,
                dps,
                damage_share: damage / total,
            })
            .collect(),
    }
}

/// An open-world mob being hit right now, with the session behind it.
pub fn open_world() -> CombatSnapshot {
    let secs = 11.0;
    let rows = [(51_907, "Mirelle", "Ranger", "1002", 38_904, 419_000, 38_120.0), (50_381, "Ferelith", "Elementalist", "1003", 35_980, 171_600, 15_600.0)];
    let players: Vec<PlayerOverviewStat> = rows
        .iter()
        .map(|&(id, name, class, server, power, damage, dps)| PlayerOverviewStat {
            actor_id: id,
            actor_name: name.to_string(),
            actor_server_id: server.to_string(),
            actor_class: class.to_string(),
            combat_power: Some(power),
            counts: 20,
            total_damage: damage,
            min_damage: 1,
            max_damage: 1,
            special_counts: HashMap::new(),
            dps,
            damage_share: damage as f64 / 590_600.0,
            damage_contribution: damage as f64 / 1_000_000.0,
        })
        .collect();
    let target = TargetInfo {
        id: MOB_ID,
        target_mob_code: Some(MOB_CODE),
        target_name: Some("Thornback Boar".to_string()),
        is_boss: false,
        current_hp: Some(410_000),
        max_hp: Some(1_000_000),
        target_start_time: HashMap::from([(51_907, 2_000.0)]),
        target_last_time: HashMap::from([(51_907, 2_000.0 + secs)]),
    };
    indexed(CombatSnapshot {
        total_damage: 590_600,
        by_target_player_skill_stats: HashMap::new(),
        by_target_player_stats: HashMap::new(),
        use_buffs_by_target: HashMap::new(),
        combat_infos: CombatInfos {
            actor_infos: HashMap::new(),
            target_infos: HashMap::new(),
            main_actor_id: Some(51_907),
            main_actor_name: Some(YOU.to_string()),
            last_target_by_main_actor: Some(MOB_ID),
            last_target: Some(MOB_ID),
            time_now: 2_000.0 + secs,
        },
        last_target_info: Some(target),
        last_target_all_players_overview_stats: players,
        main_actor_received_player_overview_stats: Vec::new(),
        main_actor_dealt_player_overview_stats: Vec::new(),
        session: session(),
    })
}

/// The open world before the game has named you: ids for names, nobody
/// marked, one class not known yet.
pub fn unidentified() -> CombatSnapshot {
    let mut snapshot = open_world();
    snapshot.combat_infos.main_actor_id = None;
    snapshot.combat_infos.main_actor_name = None;
    for player in &mut snapshot.session.players {
        player.actor_name.clear();
        player.combat_power = None;
        if player.actor_id == 52_816 {
            player.actor_class.clear();
        }
    }
    for player in &mut snapshot.last_target_all_players_overview_stats {
        player.actor_name.clear();
    }
    indexed(snapshot)
}

/// The sample boss fight under a name too long for the header.
pub fn long_boss_fight(secs: f64, hp: u32) -> CombatSnapshot {
    let mut snapshot = boss_fight(secs, hp);
    if let Some(target) = &mut snapshot.last_target_info {
        target.target_name = Some(LONG_BOSS.to_string());
    }
    indexed(snapshot)
}

/// The open world with only you on the mob: a one-row Target board.
pub fn alone() -> CombatSnapshot {
    let mut snapshot = open_world();
    snapshot.last_target_all_players_overview_stats.truncate(1);
    indexed(snapshot)
}

/// A party heavy on rangers and clerics (the lime and amber cases), so the
/// class shades show: four rangers, three or four clerics, the rest assassins.
pub fn same_classes(mut snapshot: CombatSnapshot) -> CombatSnapshot {
    fn class_of(name: &str) -> &'static str {
        match name {
            "Vaelith" | "Mirelle" | "Ilyrae" | "Ferelith" | "Saebrin" => "Ranger",
            "Korrinth" | "Brannoc" | "Hask" | "Tobiah" => "Cleric",
            _ => "Assassin",
        }
    }
    for player in &mut snapshot.session.players {
        player.actor_class = class_of(&player.actor_name).to_string();
    }
    for player in &mut snapshot.last_target_all_players_overview_stats {
        player.actor_class = class_of(&player.actor_name).to_string();
    }
    indexed(snapshot)
}

/// The sample fight after `secs` seconds, with the boss at `hp` of 1,000.
pub fn boss_fight(secs: f64, hp: u32) -> CombatSnapshot {
    let party: f64 = ROSTER.iter().map(|player| player.4).sum();
    let taken = party * 1.035;
    let players: Vec<PlayerOverviewStat> = ROSTER
        .iter()
        .enumerate()
        .map(|(index, &(name, class, server, power, dps))| PlayerOverviewStat {
            actor_id: index as u32 + 1,
            actor_name: name.to_string(),
            actor_server_id: server.to_string(),
            actor_class: class.to_string(),
            combat_power: Some(power),
            counts: 100,
            total_damage: (dps * secs) as u64,
            min_damage: 1,
            max_damage: 1,
            special_counts: HashMap::new(),
            dps,
            damage_share: dps / party,
            damage_contribution: dps / taken,
        })
        .collect();

    let skills: HashMap<u32, SkillStats> = SKILLS
        .iter()
        .map(|&(code, damage)| {
            let stats = SkillStats {
                counts: 100,
                total_damage: damage,
                min_damage: 1,
                max_damage: 1,
                special_counts: HashMap::from([("CRITICAL".to_string(), 41)]),
            };
            (code, stats)
        })
        .collect();

    let target = TargetInfo {
        id: BOSS_ID,
        target_mob_code: Some(BOSS_MOB),
        target_name: Some("Veilwarden Orsikh".to_string()),
        is_boss: true,
        current_hp: Some(hp),
        max_hp: Some(1_000),
        target_start_time: HashMap::from([(YOUR_ID, 1_000.0)]),
        target_last_time: HashMap::from([(YOUR_ID, 1_000.0 + secs)]),
    };

    indexed(CombatSnapshot {
        total_damage: players.iter().map(|player| player.total_damage).sum(),
        by_target_player_skill_stats: HashMap::from([(BOSS_ID, HashMap::from([(YOUR_ID, skills)]))]),
        by_target_player_stats: HashMap::new(),
        use_buffs_by_target: HashMap::new(),
        combat_infos: CombatInfos {
            actor_infos: HashMap::new(),
            target_infos: HashMap::new(),
            main_actor_id: Some(YOUR_ID),
            main_actor_name: Some(YOU.to_string()),
            last_target_by_main_actor: Some(BOSS_ID),
            last_target: Some(BOSS_ID),
            time_now: 1_000.0 + secs,
        },
        last_target_info: Some(target),
        last_target_all_players_overview_stats: players,
        main_actor_received_player_overview_stats: Vec::new(),
        main_actor_dealt_player_overview_stats: Vec::new(),
        session: session(),
    })
}

/// The per-target views the meter reads, filled in from the last target as
/// the core does for every target it keeps. Run again after editing.
fn indexed(mut snapshot: CombatSnapshot) -> CombatSnapshot {
    if let Some(target) = snapshot.last_target_info.clone() {
        snapshot.by_target_player_stats.insert(
            target.id,
            snapshot
                .last_target_all_players_overview_stats
                .iter()
                .map(|player| (player.actor_id, player.clone()))
                .collect(),
        );
        snapshot.combat_infos.target_infos.insert(target.id, target);
    }
    snapshot
}
