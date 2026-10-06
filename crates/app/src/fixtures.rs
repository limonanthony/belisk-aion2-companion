//! Synthetic snapshots for `--capture --fixtures`, built around the sample
//! fight in `sample.rs`. Every player and boss name here is invented.

use std::collections::HashMap;

use meter_core::history::HistoryRecord;
use meter_core::models::combat::{
    ActorInfo, BuffSummary, CombatInfos, CombatSnapshot, PlayerOverviewStat, PvpCombatStatsRow,
    PvpDamageOverviewStat, PvpKnownPlayer, PvpWatchInfo, PvpWatchInfoResponse, SkillStats, TargetInfo,
};

use crate::sample::{boss_fight, indexed, session, BOSS_ID, BOSS_MOB, ROSTER, YOU, YOUR_ID};

pub const LONG_BOSS: &str = "Ashen Tide Colossus, Warden of the Sunken Vault";
const MOB_ID: u32 = 90_002;
const MOB_CODE: u32 = 1_100_017;

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

/// Your skills on the boss: (code, hits, damage, special counts). Real
/// skill codes, so names and icons resolve; the numbers are invented and sum
/// to the roster's 48,210 DPS over 221 s.
const DETAIL_SKILLS: [(u32, u32, u64, &[(&str, u32)]); 10] = [
    (11_020_150, 38, 3_700_000, &[("CRITICAL", 17), ("BACK", 20), ("FRONT", 9), ("PERFECT", 6), ("DOUBLE", 3)]),
    (11_032_340, 24, 2_600_000, &[("CRITICAL", 11), ("BACK", 12), ("MULTIHIT", 10), ("MULTIHIT2", 6), ("MULTIHIT3", 4), ("MULTIHITDMG", 640_000)]),
    (11_050_130, 16, 1_700_000, &[("CRITICAL", 8), ("PARRY", 2)]),
    (11_061_350, 12, 1_250_000, &[("CRITICAL", 5), ("BACK", 6)]),
    (11_101_340, 6, 900_000, &[("CRITICAL", 3)]),
    (11_171_240, 4, 204_000, &[]),
    (11_080_030, 5, 120_000, &[("CRITICAL", 2)]),
    (11_090_030, 3, 90_000, &[("BACK", 1)]),
    (11_130_030, 2, 60_000, &[]),
    (11_120_000, 1, 30_000, &[]),
];

fn detail_skill_table(scale: u64) -> HashMap<u32, SkillStats> {
    DETAIL_SKILLS
        .iter()
        .map(|&(code, hits, damage, special)| {
            let average = damage / u64::from(hits);
            let stats = SkillStats {
                counts: hits,
                total_damage: damage / scale,
                min_damage: average / 3,
                max_damage: average * 2,
                special_counts: special.iter().map(|&(kind, count)| (kind.to_string(), count)).collect(),
            };
            (code, stats)
        })
        .collect()
}

/// The boss fight with everything the detail window draws: skills with their
/// figures, spec slots, buffs on you and on the boss, and a second mob behind
/// it so the Session board's totals add two targets.
pub fn detail_fight(secs: f64, hp: u32) -> CombatSnapshot {
    let mut snapshot = boss_fight(secs, hp);
    snapshot.by_target_player_skill_stats = HashMap::from([
        (BOSS_ID, HashMap::from([(YOUR_ID, detail_skill_table(1))])),
        (MOB_ID, HashMap::from([(YOUR_ID, detail_skill_table(40))])),
    ]);
    snapshot.combat_infos.target_infos.insert(
        MOB_ID,
        TargetInfo {
            id: MOB_ID,
            target_mob_code: Some(MOB_CODE),
            target_name: Some("Thornback Boar".to_string()),
            is_boss: false,
            current_hp: Some(0),
            max_hp: Some(1_000_000),
            target_start_time: HashMap::from([(YOUR_ID, 800.0)]),
            target_last_time: HashMap::from([(YOUR_ID, 880.0)]),
        },
    );
    snapshot.combat_infos.actor_infos = ROSTER
        .iter()
        .enumerate()
        .map(|(index, &(name, class, server, power, _))| {
            let id = index as u32 + 1;
            let spec = HashMap::from([(DETAIL_SKILLS[0].0, vec![1, 2, 4]), (DETAIL_SKILLS[1].0, vec![3])]);
            let info = ActorInfo {
                id,
                actor_name: Some(name.to_string()),
                actor_server_id: Some(server.to_string()),
                actor_class: Some(class.to_string()),
                combat_power: Some(power),
                actor_skill_spec: if id == YOUR_ID { spec } else { HashMap::new() },
            };
            (id, info)
        })
        .collect();
    let buff = |target: u32, caster: u32, code: u32, coverage: f64, active: bool| BuffSummary {
        target_id: target,
        actor_id: caster,
        skill_code: code,
        coverage,
        active,
        last_start_ms: 0,
        last_end_ms: 0,
    };
    snapshot.use_buffs_by_target = HashMap::from([
        (
            YOUR_ID,
            vec![
                buff(YOUR_ID, 7, 11_263_450, 0.62, true),
                buff(YOUR_ID, 3, 11_290_240, 0.91, true),
                buff(YOUR_ID, 7, 11_302_350, 0.34, false),
            ],
        ),
        (BOSS_ID, vec![buff(BOSS_ID, YOUR_ID, 11_360_020, 0.78, true), buff(BOSS_ID, 4, 11_212_340, 0.15, false)]),
    ]);
    if let Some(you) = snapshot.session.players.iter_mut().find(|player| player.actor_name == YOU) {
        you.actor_id = YOUR_ID;
    }
    indexed(snapshot)
}

