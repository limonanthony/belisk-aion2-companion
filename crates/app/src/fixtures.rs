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

/// The sample boss fight where every other player is not named yet, so the
/// Target board has ID rows unless unknown players are hidden.
pub fn boss_with_unknown_players() -> CombatSnapshot {
    let mut snapshot = boss_fight(221.0, 624);
    for (index, player) in snapshot.last_target_all_players_overview_stats.iter_mut().enumerate() {
        if player.actor_id != YOUR_ID && index % 2 == 1 {
            player.actor_name.clear();
        }
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

/// Saved fights, newest first in the vector; the history sorts them. Two
/// targets and two main players, so the filters have something to filter;
/// the second newest is a wipe at 12.8%, under a kill.
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
            let is_boss = boss != "Thornback Boar";
            let wiped = index == 1;
            let target = TargetInfo {
                id: target_id,
                target_mob_code: Some(if is_boss { BOSS_MOB } else { MOB_CODE }),
                target_name: Some(boss.to_string()),
                is_boss,
                current_hp: Some(if wiped { 128 } else { 0 }),
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
                ledger: None,
                wiped,
                created_at: NOW - index as u64 * 5_400_000,
            }
        })
        .collect()
}

// ---- Fight reports (docs/gpui-mockups/approved/report.html) ----
//
// Bosses, monsters, the dungeon, servers and skills are game data; which
// monster used which skill, the players and every figure are invented, as
// in the approved mockup.

