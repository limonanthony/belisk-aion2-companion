//! Replays the recordings through the shipped `StreamAssembler` and lists the
//! skill ids the app would show as "Skill <id>": no key in the bundled skill
//! names matches them, even after the app's id normalisation.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use meter_core::capture::assembler::StreamAssembler;
use meter_core::config::DpsMeterConfig;
use meter_core::events::{EventSink, MeterEvent};
use meter_core::logger::AppLogger;
use meter_core::mobs::catalog::MobCatalog;
use meter_core::skill_names::resolve_skill_key;
use meter_core::storage::data_storage::DataStorage;
use serde::Serialize;

use crate::args::Args;
use crate::capture::{self, contains_heartbeat, RawFile};

const SKILL_NAMES_JSON: &str = include_str!("../../../app/assets/game/data/skill_names_en.json");

struct DiscardEvents;

impl EventSink for DiscardEvents {
    fn emit(&self, _event: MeterEvent) {}
}

#[derive(Default)]
struct SkillUse {
    hits: u64,
    damage: u64,
    classes: BTreeMap<String, u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UnnamedSkill {
    skill_id: u32,
    hits: u64,
    damage: u64,
    class: String,
}

fn is_named(names: &HashMap<String, String>, skill_id: u32) -> bool {
    names.contains_key(&resolve_skill_key(skill_id, |key| names.contains_key(key)))
}

fn skill_uses(raws: &[RawFile]) -> BTreeMap<u32, SkillUse> {
    let log = std::env::temp_dir().join("packet-explorer").join("core.log");
    let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
    let mut uses: BTreeMap<u32, SkillUse> = BTreeMap::new();

    for raw in raws {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = Arc::new(DataStorage::new(Arc::new(DiscardEvents), Arc::clone(&config), mobs));
        storage.set_fight_tracking(true);
        let mut assemblers: HashMap<usize, StreamAssembler> = HashMap::new();
        for chunk in &raw.chunks {
            if contains_heartbeat(&chunk.data) {
                assemblers.entry(chunk.flow).or_insert_with(|| {
                    StreamAssembler::new(
                        Arc::clone(&storage),
                        Arc::clone(&logger),
                        format!("flow{}", chunk.flow),
                        Arc::clone(&config),
                    )
                });
            }
            if let Some(assembler) = assemblers.get_mut(&chunk.flow) {
                assembler.process_chunk(&chunk.data, true, chunk.t);
            }
        }

        let classes = storage.actor_id_class_snapshot();
        for by_actor in storage.get_dps_stats_snapshot().values() {
            for (actor_id, by_skill) in by_actor {
                for (skill_id, stats) in by_skill {
                    let skill = uses.entry(*skill_id).or_default();
                    skill.hits += u64::from(stats.counts);
                    skill.damage += stats.total_damage;
                    let class = classes.get(actor_id).map_or("unknown", String::as_str);
                    *skill.classes.entry(class.to_string()).or_default() += u64::from(stats.counts);
                }
            }
        }
    }
    uses
}

fn unnamed_skills(uses: BTreeMap<u32, SkillUse>, names: &HashMap<String, String>) -> Vec<UnnamedSkill> {
    let mut rows: Vec<UnnamedSkill> = uses
        .into_iter()
        .filter(|(skill_id, _)| !is_named(names, *skill_id))
        .map(|(skill_id, skill)| UnnamedSkill {
            skill_id,
            hits: skill.hits,
            damage: skill.damage,
            class: skill
                .classes
                .into_iter()
                .max_by_key(|(_, hits)| *hits)
                .map_or_else(|| "unknown".to_string(), |(class, _)| class),
        })
        .collect();
    rows.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.skill_id.cmp(&b.skill_id)));
    rows
}

pub fn run(files: &[PathBuf], args: &Args) -> Result<(), String> {
    let raws = files
        .iter()
        .map(|path| capture::read_raw(path).map_err(|error| format!("{}: {error}", path.display())))
        .collect::<Result<Vec<_>, _>>()?;
    let names: HashMap<String, String> = serde_json::from_str(SKILL_NAMES_JSON)
        .map_err(|error| format!("the bundled skill names are not valid JSON: {error}"))?;
    let uses = skill_uses(&raws);
    let seen = uses.len();
    let rows = unnamed_skills(uses, &names);

    if args.flag("json") {
        return crate::output::print_json(&rows);
    }
    crate::out!("{:<12} {:>8} {:>14}  {}", "skill id", "hits", "damage", "class");
    for row in &rows {
        crate::out!("{:<12} {:>8} {:>14}  {}", row.skill_id, row.hits, row.damage, row.class);
    }
    crate::out!("{} of {seen} skill ids seen would show as \"Skill <id>\" (by damage)", rows.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(keys: &[&str]) -> HashMap<String, String> {
        keys.iter().map(|key| ((*key).to_string(), "Named".to_string())).collect()
    }

    fn uses(entries: &[(u32, u64, u64, &str)]) -> BTreeMap<u32, SkillUse> {
        entries
            .iter()
            .map(|(skill_id, hits, damage, class)| {
                let skill = SkillUse {
                    hits: *hits,
                    damage: *damage,
                    classes: BTreeMap::from([((*class).to_string(), *hits)]),
                };
                (*skill_id, skill)
            })
            .collect()
    }

    #[test]
    fn lists_only_ids_without_a_name_most_damage_first() {
        let known = names(&["16040000"]);
        let rows = unnamed_skills(
            uses(&[
                (16_040_000, 3, 900, "ELEMENTALIST"),
                (160_467_011, 2, 50, "ELEMENTALIST"),
                (4_000_000, 1, 70, "unknown"),
            ]),
            &known,
        );
        let ids: Vec<u32> = rows.iter().map(|row| row.skill_id).collect();
        assert_eq!(ids, vec![4_000_000, 160_467_011]);
    }

    #[test]
    fn an_id_the_app_normalises_to_a_named_key_is_not_listed() {
        let known = names(&["16040000"]);
        assert!(is_named(&known, 160_400_017));
        assert!(!is_named(&known, 160_500_017));
    }

    #[test]
    fn the_bundled_names_parse() {
        let parsed: HashMap<String, String> = serde_json::from_str(SKILL_NAMES_JSON).unwrap();
        assert!(is_named(&parsed, 100_001));
    }
}
