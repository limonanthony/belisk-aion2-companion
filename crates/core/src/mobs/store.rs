//! `mobs.json`: what the user told the meter about mobs, plus what it saw of
//! them while collection was on. Holds no player data, so it can be shared.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "mobs.json";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NameSource {
    Bundled,
    User,
    Reviewed,
}

/// The user's ruling on whether a mob is a boss, overriding every other rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BossCorrection {
    Boss,
    NotBoss,
}

/// Times are seconds since the epoch. The core does not learn which zone a
/// mob was met in, so there is no zone field.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_source: Option<NameSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hp_seen: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_seen: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<u64>,
    #[serde(default)]
    pub times_fought: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boss_correction: Option<BossCorrection>,
}

impl MobRecord {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileOut<'a> {
    version: u32,
    mobs: BTreeMap<u32, &'a MobRecord>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileIn {
    version: u32,
    #[serde(default)]
    mobs: BTreeMap<u32, MobRecord>,
}

pub struct Loaded {
    pub records: HashMap<u32, MobRecord>,
    /// Something the log should say: a file set aside, or one that could not
    /// be read.
    pub notice: Option<String>,
}

/// A missing file is a fresh start. A file that cannot be parsed, or that a
/// newer build wrote, is renamed to `mobs.json.bad-<seconds>` rather than
/// overwritten, and the meter starts empty.
pub fn load(path: &Path, now_secs: u64) -> Loaded {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Loaded {
                records: HashMap::new(),
                notice: None,
            }
        }
        Err(error) => {
            return Loaded {
                records: HashMap::new(),
                notice: Some(format!(
                    "cannot read {}: {error}; starting with no mob data",
                    path.display()
                )),
            }
        }
    };

    let problem = match serde_json::from_slice::<FileIn>(&bytes) {
        Ok(file) if file.version <= SCHEMA_VERSION => {
            return Loaded {
                records: file.mobs.into_iter().collect(),
                notice: None,
            }
        }
        Ok(file) => format!("schema version {} is newer than {SCHEMA_VERSION}", file.version),
        Err(error) => format!("not valid mob data: {error}"),
    };

    let aside = set_aside_path(path, now_secs);
    let notice = match fs::rename(path, &aside) {
        Ok(()) => format!(
            "{} {problem}; moved to {} and starting with no mob data",
            path.display(),
            aside.display()
        ),
        Err(error) => format!(
            "{} {problem}, and it could not be moved aside ({error}); starting with no mob data",
            path.display()
        ),
    };
    Loaded {
        records: HashMap::new(),
        notice: Some(notice),
    }
}

fn set_aside_path(path: &Path, now_secs: u64) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bad-{now_secs}"));
    path.with_file_name(name)
}

pub fn encode(records: &HashMap<u32, MobRecord>) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec_pretty(&FileOut {
        version: SCHEMA_VERSION,
        mobs: records.iter().map(|(code, record)| (*code, record)).collect(),
    })
}

/// Writes beside the target and renames over it, so a crash mid-write leaves
/// the previous file intact.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut temp_name = path.file_name().unwrap_or_default().to_os_string();
    temp_name.push(".tmp");
    let temp = path.with_file_name(temp_name);

    let mut file = fs::File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, path)
}

#[cfg(test)]
pub(crate) mod test_dir {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(0);

    pub struct TestDir(PathBuf);

    impl TestDir {
        pub fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "meter-core-mobs-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_dir::TestDir;
    use super::*;

    fn sample() -> HashMap<u32, MobRecord> {
        HashMap::from([
            (
                7,
                MobRecord {
                    name: Some("Invented Wolf".to_string()),
                    name_source: Some(NameSource::Reviewed),
                    max_hp_seen: Some(12_000),
                    first_seen: Some(100),
                    last_seen: Some(200),
                    times_fought: 3,
                    boss_correction: Some(BossCorrection::NotBoss),
                },
            ),
            (9, MobRecord::default()),
        ])
    }

    #[test]
    fn a_saved_file_loads_back_unchanged() {
        let dir = TestDir::new();
        let path = dir.path().join(FILE_NAME);

        write_atomic(&path, &encode(&sample()).unwrap()).unwrap();
        let loaded = load(&path, 1);

        assert_eq!(loaded.records, sample());
        assert_eq!(loaded.notice, None);
        assert!(!dir.path().join("mobs.json.tmp").exists());
    }

    #[test]
    fn the_file_carries_a_version_and_keys_mobs_by_code() {
        let json: serde_json::Value = serde_json::from_slice(&encode(&sample()).unwrap()).unwrap();

        assert_eq!(json["version"], SCHEMA_VERSION);
        assert_eq!(json["mobs"]["7"]["bossCorrection"], "notBoss");
        assert_eq!(json["mobs"]["7"]["nameSource"], "reviewed");
        assert!(json["mobs"]["9"].get("name").is_none());
    }

    #[test]
    fn a_missing_file_is_a_fresh_start() {
        let dir = TestDir::new();

        let loaded = load(&dir.path().join(FILE_NAME), 1);

        assert!(loaded.records.is_empty());
        assert_eq!(loaded.notice, None);
    }

    #[test]
    fn a_corrupt_file_is_set_aside_and_reported() {
        let dir = TestDir::new();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, b"{ not json").unwrap();

        let loaded = load(&path, 1_234);

        assert!(loaded.records.is_empty());
        assert!(loaded.notice.unwrap().contains("mobs.json.bad-1234"));
        assert!(!path.exists());
        assert_eq!(
            fs::read(dir.path().join("mobs.json.bad-1234")).unwrap(),
            b"{ not json"
        );
    }

    #[test]
    fn a_file_from_a_newer_build_is_set_aside_too() {
        let dir = TestDir::new();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, br#"{"version": 99, "mobs": {}}"#).unwrap();

        let loaded = load(&path, 5);

        assert!(loaded.notice.unwrap().contains("newer"));
        assert!(dir.path().join("mobs.json.bad-5").exists());
    }

    #[test]
    fn a_write_replaces_the_previous_file() {
        let dir = TestDir::new();
        let path = dir.path().join(FILE_NAME);
        write_atomic(&path, &encode(&sample()).unwrap()).unwrap();

        write_atomic(&path, &encode(&HashMap::new()).unwrap()).unwrap();

        assert!(load(&path, 1).records.is_empty());
    }
}