/// Which of the approved mockup's fights.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReportFight {
    /// Basilus the False in the Citadel of the Fallen Daeva (Hard), six players.
    Basilus,
    /// Tyrant Tassin, a field boss with fourteen players.
    Tassin,
    /// Superbia with long player names.
    Superbia,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReportDeaths {
    /// The fight's own: one for Basilus, two on Tassin, one on Superbia.
    Usual,
    Nobody,
    /// Four on Basilus, Saebrin twice.
    Several,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReportEnd {
    Defeated,
    /// The boss lived: "Fight ended, may resume".
    Ended,
    /// The attempt was wiped with the boss at 12.8%.
    Wiped,
}

#[derive(Clone, Copy)]
pub struct ReportFixture {
    pub fight: ReportFight,
    pub deaths: ReportDeaths,
    pub end: ReportEnd,
    /// Saved before the meter kept healing, damage taken and deaths.
    pub old: bool,
}

impl ReportFixture {
    pub const fn of(fight: ReportFight) -> Self {
        Self { fight, deaths: ReportDeaths::Usual, end: ReportEnd::Defeated, old: false }
    }
}

const REPORT_BOSS_ID: u32 = 80_001;

/// (name, class, server, combat power, damage, healing, damage taken, you)
type Member = (&'static str, &'static str, &'static str, u64, f64, f64, f64, bool);

const PARTY: [Member; 6] = [
    ("Vaelith", "Sorcerer", "1001", 41_820, 19.11e6, 0.15e6, 1.35e6, false),
    ("Mirelle", "Ranger", "1002", 38_904, 15.04e6, 0.21e6, 0.962e6, true),
    ("Korrinth", "Gladiator", "1001", 40_112, 12.44e6, 0.62e6, 2.91e6, false),
    ("Saebrin", "Assassin", "1003", 36_455, 10.8e6, 0.0, 1.88e6, false),
    ("Quenna", "Templar", "1001", 39_670, 6.96e6, 0.84e6, 7.42e6, false),
    ("Dovrek", "Cleric", "1002", 33_880, 4.04e6, 6.12e6, 1.12e6, false),
];

const FIELD: [Member; 14] = [
    ("Kaedric", "Gladiator", "1003", 42_310, 24.6e6, 0.94e6, 9.8e6, false),
    ("Vaelith", "Sorcerer", "1001", 41_820, 22.9e6, 0.12e6, 2.1e6, false),
    ("Mirelle", "Ranger", "1002", 38_904, 18.4e6, 0.18e6, 1.4e6, true),
    ("Pellam", "Ranger", "1003", 37_120, 15.2e6, 0.09e6, 1.7e6, false),
    ("Korrinth", "Gladiator", "1001", 40_112, 14.7e6, 0.71e6, 4.2e6, false),
    ("Tobiah", "Elementalist", "1002", 35_020, 13.9e6, 0.11e6, 1.9e6, false),
    ("Saebrin", "Assassin", "1003", 36_455, 12.8e6, 0.0, 3.1e6, false),
    ("Brannoc", "Templar", "1001", 37_450, 9.6e6, 1.21e6, 11.4e6, false),
    ("Quenna", "Templar", "1001", 39_670, 8.1e6, 0.98e6, 8.7e6, false),
    ("Ilyrae", "Chanter", "1003", 34_210, 6.4e6, 3.42e6, 2.6e6, false),
    ("Ysolde", "Chanter", "1001", 33_940, 5.1e6, 2.88e6, 2.2e6, false),
    ("Dovrek", "Cleric", "1002", 33_880, 4.6e6, 7.94e6, 1.8e6, false),
    ("Nalla", "Cleric", "1002", 30_420, 3.2e6, 6.31e6, 2.4e6, false),
    ("Ferelith", "Elementalist", "1003", 35_980, 0.96e6, 0.0, 0.0, false),
];

/// Longer than real names, so the ellipsis shows.
const LONG_NAMES: [Member; 6] = [
    ("Seraphinaveyllithaerinelle", "Elementalist", "1001", 41_210, 17.62e6, 0.08e6, 1.21e6, false),
    ("Mirelle", "Ranger", "1002", 38_904, 14.11e6, 0.19e6, 0.88e6, true),
    ("Vexmorradunelle", "Assassin", "1003", 36_980, 12.37e6, 0.0, 2.02e6, false),
    ("Thornwallowbrandethor", "Templar", "1001", 40_350, 7.45e6, 0.92e6, 6.84e6, false),
    ("Aurelindoravaine", "Chanter", "1003", 35_110, 6.02e6, 3.11e6, 1.64e6, false),
    ("Kassiopheliandrael", "Cleric", "1002", 34_470, 3.86e6, 5.74e6, 1.33e6, false),
];

/// (skill, share of the player's damage, hits)
fn class_skills(class: &str) -> &'static [(u32, f64, u32)] {
    match class {
        "Ranger" => &[
            (14_020_000, 0.263, 188),
            (14_110_000, 0.189, 342),
            (14_270_000, 0.147, 516),
            (14_040_000, 0.115, 44),
            (14_080_000, 0.092, 96),
            (14_090_000, 0.068, 71),
            (14_100_000, 0.053, 58),
            (14_060_000, 0.041, 39),
            (14_220_000, 0.02, 22),
            (100_001, 0.012, 211),
        ],
        "Sorcerer" => &[
            (15_060_000, 0.24, 61),
            (15_210_000, 0.2, 288),
            (15_040_000, 0.16, 94),
            (15_250_000, 0.13, 27),
            (15_120_000, 0.11, 66),
            (15_030_000, 0.09, 112),
            (15_090_000, 0.07, 73),
        ],
        "Gladiator" => &[
            (11_040_000, 0.25, 141),
            (11_010_000, 0.21, 233),
            (11_090_000, 0.17, 96),
            (11_100_000, 0.15, 38),
            (11_050_000, 0.12, 77),
            (11_020_000, 0.1, 164),
        ],
        "Assassin" => &[
            (13_070_000, 0.27, 118),
            (13_350_000, 0.2, 42),
            (13_040_000, 0.16, 301),
            (13_060_000, 0.13, 37),
            (13_210_000, 0.11, 88),
            (13_050_000, 0.08, 129),
            (13_160_000, 0.05, 402),
        ],
        "Templar" => &[(12_090_000, 0.29, 74), (12_100_000, 0.23, 118), (12_240_000, 0.19, 41), (12_010_000, 0.17, 247), (12_060_000, 0.12, 63)],
        "Cleric" => &[(17_060_000, 0.32, 206), (17_040_000, 0.23, 48), (17_010_000, 0.18, 61), (17_070_000, 0.14, 52), (17_050_000, 0.13, 34)],
        "Chanter" => &[
            (18_050_000, 0.26, 102),
            (18_010_000, 0.22, 248),
            (18_070_000, 0.19, 87),
            (18_040_000, 0.14, 53),
            (18_210_000, 0.11, 39),
            (18_300_000, 0.08, 21),
        ],
        _ => &[
            (16_240_000, 0.24, 36),
            (16_001_301, 0.19, 22),
            (16_020_000, 0.16, 91),
            (16_040_000, 0.14, 77),
            (16_710_000, 0.11, 140),
            (16_010_000, 0.09, 118),
            (16_210_000, 0.07, 64),
        ],
    }
}

/// (skill, share of the player's healing, heals, over time)
fn class_heals(class: &str) -> &'static [(u32, f64, u32, bool)] {
    match class {
        "Cleric" => &[
            (17_100_000, 0.377, 58, false),
            (17_120_000, 0.239, 24, false),
            (17_090_000, 0.176, 212, true),
            (17_320_000, 0.121, 96, true),
            (17_270_000, 0.087, 3, false),
        ],
        "Chanter" => &[(18_170_000, 0.48, 40, false), (18_120_000, 0.31, 22, true), (18_710_000, 0.21, 8, false)],
        "Templar" => &[(12_230_000, 0.61, 14, false), (12_250_000, 0.39, 9, false)],
        "Gladiator" => &[(11_120_000, 1.0, 31, false)],
        "Ranger" => &[(14_790_000, 1.0, 6, false)],
        "Sorcerer" => &[(15_160_000, 1.0, 4, false)],
        "Elementalist" => &[(16_790_000, 1.0, 3, false)],
        _ => &[],
    }
}

