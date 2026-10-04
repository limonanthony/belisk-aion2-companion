//! What the meter knows about mobs: the bundled boss list and names, the user's
//! corrections, and, when collection is on, what fights revealed. The one place
//! that decides whether a mob is a boss.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::SharedDpsMeterConfig;
use crate::logger::AppLogger;
use crate::mobs::store::{self, BossCorrection, MobRecord, NameSource};
use crate::models::combat::TargetInfo;
use crate::storage::loaders::{load_boss_ids, load_npc_names};

pub const WRITE_INTERVAL: Duration = Duration::from_secs(30);

pub struct MobStorage {
    pub dir: PathBuf,
    pub logger: Arc<AppLogger>,
    pub write_interval: Duration,
}

impl MobStorage {
    pub fn new(dir: PathBuf, logger: Arc<AppLogger>) -> Self {
        Self {
            dir,
            logger,
            write_interval: WRITE_INTERVAL,
        }
    }
}

pub struct MobCatalog {
    shared: Arc<Shared>,
}

struct Shared {
    config: SharedDpsMeterConfig,
    bundled_bosses: HashSet<u32>,
    bundled_names: HashMap<u32, String>,
    records: RwLock<HashMap<u32, MobRecord>>,
    /// Mobs whose HP passed the boss threshold. Kept across resets: it is a
    /// fact about the mob, not about the fight.
    possible_bosses: RwLock<HashSet<u32>>,
    disk: Option<Disk>,
}

/// Saving happens on a thread of its own, so a slow disk never holds up
/// packet processing or snapshots; they only set a flag.
struct Disk {
    path: PathBuf,
    logger: Arc<AppLogger>,
    write_interval: Duration,
    pending: Mutex<Pending>,
    wake: Condvar,
    /// One write at a time: the temp file is shared.
    writing: Mutex<()>,
}

#[derive(Default)]
struct Pending {
    dirty: bool,
    shutdown: bool,
}

impl MobCatalog {
    /// With `storage`, loads `mobs.json` from its folder and saves changes
    /// back. Without, the catalog lives in memory only.
    pub fn open(config: SharedDpsMeterConfig, storage: Option<MobStorage>) -> Self {
        let mut records = HashMap::new();
        let disk = storage.map(|storage| {
            let path = storage.dir.join(store::FILE_NAME);
            let loaded = store::load(&path, now_secs());
            if let Some(notice) = &loaded.notice {
                storage.logger.error(format!("mobs.json: {notice}"));
            }
            records = loaded.records;
            Disk {
                path,
                logger: storage.logger,
                write_interval: storage.write_interval,
                pending: Mutex::new(Pending::default()),
                wake: Condvar::new(),
                writing: Mutex::new(()),
            }
        });

        let shared = Arc::new(Shared {
            config,
            bundled_bosses: load_boss_ids(),
            bundled_names: load_npc_names(),
            records: RwLock::new(records),
            possible_bosses: RwLock::new(HashSet::new()),
            disk,
        });
        if let Some(disk) = &shared.disk {
            let writer = Arc::clone(&shared);
            if let Err(error) = thread::Builder::new()
                .name("mob-store-writer".to_string())
                .spawn(move || writer.write_when_dirty())
            {
                disk.logger.error(format!(
                    "mobs.json: cannot start the writer thread ({error}); changes are saved only on exit"
                ));
            }
        }
        Self { shared }
    }

    /// The user's correction first, then the bundled list, then the HP rule
    /// (when "possible boss" is on).
    pub fn is_boss(&self, mob_code: u32) -> bool {
        let shared = &self.shared;
        let correction = shared
            .records
            .read()
            .unwrap()
            .get(&mob_code)
            .and_then(|record| record.boss_correction);
        match correction {
            Some(BossCorrection::NotBoss) => false,
            Some(BossCorrection::Boss) => true,
            None => {
                shared.bundled_bosses.contains(&mob_code)
                    || (shared.config.read().unwrap().show_possible_boss
                        && shared.possible_bosses.read().unwrap().contains(&mob_code))
            }
        }
    }

    pub fn boss_correction(&self, mob_code: u32) -> Option<BossCorrection> {
        self.shared
            .records
            .read()
            .unwrap()
            .get(&mob_code)
            .and_then(|record| record.boss_correction)
    }

    /// `None` returns the mob to automatic. Saved even when collection is off.
    pub fn set_boss_correction(&self, mob_code: u32, correction: Option<BossCorrection>) {
        {
            let mut records = self.shared.records.write().unwrap();
            let record = records.entry(mob_code).or_default();
            record.boss_correction = correction;
            if record.is_empty() {
                records.remove(&mob_code);
            }
        }
        self.shared.mark_dirty();
    }

