//! Personal bests: your best DPS against each boss, per character.
//!
//! Kept apart from the history on purpose. History is capped and can be
//! deleted in one click; a personal best should survive both. It is learnt from
//! every fight the history saves, and seeded from the history already on disk
//! the first time it runs, so nobody starts from zero.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::background_save::BackgroundSave;
use crate::history::HistoryRecord;
use crate::logger::AppLogger;
use crate::mobs::store::{set_aside_path, write_atomic};

/// Fights shorter than this do not count: a boss finished off in a few seconds
/// gives a DPS figure no real fight can reach, and it would stand forever.
const MIN_FIGHT_SECS: f64 = 15.0;
const FILE_NAME: &str = "personal_bests.json";
const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBest {
    pub character: String,
    pub mob_code: u32,
    pub target_name: Option<String>,
    pub dps: f64,
    pub total_damage: u64,
    pub duration_secs: f64,
    /// Milliseconds since the epoch.
    pub achieved_at: u64,
}

/// What the file holds. Files written before the version field existed are a
/// bare list of bests and still load.
#[derive(Deserialize)]
#[serde(untagged)]
enum FileIn {
    Versioned(VersionedFile),
    Unversioned(Vec<PersonalBest>),
}

#[derive(Deserialize)]
struct VersionedFile {
    version: u32,
    bests: Vec<PersonalBest>,
}

#[derive(Serialize)]
struct FileOut<'a> {
    version: u32,
    bests: &'a [PersonalBest],
}

/// The bests live in memory and are what every reader sees; the file is
/// written from a copy by `BackgroundSave`, never by the thread that changed
/// them and never under the lock.
#[derive(Default)]
pub struct PersonalBestStore {
    bests: Arc<Mutex<HashMap<String, PersonalBest>>>,
    saver: Option<BackgroundSave>,
}

fn key(character: &str, mob_code: u32) -> String {
    format!("{character}|{mob_code}")
}

fn sorted(bests: &HashMap<String, PersonalBest>) -> Vec<PersonalBest> {
    let mut list: Vec<PersonalBest> = bests.values().cloned().collect();
    list.sort_by(|a, b| b.achieved_at.cmp(&a.achieved_at));
    list
}

impl PersonalBestStore {
    /// A store that lives in memory only.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads `personal_bests.json` from `dir` and saves every change back to
    /// it. Only when there is no file at all (a first run) are the bests built
    /// from `history`. A file that cannot be read is set aside as
    /// `personal_bests.json.bad-<seconds>`, never overwritten, and the meter
    /// starts with no bests: they are not rebuilt from the history, which may
    /// hold fights the player cleared the bests of.
    pub fn open(dir: PathBuf, history: &[HistoryRecord], logger: Arc<AppLogger>) -> Self {
        let path = dir.join(FILE_NAME);
        let (list, rebuild) = match read_file(&path) {
            Ok(Some(list)) => (list, false),
            Ok(None) => (Vec::new(), true),
            Err(problem) => {
                logger.error(format!("{FILE_NAME}: {}", set_aside(&path, &problem)));
                (Vec::new(), false)
            }
        };
        let bests = Arc::new(Mutex::new(
            list.into_iter().map(|best| (key(&best.character, best.mob_code), best)).collect(),
        ));
        let shared = Arc::clone(&bests);
        let saver = BackgroundSave::new(FILE_NAME, logger, move || {
            let list = sorted(&shared.lock().unwrap());
            write_file(&path, &FileOut { version: FILE_VERSION, bests: &list })
        });
        let store = Self { bests, saver: Some(saver) };
        if rebuild {
            store.record(history);
            store.request_save();
        }
        store
    }

    pub fn get(&self, character: &str, mob_code: u32) -> Option<PersonalBest> {
        self.bests
            .lock()
            .unwrap()
            .get(&key(character, mob_code))
            .cloned()
    }

    pub fn all(&self) -> Vec<PersonalBest> {
        sorted(&self.bests.lock().unwrap())
    }

    /// Learn from saved fights. Returns whether any best improved, and saves
    /// the file when one did.
    pub fn record_and_save(&self, records: &[HistoryRecord]) -> bool {
        let changed = self.record(records);
        if changed {
            self.request_save();
        }
        changed
    }