/// What the PvP window lists: players you hit and players who hit you.
pub fn pvp_fight() -> CombatSnapshot {
    let mut snapshot = boss_fight(60.0, 500);
    let dealt = |id: u32, name: &str, class: &str, damage: u64, dps: f64, share: f64| PvpDamageOverviewStat {
        player_id: id,
        player_name: name.to_string(),
        player_server_id: "1002".to_string(),
        player_class: class.to_string(),
        counts: 30,
        total_damage: damage,
        min_damage: 1,
        max_damage: 9,
        dps,
        damage_contribution: share,
        hp_damage_ratio: None,
        battle_start_time: 0.0,
    };
    snapshot.main_actor_dealt_player_overview_stats = vec![
        dealt(11, "Dovrek", "Cleric", 412_000, 6_870.0, 0.88),
        dealt(12, "Quenna", "Templar", 188_000, 3_130.0, 0.41),
        dealt(13, "Hask", "Fighter", 64_000, 1_070.0, 0.19),
    ];
    let received = |id: u32, name: &str, class: &str, damage: u64, dps: f64, share: f64| PlayerOverviewStat {
        actor_id: id,
        actor_name: name.to_string(),
        actor_server_id: "1003".to_string(),
        actor_class: class.to_string(),
        combat_power: None,
        counts: 25,
        total_damage: damage,
        min_damage: 1,
        max_damage: 9,
        special_counts: HashMap::new(),
        dps,
        damage_share: share,
        damage_contribution: share,
    };
    snapshot.main_actor_received_player_overview_stats = vec![
        received(21, "Saebrin", "Assassin", 301_000, 5_010.0, 0.56),
        received(22, "Nalla", "Cleric", 154_000, 2_560.0, 0.29),
        received(23, "", "", 80_000, 1_330.0, 0.15),
    ];
    snapshot
}

/// The names to watch, what the meter would answer for them, and the kill
/// stats. Korrinth is not in range.
pub fn pvp_watch() -> (Vec<&'static str>, PvpWatchInfoResponse, Vec<PvpCombatStatsRow>) {
    let info = |name: &str, id: u32, class: &str, hp: (u32, u32)| PvpWatchInfo {
        query_name: name.to_string(),
        actor_id: Some(id),
        actor_name: Some(name.to_string()),
        server_id: Some("1002".to_string()),
        actor_class: Some(class.to_string()),
        current_hp: Some(hp.0),
        max_hp: Some(hp.1),
    };
    let known = |id: u32, name: &str| PvpKnownPlayer {
        actor_id: id,
        actor_name: name.to_string(),
        server_id: Some("1002".to_string()),
        actor_class: None,
    };
    let response = PvpWatchInfoResponse {
        watch_info: vec![info("Vaelith", 5, "Sorcerer", (31_200, 46_000)), info("Tobiah", 7, "Elementalist", (8_400, 52_000))],
        known_players: vec![
            known(5, "Vaelith"),
            known(7, "Tobiah"),
            known(11, "Dovrek"),
            known(12, "Quenna"),
            known(13, "Hask"),
            known(31, "Kaedric"),
            known(32, "Kaelis"),
            known(33, "Karrow"),
        ],
        last_dealt_player: Some(info("Dovrek", 11, "Cleric", (52_800, 58_000))),
    };
    let stats = |name: &str, damage: u64, kills: u32, assists: u32, deaths: u32| PvpCombatStatsRow {
        actor_name: name.to_string(),
        server_id: "1002".to_string(),
        damage,
        kills,
        assists,
        deaths,
    };
    let rows = vec![
        stats("Mirelle", 1_240_000, 6, 2, 1),
        stats("Vaelith", 1_810_000, 4, 5, 3),
        stats("Korrinth", 640_000, 5, 0, 0),
        stats("Saebrin", 975_000, 2, 7, 2),
        stats("Ilyrae", 220_000, 0, 9, 4),
    ];
    (vec!["Vaelith", "Korrinth", "Tobiah"], response, rows)
}

