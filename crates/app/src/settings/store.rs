//! `settings.json` in the app's data folder. A missing file is a fresh start;
//! a file that cannot be read as settings, or that a newer build wrote, is
//! renamed to `settings.json.bad-<seconds>` and the defaults are used. Fields
//! missing from the file take their defaults and unknown fields are ignored.

use std::fs;
use std::path::{Path, PathBuf};

use meter_core::mobs::store::write_atomic;

use super::model::AppSettings;

pub const FILE_NAME: &str = "settings.json";
pub const SCHEMA_VERSION: u32 = 1;

pub struct Loaded {
    pub settings: AppSettings,
    /// Why the stored file was not used, for the settings page to show.
    pub notice: Option<String>,
}

pub fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load(path: &Path, now_secs: u64) -> Loaded {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return fresh(None),
        Err(error) => {
            return fresh(Some(format!("cannot read {}: {error}; using the default settings", path.display())))
        }
    };

    let problem = match parse(&bytes) {
        Ok(settings) => return Loaded { settings: settings.clamped(), notice: None },
        Err(problem) => problem,
    };

    let aside = set_aside_path(path, now_secs);
    let notice = match fs::rename(path, &aside) {
        Ok(()) => format!(
            "{} {problem}; moved to {} and using the default settings",
            path.display(),
            aside.display()
        ),
        Err(error) => format!(
            "{} {problem}, and it could not be moved aside ({error}); using the default settings",
            path.display()
        ),
    };
    fresh(Some(notice))
}

fn fresh(notice: Option<String>) -> Loaded {
    Loaded { settings: AppSettings::default(), notice }
}

fn parse(bytes: &[u8]) -> Result<AppSettings, String> {
    let stored: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| format!("is not valid JSON: {error}"))?;
    let Some(fields) = stored.as_object() else {
        return Err("is not a JSON object".to_string());
    };
    let version = fields.get("version").and_then(serde_json::Value::as_u64).unwrap_or(0);
    if version > u64::from(SCHEMA_VERSION) {
        return Err(format!("is schema version {version}, newer than {SCHEMA_VERSION}"));
    }
    Ok(keep_valid_fields(&stored))
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

fn set_aside_path(path: &Path, now_secs: u64) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bad-{now_secs}"));
    path.with_file_name(name)
}

pub fn encode(settings: &AppSettings) -> serde_json::Result<Vec<u8>> {
    let mut value = serde_json::to_value(settings)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("version".to_string(), SCHEMA_VERSION.into());
    }
    serde_json::to_vec_pretty(&value)
}

pub fn save(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let bytes = encode(settings).map_err(|error| format!("cannot encode the settings: {error}"))?;
    write_atomic(path, &bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
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
        assert!(loaded.notice.unwrap().contains("settings.json.bad-42"));
        assert!(!dir.file().exists());
        assert_eq!(fs::read_to_string(dir.0.join("settings.json.bad-42")).unwrap(), "not json {");
    }

    #[test]
    fn a_file_from_a_newer_build_is_set_aside_too() {
        let dir = TestDir::new("newer");
        fs::write(dir.file(), r#"{"version":99,"overlay":{"showDps":false}}"#).unwrap();

        let loaded = load(&dir.file(), 5);

        assert_eq!(loaded.settings, AppSettings::default());
        assert!(loaded.notice.unwrap().contains("newer"));
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
}