    pub fn clear(&self) -> usize {
        let count = {
            let mut bests = self.bests.lock().unwrap();
            let count = bests.len();
            bests.clear();
            count
        };
        self.request_save();
        count
    }

    /// Writes any change not yet on disk, on the calling thread: for quitting.
    pub fn flush(&self) {
        if let Some(saver) = &self.saver {
            saver.flush();
        }
    }

    fn request_save(&self) {
        if let Some(saver) = &self.saver {
            saver.request();
        }
    }

    fn record(&self, records: &[HistoryRecord]) -> bool {
        let mut bests = self.bests.lock().unwrap();
        let mut changed = false;
        for candidate in records.iter().filter_map(candidate) {
            let slot = key(&candidate.character, candidate.mob_code);
            let better = bests
                .get(&slot)
                .map_or(true, |current| candidate.dps > current.dps);
            if better {
                bests.insert(slot, candidate);
                changed = true;
            }
        }
        changed
    }
}

fn write_file(path: &Path, bests: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(bests).map_err(|error| format!("cannot encode the personal bests: {error}"))?;
    write_atomic(path, &json).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// `Ok(None)` when there is no file yet. `Err` says why a file that exists
/// cannot be used.
fn read_file(path: &Path) -> Result<Option<Vec<PersonalBest>>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot be read: {error}")),
    };
    match serde_json::from_slice::<FileIn>(&bytes) {
        Ok(FileIn::Versioned(file)) if file.version <= FILE_VERSION => Ok(Some(file.bests)),
        Ok(FileIn::Versioned(file)) => Err(format!("is version {}, this build reads up to {FILE_VERSION}", file.version)),
        Ok(FileIn::Unversioned(list)) => Ok(Some(list)),
        Err(error) => Err(format!("holds no readable personal bests: {error}")),
    }
}

/// Renames a file the store cannot use to `<name>.bad-<seconds>`, so the next
/// save does not overwrite it. Returns what the log should say.
fn set_aside(path: &Path, problem: &str) -> String {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let aside = set_aside_path(path, now_secs);
    match std::fs::rename(path, &aside) {
        Ok(()) => format!(
            "{} {problem}; moved to {} and starting with no personal bests",
            path.display(),
            aside.display()
        ),
        Err(error) => format!(
            "{} {problem}, and it could not be moved aside ({error}); starting with no personal bests",
            path.display()
        ),
    }
}

