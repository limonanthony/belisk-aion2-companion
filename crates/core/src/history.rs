use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::mobs::store::write_atomic;
use crate::models::combat::{
    BuffSummary, CombatInfos, CombatSnapshot, PlayerOverviewStat, SkillStats, TargetInfo,
};

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
    pub created_at: u64,
}

// =============================================================================
// Store
// =============================================================================

pub struct HistoryStore {
    records: Mutex<VecDeque<HistoryRecord>>,
    dir: Mutex<Option<PathBuf>>,
}

impl HistoryStore {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(VecDeque::new()),
            dir: Mutex::new(None),
        }
    }

    pub fn get_records(&self) -> Vec<HistoryRecord> {
        self.records.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear_all(&self) -> usize {
        let mut stored = self.records.lock().unwrap();
        let count = stored.len();
        stored.clear();
        self.write_to_disk(&[]);
        count
    }

    pub fn delete_record(&self, id: &str) -> bool {
        let mut stored = self.records.lock().unwrap();
        let old_len = stored.len();
        stored.retain(|r| r.id != id);
        if stored.len() != old_len {
            let snapshot: Vec<HistoryRecord> = stored.iter().cloned().collect();
            self.write_to_disk(&snapshot);
            true
        } else {
            false
        }
    }

    pub fn delete_records(&self, ids: &[String]) -> usize {
        if ids.is_empty() {
            return 0;
        }

        let ids: std::collections::HashSet<&str> = ids.iter().map(String::as_str).collect();
        let mut stored = self.records.lock().unwrap();
        let old_len = stored.len();
        stored.retain(|record| !ids.contains(record.id.as_str()));
        let deleted = old_len - stored.len();

        if deleted > 0 {
            let snapshot: Vec<HistoryRecord> = stored.iter().cloned().collect();
            self.write_to_disk(&snapshot);
        }

        deleted
    }

    pub fn init_dir(&self, app_data_dir: PathBuf) {
        let dir = app_data_dir.join("history");
        let _ = std::fs::create_dir_all(&dir);
        *self.dir.lock().unwrap() = Some(dir);
    }

    /// Persist current snapshot as per-target history records, then clear.
    /// Returns what was saved, for the personal bests to learn from.
    pub fn save_and_clear(&self, snapshot: CombatSnapshot) -> Vec<HistoryRecord> {
        let records = Self::extract_records(snapshot);
        if !records.is_empty() {
            self.push(records.clone());
        }
        records
    }

    fn extract_records(snapshot: CombatSnapshot) -> Vec<HistoryRecord> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        snapshot
            .by_target_player_stats
            .into_iter()
            .filter_map(|(target_id, player_stats)| {
                let total_damage: u64 = player_stats.values().map(|s| s.total_damage).sum();

                if total_damage <= HISTORY_THRESHOLD {
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

                Some(HistoryRecord {
                    id: format!("{}-{}", target_id, now_ms),
                    target_id,
                    total_damage,
                    target_info,
                    combat_infos,
                    player_stats,
                    player_skill_stats: skill_stats,
                    use_buffs_by_target,
                    created_at: now_ms,
                })
            })
            .collect()
    }

    fn push(&self, records: Vec<HistoryRecord>) {
        let mut stored = self.records.lock().unwrap();
        stored.extend(records);

        // Trim to max
        while stored.len() > MAX_RECORDS {
            stored.pop_front();
        }

        // Write to disk
        let snapshot: Vec<HistoryRecord> = stored.iter().cloned().collect();
        self.write_to_disk(&snapshot);
    }

    // =========================================================================
    // Disk I/O (lz4 compressed binary)
    // =========================================================================

    fn write_to_disk(&self, records: &[HistoryRecord]) {
        let dir = match self.dir.lock().unwrap().as_ref() {
            Some(d) => d.clone(),
            None => return,
        };

        let json = serde_json::to_vec(records).unwrap_or_default();
        let compressed = lz4_flex::compress_prepend_size(&json);

        let crc = crc32fast::hash(&compressed);

        let mut bytes = Vec::with_capacity(MAGIC.len() + 2 + 4 + compressed.len());
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&compressed);

        let file_path = dir.join("combat_history.bin");
        if let Err(e) = write_atomic(&file_path, &bytes) {
            eprintln!("[history] failed to write {}: {}", file_path.display(), e);
        }
    }

    pub fn load_from_disk(&self) {
        let dir = match self.dir.lock().unwrap().as_ref() {
            Some(d) => d.clone(),
            None => return,
        };

        let file_path = dir.join("combat_history.bin");
        let data = match std::fs::read(&file_path) {
            Ok(d) => d,
            Err(_) => return,
        };

        if data.len() < 10 {
            return;
        }

        // Verify magic
        if &data[0..4] != MAGIC {
            eprintln!("[history] bad magic");
            return;
        }

        // Verify version
        let version = u16::from_le_bytes([data[4], data[5]]);
        if version != VERSION {
            eprintln!("[history] unsupported version: {}", version);
            return;
        }

        // Verify CRC
        let expected_crc = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
        let compressed = &data[10..];
        let actual_crc = crc32fast::hash(compressed);
        if actual_crc != expected_crc {
            eprintln!("[history] CRC mismatch");
            return;
        }

        // Decompress and parse
        match lz4_flex::decompress_size_prepended(compressed) {
            Ok(json) => {
                if let Ok(records) = serde_json::from_slice::<Vec<HistoryRecord>>(&json) {
                    let mut stored = self.records.lock().unwrap();
                    stored.clear();
                    stored.extend(records);
                }
            }
            Err(e) => {
                eprintln!("[history] decompress failed: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::session::SessionSummary;

    use super::*;

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

        assert!(store.save_and_clear(small).is_empty());
        assert!(store.get_records().is_empty());
    }

    #[test]
    fn one_record_is_deleted_by_id_and_the_others_stay() {
        let store = HistoryStore::new();
        store.save_and_clear(fight_on(&[1, 2, 3]));
        let doomed = ids(&store).remove(1);

        assert!(store.delete_record(&doomed));
        assert!(!store.delete_record(&doomed));
        assert_eq!(ids(&store).len(), 2);
        assert!(!ids(&store).contains(&doomed));
    }

    #[test]
    fn deleting_everything_reports_how_many_went() {
        let store = HistoryStore::new();
        store.save_and_clear(fight_on(&[1, 2, 3]));

        assert_eq!(store.clear_all(), 3);
        assert!(store.get_records().is_empty());
        assert_eq!(store.clear_all(), 0);
    }

    #[test]
    fn saved_history_loads_back_in_a_new_store_and_leaves_no_temporary_file() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = HistoryStore::new();
        store.init_dir(dir.path().to_path_buf());
        store.save_and_clear(fight_on(&[1, 2]));

        let reloaded = HistoryStore::new();
        reloaded.init_dir(dir.path().to_path_buf());
        reloaded.load_from_disk();

        assert_eq!(ids(&reloaded), ids(&store));
        assert_eq!(ids(&reloaded).len(), 2);
        assert!(!dir.path().join("history").join("combat_history.bin.tmp").exists());
    }

    #[test]
    fn an_interrupted_write_leaves_the_previous_history_loadable() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let store = HistoryStore::new();
        store.init_dir(dir.path().to_path_buf());
        store.save_and_clear(fight_on(&[1]));
        std::fs::write(dir.path().join("history").join("combat_history.bin.tmp"), b"half a wri").unwrap();

        let reloaded = HistoryStore::new();
        reloaded.init_dir(dir.path().to_path_buf());
        reloaded.load_from_disk();

        assert_eq!(ids(&reloaded), ids(&store));
        assert_eq!(ids(&reloaded).len(), 1);
    }
}