/// The kill stats with fourteen players, so the table scrolls.
pub fn pvp_stats_long() -> Vec<PvpCombatStatsRow> {
    let more = [
        ("Brannoc", 198_000, 0, 1, 1),
        ("Ferelith", 176_000, 1, 0, 2),
        ("Hask", 152_000, 0, 0, 1),
        ("Iridel", 131_000, 0, 2, 0),
        ("Morrow", 98_000, 0, 0, 2),
        ("Nalla", 64_000, 0, 1, 1),
        ("Quill", 31_000, 0, 0, 1),
        ("Tobiah", 22_000, 0, 1, 0),
        ("Dovrek", 12_000, 0, 0, 3),
    ];
    let (_, _, mut rows) = pvp_watch();
    rows.extend(more.iter().map(|&(name, damage, kills, assists, deaths)| PvpCombatStatsRow {
        actor_name: name.to_string(),
        server_id: "1003".to_string(),
        damage,
        kills,
        assists,
        deaths,
    }));
    rows
}

/// Saved fights, newest last in the vector; the history sorts them. Two
/// targets and two main players, so the filters have something to filter.
pub fn history_records() -> Vec<HistoryRecord> {
    const NOW: u64 = 1_791_052_949_892;
    let fights: [(&str, &str, f64, u64); 14] = [
        ("Veilwarden Orsikh", YOU, 247.0, 10_654_000),
        ("Ashen Tide Colossus", YOU, 301.0, 18_820_000),
        ("Veilwarden Orsikh", "Vaelith", 233.0, 9_410_000),
        ("Thornback Boar", YOU, 41.0, 2_050_000),
        ("Ashen Tide Colossus", "Vaelith", 318.0, 20_110_000),
        ("Veilwarden Orsikh", YOU, 262.0, 9_980_000),
        ("Ashen Tide Colossus", YOU, 290.0, 17_640_000),
        ("Veilwarden Orsikh", "Vaelith", 281.0, 9_120_000),
        ("Thornback Boar", YOU, 38.0, 1_910_000),
        ("Veilwarden Orsikh", YOU, 299.0, 8_870_000),
        ("Ashen Tide Colossus", "Vaelith", 305.0, 16_930_000),
        ("Veilwarden Orsikh", YOU, 271.0, 9_300_000),
        ("Ashen Tide Colossus", YOU, 312.0, 18_100_000),
        ("Veilwarden Orsikh", "Vaelith", 255.0, 8_640_000),
    ];
    fights
        .iter()
        .enumerate()
        .map(|(index, &(boss, main, secs, damage))| {
            let boss_snapshot = detail_fight(secs, 0);
            let target_id = 70_000 + index as u32;
            let players: HashMap<u32, PlayerOverviewStat> = ROSTER
                .iter()
                .enumerate()
                .map(|(roster_index, &(name, class, server, power, dps))| {
                    let id = roster_index as u32 + 1;
                    let total = (dps * secs) as u64;
                    let stat = PlayerOverviewStat {
                        actor_id: id,
                        actor_name: if index == 3 && id > 2 { String::new() } else { name.to_string() },
                        actor_server_id: server.to_string(),
                        actor_class: class.to_string(),
                        combat_power: Some(power),
                        counts: 100,
                        total_damage: total,
                        min_damage: 1,
                        max_damage: 9,
                        special_counts: HashMap::new(),
                        dps,
                        damage_share: dps / ROSTER.iter().map(|player| player.4).sum::<f64>(),
                        damage_contribution: 0.5,
                    };
                    (id, stat)
                })
                .collect();
            let target = TargetInfo {
                id: target_id,
                target_mob_code: Some(BOSS_MOB),
                target_name: Some(boss.to_string()),
                is_boss: true,
                current_hp: Some(0),
                max_hp: Some(1_000),
                target_start_time: HashMap::from([(YOUR_ID, 1_000.0)]),
                target_last_time: HashMap::from([(YOUR_ID, 1_000.0 + secs)]),
            };
            HistoryRecord {
                id: format!("{target_id}-{}", NOW - index as u64 * 5_400_000),
                target_id,
                total_damage: damage,
                target_info: Some(target),
                combat_infos: CombatInfos {
                    main_actor_name: Some(main.to_string()),
                    main_actor_id: Some(YOUR_ID),
                    ..boss_snapshot.combat_infos
                },
                player_skill_stats: HashMap::from([(YOUR_ID, detail_skill_table(1))]),
                player_stats: players,
                use_buffs_by_target: boss_snapshot.use_buffs_by_target,
                created_at: NOW - index as u64 * 5_400_000,
            }
        })
        .collect()
}
