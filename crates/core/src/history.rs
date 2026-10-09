use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::background_save::BackgroundSave;
use crate::logger::AppLogger;
use crate::mobs::store::{set_aside_path, write_atomic};
use crate::models::combat::{
    BuffSummary, CombatInfos, CombatSnapshot, PlayerOverviewStat, SkillStats, TargetInfo,
};
use crate::report::ledger::FightLedger;

const MAGIC: &[u8; 4] = b"DPSH";
const VERSION: u16 = 1;
const HISTORY_THRESHOLD: u64 = 1_000_000;
const MAX_RECORDS: usize = 500;

// =============================================================================
// Per-target history record
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub target_id: u32,
    pub total_damage: u64,
    pub target_info: Option<TargetInfo>,
    pub combat_infos: CombatInfos,
    pub player_skill_stats: HashMap<u32, HashMap<u32, SkillStats>>,
    pub player_stats: HashMap<u32, PlayerOverviewStat>,
    #[serde(default)]
    pub use_buffs_by_target: HashMap<u32, Vec<BuffSummary>>,
    /// What the party took, healed and lost in a boss fight. `None` in a fight
    /// saved before the meter kept it, and in every fight that is not a boss's.
    #[serde(default)]
    pub ledger: Option<FightLedger>,
    /// A boss attempt that ended with the boss alive and the party gone: its
    /// health went back to full, or it left the field. `target_info.current_hp`
    /// holds the health it had left. Records saved before wipes were kept
    /// read `false`.
    #[serde(default)]
    pub wiped: bool,
    pub created_at: u64,
}

// =============================================================================
// Store
// =============================================================================

const FILE_NAME: &str = "combat_history.bin";

/// The records live in memory and are what every reader sees; the file is
/// written from a copy by `BackgroundSave`, never by the thread that changed
/// them and never under the lock.
pub struct HistoryStore {
    records: Arc<Mutex<VecDeque<HistoryRecord>>>,
    saver: Option<BackgroundSave>,
}