    /// The bundled name, else one the user or a review supplied.
    pub fn mob_name(&self, mob_code: u32) -> Option<String> {
        self.shared.bundled_names.get(&mob_code).cloned().or_else(|| {
            self.shared
                .records
                .read()
                .unwrap()
                .get(&mob_code)
                .and_then(|record| record.name.clone())
        })
    }

    pub fn add_possible_boss(&self, mob_code: u32) {
        if !self.shared.config.read().unwrap().show_possible_boss {
            return;
        }
        if self.shared.possible_bosses.read().unwrap().contains(&mob_code) {
            return;
        }
        self.shared.possible_bosses.write().unwrap().insert(mob_code);
    }

    /// Learn from the mobs of a finished fight. Does nothing unless collection
    /// is on.
    pub fn record_fight(&self, targets: &HashMap<u32, TargetInfo>) {
        if !self.shared.config.read().unwrap().collect_mobs {
            return;
        }
        let now = now_secs();
        let mut changed = false;
        {
            let mut records = self.shared.records.write().unwrap();
            for target in targets.values() {
                let Some(mob_code) = target.target_mob_code else {
                    continue;
                };
                let record = records.entry(mob_code).or_default();
                record.times_fought += 1;
                record.max_hp_seen = record.max_hp_seen.max(target.max_hp);
                record.first_seen.get_or_insert(now);
                record.last_seen = Some(now);
                if record.name.is_none() {
                    if let Some(name) = self.shared.bundled_names.get(&mob_code) {
                        record.name = Some(name.clone());
                        record.name_source = Some(NameSource::Bundled);
                    }
                }
                changed = true;
            }
        }
        if changed {
            self.shared.mark_dirty();
        }
    }

    /// Write now on the calling thread, for quitting. Does nothing when there
    /// is nothing to save.
    pub fn save_now(&self) {
        let Some(disk) = &self.shared.disk else {
            return;
        };
        {
            let mut pending = disk.pending.lock().unwrap();
            if !pending.dirty {
                return;
            }
            pending.dirty = false;
        }
        self.shared.write_file(disk);
    }
}

impl Drop for MobCatalog {
    fn drop(&mut self) {
        if let Some(disk) = &self.shared.disk {
            disk.pending.lock().unwrap().shutdown = true;
            disk.wake.notify_all();
        }
    }
}

impl Shared {
    fn mark_dirty(&self) {
        if let Some(disk) = &self.disk {
            disk.pending.lock().unwrap().dirty = true;
            disk.wake.notify_all();
        }
    }

    /// Saves a change as soon as the last save is `write_interval` old, so a
    /// burst of changes costs one write. The first change after start-up goes
    /// out at once.
    fn write_when_dirty(&self) {
        let Some(disk) = &self.disk else {
            return;
        };
        let mut last_write: Option<Instant> = None;
        loop {
            let mut pending = disk.pending.lock().unwrap();
            while !pending.dirty && !pending.shutdown {
                pending = disk.wake.wait(pending).unwrap();
            }
            if !pending.dirty {
                return;
            }
            while !pending.shutdown {
                let due = last_write.map_or_else(Instant::now, |at| at + disk.write_interval);
                let Some(wait) = due.checked_duration_since(Instant::now()) else {
                    break;
                };
                pending = disk.wake.wait_timeout(pending, wait).unwrap().0;
            }
            pending.dirty = false;
            drop(pending);

            self.write_file(disk);
            last_write = Some(Instant::now());
        }
    }

