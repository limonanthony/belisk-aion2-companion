//! The fight history as the window lists it: records newest first, filtered by
//! target and by main player.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use meter_core::history::HistoryRecord;

use super::when;
use crate::format;
use crate::game_data::PlayerClass;
use crate::i18n::fl;
use crate::meter::{target_name, ShadeBook};

/// The label of a record whose main player is not known; it is also what the
/// player filter lists them under.
pub fn unknown_player() -> String {
    fl!("history-unknown-player")
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordRow {
    pub id: String,
    pub target: String,
    pub created_at: u64,
    pub when: String,
    /// The player whose meter recorded the fight; none when nobody is named.
    pub main_player: Option<String>,
    /// Players the game has named.
    pub player_count: usize,
    pub damage: u64,
    /// First hit anyone landed to the last; zero when unknown.
    pub duration_secs: f64,
    /// A boss fight: it has a fight report.
    pub is_boss: bool,
    /// The attempt was lost with the boss alive; no kill, no personal best.
    pub wiped: bool,
    /// For a wipe, the boss's health left as a share of its maximum, 0–1.
    pub hp_left: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerLine {
    pub actor_id: u32,
    /// `You` for the record's own player and `ID:<n>` for anyone else the game
    /// never named.
    pub name: String,
    /// Whether `name` is a name the game gave, so a mask applies to it.
    pub named: bool,
    pub class: Option<PlayerClass>,
    /// The player's shade of their class colour within this record.
    pub colour: Option<u32>,
    /// The record's main player: whose meter saved it.
    pub you: bool,
    pub damage: u64,
    pub dps: f64,
    pub share: f64,
}

/// One entry of a filter dropdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterOption {
    pub value: String,
    /// Records that carry this value, among all records.
    pub count: usize,
}

struct Entry {
    row: RecordRow,
    record: Arc<HistoryRecord>,
}

#[derive(Default)]
pub struct HistoryList {
    entries: Vec<Entry>,
    target_filter: Option<String>,
    player_filter: Option<String>,
}

impl HistoryList {
    /// Replaces the records. A filter whose value no record has any more falls
    /// back to "all".
    pub fn set_records(&mut self, records: Vec<HistoryRecord>) {
        let mut entries: Vec<Entry> = records
            .into_iter()
            .map(|record| Entry { row: RecordRow::from_record(&record), record: Arc::new(record) })
            .collect();
        entries.sort_by(|a, b| b.row.created_at.cmp(&a.row.created_at));
        self.entries = entries;
        self.drop_filters_without_records();
    }

    /// Writes every row again from its record, after the language changed.
    pub fn rebuild_words(&mut self) {
        for entry in &mut self.entries {
            entry.row = RecordRow::from_record(&entry.record);
        }
        self.drop_filters_without_records();
    }

    fn drop_filters_without_records(&mut self) {
        if !self.target_options().iter().any(|option| Some(&option.value) == self.target_filter.as_ref()) {
            self.target_filter = None;
        }
        if !self.player_options().iter().any(|option| Some(&option.value) == self.player_filter.as_ref()) {
            self.player_filter = None;
        }
    }

    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// Newest first, after the filters.
    pub fn visible(&self) -> Vec<&RecordRow> {
        self.shown().collect()
    }

    /// [`Self::visible`], one at a time.
    pub fn shown(&self) -> impl Iterator<Item = &RecordRow> {
        self.entries
            .iter()
            .map(|entry| &entry.row)
            .filter(|row| self.target_filter.as_ref().is_none_or(|target| *target == row.target))
            .filter(|row| self.player_filter.as_ref().is_none_or(|player| *player == main_player_label(row)))
    }

    pub fn record(&self, id: &str) -> Option<&Arc<HistoryRecord>> {
        self.entries.iter().find(|entry| entry.row.id == id).map(|entry| &entry.record)
    }

    pub fn target_options(&self) -> Vec<FilterOption> {
        options(self.entries.iter().map(|entry| entry.row.target.as_str()))
    }

    pub fn player_options(&self) -> Vec<FilterOption> {
        options(self.entries.iter().map(|entry| main_player_label(&entry.row)))
    }

    pub fn target_filter(&self) -> Option<&str> {
        self.target_filter.as_deref()
    }

    pub fn player_filter(&self) -> Option<&str> {
        self.player_filter.as_deref()
    }

    pub fn filter_target(&mut self, target: Option<String>) {
        self.target_filter = target;
    }

    pub fn filter_player(&mut self, player: Option<String>) {
        self.player_filter = player;
    }

    /// "14 records", or "6 / 14 records" while a filter hides some.
    pub fn count_label(&self) -> String {
        let (shown, total) = (self.visible().len(), self.total());
        if shown == total {
            fl!("history-records", count = total)
        } else {
            fl!("history-records-filtered", shown = shown, total = total)
        }
    }

    /// The players of one record, most damage first. With `hide_unknown_players`
    /// the ones the game never named are left out, except the record's own
    /// player; the record and every share stay as saved.
    pub fn players(&self, id: &str, hide_unknown_players: bool) -> Vec<PlayerLine> {
        let Some(record) = self.record(id) else {
            return Vec::new();
        };
        let colours = record_colours(record);
        let mut players: Vec<PlayerLine> = record
            .player_stats
            .values()
            .filter(|stat| {
                !hide_unknown_players
                    || !stat.actor_name.trim().is_empty()
                    || record.combat_infos.main_actor_id == Some(stat.actor_id)
            })
            .map(|stat| PlayerLine {
                actor_id: stat.actor_id,
                name: if stat.actor_name.trim().is_empty() {
                    format::unnamed_player(stat.actor_id, record.combat_infos.main_actor_id == Some(stat.actor_id))
                } else {
                    stat.actor_name.clone()
                },
                named: !stat.actor_name.trim().is_empty(),
                class: PlayerClass::parse(&stat.actor_class),
                colour: colours.get(&stat.actor_id).copied(),
                you: record.combat_infos.main_actor_id == Some(stat.actor_id),
                damage: stat.total_damage,
                dps: stat.dps,
                share: stat.damage_share,
            })
            .collect();
        players.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.actor_id.cmp(&b.actor_id)));
        players
    }
}