impl HistoryStore {
    /// A store that lives in memory only.
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(VecDeque::new())),
            saver: None,
        }
    }

    /// Loads `history/combat_history.bin` from `app_data_dir` and saves every
    /// change back to it. A file that cannot be read is set aside as
    /// `combat_history.bin.bad-<seconds>` and never overwritten; the store
    /// then starts empty. Without a folder the store lives in memory only.
    pub fn open(app_data_dir: Option<PathBuf>, logger: Arc<AppLogger>) -> Self {
        let Some(app_data_dir) = app_data_dir else {
            return Self::new();
        };
        let path = app_data_dir.join("history").join(FILE_NAME);
        let records = match read_file(&path) {
            Ok(records) => records.unwrap_or_default(),
            Err(problem) => {
                logger.error(format!("{FILE_NAME}: {}", set_aside(&path, &problem)));
                VecDeque::new()
            }
        };
        Self::with_writer(records, logger, move |records| write_file(&path, &records))
    }

    fn with_writer(
        initial: VecDeque<HistoryRecord>,
        logger: Arc<AppLogger>,
        write: impl Fn(Vec<HistoryRecord>) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        let records = Arc::new(Mutex::new(initial));
        let shared = Arc::clone(&records);
        let saver = BackgroundSave::new(FILE_NAME, logger, move || {
            let copy: Vec<HistoryRecord> = shared.lock().unwrap().iter().cloned().collect();
            write(copy)
        });
        Self { records, saver: Some(saver) }
    }

    pub fn get_records(&self) -> Vec<HistoryRecord> {
        self.records.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear_all(&self) -> usize {
        let count = {
            let mut stored = self.records.lock().unwrap();
            let count = stored.len();
            stored.clear();
            count
        };
        self.request_save();
        count
    }

    pub fn delete_record(&self, id: &str) -> bool {
        let deleted = {
            let mut stored = self.records.lock().unwrap();
            let old_len = stored.len();
            stored.retain(|r| r.id != id);
            stored.len() != old_len
        };
        if deleted {
            self.request_save();
        }
        deleted
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

    /// Persist current snapshot as per-target history records, then clear.
    /// `ledger_of` gives the ledger of a boss fight from its target and the
    /// players who hit it. Returns what was saved, for the personal bests to
    /// learn from.
    pub fn save_and_clear(
        &self,
        snapshot: CombatSnapshot,
        ledger_of: impl Fn(&TargetInfo, &[u32]) -> Option<FightLedger>,
    ) -> Vec<HistoryRecord> {
        let records = Self::extract_records(snapshot, None, false, ledger_of);
        if !records.is_empty() {
            self.push(records.clone());
        }
        records
    }

    /// Persist the attempt on `target_id` alone, as a wipe. `None` when it
    /// holds too little damage to keep.
    pub fn save_wiped(
        &self,
        snapshot: CombatSnapshot,
        target_id: u32,
        ledger_of: impl Fn(&TargetInfo, &[u32]) -> Option<FightLedger>,
    ) -> Option<HistoryRecord> {
        let record = Self::extract_records(snapshot, Some(target_id), true, ledger_of).pop()?;
        self.push(vec![record.clone()]);
        Some(record)
    }

    fn extract_records(
        snapshot: CombatSnapshot,
        only_target: Option<u32>,
        wiped: bool,
        ledger_of: impl Fn(&TargetInfo, &[u32]) -> Option<FightLedger>,
    ) -> Vec<HistoryRecord> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        snapshot
            .by_target_player_stats
            .into_iter()
            .filter_map(|(target_id, player_stats)| {
                let total_damage: u64 = player_stats.values().map(|s| s.total_damage).sum();

                if only_target.is_some_and(|only| only != target_id) || total_damage <= HISTORY_THRESHOLD {
                    return None;
                }

                let target_info = snapshot.combat_infos.target_infos.get(&target_id).cloned();

                let skill_stats = snapshot
                    .by_target_player_skill_stats
                    .get(&target_id)
                    .cloned()
                    .unwrap_or_default();

                let relevant_buff_targets: HashSet<u32> = std::iter::once(target_id)
                    .chain(player_stats.keys().copied())
                    .collect();
                let use_buffs_by_target = snapshot
                    .use_buffs_by_target
                    .iter()
                    .filter_map(|(buff_target_id, buffs)| {
                        relevant_buff_targets
                            .contains(buff_target_id)
                            .then(|| (*buff_target_id, buffs.clone()))
                    })
                    .collect();

                let combat_infos = CombatInfos {
                    target_infos: target_info
                        .as_ref()
                        .map(|ti| {
                            let mut m = HashMap::new();
                            m.insert(target_id, ti.clone());
                            m
                        })
                        .unwrap_or_default(),
                    ..snapshot.combat_infos.clone()
                };

                let ledger = target_info.as_ref().filter(|target| target.is_boss).and_then(|target| {
                    let dealers: Vec<u32> = player_stats.keys().copied().collect();
                    ledger_of(target, &dealers)
                });
                Some(HistoryRecord {
                    id: format!("{}-{}", target_id, now_ms),
                    target_id,
                    total_damage,
                    target_info,
                    combat_infos,
                    player_stats,
                    player_skill_stats: skill_stats,
                    use_buffs_by_target,
                    ledger,
                    wiped,
                    created_at: now_ms,
                })
            })
            .collect()
    }

    fn push(&self, records: Vec<HistoryRecord>) {
        {
            let mut stored = self.records.lock().unwrap();
            stored.extend(records);
            while stored.len() > MAX_RECORDS {
                stored.pop_front();
            }
        }
        self.request_save();
    }
}

// =============================================================================
// Disk I/O (lz4 compressed binary)
// =============================================================================

