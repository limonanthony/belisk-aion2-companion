//! `settings.json` in the app's data folder. A missing file is a fresh start;
//! a file that cannot be read as settings, or that a newer build wrote, is
//! renamed to `settings.json.bad-<seconds>` and the defaults are used. Fields
//! missing from the file take their defaults and unknown fields are ignored.

use std::path::{Path, PathBuf};

use super::model::AppSettings;
use crate::stored_file::{self, LoadNotice, Problem, Read, SaveError};

pub const FILE_NAME: &str = "settings.json";
pub const SCHEMA_VERSION: u32 = 2;

pub struct Loaded {
    pub settings: AppSettings,
    /// Why the stored file was not used, for the settings page to show.
    pub notice: Option<LoadNotice>,
    /// There was no file at all: the app has never run here.
    pub fresh_install: bool,
}

pub fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load(path: &Path, now_secs: u64) -> Loaded {
    match stored_file::read(path, now_secs, parse) {
        Read::Used(settings) => Loaded { settings: settings.clamped(), notice: None, fresh_install: false },
        Read::Missing => fresh(None, true),
        Read::Unusable(notice) => fresh(Some(notice), false),
    }
}

fn fresh(notice: Option<LoadNotice>, fresh_install: bool) -> Loaded {
    Loaded { settings: AppSettings::default(), notice, fresh_install }
}

fn parse(bytes: &[u8]) -> Result<AppSettings, Problem> {
    let (mut stored, version) = stored_file::parse_versioned_object(bytes, SCHEMA_VERSION)?;
    if version < 2 {
        forget_the_old_default_reset_hour(&mut stored);
    }
    Ok(keep_valid_fields(&stored))
}

/// Before version 2 the to-do reset hour defaulted to 09:00 and was saved as
/// such, so a stored 9 was the default, not a choice: it goes back to
/// following the game's grid. A 9 picked since is saved as version 2 and stays.
fn forget_the_old_default_reset_hour(stored: &mut serde_json::Value) {
    let Some(todos) = stored.get_mut("todos").and_then(serde_json::Value::as_object_mut) else {
        return;
    };
    if todos.get("resetHour").and_then(serde_json::Value::as_u64) == Some(9) {
        todos.remove("resetHour");
    }
}

/// Starts from the defaults and takes each stored value that is a valid
/// setting, so one unknown or mistyped value (an option name from another
/// build, say) costs that setting only.
fn keep_valid_fields(stored: &serde_json::Value) -> AppSettings {
    let mut leaves = Vec::new();
    collect_leaves(stored, &mut Vec::new(), &mut leaves);
    let mut merged = serde_json::to_value(AppSettings::default()).unwrap_or_default();
    for (path, leaf) in leaves {
        let mut trial = merged.clone();
        if set_path(&mut trial, &path, leaf) && serde_json::from_value::<AppSettings>(trial.clone()).is_ok() {
            merged = trial;
        }
    }
    serde_json::from_value(merged).unwrap_or_default()
}

fn collect_leaves(
    value: &serde_json::Value,
    path: &mut Vec<String>,
    leaves: &mut Vec<(Vec<String>, serde_json::Value)>,
) {
    match value.as_object() {
        Some(object) => {
            for (key, inner) in object {
                path.push(key.clone());
                collect_leaves(inner, path, leaves);
                path.pop();
            }
        }
        None => leaves.push((path.clone(), value.clone())),
    }
}

fn set_path(root: &mut serde_json::Value, path: &[String], leaf: serde_json::Value) -> bool {
    let Some((last, parents)) = path.split_last() else {
        return false;
    };
    let mut node = root;
    for key in parents {
        match node.get_mut(key) {
            Some(next) if next.is_object() => node = next,
            _ => return false,
        }
    }
    match node.as_object_mut() {
        Some(object) => {
            object.insert(last.clone(), leaf);
            true
        }
        None => false,
    }
}