    fn write_file(&self, disk: &Disk) {
        let _writing = disk.writing.lock().unwrap();
        let encoded = store::encode(&self.records.read().unwrap());
        let result = encoded
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                store::write_atomic(&disk.path, &bytes).map_err(|error| error.to_string())
            });
        if let Err(error) = result {
            disk.logger.error(format!(
                "mobs.json: cannot save {} ({error}); will retry",
                disk.path.display()
            ));
            self.mark_dirty();
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DpsMeterConfig;
    use crate::mobs::store::test_dir::TestDir;

    const UNLISTED: u32 = u32::MAX - 1;

    fn config(edit: impl FnOnce(&mut DpsMeterConfig)) -> SharedDpsMeterConfig {
        let mut config = DpsMeterConfig::default();
        edit(&mut config);
        Arc::new(RwLock::new(config))
    }

    fn memory_catalog(edit: impl FnOnce(&mut DpsMeterConfig)) -> MobCatalog {
        MobCatalog::open(config(edit), None)
    }

    fn disk_catalog(
        dir: &TestDir,
        write_interval: Duration,
        edit: impl FnOnce(&mut DpsMeterConfig),
    ) -> MobCatalog {
        let logger = Arc::new(AppLogger::new(dir.path().join("test.log"), Box::new(|_| {})));
        let mut storage = MobStorage::new(dir.path().to_path_buf(), logger);
        storage.write_interval = write_interval;
        MobCatalog::open(config(edit), Some(storage))
    }

    fn bundled_boss() -> u32 {
        *load_boss_ids().iter().min().expect("bundled bosses")
    }

    fn bundled_named_mob() -> u32 {
        let bosses = load_boss_ids();
        *load_npc_names()
            .keys()
            .filter(|code| !bosses.contains(code))
            .min()
            .expect("bundled names")
    }

    fn target(mob_code: u32, max_hp: u32) -> TargetInfo {
        TargetInfo {
            id: mob_code,
            target_mob_code: Some(mob_code),
            target_name: None,
            is_boss: false,
            current_hp: None,
            max_hp: Some(max_hp),
            target_start_time: HashMap::new(),
            target_last_time: HashMap::new(),
        }
    }

    fn fight(targets: &[TargetInfo]) -> HashMap<u32, TargetInfo> {
        targets.iter().map(|t| (t.id, t.clone())).collect()
    }

    fn saved(dir: &TestDir) -> Option<HashMap<u32, MobRecord>> {
        let path = dir.path().join(store::FILE_NAME);
        path.exists().then(|| store::load(&path, 0).records)
    }

    fn wait_for(what: &str, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn bundled_bosses_are_bosses_and_ordinary_mobs_are_not() {
        let catalog = memory_catalog(|_| {});

        assert!(catalog.is_boss(bundled_boss()));
        assert!(!catalog.is_boss(bundled_named_mob()));
    }

    #[test]
    fn the_hp_rule_applies_only_when_possible_bosses_are_shown() {
        let off = memory_catalog(|_| {});
        off.add_possible_boss(UNLISTED);
        assert!(!off.is_boss(UNLISTED));

        let on = memory_catalog(|c| c.show_possible_boss = true);
        assert!(!on.is_boss(UNLISTED));
        on.add_possible_boss(UNLISTED);
        assert!(on.is_boss(UNLISTED));
    }

    #[test]
    fn an_exclusion_beats_the_bundled_list_and_the_hp_rule() {
        let catalog = memory_catalog(|c| c.show_possible_boss = true);
        catalog.add_possible_boss(UNLISTED);

        catalog.set_boss_correction(bundled_boss(), Some(BossCorrection::NotBoss));
        catalog.set_boss_correction(UNLISTED, Some(BossCorrection::NotBoss));

        assert!(!catalog.is_boss(bundled_boss()));
        assert!(!catalog.is_boss(UNLISTED));
    }

    #[test]
    fn an_addition_makes_any_mob_a_boss_even_with_the_hp_rule_off() {
        let catalog = memory_catalog(|_| {});

        catalog.set_boss_correction(UNLISTED, Some(BossCorrection::Boss));

        assert!(catalog.is_boss(UNLISTED));
        assert_eq!(catalog.boss_correction(UNLISTED), Some(BossCorrection::Boss));
    }

    #[test]
    fn clearing_a_correction_returns_the_mob_to_the_automatic_rules() {
        let catalog = memory_catalog(|_| {});
        catalog.set_boss_correction(bundled_boss(), Some(BossCorrection::NotBoss));
        catalog.set_boss_correction(UNLISTED, Some(BossCorrection::Boss));

        catalog.set_boss_correction(bundled_boss(), None);
        catalog.set_boss_correction(UNLISTED, None);

        assert!(catalog.is_boss(bundled_boss()));
        assert!(!catalog.is_boss(UNLISTED));
        assert_eq!(catalog.boss_correction(UNLISTED), None);
    }

    #[test]
    fn names_come_from_the_bundle_first_then_from_mobs_json() {
        let dir = TestDir::new();
        let bundled = bundled_named_mob();
        let file = format!(
            r#"{{"version":1,"mobs":{{"{UNLISTED}":{{"name":"Invented Drake","nameSource":"user"}},"{bundled}":{{"name":"Overridden","nameSource":"user"}}}}}}"#
        );
        std::fs::write(dir.path().join(store::FILE_NAME), file).unwrap();

        let catalog = disk_catalog(&dir, WRITE_INTERVAL, |_| {});

        assert_eq!(catalog.mob_name(UNLISTED).as_deref(), Some("Invented Drake"));
        assert_eq!(
            catalog.mob_name(bundled),
            load_npc_names().get(&bundled).cloned()
        );
        assert_eq!(catalog.mob_name(UNLISTED - 1), None);
    }

    #[test]
    fn collection_off_records_no_fight_data() {
        let dir = TestDir::new();
        let catalog = disk_catalog(&dir, Duration::ZERO, |_| {});

        catalog.record_fight(&fight(&[target(UNLISTED, 5_000)]));
        catalog.save_now();

        assert_eq!(saved(&dir), None);
    }

    #[test]
    fn collection_on_records_counts_hp_and_times() {
        let dir = TestDir::new();
        let catalog = disk_catalog(&dir, Duration::ZERO, |c| c.collect_mobs = true);

        catalog.record_fight(&fight(&[target(UNLISTED, 5_000)]));
        catalog.record_fight(&fight(&[target(UNLISTED, 9_000), target(bundled_named_mob(), 100)]));
        catalog.save_now();

        wait_for("the file", || saved(&dir).is_some());
        let records = saved(&dir).unwrap();
        let unlisted = &records[&UNLISTED];
        assert_eq!(unlisted.times_fought, 2);
        assert_eq!(unlisted.max_hp_seen, Some(9_000));
        assert!(unlisted.first_seen.is_some() && unlisted.last_seen >= unlisted.first_seen);
        assert_eq!(unlisted.name, None);
        let named = &records[&bundled_named_mob()];
        assert_eq!(named.name_source, Some(NameSource::Bundled));
    }

    #[test]
    fn corrections_are_saved_with_collection_off() {
        let dir = TestDir::new();
        let catalog = disk_catalog(&dir, Duration::ZERO, |_| {});

        catalog.set_boss_correction(UNLISTED, Some(BossCorrection::Boss));
        catalog.save_now();

        wait_for("the file", || saved(&dir).is_some());
        assert_eq!(
            saved(&dir).unwrap()[&UNLISTED].boss_correction,
            Some(BossCorrection::Boss)
        );
        let reopened = disk_catalog(&dir, WRITE_INTERVAL, |_| {});
        assert!(reopened.is_boss(UNLISTED));
    }

    #[test]
    fn changes_are_saved_by_the_writer_thread_in_batches() {
        let dir = TestDir::new();
        let catalog = disk_catalog(&dir, Duration::from_millis(800), |_| {});

        catalog.set_boss_correction(1, Some(BossCorrection::Boss));
        wait_for("the first write", || saved(&dir).is_some());
        catalog.set_boss_correction(2, Some(BossCorrection::Boss));
        catalog.set_boss_correction(3, Some(BossCorrection::Boss));

        assert_eq!(saved(&dir).unwrap().len(), 1, "written again inside the interval");
        wait_for("the batched write", || saved(&dir).is_some_and(|r| r.len() == 3));
    }

    #[test]
    fn save_now_writes_pending_changes_without_waiting_for_the_interval() {
        let dir = TestDir::new();
        let catalog = disk_catalog(&dir, Duration::from_secs(3600), |_| {});
        catalog.set_boss_correction(1, Some(BossCorrection::Boss));
        wait_for("the first write", || saved(&dir).is_some());

        catalog.set_boss_correction(2, Some(BossCorrection::NotBoss));
        catalog.save_now();

        assert_eq!(saved(&dir).unwrap().len(), 2);
    }

    #[test]
    fn a_corrupt_file_is_set_aside_and_the_catalog_starts_empty() {
        let dir = TestDir::new();
        std::fs::write(dir.path().join(store::FILE_NAME), b"garbage").unwrap();

        let catalog = disk_catalog(&dir, WRITE_INTERVAL, |_| {});

        assert!(!catalog.is_boss(UNLISTED));
        let aside: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("mobs.json.bad-"))
            .collect();
        assert_eq!(aside.len(), 1);
        assert!(!dir.path().join(store::FILE_NAME).exists());
    }

    #[test]
    fn mob_files_hold_no_player_fields() {
        let dir = TestDir::new();
        let catalog = disk_catalog(&dir, Duration::ZERO, |c| c.collect_mobs = true);
        catalog.record_fight(&fight(&[target(UNLISTED, 1)]));
        catalog.save_now();
        wait_for("the file", || saved(&dir).is_some());

        let text = std::fs::read_to_string(dir.path().join(store::FILE_NAME)).unwrap();

        assert!(!text.to_lowercase().contains("player") && !text.to_lowercase().contains("actor"));
    }
}
