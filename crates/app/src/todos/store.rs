//! `todos.json` in the app's data folder, beside `settings.json`. A missing file
//! is a fresh start; a file that is not JSON, not an object, or written by a
//! newer build is renamed to `todos.json.bad-<seconds>` and the tracker starts
//! empty. Inside a readable file a bad entry costs that entry only (see
//! `data.rs`).
//!
//! Kept apart from `settings.json` because counts change all day while
//! settings rarely do, and a bad to-do file must never take the settings with it.

use std::path::{Path, PathBuf};

use super::data::TodoData;
use crate::stored_file::{self, LoadNotice, Problem, Read, SaveError};

pub const FILE_NAME: &str = "todos.json";
pub const SCHEMA_VERSION: u32 = 1;

pub struct Loaded {
    pub data: TodoData,
    /// Why the stored file was not used.
    pub notice: Option<LoadNotice>,
}

pub fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load(path: &Path, now_secs: u64) -> Loaded {
    match stored_file::read(path, now_secs, parse) {
        Read::Used(data) => Loaded { data, notice: None },
        Read::Missing => Loaded { data: TodoData::default(), notice: None },
        Read::Unusable(notice) => Loaded { data: TodoData::default(), notice: Some(notice) },
    }
}

fn parse(bytes: &[u8]) -> Result<TodoData, Problem> {
    let (stored, _) = stored_file::parse_versioned_object(bytes, SCHEMA_VERSION)?;
    serde_json::from_value::<TodoData>(stored)
        .map(TodoData::sanitised)
        .map_err(|error| Problem::WrongShape(error.to_string()))
}