/// The boss and its monsters: (code, name), then (monster index, skill, typical hit).
struct Arena {
    boss: (u32, &'static str),
    monsters: &'static [(u32, &'static str)],
    sources: &'static [(usize, u32, u64)],
    /// Share of a tank's, a melee's and a ranged player's damage taken, per source.
    weights: [&'static [f64]; 3],
    secs: f64,
}

fn arena(fight: ReportFight) -> Arena {
    match fight {
        ReportFight::Basilus => Arena {
            boss: (2_301_723, "Basilus the False"),
            monsters: &[(2_301_723, "Basilus the False"), (2_301_726, "Corrupted Daeva Executioner"), (2_301_731, "Corrupted Daeva Elite Sorcerer")],
            sources: &[
                (0, 1_000_010, 16_900),
                (0, 1_217_100, 47_800),
                (0, 1_800_250, 58_200),
                (0, 1_216_520, 39_400),
                (0, 1_501_490, 96_000),
                (1, 1_223_840, 28_700),
                (1, 1_000_020, 7_800),
                (2, 1_226_600, 29_600),
            ],
            weights: [
                &[0.42, 0.22, 0.16, 0.08, 0.05, 0.0, 0.07, 0.0],
                &[0.06, 0.26, 0.18, 0.19, 0.08, 0.13, 0.05, 0.05],
                &[0.0, 0.245, 0.0, 0.43, 0.1, 0.0, 0.073, 0.152],
            ],
            secs: 312.0,
        },
        ReportFight::Tassin => Arena {
            boss: (2_700_691, "Tyrant Tassin"),
            monsters: &[(2_700_691, "Tyrant Tassin")],
            sources: &[(0, 1_000_100, 21_400), (0, 1_228_680, 82_000), (0, 1_216_630, 61_500), (0, 1_216_760, 34_800)],
            weights: [&[0.55, 0.2, 0.15, 0.1], &[0.2, 0.35, 0.3, 0.15], &[0.0, 0.4, 0.6, 0.0]],
            secs: 401.0,
        },
        ReportFight::Superbia => Arena {
            boss: (2_101_279, "Superbia, Sacred Beast of Life"),
            monsters: &[(2_101_279, "Superbia, Sacred Beast of Life")],
            sources: &[(0, 1_215_970, 24_800), (0, 1_224_320, 12_600), (0, 1_220_880, 41_300), (0, 1_000_110, 15_200)],
            weights: [&[0.5, 0.2, 0.1, 0.2], &[0.3, 0.3, 0.2, 0.2], &[0.05, 0.5, 0.45, 0.0]],
            secs: 287.0,
        },
    }
}