pub fn save(path: &Path, settings: &AppSettings) -> Result<(), SaveError> {
    stored_file::save_versioned(path, settings, SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use meter_core::config::CaptureBackendPriority;
    use meter_core::identity::KnownCharacter;

    use super::*;
    use crate::meter::{Layout, PctMode};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("aion-settings-{name}-{}-{nanos}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            path_in(&self.0)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_missing_file_is_a_quiet_fresh_start() {
        let dir = TestDir::new("missing");
        let loaded = load(&dir.file(), 7);
        assert_eq!(loaded.settings, AppSettings::default());
        assert!(loaded.notice.is_none());
        assert!(loaded.fresh_install);
    }

    #[test]
    fn an_existing_or_set_aside_file_is_not_a_fresh_install() {
        let dir = TestDir::new("not-fresh");
        fs::write(dir.file(), r#"{ "version": 2 }"#).unwrap();
        assert!(!load(&dir.file(), 7).fresh_install);
        fs::write(dir.file(), "not json").unwrap();
        let loaded = load(&dir.file(), 7);
        assert!(loaded.notice.is_some() && !loaded.fresh_install);
    }

    #[test]
    fn saved_settings_come_back_unchanged() {
        let dir = TestDir::new("roundtrip");
        let mut settings = AppSettings::default();
        settings.overlay.layout = Layout::Capsule;
        settings.overlay.pct_mode = PctMode::Share;
        settings.overlay.background_alpha = 33;
        settings.overlay.players_shown = 14;
        settings.behaviour.end_fight_after_secs = 0;
        settings.capture.backend_priority = CaptureBackendPriority::WinDivertFirst;
        settings.capture.collect_mob_data = true;
        settings.capture.record_every_session = true;

        save(&dir.file(), &settings).unwrap();

        assert_eq!(load(&dir.file(), 7).settings, settings);
    }

    #[test]
    fn remembered_characters_come_back_in_order() {
        let dir = TestDir::new("characters");
        let mut settings = AppSettings::default();
        settings.known_characters = vec![
            KnownCharacter { name: "Alder".into(), server_id: 1001, class: "RANGER".into(), key: Some("3AA503000000".into()) },
            KnownCharacter { name: "Birch".into(), server_id: 2002, class: "CLERIC".into(), key: None },
        ];

        save(&dir.file(), &settings).unwrap();

        assert_eq!(load(&dir.file(), 7).settings.known_characters, settings.known_characters);
    }

    #[test]
    fn a_bad_remembered_character_costs_that_entry_only() {
        let dir = TestDir::new("characters-bad");
        fs::write(
            dir.file(),
            r#"{"version":1,"overlay":{"showDps":false},"knownCharacters":[
                {"name":"Alder","serverId":1001,"class":"RANGER"},
                {"name":"Broken","serverId":"x","class":"RANGER"},
                7]}"#,
        )
        .unwrap();

        let loaded = load(&dir.file(), 7);

        assert!(loaded.notice.is_none());
        assert!(!loaded.settings.overlay.show_dps);
        let names: Vec<_> = loaded.settings.known_characters.iter().map(|known| known.name.as_str()).collect();
        assert_eq!(names, ["Alder"]);
    }

    #[test]
    fn remembered_characters_that_are_not_a_list_are_dropped_with_the_rest_kept() {
        let dir = TestDir::new("characters-type");
        fs::write(dir.file(), r#"{"version":1,"overlay":{"showDps":false},"knownCharacters":"oops"}"#).unwrap();

        let loaded = load(&dir.file(), 7);

        assert!(loaded.settings.known_characters.is_empty());
        assert!(!loaded.settings.overlay.show_dps);
    }

    #[test]
    fn missing_fields_take_defaults_and_unknown_fields_are_ignored() {
        let dir = TestDir::new("partial");
        fs::write(
            dir.file(),
            r#"{"version":1,"overlay":{"showDps":false,"fromTheFuture":1},"extraSection":{"a":1}}"#,
        )
        .unwrap();

        let loaded = load(&dir.file(), 7);

        assert!(loaded.notice.is_none());
        assert!(!loaded.settings.overlay.show_dps);
        let mut expected = AppSettings::default();
        expected.overlay.show_dps = false;
        assert_eq!(loaded.settings, expected);
    }

    #[test]
    fn automatic_download_is_on_without_the_key_and_a_stored_off_is_kept() {
        let dir = TestDir::new("updates-key");
        fs::write(dir.file(), r#"{"version":2,"overlay":{"showDps":false}}"#).unwrap();
        assert!(load(&dir.file(), 7).settings.updates.download_automatically);

        fs::write(dir.file(), r#"{"version":2,"updates":{"downloadAutomatically":false}}"#).unwrap();
        assert!(!load(&dir.file(), 7).settings.updates.download_automatically);
    }

    #[test]
    fn stored_values_outside_their_range_are_pulled_in() {
        let dir = TestDir::new("clamp");
        fs::write(
            dir.file(),
            r#"{"version":1,"overlay":{"contentScale":9.0,"playersShown":99},"behaviour":{"endFightAfterSecs":5}}"#,
        )
        .unwrap();

        let settings = load(&dir.file(), 7).settings;

        assert_eq!(settings.overlay.content_scale, 1.5);
        assert_eq!(settings.overlay.players_shown, 20);
        assert_eq!(settings.behaviour.end_fight_after_secs, 60);
    }

    #[test]
    fn an_unreadable_file_is_set_aside_not_overwritten() {
        let dir = TestDir::new("garbage");
        fs::write(dir.file(), "not json {").unwrap();

        let loaded = load(&dir.file(), 42);

        assert_eq!(loaded.settings, AppSettings::default());
        assert!(matches!(&loaded.notice, Some(LoadNotice::SetAside { problem: Problem::NotJson(_), aside, .. }) if aside.ends_with("settings.json.bad-42")));
        assert!(!dir.file().exists());
        assert_eq!(fs::read_to_string(dir.0.join("settings.json.bad-42")).unwrap(), "not json {");
    }

    #[test]
    fn a_file_from_a_newer_build_is_set_aside_too() {
        let dir = TestDir::new("newer");
        fs::write(dir.file(), r#"{"version":99,"overlay":{"showDps":false}}"#).unwrap();

        let loaded = load(&dir.file(), 5);

        assert_eq!(loaded.settings, AppSettings::default());
        assert!(matches!(loaded.notice, Some(LoadNotice::SetAside { problem: Problem::Newer { version: 99, supported: SCHEMA_VERSION }, .. })));
        assert!(dir.0.join("settings.json.bad-5").exists());
    }

    #[test]
    fn saving_replaces_the_file_without_leaving_the_temporary_one() {
        let dir = TestDir::new("atomic");
        save(&dir.file(), &AppSettings::default()).unwrap();
        let mut changed = AppSettings::default();
        changed.overlay.show_dps = false;

        save(&dir.file(), &changed).unwrap();

        assert_eq!(load(&dir.file(), 1).settings, changed);
        let names: Vec<_> = fs::read_dir(&dir.0).unwrap().map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("settings.json")]);
    }

    #[test]
    fn an_unknown_option_name_costs_that_setting_only() {
        let dir = TestDir::new("variant");
        fs::write(
            dir.file(),
            r#"{"version":1,"overlay":{"layout":"hologram","showDps":false},"capture":{"backendPriority":"carrierPigeon","collectMobData":true}}"#,
        )
        .unwrap();

        let loaded = load(&dir.file(), 7);

        assert!(loaded.notice.is_none());
        assert_eq!(loaded.settings.overlay.layout, Layout::Full);
        assert!(!loaded.settings.overlay.show_dps);
        assert_eq!(loaded.settings.capture.backend_priority, CaptureBackendPriority::NpcapFirst);
        assert!(loaded.settings.capture.collect_mob_data);
        assert!(dir.file().exists());
    }

    #[test]
    fn the_language_follows_windows_until_chosen_and_an_unknown_one_is_ignored() {
        use crate::i18n::LanguageChoice;
        let dir = TestDir::new("language");

        fs::write(dir.file(), r#"{"version":1,"overlay":{"showDps":false}}"#).unwrap();
        assert_eq!(load(&dir.file(), 7).settings.general.language, LanguageChoice::System);

        fs::write(dir.file(), r#"{"version":1,"general":{"language":"french"}}"#).unwrap();
        assert_eq!(load(&dir.file(), 7).settings.general.language, LanguageChoice::French);

        fs::write(dir.file(), r#"{"version":1,"general":{"language":"klingon"},"overlay":{"showDps":false}}"#).unwrap();
        let loaded = load(&dir.file(), 7);
        assert_eq!(loaded.settings.general.language, LanguageChoice::System);
        assert!(!loaded.settings.overlay.show_dps);
        assert!(loaded.notice.is_none());

        let mut settings = AppSettings::default();
        settings.general.language = LanguageChoice::English;
        save(&dir.file(), &settings).unwrap();
        assert_eq!(load(&dir.file(), 7).settings.general.language, LanguageChoice::English);
    }

    #[test]
    fn an_old_file_without_reminders_gets_the_reminder_defaults() {
        let dir = TestDir::new("no-reminders");
        fs::write(dir.file(), r#"{"version":1,"overlay":{"showDps":false}}"#).unwrap();

        let reminders = load(&dir.file(), 7).settings.reminders;

        assert_eq!(reminders, crate::reminders::ReminderSettings::default());
        assert!(reminders.enabled && reminders.sound && reminders.events.rift_portals);
        assert!(!reminders.events.watcher_kaira);
    }

    #[test]
    fn a_file_from_before_the_between_fights_switch_keeps_the_meter_on_screen_and_the_switch_comes_back() {
        let dir = TestDir::new("no-hide-between-fights");
        fs::write(dir.file(), r#"{"version":2,"windows":{"openMeterAtLaunch":true}}"#).unwrap();

        let mut settings = load(&dir.file(), 7).settings;
        assert!(settings.windows.open_meter_at_launch);
        assert!(!settings.windows.hide_meter_between_fights);

        settings.windows.hide_meter_between_fights = true;
        save(&dir.file(), &settings).unwrap();
        assert!(load(&dir.file(), 7).settings.windows.hide_meter_between_fights);
    }

    #[test]
    fn one_bad_reminder_value_costs_that_setting_only_and_the_lead_time_is_pulled_to_an_offered_one() {
        let dir = TestDir::new("reminders-tolerance");
        fs::write(
            dir.file(),
            r#"{"version":1,"reminders":{"enabled":"yes","leadMinutes":4,"sound":false,"events":{"shugoFestival":false,"riftPortals":7,"artifactSiege":true}}}"#,
        )
        .unwrap();

        let loaded = load(&dir.file(), 7);

        assert!(loaded.notice.is_none());
        let reminders = loaded.settings.reminders;
        assert!(reminders.enabled, "a mistyped switch keeps its default");
        assert!(!reminders.sound);
        assert_eq!(reminders.lead_minutes, 3, "4 is between the offered 3 and 5, and ties go to the shorter");
        assert!(!reminders.events.shugo_festival);
        assert!(reminders.events.rift_portals, "a mistyped event switch keeps its default");
        assert!(reminders.events.artifact_siege);
        assert!(reminders.events.dimensional_invasion);
    }

    #[test]
    fn a_lead_time_far_off_the_list_is_pulled_in_and_reminder_settings_survive_a_round_trip() {
        let dir = TestDir::new("reminders-roundtrip");
        fs::write(dir.file(), r#"{"version":1,"reminders":{"leadMinutes":200}}"#).unwrap();
        let mut settings = load(&dir.file(), 7).settings;
        assert_eq!(settings.reminders.lead_minutes, 5);

        settings.reminders.enabled = false;
        settings.reminders.lead_minutes = 2;
        settings.reminders.events.arena_of_tactics = true;
        save(&dir.file(), &settings).unwrap();

        assert_eq!(load(&dir.file(), 7).settings, settings);
    }

    #[test]
    fn a_reset_hour_of_9_saved_before_version_2_was_the_default_and_follows_the_game() {
        let dir = TestDir::new("old-default-reset-hour");
        fs::write(dir.file(), r#"{"version":1,"todos":{"resetHour":9,"membership":true}}"#).unwrap();

        let todos = load(&dir.file(), 7).settings.todos;

        assert_eq!(todos.reset_hour, None);
        assert!(todos.membership, "the rest of the section is kept");
    }

    #[test]
    fn a_file_without_a_version_is_migrated_too() {
        let dir = TestDir::new("unversioned-reset-hour");
        fs::write(dir.file(), r#"{"todos":{"resetHour":9}}"#).unwrap();

        assert_eq!(load(&dir.file(), 7).settings.todos.reset_hour, None);
    }

    #[test]
    fn another_hour_saved_before_version_2_was_a_choice_and_stays() {
        let dir = TestDir::new("old-chosen-reset-hour");
        fs::write(dir.file(), r#"{"version":1,"todos":{"resetHour":6}}"#).unwrap();

        assert_eq!(load(&dir.file(), 7).settings.todos.reset_hour, Some(6));
    }

    #[test]
    fn a_9_chosen_after_the_migration_stays_pinned() {
        let dir = TestDir::new("new-chosen-reset-hour");
        let mut settings = AppSettings::default();
        settings.todos.reset_hour = Some(9);
        save(&dir.file(), &settings).unwrap();

        assert_eq!(load(&dir.file(), 7).settings.todos.reset_hour, Some(9));
        assert_eq!(load(&dir.file(), 7).settings.todos.reset_hour, Some(9), "loading does not rewrite it");
    }

    #[test]
    fn an_old_file_without_pets_gets_the_pets_defaults() {
        let dir = TestDir::new("no-pets");
        fs::write(dir.file(), r#"{"version":2,"overlay":{"showDps":false}}"#).unwrap();

        let pets = load(&dir.file(), 7).settings.pets;

        assert_eq!(pets, crate::pets::PetsSettings::default());
        assert!(pets.soul_tracker && !pets.only_pinned && pets.maxed_pets && pets.black_market);
    }

    #[test]
    fn one_bad_pets_value_costs_that_setting_only() {
        let dir = TestDir::new("pets-tolerance");
        fs::write(dir.file(), r#"{"version":2,"pets":{"soulTracker":"yes","onlyPinned":true,"blackMarket":false,"unknown":1}}"#).unwrap();

        let loaded = load(&dir.file(), 7);

        assert!(loaded.notice.is_none());
        let pets = loaded.settings.pets;
        assert!(pets.soul_tracker, "a mistyped switch keeps its default");
        assert!(pets.only_pinned && !pets.black_market);
        assert!(pets.maxed_pets, "a missing switch keeps its default");
    }

    #[test]
    fn pets_settings_survive_a_round_trip() {
        let dir = TestDir::new("pets-roundtrip");
        let mut settings = AppSettings::default();
        settings.pets.soul_tracker = false;
        settings.pets.only_pinned = true;
        settings.pets.maxed_pets = false;
        settings.pets.black_market = false;
        save(&dir.file(), &settings).unwrap();

        assert_eq!(load(&dir.file(), 7).settings.pets, settings.pets);
        assert_eq!(load(&dir.file(), 7).settings, settings);
    }
}