pub fn save(path: &Path, data: &TodoData) -> Result<(), SaveError> {
    stored_file::save_versioned(path, data, SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use meter_core::capture::parser::ticket_record::TicketRecord;
    use meter_core::identity::KnownCharacter;
    use meter_core::tickets::{TicketContent, TicketReport};

    use super::*;
    use crate::todos::data::{Cadence, NewOwnTodo, OwnKind, RowId};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("aion-todos-{name}-{nanos}"));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            path_in(&self.0)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample() -> TodoData {
        let mut data = TodoData::default();
        data.sync_characters(&[KnownCharacter {
            name: "Alder".to_string(),
            server_id: 1001,
            class: "RANGER".to_string(),
            key: None,
        }]);
        let id = data.characters()[0].id;
        data.adjust_count(id, RowId::Builtin("trial"), 2);
        data.add_own(id, NewOwnTodo { name: "Mine".into(), kind: OwnKind::Count(4), cadence: Cadence::Weekly }).unwrap();
        data
    }

    #[test]
    fn a_missing_file_is_a_fresh_start() {
        let dir = TempDir::new("missing");
        let loaded = load(&dir.file(), 0);
        assert_eq!(loaded.data, TodoData::default());
        assert_eq!(loaded.notice, None);
    }

    #[test]
    fn saved_to_dos_read_back_the_same_with_a_version() {
        let dir = TempDir::new("round-trip");
        let data = sample();
        save(&dir.file(), &data).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(text.contains("\"version\": 1"), "{text}");
        assert_eq!(load(&dir.file(), 0).data, data);
    }

    #[test]
    fn a_file_that_is_not_json_is_set_aside() {
        let dir = TempDir::new("garbage");
        fs::write(dir.file(), b"{ not json").unwrap();
        let loaded = load(&dir.file(), 77);
        assert_eq!(loaded.data, TodoData::default());
        assert!(matches!(&loaded.notice, Some(LoadNotice::SetAside { aside, .. }) if aside.ends_with("todos.json.bad-77")), "{:?}", loaded.notice);
        assert!(dir.0.join("todos.json.bad-77").exists());
        assert!(!dir.file().exists());
    }

    #[test]
    fn a_file_from_a_newer_build_is_set_aside_untouched() {
        let dir = TempDir::new("newer");
        fs::write(dir.file(), br#"{"version": 99, "characters": []}"#).unwrap();
        let loaded = load(&dir.file(), 5);
        assert!(matches!(loaded.notice, Some(LoadNotice::SetAside { problem: Problem::Newer { version: 99, supported: 1 }, .. })));
        assert_eq!(fs::read(dir.0.join("todos.json.bad-5")).unwrap(), br#"{"version": 99, "characters": []}"#);
    }

    #[test]
    fn a_bad_entry_costs_that_entry_and_not_the_file() {
        let dir = TempDir::new("tolerant");
        fs::write(
            dir.file(),
            br#"{"version":1,"characters":[{"id":"x"},{"id":2,"name":"Birch","serverId":1001,"progress":{"trial":2}}],"folded":["today"]}"#,
        )
        .unwrap();
        let loaded = load(&dir.file(), 0);
        assert_eq!(loaded.notice, None);
        assert_eq!(loaded.data.characters().len(), 1);
        assert_eq!(loaded.data.characters()[0].name, "Birch");
        assert!(loaded.data.is_folded(crate::todos::catalogue::Section::Today));
        assert!(dir.file().exists(), "the file is not moved aside");
    }

    #[test]
    fn a_file_written_before_the_game_filled_the_rows_loads_with_its_typed_amounts() {
        let dir = TempDir::new("before-tickets");
        fs::write(
            dir.file(),
            br#"{"version":1,"dailyResetMs":5,"characters":[{"id":1,"name":"Alder","serverId":1001,
                "amounts":{"odyle":{"value":425,"atMs":10},"conquest":{"value":9,"atMs":10},"shugo":{"value":28,"atMs":10}},
                "progress":{"fissure":4}}],"dutyMissions":{"1001":2}}"#,
        )
        .unwrap();
        let loaded = load(&dir.file(), 0);
        assert_eq!(loaded.notice, None);
        let alder = &loaded.data.characters()[0];
        assert_eq!(alder.typed_amount("odyle").map(|typed| typed.value), Some(425));
        assert_eq!(alder.typed_amount("shugo").map(|typed| typed.value), Some(28), "a typed amount above the cap is kept");
        assert_eq!(alder.typed_amount("conquest"), None, "a row the catalogue no longer has costs that entry only");
        assert_eq!(alder.logged_in_ms(), None);
        assert_eq!(loaded.data.duty_missions_done(1001), 2);
    }

    #[test]
    fn rows_removed_from_the_catalogue_are_dropped_on_load_and_not_saved_back() {
        let dir = TempDir::new("removed-rows");
        fs::write(
            dir.file(),
            br#"{"version":1,"openGroups":["conquest","sanct","explore"],"characters":[{"id":1,"name":"Alder","serverId":1001,
                "amounts":{"odyle":{"value":425,"atMs":10},"c-krao":{"value":9,"atMs":10},"deus":{"value":3,"atMs":10},"arkanis":{"value":2,"atMs":10},"balaur":{"value":5,"atMs":10},"fafnium":{"value":5,"atMs":10}},
                "progress":{"trial":2,"subj":1,"transc":12,"conqk":3,"cradle":1,"dramata":1,"ordeal":2,"forge":3,"forgeboss":1,"corr":1,"corrboss":1,"chal":1,"chalboss":1,"x-krao":4,"x-azure":2},
                "hidden":["trial","c-horn","deus","conquest","balaur","fafnium","subj","sanct","explore","x-horn","forgeboss","ordeal"],
                "tickets":{"60000001":{"count":5,"charge":0,"atMs":10},"60000101":{"count":14,"charge":0,"atMs":10},
                           "60005301":{"count":4,"charge":0,"atMs":10},"60006301":{"count":4,"charge":0,"atMs":10},
                           "102":{"count":7,"charge":0,"atMs":10},"103":{"count":7,"charge":0,"atMs":10},
                           "60000008":{"count":5,"charge":0,"atMs":10},"60000007":{"count":5,"charge":0,"atMs":10},
                           "60012201":{"count":5,"charge":0,"atMs":10},"60004201":{"count":5,"charge":0,"atMs":10},
                           "60015101":{"count":5,"charge":0,"atMs":10},"70000001":{"count":2,"charge":0,"atMs":10},
                           "90000001":{"count":2,"charge":0,"atMs":10},"90000006":{"count":2,"charge":0,"atMs":10},
                           "60014101":{"count":2,"charge":0,"atMs":10},"60007101":{"count":2,"charge":0,"atMs":10}}}]}"#,
        )
        .unwrap();
        let loaded = load(&dir.file(), 0);
        assert_eq!(loaded.notice, None);
        let alder = &loaded.data.characters()[0];
        assert_eq!(alder.typed_amount("odyle").map(|typed| typed.value), Some(425));
        assert_eq!(alder.progress.iter().map(|(id, count)| (id.as_str(), *count)).collect::<Vec<_>>(), [("trial", 2)]);
        assert!(alder.is_hidden("trial") && !alder.is_hidden("subj") && !alder.is_hidden("sanct") && !alder.is_hidden("balaur"));

        save(&dir.file(), &loaded.data).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        let removed_ids = [
            "c-krao", "c-horn", "deus", "arkanis", "balaur", "fafnium", "conquest", "subj", "transc", "conqk", "cradle", "dramata",
            "ordeal", "forge", "corr", "chal", "sanct", "explore", "x-krao", "x-azure", "x-horn", "openGroups",
        ];
        let removed_tickets = [
            "60000101", "60005301", "60006301", "\"102\"", "\"103\"", "60000008", "60000007", "60012201", "60004201", "60015101",
            "70000001", "90000001", "90000006", "60014101", "60007101",
        ];
        for removed in removed_ids.into_iter().chain(removed_tickets) {
            assert!(!text.contains(removed), "{removed} was written back: {text}");
        }
        assert!(text.contains("60000001") && text.contains("\"trial\""), "{text}");
    }

    #[test]
    fn the_weekly_count_of_the_daily_dungeon_in_an_older_file_is_dropped_and_not_saved_back() {
        let dir = TempDir::new("daily-dungeon-count");
        fs::write(
            dir.file(),
            br#"{"version":1,"characters":[{"id":1,"name":"Alder","serverId":1001,"progress":{"fissure":4,"trial":1}}]}"#,
        )
        .unwrap();
        let loaded = load(&dir.file(), 0);
        assert_eq!(loaded.notice, None);
        let alder = &loaded.data.characters()[0];
        assert_eq!(alder.progress.keys().collect::<Vec<_>>(), ["trial"]);
        assert_eq!(alder.typed_amount("fissure"), None);

        save(&dir.file(), &loaded.data).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(!text.contains("fissure") && text.contains("trial"), "{text}");
    }

    #[test]
    fn resources_hidden_in_an_older_file_are_dropped_on_load_and_not_saved_back() {
        let dir = TempDir::new("hidden-resources");
        fs::write(
            dir.file(),
            br#"{"version":1,"folded":["recharges","thisWeek"],"characters":[{"id":1,"name":"Alder","serverId":1001,
                "hidden":["odyle","shugo","invasion","nightmare","fissure","trial"]}]}"#,
        )
        .unwrap();
        let loaded = load(&dir.file(), 0);
        assert_eq!(loaded.notice, None);
        let alder = &loaded.data.characters()[0];
        assert_eq!(alder.hidden.iter().collect::<Vec<_>>(), ["trial"], "only to-dos stay hidden");
        assert!(loaded.data.is_folded(crate::todos::catalogue::Section::ThisWeek));

        save(&dir.file(), &loaded.data).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        for removed in ["odyle", "shugo", "invasion", "nightmare", "fissure", "recharges"] {
            assert!(!text.contains(removed), "{removed} was written back: {text}");
        }
        assert!(text.contains("\"trial\"") && text.contains("thisWeek"), "{text}");
    }

    #[test]
    fn what_the_game_said_is_saved_and_read_back() {
        let dir = TempDir::new("tickets");
        let mut data = sample();
        let who = KnownCharacter { name: "Alder".to_string(), server_id: 1001, class: "RANGER".to_string(), key: None };
        let records = [(60_000_001, 125, 245), (8, 2, 0)]
            .map(|(id, count, charge)| TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge });
        data.apply_tickets(&TicketReport { character: who, captured_at: 1_791_000_000.5, content: TicketContent::Snapshot(records.to_vec()) });
        save(&dir.file(), &data).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(text.contains("\"serverTickets\"") && text.contains("\"loggedInMs\": 1791000000500"), "{text}");
        assert_eq!(load(&dir.file(), 0).data, data);
    }
}