/// (seconds into the fight, who, monster index, skill, last hit)
fn arena_deaths(fight: ReportFight, deaths: ReportDeaths) -> &'static [(f64, &'static str, usize, u32, u64)] {
    match (fight, deaths) {
        (_, ReportDeaths::Nobody) => &[],
        (ReportFight::Basilus, ReportDeaths::Usual) => &[(167.0, "Saebrin", 0, 1_217_100, 52_310)],
        (ReportFight::Basilus, ReportDeaths::Several) => &[
            (167.0, "Saebrin", 0, 1_217_100, 52_310),
            (245.0, "Vaelith", 1, 1_223_840, 31_840),
            (298.0, "Saebrin", 0, 1_800_250, 61_200),
            (301.0, "Dovrek", 0, 1_501_490, 98_400),
        ],
        (ReportFight::Tassin, _) => &[(133.0, "Saebrin", 0, 1_228_680, 88_400), (287.0, "Pellam", 0, 1_216_630, 71_200)],
        (ReportFight::Superbia, _) => &[(214.0, "Seraphinaveyllithaerinelle", 0, 1_220_880, 44_100)],
    }
}

/// (buff, caster, seconds up): on every player, then on the boss.
const BUFFS_ON_PLAYERS: [(u32, &str, f64); 7] = [
    (17_720_000, "Dovrek", 287.0),
    (12_450_000, "Quenna", 241.0),
    (14_710_000, "Mirelle", 198.0),
    (14_720_000, "Mirelle", 126.0),
    (17_430_000, "Dovrek", 94.0),
    (14_220_000, "Mirelle", 61.0),
    (17_410_000, "Dovrek", 38.0),
];
const BUFFS_ON_BOSS: [(u32, &str, f64); 4] =
    [(14_090_000, "Mirelle", 264.0), (11_140_000, "Korrinth", 172.0), (12_270_000, "Quenna", 149.0), (17_080_000, "Dovrek", 88.0)];

