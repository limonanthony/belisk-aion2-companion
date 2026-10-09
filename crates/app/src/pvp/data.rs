//! What the PvP window lists, and the rules for the watch list.

use meter_core::models::combat::{
    PlayerOverviewStat, PvpCombatStatsRow, PvpDamageOverviewStat, PvpKnownPlayer, PvpWatchInfo, PvpWatchInfoResponse,
};

use crate::game_data::{self, PlayerClass};

const MAX_SUGGESTIONS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct WatchRow {
    /// The name that was added; for your last attacker, their own name.
    pub query_name: String,
    pub actor_id: Option<u32>,
    /// None until the game has shown the player: "not in range".
    pub actor_name: Option<String>,
    pub server: String,
    pub class: Option<PlayerClass>,
    pub hp: Option<(u32, u32)>,
    /// Your last attacker, pinned on top and never removable.
    pub last_attacker: bool,
}

impl WatchRow {
    pub fn removable(&self) -> bool {
        !self.last_attacker
    }

    pub fn in_range(&self) -> bool {
        self.actor_name.is_some()
    }

    /// 0 to 1; none while the player's health is unknown.
    pub fn hp_fraction(&self) -> Option<f64> {
        self.hp.filter(|(_, max)| *max > 0).map(|(current, max)| (f64::from(current) / f64::from(max)).clamp(0.0, 1.0))
    }

    fn from_info(info: &PvpWatchInfo, last_attacker: bool) -> Self {
        Self {
            query_name: info.query_name.clone(),
            actor_id: info.actor_id,
            actor_name: info.actor_id.and(info.actor_name.clone()).or_else(|| info.actor_id.map(|id| format!("ID:{id}"))),
            server: info.server_id.as_deref().map(game_data::server_name).unwrap_or_default(),
            class: info.actor_class.as_deref().and_then(PlayerClass::parse),
            hp: info.current_hp.zip(info.max_hp),
            last_attacker,
        }
    }

    fn missing(name: &str) -> Self {
        Self {
            query_name: name.to_string(),
            actor_id: None,
            actor_name: None,
            server: String::new(),
            class: None,
            hp: None,
            last_attacker: false,
        }
    }
}

/// Adds `name` unless it is blank or already watched. Returns whether the
/// list changed.
pub fn add_name(names: &mut Vec<String>, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || names.iter().any(|known| known == name) {
        return false;
    }
    names.push(name.to_string());
    true
}

pub fn remove_name(names: &mut Vec<String>, name: &str) -> bool {
    let before = names.len();
    names.retain(|known| known != name);
    names.len() != before
}

/// The rows the window shows: your last attacker on top, then the watched
/// names in the order they were added, each as the game's answer or "not in
/// range". A watched name that is the last attacker shows once, on top.
pub fn watch_rows(names: &[String], response: &PvpWatchInfoResponse) -> Vec<WatchRow> {
    let mut manual: Vec<WatchRow> = names
        .iter()
        .flat_map(|name| {
            let matches: Vec<WatchRow> = response
                .watch_info
                .iter()
                .filter(|info| info.query_name == *name)
                .map(|info| WatchRow::from_info(info, false))
                .collect();
            if matches.is_empty() {
                vec![WatchRow::missing(name)]
            } else {
                matches
            }
        })
        .collect();

    let Some(last) = response.last_dealt_player.as_ref().filter(|last| last.actor_id.is_some()) else {
        return manual;
    };
    manual.retain(|row| row.actor_id != last.actor_id && Some(&row.query_name) != last.actor_name.as_ref());
    manual.insert(0, WatchRow::from_info(last, true));
    manual
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub name: String,
    pub server: String,
}