/// The best this record can stand for: your DPS against a boss, in a fight
/// long enough to mean something.
fn candidate(record: &HistoryRecord) -> Option<PersonalBest> {
    let target = record.target_info.as_ref()?;
    if record.wiped || !target.is_boss {
        return None;
    }
    let mob_code = target.target_mob_code?;
    let character = record.combat_infos.main_actor_name.as_deref()?.trim();
    if character.is_empty() {
        return None;
    }
    let you = record
        .player_stats
        .values()
        .find(|player| player.actor_name == character)?;
    if !(you.dps.is_finite() && you.dps > 0.0) {
        return None;
    }

    // Your own time on the target, and the fight's when yours is missing.
    let own = target
        .target_start_time
        .get(&you.actor_id)
        .zip(target.target_last_time.get(&you.actor_id))
        .map(|(start, end)| end - start);
    let team = || {
        let start = target.target_start_time.values().copied().fold(f64::MAX, f64::min);
        let end = target.target_last_time.values().copied().fold(f64::MIN, f64::max);
        (end > start).then_some(end - start)
    };
    let duration = own.filter(|d| *d > 0.0).or_else(team)?;
    if duration < MIN_FIGHT_SECS {
        return None;
    }

    Some(PersonalBest {
        character: character.to_string(),
        mob_code,
        target_name: target.target_name.clone(),
        dps: you.dps,
        total_damage: you.total_damage,
        duration_secs: duration,
        achieved_at: record.created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::combat::{CombatInfos, PlayerOverviewStat, TargetInfo};

    fn record(character: &str, mob: u32, is_boss: bool, dps: f64, secs: f64) -> HistoryRecord {
        let mut player_stats = HashMap::new();
        player_stats.insert(
            1,
            PlayerOverviewStat {
                actor_id: 1,
                actor_name: character.to_string(),
                actor_server_id: String::new(),
                actor_class: String::new(),
                combat_power: None,
                counts: 10,
                total_damage: (dps * secs) as u64,
                min_damage: 1,
                max_damage: 1,
                special_counts: HashMap::new(),
                dps,
                damage_share: 1.0,
                damage_contribution: 1.0,
            },
        );
        HistoryRecord {
            id: format!("{mob}-{dps}"),
            target_id: 99,
            total_damage: (dps * secs) as u64,
            target_info: Some(TargetInfo {
                id: 99,
                target_mob_code: Some(mob),
                target_name: Some("Kromede".into()),
                is_boss,
                current_hp: None,
                max_hp: None,
                target_start_time: HashMap::from([(1, 100.0)]),
                target_last_time: HashMap::from([(1, 100.0 + secs)]),
            }),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: Some(1),
                main_actor_name: Some(character.to_string()),
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 0.0,
            },
            player_skill_stats: HashMap::new(),
            player_stats,
            use_buffs_by_target: HashMap::new(),
            ledger: None,
            wiped: false,
            created_at: 1,
        }
    }

    #[test]
    fn keeps_the_best_per_character_and_boss() {
        let store = PersonalBestStore::new();
        assert!(store.record(&[record("Kestrel", 7, true, 8_000.0, 60.0)]));
        assert!(!store.record(&[record("Kestrel", 7, true, 7_000.0, 60.0)]), "a worse fight changes nothing");
        assert!(store.record(&[record("Kestrel", 7, true, 9_000.0, 60.0)]));
        assert!(store.record(&[record("Rin", 7, true, 5_000.0, 60.0)]), "another character has their own");

        assert_eq!(store.get("Kestrel", 7).map(|b| b.dps), Some(9_000.0));
        assert_eq!(store.get("Rin", 7).map(|b| b.dps), Some(5_000.0));
    }

    #[test]
    fn ordinary_mobs_and_short_fights_do_not_count() {
        let store = PersonalBestStore::new();
        assert!(!store.record(&[record("Kestrel", 7, false, 8_000.0, 60.0)]));
        assert!(!store.record(&[record("Kestrel", 7, true, 90_000.0, 4.0)]));
        assert!(store.get("Kestrel", 7).is_none());
    }

    #[test]
    fn a_wiped_attempt_never_counts() {
        let store = PersonalBestStore::new();
        let wiped = HistoryRecord { wiped: true, ..record("Kestrel", 7, true, 90_000.0, 60.0) };

        assert!(!store.record(&[wiped]));
        assert!(store.get("Kestrel", 7).is_none());
    }

    use crate::mobs::store::test_dir::TestDir;

    fn logger_collecting(lines: Arc<Mutex<Vec<String>>>) -> Arc<AppLogger> {
        let path = std::env::temp_dir().join(format!("meter-core-bests-{}.log", std::process::id()));
        Arc::new(AppLogger::new(path, Box::new(move |event| lines.lock().unwrap().push(event.line))))
    }

    fn open(dir: &TestDir, history: &[HistoryRecord]) -> PersonalBestStore {
        PersonalBestStore::open(dir.path().to_path_buf(), history, logger_collecting(Arc::default()))
    }

    fn aside_files(dir: &TestDir) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("personal_bests.json.bad-"))
            .collect()
    }

    fn one_fight() -> Vec<HistoryRecord> {
        vec![record("Kestrel", 7, true, 8_000.0, 60.0)]
    }

    #[test]
    fn bests_are_saved_in_a_versioned_file_and_load_back() {
        let dir = TestDir::new();
        let store = open(&dir, &[]);
        assert!(store.record_and_save(&one_fight()));
        store.flush();

        let file: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.path().join(FILE_NAME)).unwrap()).unwrap();
        assert_eq!(file["version"], FILE_VERSION);
        assert_eq!(file["bests"].as_array().unwrap().len(), 1);
        assert_eq!(open(&dir, &[]).get("Kestrel", 7).map(|best| best.dps), Some(8_000.0));
    }

    #[test]
    fn a_file_from_before_the_version_field_still_loads_and_is_not_rebuilt_from_history() {
        let dir = TestDir::new();
        let old = PersonalBest {
            character: "Rin".into(),
            mob_code: 9,
            target_name: None,
            dps: 5_000.0,
            total_damage: 300_000,
            duration_secs: 60.0,
            achieved_at: 3,
        };
        std::fs::write(dir.path().join(FILE_NAME), serde_json::to_vec_pretty(&vec![old.clone()]).unwrap()).unwrap();

        let store = open(&dir, &one_fight());

        assert_eq!(store.all(), vec![old]);
    }

    #[test]
    fn an_unreadable_file_is_set_aside_not_rebuilt_from_history_and_not_overwritten() {
        let newer = br#"{"version": 99, "bests": []}"#.to_vec();
        for (why, bytes) in [("garbage", b"{ half a wri".to_vec()), ("a newer version", newer)] {
            let dir = TestDir::new();
            std::fs::write(dir.path().join(FILE_NAME), &bytes).unwrap();
            let lines = Arc::new(Mutex::new(Vec::new()));

            let store = PersonalBestStore::open(dir.path().to_path_buf(), &one_fight(), logger_collecting(Arc::clone(&lines)));

            assert!(store.all().is_empty(), "{why}: no bests, none rebuilt from the history");
            let aside = aside_files(&dir);
            assert_eq!(aside.len(), 1, "{why}");
            assert!(!dir.path().join(FILE_NAME).exists(), "{why}");
            assert!(lines.lock().unwrap().iter().any(|line| line.contains("moved to")), "{why}: the log says so");

            store.record_and_save(&[record("Rin", 9, true, 1_000.0, 60.0)]);
            store.flush();

            assert_eq!(std::fs::read(&aside[0]).unwrap(), bytes, "{why}: the set-aside file is untouched");
            assert_eq!(open(&dir, &[]).get("Rin", 9).map(|best| best.dps), Some(1_000.0), "{why}");
        }
    }

    #[test]
    fn a_missing_file_is_a_first_run_and_builds_the_bests_from_history() {
        let dir = TestDir::new();

        let store = open(&dir, &one_fight());
        store.flush();

        assert_eq!(store.get("Kestrel", 7).map(|best| best.dps), Some(8_000.0));
        assert!(aside_files(&dir).is_empty());
        assert_eq!(open(&dir, &[]).get("Kestrel", 7).map(|best| best.dps), Some(8_000.0), "the rebuilt bests were written");
    }

    #[test]
    fn a_cleared_best_stays_cleared_after_a_restart_even_though_the_history_still_holds_the_fight() {
        let dir = TestDir::new();
        let history = one_fight();
        let store = open(&dir, &history);
        store.flush();
        assert_eq!(store.clear(), 1);
        store.flush();

        let restarted = open(&dir, &history);

        assert!(restarted.all().is_empty());
    }

    #[test]
    fn a_write_that_is_cut_short_leaves_the_previous_file_intact() {
        let dir = TestDir::new();
        let store = open(&dir, &one_fight());
        store.flush();
        std::fs::write(dir.path().join("personal_bests.json.tmp"), b"[{\"half").unwrap();

        let restarted = open(&dir, &[]);

        assert_eq!(restarted.get("Kestrel", 7).map(|best| best.dps), Some(8_000.0));
        assert!(aside_files(&dir).is_empty());
    }

    #[test]
    fn a_failed_write_is_logged_and_leaves_the_file_alone() {
        let dir = TestDir::new();
        let store = open(&dir, &one_fight());
        store.flush();
        let before = std::fs::read(dir.path().join(FILE_NAME)).unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let blocked = PersonalBestStore::open(dir.path().to_path_buf(), &[], logger_collecting(Arc::clone(&lines)));
        std::fs::create_dir(dir.path().join("personal_bests.json.tmp")).unwrap();

        blocked.record_and_save(&[record("Kestrel", 7, true, 9_000.0, 60.0)]);
        blocked.flush();

        assert!(lines.lock().unwrap().iter().any(|line| line.contains("ERROR") && line.contains("personal_bests.json: cannot save")));
        assert_eq!(std::fs::read(dir.path().join(FILE_NAME)).unwrap(), before);
    }
}