/// A boss fight as History saves it, with what the report reads.
pub fn report_record(fixture: ReportFixture) -> HistoryRecord {
    use meter_core::report::events::HealKind;
    use meter_core::report::ledger::{
        BuffUptime, FightLedger, FightWindow, HealSkill, KillerKind, LedgerDeath, LedgerKiller, LedgerLastHit, LedgerPlayer,
        TakenSkill,
    };

    let arena = arena(fixture.fight);
    let roster: &[Member] = match fixture.fight {
        ReportFight::Basilus => &PARTY,
        ReportFight::Tassin => &FIELD,
        ReportFight::Superbia => &LONG_NAMES,
    };
    let id_of = |name: &str| roster.iter().position(|member| member.0 == name).map(|index| index as u32 + 1);
    let you = roster.iter().position(|member| member.7).map_or(1, |index| index as u32 + 1);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64());
    let started_at = now - arena.secs - 90.0;
    let window = FightWindow { started_at, ended_at: started_at + arena.secs };

    let mut player_stats = HashMap::new();
    let mut player_skill_stats = HashMap::new();
    let mut actor_infos = HashMap::new();
    let mut ledger_players = Vec::new();
    for (index, &(name, class, server, power, damage, healing, taken, _)) in roster.iter().enumerate() {
        let id = index as u32 + 1;
        let mut totals: HashMap<String, u32> = HashMap::new();
        let mut hits = 0;
        let skills: HashMap<u32, SkillStats> = class_skills(class)
            .iter()
            .enumerate()
            .map(|(order, &(code, share, count))| {
                let rate = |base: f64, step: f64| (f64::from(count) * (base + step * ((order % 4) as f64))).round() as u32;
                let mut special = vec![
                    ("CRITICAL", rate(0.36, 0.03)),
                    ("BACK", rate(0.08, 0.015)),
                    ("FRONT", rate(0.05, 0.01)),
                    ("PARRY", rate(0.0, 0.005)),
                    ("PERFECT", rate(0.02, 0.008)),
                    ("DOUBLE", rate(0.04, 0.01)),
                ];
                let total = (damage * share) as u64;
                if order % 3 == 1 {
                    special.push(("MULTIHIT", rate(0.06, 0.03)));
                    special.push(("MULTIHITDMG", (total / 9) as u32));
                }
                hits += count;
                for (kind, value) in &special {
                    if *kind != "MULTIHITDMG" {
                        *totals.entry(kind.to_string()).or_default() += value;
                    }
                }
                let average = total / u64::from(count);
                let stats = SkillStats {
                    counts: count,
                    total_damage: total,
                    min_damage: average * 37 / 100,
                    max_damage: average * 26 / 10,
                    special_counts: special.into_iter().map(|(kind, value)| (kind.to_string(), value)).collect(),
                };
                (code, stats)
            })
            .collect();
        let spec = class_skills(class)
            .iter()
            .enumerate()
            .map(|(order, &(code, ..))| (code, (1..=5u32).filter(|slot| (order as u32 + slot) % 3 == 0).collect()))
            .collect();
        actor_infos.insert(
            id,
            ActorInfo {
                id,
                actor_name: Some(name.to_string()),
                actor_server_id: Some(server.to_string()),
                actor_class: Some(class.to_string()),
                combat_power: Some(power),
                actor_skill_spec: spec,
            },
        );
        player_skill_stats.insert(id, skills);
        player_stats.insert(
            id,
            PlayerOverviewStat {
                actor_id: id,
                actor_name: name.to_string(),
                actor_server_id: server.to_string(),
                actor_class: class.to_string(),
                combat_power: Some(power),
                counts: hits,
                total_damage: damage as u64,
                min_damage: 1,
                max_damage: 1,
                special_counts: totals,
                dps: damage / arena.secs,
                damage_share: 0.0,
                damage_contribution: 0.0,
            },
        );
        let role = match class {
            "Templar" => 0,
            "Gladiator" | "Assassin" | "Chanter" => 1,
            _ => 2,
        };
        let mut damage_taken: Vec<TakenSkill> = arena
            .sources
            .iter()
            .zip(arena.weights[role])
            .filter(|(_, weight)| **weight > 0.0 && taken > 0.0)
            .map(|(&(monster, skill, hit), weight)| {
                let amount = (taken * weight) as u64;
                let (code, monster_name) = arena.monsters[monster];
                TakenSkill {
                    monster_code: Some(code),
                    monster_name: Some(monster_name.to_string()),
                    skill_code: skill,
                    hits: (amount / hit).max(1) as u32,
                    largest: hit * 16 / 10,
                    amount,
                }
            })
            .collect();
        damage_taken.sort_by(|a, b| b.amount.cmp(&a.amount));
        let heals: &[(u32, f64, u32, bool)] = if healing > 0.0 { class_heals(class) } else { &[] };
        ledger_players.push(LedgerPlayer {
            actor_id: id,
            actor_name: name.to_string(),
            actor_server_id: server.to_string(),
            actor_class: class.to_string(),
            healing: heals
                .iter()
                .map(|&(code, share, count, over_time)| HealSkill {
                    skill_code: code,
                    kind: if over_time { HealKind::OverTime } else { HealKind::Direct },
                    heals: count,
                    crits: count * 35 / 100,
                    amount: (healing * share) as u64,
                })
                .collect(),
            damage_taken,
        });
    }

    let deaths = arena_deaths(fixture.fight, fixture.deaths)
        .iter()
        .filter_map(|&(at_secs, who, monster, skill, hit)| {
            let (code, monster_name) = arena.monsters[monster];
            Some(LedgerDeath {
                at_secs,
                actor_id: id_of(who)?,
                killer: LedgerKiller { kind: KillerKind::Monster, name: Some(monster_name.to_string()), monster_code: Some(code) },
                skill_code: skill,
                last_hit: Some(LedgerLastHit {
                    monster_code: Some(code),
                    monster_name: Some(monster_name.to_string()),
                    skill_code: skill,
                    amount: hit,
                }),
            })
        })
        .collect();
    let buff = |target: u32, (code, caster, secs): (u32, &str, f64)| {
        id_of(caster).map(|caster| (target, caster, code, secs))
    };
    let buffs: Vec<(u32, u32, u32, f64)> = (1..=roster.len() as u32)
        .flat_map(|target| BUFFS_ON_PLAYERS.iter().filter_map(move |&entry| buff(target, entry)))
        .chain(BUFFS_ON_BOSS.iter().filter_map(|&entry| buff(REPORT_BOSS_ID, entry)))
        .collect();

    let ledger = (!fixture.old).then(|| FightLedger {
        window,
        players: ledger_players,
        deaths,
        buffs: buffs
            .iter()
            .map(|&(target_id, caster_id, skill_code, secs)| BuffUptime { target_id, caster_id, skill_code, uptime_ms: (secs * 1000.0) as u64 })
            .collect(),
    });
    let mut use_buffs_by_target: HashMap<u32, Vec<BuffSummary>> = HashMap::new();
    for &(target_id, actor_id, skill_code, secs) in &buffs {
        use_buffs_by_target.entry(target_id).or_default().push(BuffSummary {
            target_id,
            actor_id,
            skill_code,
            coverage: secs / arena.secs,
            active: false,
            last_start_ms: 0,
            last_end_ms: 0,
        });
    }

    let total_damage = roster.iter().map(|member| member.4 as u64).sum();
    HistoryRecord {
        id: format!("{REPORT_BOSS_ID}-report"),
        target_id: REPORT_BOSS_ID,
        total_damage,
        target_info: Some(TargetInfo {
            id: REPORT_BOSS_ID,
            target_mob_code: Some(arena.boss.0),
            target_name: Some(arena.boss.1.to_string()),
            is_boss: true,
            current_hp: Some(match fixture.end {
                ReportEnd::Defeated => 0,
                ReportEnd::Ended => 183,
                ReportEnd::Wiped => 128,
            }),
            max_hp: Some(1_000),
            target_start_time: HashMap::from([(you, window.started_at)]),
            target_last_time: HashMap::from([(you, window.ended_at)]),
        }),
        combat_infos: CombatInfos {
            actor_infos,
            target_infos: HashMap::new(),
            main_actor_id: Some(you),
            main_actor_name: Some(YOU.to_string()),
            last_target_by_main_actor: Some(REPORT_BOSS_ID),
            last_target: Some(REPORT_BOSS_ID),
            time_now: window.ended_at,
        },
        player_skill_stats,
        player_stats,
        use_buffs_by_target,
        ledger,
        wiped: fixture.end == ReportEnd::Wiped,
        created_at: (window.ended_at * 1000.0) as u64,
    }
}