/// Each player's shade within a saved fight. The meter's own shades are gone
/// by the time a record is opened, so a record shades its players afresh,
/// most damage first: the record's list and its detail window agree.
pub fn record_colours(record: &HistoryRecord) -> HashMap<u32, u32> {
    let mut stats: Vec<_> = record.player_stats.values().collect();
    stats.sort_by(|a, b| b.total_damage.cmp(&a.total_damage).then(a.actor_id.cmp(&b.actor_id)));
    let mut book = ShadeBook::default();
    stats
        .into_iter()
        .filter_map(|stat| {
            let class = PlayerClass::parse(&stat.actor_class)?;
            Some((stat.actor_id, book.colour(class, stat.actor_id, stat.actor_name.trim())))
        })
        .collect()
}

fn main_player_label(row: &RecordRow) -> Cow<'_, str> {
    row.main_player.as_deref().map_or_else(|| Cow::Owned(unknown_player()), Cow::Borrowed)
}

/// Each distinct value with its record count, most frequent first.
fn options<S: AsRef<str>>(values: impl Iterator<Item = S>) -> Vec<FilterOption> {
    let mut counts: Vec<FilterOption> = Vec::new();
    for value in values.filter(|value| !value.as_ref().is_empty()) {
        let value = value.as_ref();
        match counts.iter_mut().find(|option| option.value == value) {
            Some(option) => option.count += 1,
            None => counts.push(FilterOption { value: value.to_string(), count: 1 }),
        }
    }
    counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.to_lowercase().cmp(&b.value.to_lowercase())));
    counts
}

impl RecordRow {
    fn from_record(record: &HistoryRecord) -> Self {
        let named = |name: &str| (!name.trim().is_empty()).then(|| name.trim().to_string());
        let target = record
            .target_info
            .as_ref()
            .or_else(|| record.combat_infos.target_infos.get(&record.target_id))
            .map_or_else(|| fl!("common-target-numbered", id = record.target_id), target_name);
        let mut named_players: Vec<_> = record.player_stats.values().filter(|stat| named(&stat.actor_name).is_some()).collect();
        named_players.sort_by(|a, b| b.total_damage.cmp(&a.total_damage));
        let main_player = record
            .combat_infos
            .main_actor_name
            .as_deref()
            .and_then(named)
            .or_else(|| {
                let main = record.combat_infos.main_actor_id?;
                named(&record.player_stats.get(&main)?.actor_name)
            })
            .or_else(|| {
                let main = record.combat_infos.main_actor_id?;
                record.player_stats.contains_key(&main).then(|| format::unnamed_player(main, true))
            })
            .or_else(|| named(&named_players.first()?.actor_name));
        Self {
            id: record.id.clone(),
            target,
            created_at: record.created_at,
            when: when::record_time(record.created_at),
            main_player,
            player_count: named_players.len(),
            damage: record.total_damage,
            duration_secs: duration_secs(record),
            is_boss: record.target_info.as_ref().is_some_and(|target| target.is_boss),
            wiped: record.wiped,
            hp_left: record.wiped.then(|| hp_left(record)).flatten(),
        }
    }
}