/// Players seen this session whose names match what was typed, for the add
/// field: names starting with the text first, then the ones containing it.
pub fn suggestions(known: &[PvpKnownPlayer], watched: &[String], typed: &str) -> Vec<Suggestion> {
    let typed = typed.trim().to_lowercase();
    let mut seen: Vec<&str> = Vec::new();
    let mut starts = Vec::new();
    let mut contains = Vec::new();
    for player in known {
        let name = player.actor_name.as_str();
        if name.is_empty() || seen.contains(&name) || watched.iter().any(|watched| watched == name) {
            continue;
        }
        seen.push(name);
        let suggestion = Suggestion {
            name: name.to_string(),
            server: player.server_id.as_deref().map(game_data::server_name).unwrap_or_default(),
        };
        let lower = name.to_lowercase();
        if lower.starts_with(&typed) {
            starts.push(suggestion);
        } else if lower.contains(&typed) {
            contains.push(suggestion);
        }
    }
    starts.extend(contains);
    starts.truncate(MAX_SUGGESTIONS);
    starts
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatsSort {
    #[default]
    Damage,
    Kills,
}

/// The column's value first, most first; ties fall back to kills, assists,
/// fewer deaths, then name.
pub fn sort_stats(rows: &mut [PvpCombatStatsRow], sort: StatsSort) {
    rows.sort_by(|a, b| {
        let primary = match sort {
            StatsSort::Damage => b.damage.cmp(&a.damage),
            StatsSort::Kills => b.kills.cmp(&a.kills),
        };
        primary
            .then(b.kills.cmp(&a.kills))
            .then(b.assists.cmp(&a.assists))
            .then(a.deaths.cmp(&b.deaths))
            .then_with(|| a.actor_name.cmp(&b.actor_name))
    });
}

/// A player in the damage dealt or damage taken list.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageRow {
    pub actor_id: u32,
    pub name: String,
    /// The game has named the player, so the name can be watched.
    pub named: bool,
    pub server: String,
    pub class: Option<PlayerClass>,
    pub damage: u64,
    pub dps: f64,
    /// Of what the target took.
    pub share: f64,
    /// Bar length relative to the biggest row of its list, 0 to 1.
    pub bar: f32,
}

/// Players you hit, in the order you started on them.
pub fn dealt_rows(players: &[PvpDamageOverviewStat]) -> Vec<DamageRow> {
    let top = players.iter().map(|player| player.total_damage).max().unwrap_or(0);
    players
        .iter()
        .map(|player| DamageRow {
            actor_id: player.player_id,
            name: display_name(&player.player_name, player.player_id),
            named: !player.player_name.trim().is_empty(),
            server: game_data::server_name(&player.player_server_id),
            class: PlayerClass::parse(&player.player_class),
            damage: player.total_damage,
            dps: player.dps,
            share: player.damage_contribution,
            bar: if top > 0 { player.total_damage as f32 / top as f32 } else { 0.0 },
        })
        .collect()
}

/// Players who hit you, in the order they started on you.
pub fn received_rows(players: &[PlayerOverviewStat]) -> Vec<DamageRow> {
    let top = players.iter().map(|player| player.total_damage).max().unwrap_or(0);
    players
        .iter()
        .map(|player| DamageRow {
            actor_id: player.actor_id,
            name: display_name(&player.actor_name, player.actor_id),
            named: !player.actor_name.trim().is_empty(),
            server: game_data::server_name(&player.actor_server_id),
            class: PlayerClass::parse(&player.actor_class),
            damage: player.total_damage,
            dps: player.dps,
            share: player.damage_share,
            bar: if top > 0 { player.total_damage as f32 / top as f32 } else { 0.0 },
        })
        .collect()
}

