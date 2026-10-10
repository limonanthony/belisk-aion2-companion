//! `pets.json` in the app's data folder, beside `settings.json` and
//! `todos.json`. A missing file is a fresh start; a file that is not JSON, not
//! an object, or written by a newer build is renamed to `pets.json.bad-<seconds>`
//! and the collection starts empty until the game sends it again at login.
//!
//! Kept apart from the other files because a soul gain changes it all day and a
//! bad pet file must never take the settings or the to-dos with it.

use std::path::{Path, PathBuf};

use super::data::PetData;
use crate::stored_file::{self, LoadNotice, Problem, Read, SaveError};

pub const FILE_NAME: &str = "pets.json";
pub const SCHEMA_VERSION: u32 = 1;

pub struct Loaded {
    pub data: PetData,
    /// Why the stored file was not used.
    pub notice: Option<LoadNotice>,
}

pub fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load(path: &Path, now_secs: u64) -> Loaded {
    match stored_file::read(path, now_secs, parse) {
        Read::Used(data) => Loaded { data, notice: None },
        Read::Missing => Loaded { data: PetData::default(), notice: None },
        Read::Unusable(notice) => Loaded { data: PetData::default(), notice: Some(notice) },
    }
}

fn parse(bytes: &[u8]) -> Result<PetData, Problem> {
    let (stored, _) = stored_file::parse_versioned_object(bytes, SCHEMA_VERSION)?;
    serde_json::from_value(stored).map_err(|error| Problem::WrongShape(error.to_string()))
}

pub fn save(path: &Path, data: &PetData) -> Result<(), SaveError> {
    stored_file::save_versioned(path, data, SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use meter_core::pets::PetState;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("aion-pets-{name}-{}-{nanos}", std::process::id()));
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

    fn sample() -> PetData {
        let mut data = PetData::default();
        data.current_server = Some(1302);
        let server = data.server_mut(1302);
        server.snapshot_at = Some(1_791_634_680.352);
        server.pets.insert(1086, PetState { level: 2, souls: 27 });
        server.pets.insert(1214, PetState { level: 3, souls: 0 });
        data.server_mut(1001).pets.insert(1010, PetState { level: 1, souls: 3 });
        data.pin(1302, 1214);
        data.pin(1302, 1086);
        data.pin(1001, 1010);
        data
    }

    #[test]
    fn a_missing_file_is_a_quiet_fresh_start() {
        let dir = TempDir::new("missing");

        let loaded = load(&dir.file(), 7);

        assert_eq!(loaded.data, PetData::default());
        assert!(loaded.notice.is_none());
    }

    #[test]
    fn what_was_saved_is_what_loads() {
        let dir = TempDir::new("round-trip");

        save(&dir.file(), &sample()).unwrap();
        let loaded = load(&dir.file(), 7);

        assert_eq!(loaded.data, sample());
        assert!(loaded.notice.is_none());
        let written = fs::read_to_string(dir.file()).unwrap();
        assert!(written.contains("\"version\": 1"), "{written}");
    }

    #[test]
    fn a_file_that_is_not_json_is_set_aside_and_the_collection_starts_empty() {
        let dir = TempDir::new("not-json");
        fs::write(dir.file(), "not json {").unwrap();

        let loaded = load(&dir.file(), 42);

        assert_eq!(loaded.data, PetData::default());
        assert!(matches!(&loaded.notice, Some(LoadNotice::SetAside { problem: Problem::NotJson(_), aside, .. }) if aside.ends_with("pets.json.bad-42")));
        assert!(!dir.file().exists());
        assert_eq!(fs::read_to_string(dir.0.join("pets.json.bad-42")).unwrap(), "not json {");
    }

    #[test]
    fn a_file_that_is_not_an_object_or_has_the_wrong_shape_is_set_aside() {
        let dir = TempDir::new("shape");
        fs::write(dir.file(), "[1, 2]").unwrap();
        assert!(matches!(load(&dir.file(), 1).notice, Some(LoadNotice::SetAside { problem: Problem::NotObject, .. })));

        fs::write(dir.file(), r#"{ "version": 1, "servers": { "1302": { "pets": { "1086": "high" } } } }"#).unwrap();
        let loaded = load(&dir.file(), 2);
        assert!(matches!(loaded.notice, Some(LoadNotice::SetAside { problem: Problem::WrongShape(_), .. })));
        assert_eq!(loaded.data, PetData::default());
    }

    #[test]
    fn a_file_from_a_newer_build_is_set_aside_untouched() {
        let dir = TempDir::new("newer");
        let newer = r#"{"version": 99, "servers": {}}"#;
        fs::write(dir.file(), newer).unwrap();

        let loaded = load(&dir.file(), 5);

        assert!(matches!(loaded.notice, Some(LoadNotice::SetAside { problem: Problem::Newer { version: 99, supported: 1 }, .. })));
        assert_eq!(fs::read_to_string(dir.0.join("pets.json.bad-5")).unwrap(), newer);
    }

    #[test]
    fn a_file_with_a_field_missing_takes_the_defaults() {
        let dir = TempDir::new("partial");
        fs::write(dir.file(), r#"{ "version": 1 }"#).unwrap();

        let loaded = load(&dir.file(), 1);

        assert_eq!(loaded.data, PetData::default());
        assert!(loaded.notice.is_none());
    }

    #[test]
    fn pins_survive_a_save_and_load_in_the_order_they_were_made_per_server() {
        let dir = TempDir::new("pins");

        save(&dir.file(), &sample()).unwrap();
        let loaded = load(&dir.file(), 7).data;

        assert_eq!(loaded.pinned(1302), [1214, 1086]);
        assert_eq!(loaded.pinned(1001), [1010]);
        assert_eq!(SCHEMA_VERSION, 1, "pins sit beside the servers and need no new version");
    }

    #[test]
    fn a_file_written_before_pins_existed_loads_with_none_and_keeps_its_collection() {
        let dir = TempDir::new("before-pins");
        fs::write(
            dir.file(),
            r#"{ "version": 1, "currentServer": 1302, "servers": { "1302": { "snapshotAt": 12.5, "pets": { "1086": { "level": 2, "souls": 27 } } } } }"#,
        )
        .unwrap();

        let loaded = load(&dir.file(), 1);

        assert!(loaded.notice.is_none());
        assert!(loaded.data.pinned.is_empty());
        assert_eq!(loaded.data.servers[&1302].pet(1086), PetState { level: 2, souls: 27 });
    }
}