fn hp_left(record: &HistoryRecord) -> Option<f32> {
    let target = record.target_info.as_ref()?;
    let max = target.max_hp.filter(|&max| max > 0)?;
    Some((target.current_hp.unwrap_or(0) as f32 / max as f32).clamp(0.0, 1.0))
}

fn duration_secs(record: &HistoryRecord) -> f64 {
    let Some(target) = &record.target_info else {
        return 0.0;
    };
    let start = target.target_start_time.values().copied().fold(f64::INFINITY, f64::min);
    let end = target.target_last_time.values().copied().fold(f64::NEG_INFINITY, f64::max);
    if start.is_finite() && end.is_finite() {
        (end - start).max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use meter_core::models::combat::{CombatInfos, PlayerOverviewStat, TargetInfo};

    use super::*;

    fn player(id: u32, name: &str, damage: u64) -> PlayerOverviewStat {
        PlayerOverviewStat {
            actor_id: id,
            actor_name: name.to_string(),
            actor_server_id: "1001".to_string(),
            actor_class: "Ranger".to_string(),
            combat_power: None,
            counts: 10,
            total_damage: damage,
            min_damage: 1,
            max_damage: 9,
            special_counts: HashMap::new(),
            dps: damage as f64 / 10.0,
            damage_share: 0.5,
            damage_contribution: 0.5,
        }
    }

    fn record(id: &str, at: u64, target: &str, main: Option<&str>, players: &[(u32, &str, u64)]) -> HistoryRecord {
        let target_info = TargetInfo {
            id: 5,
            target_mob_code: Some(1),
            target_name: Some(target.to_string()),
            is_boss: true,
            current_hp: None,
            max_hp: None,
            target_start_time: HashMap::from([(1, 100.0), (2, 104.0)]),
            target_last_time: HashMap::from([(1, 150.0), (2, 190.0)]),
        };
        HistoryRecord {
            id: id.to_string(),
            target_id: 5,
            total_damage: players.iter().map(|player| player.2).sum(),
            target_info: Some(target_info),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: None,
                main_actor_name: main.map(str::to_string),
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 0.0,
            },
            player_skill_stats: HashMap::new(),
            player_stats: players.iter().map(|&(id, name, damage)| (id, player(id, name, damage))).collect(),
            use_buffs_by_target: HashMap::new(),
            ledger: None,
            wiped: false,
            created_at: at,
        }
    }

    fn list() -> HistoryList {
        let mut list = HistoryList::default();
        list.set_records(vec![
            record("old", 1_000, "Orsikh", Some("Mirelle"), &[(1, "Mirelle", 500), (2, "", 300)]),
            record("new", 3_000, "Colossus", Some("Mirelle"), &[(1, "Mirelle", 900), (2, "Vaelith", 100)]),
            record("mid", 2_000, "Orsikh", Some("Vaelith"), &[(2, "Vaelith", 700)]),
        ]);
        list
    }

    fn ids(list: &HistoryList) -> Vec<&str> {
        list.visible().iter().map(|row| row.id.as_str()).collect()
    }

    #[test]
    fn records_come_newest_first() {
        assert_eq!(ids(&list()), vec!["new", "mid", "old"]);
    }

    #[test]
    fn a_row_carries_what_the_list_shows() {
        let list = list();
        let row = list.visible()[0];

        assert_eq!(row.target, "Colossus");
        assert_eq!(row.main_player.as_deref(), Some("Mirelle"));
        assert_eq!(row.player_count, 2);
        assert_eq!(row.damage, 1_000);
        assert_eq!(row.duration_secs, 90.0);
    }

    #[test]
    fn players_without_a_name_are_not_counted() {
        let list = list();

        assert_eq!(list.visible()[2].player_count, 1);
    }

    #[test]
    fn filters_narrow_by_target_and_by_main_player() {
        let mut list = list();

        list.filter_target(Some("Orsikh".to_string()));
        assert_eq!(ids(&list), vec!["mid", "old"]);

        list.filter_player(Some("Mirelle".to_string()));
        assert_eq!(ids(&list), vec!["old"]);
        assert_eq!(list.count_label(), "1 / 3 records");
    }

    #[test]
    fn options_count_records_most_frequent_first() {
        let list = list();

        assert_eq!(
            list.target_options(),
            vec![
                FilterOption { value: "Orsikh".to_string(), count: 2 },
                FilterOption { value: "Colossus".to_string(), count: 1 }
            ]
        );
        assert_eq!(list.player_options()[0], FilterOption { value: "Mirelle".to_string(), count: 2 });
    }

    #[test]
    fn a_record_without_a_main_player_is_listed_under_unknown() {
        let mut list = HistoryList::default();
        list.set_records(vec![record("a", 1, "Orsikh", None, &[(1, "", 100)])]);

        assert_eq!(list.visible()[0].main_player, None);
        assert_eq!(list.player_options(), vec![FilterOption { value: "Unknown".to_string(), count: 1 }]);
        list.filter_player(Some("Unknown".to_string()));
        assert_eq!(ids(&list), vec!["a"]);
    }

    #[test]
    fn the_main_player_falls_back_to_the_strongest_named_player() {
        let mut list = HistoryList::default();
        list.set_records(vec![record("a", 1, "Orsikh", None, &[(1, "Mirelle", 100), (2, "Vaelith", 300)])]);

        assert_eq!(list.visible()[0].main_player.as_deref(), Some("Vaelith"));
    }

    #[test]
    fn a_deleted_record_leaves_the_list_and_a_filter_nobody_has_falls_back_to_all() {
        let mut list = list();
        list.filter_target(Some("Colossus".to_string()));
        assert_eq!(ids(&list), vec!["new"]);

        list.set_records(vec![
            record("old", 1_000, "Orsikh", Some("Mirelle"), &[(1, "Mirelle", 500)]),
            record("mid", 2_000, "Orsikh", Some("Vaelith"), &[(2, "Vaelith", 700)]),
        ]);

        assert_eq!(list.target_filter(), None);
        assert_eq!(ids(&list), vec!["mid", "old"]);
        assert_eq!(list.count_label(), "2 records");
    }

    #[test]
    fn deleting_everything_empties_the_list() {
        let mut list = list();
        list.set_records(Vec::new());

        assert!(ids(&list).is_empty());
        assert_eq!(list.total(), 0);
        assert!(list.target_options().is_empty());
    }

    #[test]
    fn a_records_players_are_sorted_by_damage_with_ids_for_the_unnamed() {
        let list = list();
        let players = list.players("old", false);

        assert_eq!(players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>(), vec!["Mirelle", "ID:2"]);
        assert_eq!(players[0].class, Some(PlayerClass::Ranger));
        assert!(list.players("missing", false).is_empty());
    }

    #[test]
    fn hiding_unknown_players_leaves_the_unnamed_out_of_an_opened_record_but_not_the_record() {
        let list = list();

        let players = list.players("old", true);

        assert_eq!(players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>(), vec!["Mirelle"]);
        assert_eq!(players[0].share, 0.5, "shares stay as saved");
        assert_eq!(list.record("old").unwrap().player_stats.len(), 2, "the record keeps everyone");
        assert_eq!(list.players("old", false).len(), 2, "switching it off lists them again");
    }

    #[test]
    fn your_own_unnamed_player_is_you_in_the_record_and_its_list() {
        let mut own = record("own", 50, "Boss", None, &[(1, "Mirelle", 500), (2, "", 300)]);
        own.combat_infos.main_actor_id = Some(2);
        let mut list = HistoryList::default();
        list.set_records(vec![own]);

        let players = list.players("own", true);

        assert_eq!(players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>(), vec!["Mirelle", "You"]);
        assert!(players[1].you && !players[1].named);
        assert_eq!(list.visible()[0].main_player.as_deref(), Some("You"));
    }

    #[test]
    fn a_wiped_record_says_so_and_carries_the_health_its_boss_had_left() {
        let mut wiped = record("wipe", 5, "Orsikh", Some("Mirelle"), &[(1, "Mirelle", 500)]);
        wiped.wiped = true;
        if let Some(target) = wiped.target_info.as_mut() {
            target.current_hp = Some(300);
            target.max_hp = Some(1_000);
        }
        let mut list = list();
        let mut records: Vec<HistoryRecord> = ["old", "new", "mid"].iter().filter_map(|id| list.record(id).map(|record| (**record).clone())).collect();
        records.push(wiped);
        list.set_records(records);

        let rows = list.visible();
        let row = |id: &str| rows.iter().find(|row| row.id == id).copied().unwrap();
        assert!(row("wipe").wiped);
        assert_eq!(row("wipe").hp_left, Some(0.3));
        assert!(!row("new").wiped);
        assert_eq!(row("new").hp_left, None);
    }

    #[test]
    fn a_record_is_found_by_its_id() {
        let list = list();

        assert_eq!(list.record("mid").unwrap().total_damage, 700);
        assert!(list.record("missing").is_none());
    }
}