fn display_name(name: &str, actor_id: u32) -> String {
    if name.trim().is_empty() {
        format!("ID:{actor_id}")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(query: &str, id: Option<u32>, hp: Option<(u32, u32)>) -> PvpWatchInfo {
        PvpWatchInfo {
            query_name: query.to_string(),
            actor_id: id,
            actor_name: id.map(|_| query.to_string()),
            server_id: id.map(|_| "1001".to_string()),
            actor_class: id.map(|_| "Sorcerer".to_string()),
            current_hp: hp.map(|hp| hp.0),
            max_hp: hp.map(|hp| hp.1),
        }
    }

    fn response(watch: Vec<PvpWatchInfo>, last: Option<PvpWatchInfo>) -> PvpWatchInfoResponse {
        PvpWatchInfoResponse { watch_info: watch, known_players: Vec::new(), last_dealt_player: last }
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn adding_trims_and_refuses_blanks_and_duplicates() {
        let mut watched = Vec::new();

        assert!(add_name(&mut watched, "  Vaelith "));
        assert!(!add_name(&mut watched, "Vaelith"));
        assert!(!add_name(&mut watched, "   "));
        assert_eq!(watched, names(&["Vaelith"]));
    }

    #[test]
    fn removing_forgets_only_that_name() {
        let mut watched = names(&["A", "B"]);

        assert!(remove_name(&mut watched, "A"));
        assert!(!remove_name(&mut watched, "A"));
        assert_eq!(watched, names(&["B"]));
    }

    #[test]
    fn watched_names_keep_their_order_and_missing_ones_are_not_in_range() {
        let rows = watch_rows(
            &names(&["Vaelith", "Gone"]),
            &response(vec![info("Vaelith", Some(5), Some((300, 1_000)))], None),
        );

        assert_eq!(rows.iter().map(|row| row.query_name.as_str()).collect::<Vec<_>>(), vec!["Vaelith", "Gone"]);
        assert!(rows[0].in_range());
        assert_eq!(rows[0].hp_fraction(), Some(0.3));
        assert_eq!(rows[0].server, "SIE");
        assert_eq!(rows[0].class, Some(PlayerClass::Sorcerer));
        assert!(!rows[1].in_range());
        assert_eq!(rows[1].hp_fraction(), None);
    }

    #[test]
    fn the_last_attacker_is_pinned_on_top_and_cannot_be_removed() {
        let rows = watch_rows(
            &names(&["Vaelith"]),
            &response(vec![info("Vaelith", Some(5), None)], Some(info("Korrinth", Some(9), Some((10, 20))))),
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].query_name, "Korrinth");
        assert!(rows[0].last_attacker);
        assert!(!rows[0].removable());
        assert!(rows[1].removable());
    }

    #[test]
    fn a_watched_player_who_is_also_the_last_attacker_shows_once() {
        let rows = watch_rows(
            &names(&["Korrinth", "Vaelith"]),
            &response(
                vec![info("Korrinth", Some(9), None), info("Vaelith", Some(5), None)],
                Some(info("Korrinth", Some(9), Some((10, 20)))),
            ),
        );

        assert_eq!(rows.iter().map(|row| row.query_name.as_str()).collect::<Vec<_>>(), vec!["Korrinth", "Vaelith"]);
        assert!(rows[0].last_attacker);
    }

    #[test]
    fn a_watched_name_not_yet_seen_who_is_the_last_attacker_shows_once() {
        let rows = watch_rows(
            &names(&["Korrinth"]),
            &response(vec![], Some(info("Korrinth", Some(9), None))),
        );

        assert_eq!(rows.len(), 1);
        assert!(rows[0].last_attacker);
    }

    #[test]
    fn a_last_attacker_without_an_id_is_not_pinned() {
        let rows = watch_rows(&names(&["Vaelith"]), &response(vec![], Some(info("Korrinth", None, None))));

        assert_eq!(rows.len(), 1);
        assert!(!rows[0].last_attacker);
    }

    #[test]
    fn health_is_a_fraction_only_with_a_maximum() {
        let mut row = WatchRow::missing("A");
        row.hp = Some((50, 0));
        assert_eq!(row.hp_fraction(), None);
        row.hp = Some((2_000, 1_000));
        assert_eq!(row.hp_fraction(), Some(1.0));
    }

    fn known(id: u32, name: &str) -> PvpKnownPlayer {
        PvpKnownPlayer { actor_id: id, actor_name: name.to_string(), server_id: Some("1002".to_string()), actor_class: None }
    }

    #[test]
    fn suggestions_prefer_prefixes_skip_watched_names_and_repeats() {
        let seen = [known(1, "Dvaeron"), known(2, "Vaelith"), known(3, "Vaelith"), known(4, "Korrinth"), known(5, "Vaelor")];

        let found = suggestions(&seen, &names(&["Vaelor"]), "vae");

        assert_eq!(found.iter().map(|found| found.name.as_str()).collect::<Vec<_>>(), vec!["Vaelith", "Dvaeron"]);
        assert_eq!(found[0].server, "NEZ");
    }

    #[test]
    fn suggestions_are_capped_and_empty_text_lists_everyone() {
        let seen: Vec<_> = (0..10).map(|id| known(id, &format!("Player{id}"))).collect();

        assert_eq!(suggestions(&seen, &[], "").len(), MAX_SUGGESTIONS);
        assert!(suggestions(&seen, &[], "zzz").is_empty());
    }

    fn stats(name: &str, damage: u64, kills: u32, assists: u32, deaths: u32) -> PvpCombatStatsRow {
        PvpCombatStatsRow { actor_name: name.to_string(), server_id: "1001".to_string(), damage, kills, assists, deaths }
    }

    fn order(rows: &[PvpCombatStatsRow]) -> Vec<&str> {
        rows.iter().map(|row| row.actor_name.as_str()).collect()
    }

    #[test]
    fn kill_stats_sort_by_the_chosen_column() {
        let mut rows = vec![stats("a", 100, 1, 5, 0), stats("b", 300, 0, 1, 4), stats("c", 200, 3, 2, 2)];

        sort_stats(&mut rows, StatsSort::Damage);
        assert_eq!(order(&rows), vec!["b", "c", "a"]);
        sort_stats(&mut rows, StatsSort::Kills);
        assert_eq!(order(&rows), vec!["c", "a", "b"]);
    }

    #[test]
    fn dealt_rows_scale_their_bars_to_the_biggest() {
        let stat = |id: u32, damage: u64| PvpDamageOverviewStat {
            player_id: id,
            player_name: "Quenna".to_string(),
            player_server_id: "1001".to_string(),
            player_class: "Templar".to_string(),
            counts: 1,
            total_damage: damage,
            min_damage: 1,
            max_damage: 1,
            dps: 1.0,
            damage_contribution: 0.5,
            hp_damage_ratio: None,
            battle_start_time: 0.0,
        };

        let rows = dealt_rows(&[stat(1, 200), stat(2, 800)]);

        assert_eq!(rows[0].bar, 0.25);
        assert_eq!(rows[1].bar, 1.0);
    }

    #[test]
    fn ties_fall_back_to_kills_assists_fewer_deaths_then_name() {
        let mut rows = vec![
            stats("d", 100, 1, 1, 3),
            stats("c", 100, 1, 1, 1),
            stats("b", 100, 1, 2, 9),
            stats("a", 100, 1, 1, 1),
            stats("e", 100, 2, 0, 0),
        ];

        sort_stats(&mut rows, StatsSort::Damage);

        assert_eq!(order(&rows), vec!["e", "b", "a", "c", "d"]);
    }

    #[test]
    fn received_rows_scale_their_bars_to_the_biggest_and_name_the_unnamed() {
        let stat = |id: u32, name: &str, damage: u64| PlayerOverviewStat {
            actor_id: id,
            actor_name: name.to_string(),
            actor_server_id: "1001".to_string(),
            actor_class: "Templar".to_string(),
            combat_power: None,
            counts: 1,
            total_damage: damage,
            min_damage: 1,
            max_damage: 1,
            special_counts: Default::default(),
            dps: 1.0,
            damage_share: 0.5,
            damage_contribution: 0.5,
        };

        let rows = received_rows(&[stat(1, "Vaelith", 400), stat(2, "", 100)]);

        assert_eq!(rows[0].bar, 1.0);
        assert_eq!(rows[1].bar, 0.25);
        assert_eq!(rows[1].name, "ID:2");
        assert_eq!(rows[0].class, Some(PlayerClass::Templar));
    }
}