/// Encodes the records and writes them over the file. When the records cannot
/// be encoded nothing is written: the file keeps what it holds.
fn write_file(path: &Path, records: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_vec(records).map_err(|error| format!("cannot encode the history: {error}"))?;
    let compressed = lz4_flex::compress_prepend_size(&json);
    let crc = crc32fast::hash(&compressed);

    let mut bytes = Vec::with_capacity(MAGIC.len() + 2 + 4 + compressed.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes.extend_from_slice(&compressed);

    write_atomic(path, &bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// `Ok(None)` when there is no file yet. `Err` says why a file that exists
/// cannot be used.
fn read_file(path: &Path) -> Result<Option<VecDeque<HistoryRecord>>, String> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot be read: {error}")),
    };

    if data.len() < 10 {
        return Err(format!("is {} bytes, too short for a history file", data.len()));
    }
    if &data[0..4] != MAGIC {
        return Err("has the wrong magic number".to_string());
    }
    let version = u16::from_le_bytes([data[4], data[5]]);
    if version != VERSION {
        return Err(format!("is version {version}, this build reads version {VERSION}"));
    }
    let expected_crc = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
    let compressed = &data[10..];
    if crc32fast::hash(compressed) != expected_crc {
        return Err("fails its checksum".to_string());
    }
    let json = lz4_flex::decompress_size_prepended(compressed).map_err(|error| format!("cannot be decompressed: {error}"))?;
    let records = serde_json::from_slice::<Vec<HistoryRecord>>(&json)
        .map_err(|error| format!("holds no readable records: {error}"))?;
    Ok(Some(records.into()))
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
            "{} {problem}; moved to {} and starting with no history",
            path.display(),
            aside.display()
        ),
        Err(error) => format!(
            "{} {problem}, and it could not be moved aside ({error}); starting with no history",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::session::SessionSummary;

    use super::*;

    fn quiet_logger() -> Arc<AppLogger> {
        let path = std::env::temp_dir().join(format!("meter-core-history-{}.log", std::process::id()));
        Arc::new(AppLogger::new(path, Box::new(|_| {})))
    }

    fn open(dir: &crate::mobs::store::test_dir::TestDir) -> HistoryStore {
        HistoryStore::open(Some(dir.path().to_path_buf()), quiet_logger())
    }

    fn history_file(dir: &crate::mobs::store::test_dir::TestDir) -> PathBuf {
        dir.path().join("history").join(FILE_NAME)
    }
    fn fight_on(targets: &[u32]) -> CombatSnapshot {
        let player = |damage: u64| PlayerOverviewStat {
            actor_id: 1,
            actor_name: "Me".to_string(),
            actor_server_id: String::new(),
            actor_class: String::new(),
            combat_power: None,
            counts: 1,
            total_damage: damage,
            min_damage: 1,
            max_damage: 1,
            special_counts: HashMap::new(),
            dps: 1.0,
            damage_share: 1.0,
            damage_contribution: 1.0,
        };
        CombatSnapshot {
            total_damage: 0,
            by_target_player_skill_stats: HashMap::new(),
            by_target_player_stats: targets
                .iter()
                .map(|target| (*target, HashMap::from([(1, player(HISTORY_THRESHOLD + 1))])))
                .collect(),
            use_buffs_by_target: HashMap::new(),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: Some(1),
                main_actor_name: Some("Me".to_string()),
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 0.0,
            },
            last_target_info: None,
            last_target_all_players_overview_stats: Vec::new(),
            main_actor_received_player_overview_stats: Vec::new(),
            main_actor_dealt_player_overview_stats: Vec::new(),
            session: SessionSummary::default(),
        }
    }

    fn no_ledger(_: &TargetInfo, _: &[u32]) -> Option<FightLedger> {
        None
    }

    fn ids(store: &HistoryStore) -> Vec<String> {
        let mut ids: Vec<String> = store.get_records().into_iter().map(|record| record.id).collect();
        ids.sort();
        ids
    }

    #[test]
    fn a_fight_under_the_threshold_on_every_target_is_not_saved() {
        let store = HistoryStore::new();
        let mut small = fight_on(&[1]);
        small.by_target_player_stats.get_mut(&1).unwrap().get_mut(&1).unwrap().total_damage = HISTORY_THRESHOLD;

        assert!(store.save_and_clear(small, no_ledger).is_empty());
        assert!(store.get_records().is_empty());
    }

    #[test]
    fn one_record_is_deleted_by_id_and_the_others_stay() {
        let store = HistoryStore::new();
        store.save_and_clear(fight_on(&[1, 2, 3]), no_ledger);
        let doomed = ids(&store).remove(1);

        assert!(store.delete_record(&doomed));
        assert!(!store.delete_record(&doomed));
        assert_eq!(ids(&store).len(), 2);
        assert!(!ids(&store).contains(&doomed));
    }

    #[test]
    fn deleting_everything_reports_how_many_went() {
        let store = HistoryStore::new();
        store.save_and_clear(fight_on(&[1, 2, 3]), no_ledger);

        assert_eq!(store.clear_all(), 3);
        assert!(store.get_records().is_empty());
        assert_eq!(store.clear_all(), 0);
    }

    #[test]
    fn saved_history_loads_back_in_a_new_store_and_leaves_no_temporary_file() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = open(&dir);
        store.save_and_clear(fight_on(&[1, 2]), no_ledger);
        store.flush();

        let reloaded = open(&dir);

        assert_eq!(ids(&reloaded), ids(&store));
        assert_eq!(ids(&reloaded).len(), 2);
        assert!(!dir.path().join("history").join("combat_history.bin.tmp").exists());
    }

    #[test]
    fn an_interrupted_write_leaves_the_previous_history_loadable() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = open(&dir);
        store.save_and_clear(fight_on(&[1]), no_ledger);
        store.flush();
        std::fs::write(dir.path().join("history").join("combat_history.bin.tmp"), b"half a wri").unwrap();

        let reloaded = open(&dir);

        assert_eq!(ids(&reloaded), ids(&store));
        assert_eq!(ids(&reloaded).len(), 1);
    }

    #[test]
    fn changes_still_pending_when_the_store_is_dropped_reach_the_file() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = open(&dir);
        store.save_and_clear(fight_on(&[1, 2, 3]), no_ledger);
        let doomed = ids(&store).remove(0);
        store.delete_record(&doomed);
        let kept = ids(&store);
        drop(store);

        assert_eq!(ids(&open(&dir)), kept);
    }

    #[test]
    fn deletes_reach_the_file() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = open(&dir);
        store.save_and_clear(fight_on(&[1, 2, 3, 4]), no_ledger);
        let all = ids(&store);
        store.flush();

        store.delete_record(&all[0]);
        store.delete_record(&all[1]);
        store.flush();
        assert_eq!(ids(&open(&dir)), all[2..].to_vec());

        store.clear_all();
        store.flush();
        assert!(open(&dir).get_records().is_empty());
    }

    fn history_bytes(version: u16, crc_of: impl Fn(&[u8]) -> u32, magic: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(magic);
        file.extend_from_slice(&version.to_le_bytes());
        file.extend_from_slice(&crc_of(payload).to_le_bytes());
        file.extend_from_slice(payload);
        file
    }

    fn unreadable_files() -> Vec<(&'static str, Vec<u8>)> {
        let good_json = lz4_flex::compress_prepend_size(b"[]");
        let not_records = lz4_flex::compress_prepend_size(b"{\"not\":\"a list\"}");
        let not_lz4 = b"this is not an lz4 block at all".to_vec();
        vec![
            ("too short", b"DPSH".to_vec()),
            ("bad magic", history_bytes(VERSION, crc32fast::hash, b"XXXX", &good_json)),
            ("bad version", history_bytes(VERSION + 1, crc32fast::hash, MAGIC, &good_json)),
            ("bad crc", history_bytes(VERSION, |_| 7, MAGIC, &good_json)),
            ("bad decompress", history_bytes(VERSION, crc32fast::hash, MAGIC, &not_lz4)),
            ("bad json", history_bytes(VERSION, crc32fast::hash, MAGIC, &not_records)),
        ]
    }

    fn aside_files(dir: &crate::mobs::store::test_dir::TestDir) -> Vec<PathBuf> {
        std::fs::read_dir(dir.path().join("history"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("combat_history.bin.bad-"))
            .collect()
    }

    #[test]
    fn an_unreadable_history_file_is_set_aside_and_the_next_save_does_not_overwrite_it() {
        for (why, bytes) in unreadable_files() {
            let dir = crate::mobs::store::test_dir::TestDir::new();
            std::fs::create_dir_all(dir.path().join("history")).unwrap();
            std::fs::write(history_file(&dir), &bytes).unwrap();

            let store = open(&dir);

            assert!(store.get_records().is_empty(), "{why}: the store starts empty");
            let aside = aside_files(&dir);
            assert_eq!(aside.len(), 1, "{why}: the file is set aside");
            assert!(!history_file(&dir).exists(), "{why}: nothing is left at the history path");

            store.save_and_clear(fight_on(&[1]), no_ledger);
            store.flush();

            assert_eq!(std::fs::read(&aside[0]).unwrap(), bytes, "{why}: the set-aside file is untouched");
            assert_eq!(ids(&open(&dir)).len(), 1, "{why}: the new history is saved");
        }
    }

    fn logger_writing_in(dir: &crate::mobs::store::test_dir::TestDir) -> Arc<AppLogger> {
        Arc::new(AppLogger::new(dir.path().join("belisk.log"), Box::new(|_| {})))
    }

    #[test]
    fn an_unreadable_history_file_is_named_with_its_reason_in_the_log() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        std::fs::create_dir_all(dir.path().join("history")).unwrap();
        let bytes = history_bytes(VERSION, crc32fast::hash, b"XXXX", &lz4_flex::compress_prepend_size(b"[]"));
        std::fs::write(history_file(&dir), bytes).unwrap();
        let logger = logger_writing_in(&dir);

        HistoryStore::open(Some(dir.path().to_path_buf()), Arc::clone(&logger));

        let lines = logger.tail(10);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("[ERROR]"), "{}", lines[0]);
        assert!(lines[0].contains(FILE_NAME), "{}", lines[0]);
        assert!(lines[0].contains("has the wrong magic number"), "{}", lines[0]);
    }

    #[test]
    fn a_failed_history_save_is_named_with_its_reason_in_the_log() {
        use std::time::{Duration, Instant};

        let dir = crate::mobs::store::test_dir::TestDir::new();
        let logger = logger_writing_in(&dir);
        let store = HistoryStore::with_writer(VecDeque::new(), Arc::clone(&logger), |_| Err("disk full".to_string()));

        store.save_and_clear(fight_on(&[1]), no_ledger);
        store.flush();

        let deadline = Instant::now() + Duration::from_secs(10);
        while logger.tail(10).is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let lines = logger.tail(10);
        assert!(!lines.is_empty(), "nothing was logged");
        assert!(lines[0].contains("[ERROR]"), "{}", lines[0]);
        assert!(lines[0].contains(FILE_NAME), "{}", lines[0]);
        assert!(lines[0].contains("disk full"), "{}", lines[0]);
    }

    #[test]
    fn a_missing_history_file_is_a_fresh_start_and_sets_nothing_aside() {
        let dir = crate::mobs::store::test_dir::TestDir::new();

        let store = open(&dir);

        assert!(store.get_records().is_empty());
        assert!(!dir.path().join("history").exists(), "nothing was read, set aside or written");
    }

    struct CannotBeEncoded;

    impl Serialize for CannotBeEncoded {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("refuses to encode"))
        }
    }

    #[test]
    fn records_that_cannot_be_encoded_leave_the_file_as_it_was() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = open(&dir);
        store.save_and_clear(fight_on(&[1, 2]), no_ledger);
        store.flush();
        let before = std::fs::read(history_file(&dir)).unwrap();

        let error = write_file(&history_file(&dir), &CannotBeEncoded).unwrap_err();

        assert!(error.contains("cannot encode the history"), "{error}");
        assert_eq!(std::fs::read(history_file(&dir)).unwrap(), before);
        assert!(!dir.path().join("history").join("combat_history.bin.tmp").exists());
    }

    #[test]
    fn the_records_lock_is_free_while_the_file_is_being_written() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (entered, wait_entered) = mpsc::channel();
        let (release, wait_release) = mpsc::channel::<()>();
        let wait_release = Mutex::new(wait_release);
        let store = HistoryStore::with_writer(VecDeque::new(), quiet_logger(), move |written| {
            entered.send(written.len()).unwrap();
            wait_release.lock().unwrap().recv().unwrap();
            Ok(())
        });

        store.save_and_clear(fight_on(&[1]), no_ledger);
        assert_eq!(wait_entered.recv_timeout(Duration::from_secs(10)).unwrap(), 1, "the writer has the records and is stuck writing");

        assert_eq!(store.get_records().len(), 1);
        let more = store.save_and_clear(fight_on(&[2]), no_ledger);
        assert_eq!(more.len(), 1);
        let doomed = store.get_records()[0].id.clone();
        assert!(store.delete_record(&doomed));
        assert_eq!(store.get_records().len(), 1);

        release.send(()).unwrap();
        assert_eq!(wait_entered.recv_timeout(Duration::from_secs(10)).unwrap(), 1, "what changed during the write is written next");
        release.send(()).unwrap();
    }

    fn target(id: u32, is_boss: bool) -> TargetInfo {
        TargetInfo {
            id,
            target_mob_code: Some(2_300_810),
            target_name: Some("Tiere".to_string()),
            is_boss,
            current_hp: Some(0),
            max_hp: Some(1_000),
            target_start_time: HashMap::from([(1, 100.0)]),
            target_last_time: HashMap::from([(1, 160.0)]),
        }
    }

    fn ledger_with(player_ids: &[u32]) -> FightLedger {
        FightLedger {
            window: crate::report::ledger::FightWindow { started_at: 100.0, ended_at: 160.0 },
            players: player_ids
                .iter()
                .map(|&actor_id| crate::report::ledger::LedgerPlayer {
                    actor_id,
                    actor_name: String::new(),
                    actor_server_id: String::new(),
                    actor_class: String::new(),
                    healing: Vec::new(),
                    damage_taken: Vec::new(),
                })
                .collect(),
            deaths: Vec::new(),
            buffs: Vec::new(),
        }
    }

    #[test]
    fn only_a_boss_fight_is_saved_with_the_ledger_of_the_players_who_hit_it() {
        let store = HistoryStore::new();
        let mut snapshot = fight_on(&[10, 20]);
        snapshot.combat_infos.target_infos.insert(10, target(10, true));
        snapshot.combat_infos.target_infos.insert(20, target(20, false));

        store.save_and_clear(snapshot, |target, dealers| {
            assert!(target.is_boss, "asked for the ledger of a fight that is not a boss's");
            Some(ledger_with(dealers))
        });

        let records = store.get_records();
        let boss = records.iter().find(|record| record.target_id == 10).unwrap();
        let mob = records.iter().find(|record| record.target_id == 20).unwrap();
        assert_eq!(boss.ledger.as_ref().map(|ledger| ledger.players.len()), Some(1));
        assert_eq!(mob.ledger, None);
    }

    #[test]
    fn a_ledger_is_kept_through_the_file() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = open(&dir);
        let mut snapshot = fight_on(&[10]);
        snapshot.combat_infos.target_infos.insert(10, target(10, true));
        store.save_and_clear(snapshot, |_, dealers| Some(ledger_with(dealers)));
        store.flush();

        let reloaded = open(&dir);

        assert_eq!(reloaded.get_records()[0].ledger, store.get_records()[0].ledger);
        assert!(reloaded.get_records()[0].ledger.is_some());
    }

    #[test]
    fn a_history_file_written_before_the_ledger_loads_with_no_ledger() {
        let json = include_str!("report/fixtures/head-history-record.json").as_bytes();
        let compressed = lz4_flex::compress_prepend_size(json);
        let mut file = Vec::new();
        file.extend_from_slice(MAGIC);
        file.extend_from_slice(&VERSION.to_le_bytes());
        file.extend_from_slice(&crc32fast::hash(&compressed).to_le_bytes());
        file.extend_from_slice(&compressed);
        let dir = crate::mobs::store::test_dir::TestDir::new();
        std::fs::create_dir_all(dir.path().join("history")).unwrap();
        std::fs::write(history_file(&dir), file).unwrap();

        let store = open(&dir);

        let records = store.get_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target_id, 56_499);
        assert_eq!(records[0].ledger, None);
    }
}