/// The report of [`report_record`].
pub fn fight_report(fixture: ReportFixture) -> meter_core::report::fight_report::FightReport {
    meter_core::report::fight_report::FightReport::from_record(&report_record(fixture)).expect("the fixture is a boss fight")
}

/// Mirelle's tickets as the approved to-do and Resources pages draw them
/// (todos.js, hub.js): (ticket, regular pool, extra pool). Odyle 245 +380,
/// Shugo keys 7 +3, Invasion keys 2, Nightmare 0 +3, the Daily Dungeon 9 of 14 (+7);
/// this week Ascension Trial with 1 left (+1).
pub const MIRELLE_TICKETS: &[(u32, u32, u32)] = &[
    (60_000_001, 245, 380),
    (1, 7, 3),
    (8, 2, 0),
    (4, 0, 3),
    (101, 9, 7),
    (80_000_001, 1, 1),
];

/// The game listing `character`'s tickets at login: (ticket, regular pool,
/// extra pool).
pub fn ticket_login(
    character: &meter_core::identity::KnownCharacter,
    captured_at_ms: i64,
    tickets: &[(u32, u32, u32)],
) -> meter_core::tickets::TicketReport {
    use meter_core::capture::parser::ticket_record::TicketRecord;
    let records = tickets
        .iter()
        .map(|&(id, count, charge)| TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge })
        .collect();
    meter_core::tickets::TicketReport {
        character: character.clone(),
        captured_at: captured_at_ms as f64 / 1000.0,
        content: meter_core::tickets::TicketContent::Snapshot(records),
    }
}

/// The changelog the What's new and Changelog captures draw: the approved
/// mockup's ten invented versions, 1.0.0 to 1.6.0 (updates.js), in the
/// subset the app's parser reads.
pub fn changelog(language: crate::i18n::Language) -> crate::changelog::Changelog {
    let source = match language {
        crate::i18n::Language::English => include_str!("fixtures/changelog.en.md"),
        crate::i18n::Language::French => include_str!("fixtures/changelog.fr.md"),
    };
    crate::changelog::Changelog::parse(source).expect("the invented changelog is in the subset")
}
