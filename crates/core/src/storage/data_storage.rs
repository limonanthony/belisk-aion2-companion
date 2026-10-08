use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

use serde::Serialize;
use std::sync::Arc;

use crate::config::{SharedDpsMeterConfig, TRAINING_DUMMY_MOB_CODE};
use crate::events::{EventSink, MeterEvent};
use crate::identity::{character_of_class, IdentityDecision, KnownCharacter};
use crate::models::combat::{
    BuffInterval, BuffSummary, PlayerHpInfo, PvpCombatStats, PvpCombatStatsRow, PvpKnownPlayer,
    PvpWatchInfo, PvpWatchInfoResponse, SkillStats,
};
use crate::models::packet::ParsedDamagePacket;
use crate::mobs::catalog::MobCatalog;
use crate::report::events::{CombatEventLog, CombatEvents, Heal, HealKind, IncomingHit, Killer};
use crate::report::ledger::{fight_party, BuffIntervals};
use crate::storage::loaders::load_healing_skill_codes;

pub(crate) struct FightLedgerInputs {
    pub events: CombatEvents,
    pub names: HashMap<u32, String>,
    pub servers: HashMap<u32, String>,
    pub classes: HashMap<u32, String>,
    pub buff_intervals: BuffIntervals,
}

const ACTOR_METADATA_CAPACITY: usize = 2_000;
const MOB_METADATA_CAPACITY: usize = 5_000;
/// Targets kept in the combat totals. Far more than any encounter needs, and
/// still small enough that cloning the map stays cheap.
const COMBAT_TARGET_CAPACITY: usize = 512;
const SUMMON_METADATA_CAPACITY: usize = 5_000;

const BUFF_TARGET_CAPACITY: usize = 1_024;
const BUFF_INTERVALS_PER_SKILL_CAPACITY: usize = 1_024;
const BUFF_INSTANCE_CAPACITY: usize = 4_096;
const BUFF_INTERVAL_MERGE_TOLERANCE_MS: u64 = 100;

const ACTOR_CLASS_SKILL_IGNORE_LIST: [&str; 2] = ["1134", "1740"];
/// Fighter rage variants mapped to the skill they belong to, as
/// `(rage_id, base_id)`. Both are counted as one skill in the meter.
///
/// Listed by id: the English skill catalogue has no names for these yet, so a
/// name here would have to be Chinese and would go stale the moment it does.
const FIGHTER_SKILLID_MAP: &[(u32, u32)] = &[
    (19_080_000, 19_070_000),
    (19_100_000, 19_090_000),
    (19_120_000, 19_110_000),
    (19_150_000, 19_160_000),
    (19_180_000, 19_170_000),
    (19_190_000, 19_200_000),
    (19_260_000, 19_250_000),
    (19_430_000, 19_420_000),
    (19_470_000, 19_460_000),
    (19_510_000, 19_010_000),
    (19_520_000, 19_020_000),
    (19_530_000, 19_030_000),
    (19_540_000, 19_040_000),
    (19_550_000, 19_050_000),
    (19_560_000, 19_060_000),
];

#[derive(Debug, Clone)]
struct BoundedMap<K, V> {
    map: HashMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K, V> BoundedMap<K, V>
where
    K: Eq + Hash + Clone,
{
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.order.retain(|existing| existing != &key);
        self.order.push_back(key.clone());
        let previous = self.map.insert(key, value);
        self.evict_oldest();
        previous
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    fn get_mut_or_insert_with<F>(&mut self, key: K, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        if self.map.contains_key(&key) {
            self.order.retain(|existing| existing != &key);
            self.order.push_back(key.clone());
        } else {
            self.order.push_back(key.clone());
            self.map.insert(key.clone(), default());
            self.evict_oldest();
        }

        self.map
            .get_mut(&key)
            .expect("bounded map entry must exist after insert")
    }

    fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.order.retain(|existing| existing != key);
        self.map.remove(key)
    }

    fn as_hash_map(&self) -> HashMap<K, V>
    where
        V: Clone,
    {
        self.map.clone()
    }

    fn evict_oldest(&mut self) {
        while self.map.len() > self.capacity {
            let Some(oldest_key) = self.order.pop_front() else {
                break;
            };
            self.map.remove(&oldest_key);
        }
    }
}

/// How the local player's actor and name were decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnSource {
    Unknown,
    Signature,
    Remembered,
    Game,
}

/// Whether the own actor's name still waits for a class from its skills.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnNamePending {
    No,
    /// Found by its signature with no name.
    Nameless,
    /// Carried over from an earlier connection; `class` is the one the
    /// carried character had there.
    Carried { class: Option<String> },
}

#[derive(Debug)]
struct DataStorageInner {
    /// Combat totals, keyed by target.
    ///
    /// Bounded, unlike the plain map this used to be. Every other metadata map
    /// here was already bounded; this one was not, and it is deep-cloned five
    /// times a second to build the overlay snapshot. While the meter only
    /// counted bosses that was a handful of entries. Counting ordinary mobs as
    /// well -- which is now the default -- means a grinding session would add a
    /// target per kill and make every snapshot fractionally more expensive than
    /// the last, forever.
    dps_stats: BoundedMap<u32, HashMap<u32, HashMap<u32, SkillStats>>>,
    actor_id_name_map: BoundedMap<u32, String>,
    actor_id_server_map: BoundedMap<u32, String>,
    actor_id_class_map: BoundedMap<u32, String>,
    actor_id_combat_power_map: BoundedMap<u32, u64>,
    actor_id_skill_spec_map: BoundedMap<u32, HashMap<u32, Vec<u32>>>,
    mob_id_code_map: BoundedMap<u32, u32>,
    mob_id_hp_map: BoundedMap<u32, (u64, u64)>,
    player_hp_map: BoundedMap<u32, PlayerHpInfo>,
    use_buffs_by_target: BoundedMap<u32, HashMap<u32, HashMap<u32, VecDeque<BuffInterval>>>>,
    /// The buff a `(target, instance id)` pair stands for, so a removal packet
    /// that names only the instance can end the right interval.
    buff_instances: BoundedMap<(u32, u32), BuffInstance>,
    summon_owner_map: BoundedMap<u32, u32>,
    start_time: Option<f64>,
    /// Last time the player running Aether was in the fight: hit something,
    /// was hit, or their target took damage from the party. `None` until the
    /// first such hit after a reset. Drives the overlay's auto-hide and the
    /// idle reset, so fights nearby that you are not part of move neither.
    activity_at: Option<f64>,
    start_time_by_target: HashMap<u32, HashMap<u32, f64>>,
    last_time_by_target: HashMap<u32, HashMap<u32, f64>>,
    dot_skill_list: Vec<u32>,
    main_actor_id: Option<u32>,
    /// Every actor id established as the local player's since the character
    /// was identified, oldest first: the game gives the character a new id in
    /// each zone.
    own_actor_ids: Vec<u32>,
    main_actor_name: Option<String>,
    main_actor_combat_power: Option<u64>,
    own_source: OwnSource,
    /// The own actor's name is unnamed or only carried over, and may be settled
    /// once its skills show a class.
    own_name_pending: OwnNamePending,
    last_target: Option<u32>,
    last_target_by_main_actor: Option<u32>,
    last_player_target_by_main_actor: Option<u32>,
    pvp_attackers_by_target: HashMap<u32, HashMap<PvpPlayerKey, u64>>,
    pvp_last_attacker_by_target: HashMap<u32, PvpPlayerKey>,
    pvp_combat_stats: HashMap<PvpPlayerKey, PvpCombatStats>,
    pvp_dead_players: HashSet<PvpPlayerKey>,
    /// Hits taken, heals and deaths, for the report of a boss fight.
    combat_events: CombatEventLog,
}

#[derive(Debug, Clone, Copy)]
struct BuffInstance {
    actor_id: u32,
    skill_code: u32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct PvpPlayerKey {
    name: String,
    server_id: String,
}

impl Default for DataStorageInner {
    fn default() -> Self {
        Self {
            dps_stats: BoundedMap::new(COMBAT_TARGET_CAPACITY),
            actor_id_name_map: BoundedMap::new(ACTOR_METADATA_CAPACITY),
            actor_id_server_map: BoundedMap::new(ACTOR_METADATA_CAPACITY),
            actor_id_class_map: BoundedMap::new(ACTOR_METADATA_CAPACITY),
            actor_id_combat_power_map: BoundedMap::new(ACTOR_METADATA_CAPACITY),
            actor_id_skill_spec_map: BoundedMap::new(ACTOR_METADATA_CAPACITY),
            mob_id_code_map: BoundedMap::new(MOB_METADATA_CAPACITY),
            mob_id_hp_map: BoundedMap::new(MOB_METADATA_CAPACITY),
            player_hp_map: BoundedMap::new(ACTOR_METADATA_CAPACITY),
            use_buffs_by_target: BoundedMap::new(BUFF_TARGET_CAPACITY),
            buff_instances: BoundedMap::new(BUFF_INSTANCE_CAPACITY),
            summon_owner_map: BoundedMap::new(SUMMON_METADATA_CAPACITY),
            start_time: None,
            activity_at: None,
            start_time_by_target: HashMap::new(),
            last_time_by_target: HashMap::new(),
            dot_skill_list: Vec::new(),
            main_actor_id: None,
            own_actor_ids: Vec::new(),
            main_actor_name: None,
            main_actor_combat_power: None,
            own_source: OwnSource::Unknown,
            own_name_pending: OwnNamePending::No,
            last_target: None,
            last_target_by_main_actor: None,
            last_player_target_by_main_actor: None,
            pvp_attackers_by_target: HashMap::new(),
            pvp_last_attacker_by_target: HashMap::new(),
            pvp_combat_stats: HashMap::new(),
            pvp_dead_players: HashSet::new(),
            combat_events: CombatEventLog::default(),
        }
    }
}

pub type MainActorCallback = dyn Fn(u32, &str, Option<String>) + Send + Sync;

pub struct DataStorage {
    events: Arc<dyn EventSink>,
    inner: RwLock<DataStorageInner>,
    config: SharedDpsMeterConfig,
    healing_skill_codes: HashSet<u32>,
    mobs: Arc<MobCatalog>,
    pub main_actor_callback: Mutex<Option<Box<MainActorCallback>>>,
    /// Characters the player was seen as in earlier runs, most recent first.
    /// Kept apart from `inner`: a reset must not forget them.
    known_characters: RwLock<Vec<KnownCharacter>>,
    /// Counts what the Boss only filter threw away, so the UI can say so.
    boss_only_filtered: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainActorDetectedPayload {
    pub actor_id: u32,
    pub actor_name: String,
    pub sid: Option<String>,
}

/// Boss adds are summons owned by the boss, so their hits are damage taken.
/// Only a summon whose owner chain ends at a player is that player's pet.
fn is_summon_of_a_player(inner: &DataStorageInner, source_id: u32) -> bool {
    const OWNER_CHAIN_DEPTH: usize = 4;
    let mut current = source_id;
    for _ in 0..OWNER_CHAIN_DEPTH {
        match inner.summon_owner_map.get(&current) {
            Some(&owner_id) => current = owner_id,
            None => break,
        }
    }
    current != source_id
        && inner.actor_id_name_map.contains_key(&current)
        && !inner.mob_id_code_map.contains_key(&current)
}

impl DataStorage {
    pub fn new(
        events: Arc<dyn EventSink>,
        config: SharedDpsMeterConfig,
        mobs: Arc<MobCatalog>,
    ) -> Self {
        Self {
            events,
            inner: RwLock::new(DataStorageInner::default()),
            config,
            healing_skill_codes: load_healing_skill_codes(),
            mobs,
            main_actor_callback: Mutex::new(None),
            known_characters: RwLock::new(Vec::new()),
            boss_only_filtered: AtomicU64::new(0),
        }
    }

    pub fn clear(&self) {
        self.boss_only_filtered.store(0, Ordering::Relaxed);
        let mut inner = self.inner.write().unwrap();
        let main_actor_id = inner.main_actor_id;
        let own_actor_ids = std::mem::take(&mut inner.own_actor_ids);
        let main_actor_name = inner.main_actor_name.clone();
        let main_actor_combat_power = inner.main_actor_combat_power;
        let own_source = inner.own_source;
        let own_name_pending = inner.own_name_pending.clone();
        let actor_id_name_map = inner.actor_id_name_map.clone();
        let actor_id_server_map = inner.actor_id_server_map.clone();
        let actor_id_class_map = inner.actor_id_class_map.clone();
        let actor_id_combat_power_map = inner.actor_id_combat_power_map.clone();
        let actor_id_skill_spec_map = inner.actor_id_skill_spec_map.clone();

        let mob_id_code_map = inner.mob_id_code_map.clone();
        let mob_id_hp_map = inner.mob_id_hp_map.clone();
        let player_hp_map = inner.player_hp_map.clone();
        let pvp_attackers_by_target = inner.pvp_attackers_by_target.clone();
        let pvp_last_attacker_by_target = inner.pvp_last_attacker_by_target.clone();
        let pvp_combat_stats = inner.pvp_combat_stats.clone();
        let pvp_dead_players = inner.pvp_dead_players.clone();
        // let summon_owner_map = inner.summon_owner_map.clone();
        let dot_skill_list = inner.dot_skill_list.clone();

        *inner = DataStorageInner::default();
        inner.main_actor_id = main_actor_id;
        inner.own_actor_ids = own_actor_ids;
        inner.main_actor_name = main_actor_name;
        inner.main_actor_combat_power = main_actor_combat_power;
        inner.own_source = own_source;
        inner.own_name_pending = own_name_pending;
        inner.actor_id_name_map = actor_id_name_map;
        inner.actor_id_server_map = actor_id_server_map;
        inner.actor_id_class_map = actor_id_class_map;
        inner.actor_id_combat_power_map = actor_id_combat_power_map;
        inner.actor_id_skill_spec_map = actor_id_skill_spec_map;

        inner.mob_id_code_map = mob_id_code_map;
        inner.mob_id_hp_map = mob_id_hp_map;
        inner.player_hp_map = player_hp_map;
        inner.pvp_attackers_by_target = pvp_attackers_by_target;
        inner.pvp_last_attacker_by_target = pvp_last_attacker_by_target;
        inner.pvp_combat_stats = pvp_combat_stats;
        inner.pvp_dead_players = pvp_dead_players;
        // inner.summon_owner_map = summon_owner_map;
        inner.dot_skill_list = dot_skill_list;
    }

    /// Hits thrown away by the Boss only filter since the last reset.
    pub fn boss_only_filtered(&self) -> u64 {
        self.boss_only_filtered.load(Ordering::Relaxed)
    }

    pub fn append_damage(&self, packet: ParsedDamagePacket) {
        self.append_damage_at(packet, current_timestamp_seconds());
    }

    pub fn save_buff(
        &self,
        target_id: u32,
        instance_id: u32,
        actor_id: u32,
        skill_code: u32,
        _server_start_ms: u64,
        duration_ms: u64,
    ) -> BuffSummary {
        let local_start_ms = current_timestamp_millis();
        let local_end_ms = local_start_ms.saturating_add(duration_ms);

        // Buff intervals feed the detail window's coverage timeline. The live
        // buff overlay that also listened here was removed in 2.2.0.
        {
            let mut inner = self.inner.write().unwrap();
            let skill_intervals = inner
                .use_buffs_by_target
                .get_mut_or_insert_with(target_id, HashMap::new)
                .entry(actor_id)
                .or_default()
                .entry(skill_code)
                .or_default();
            if let Some(last_interval) = skill_intervals.back_mut() {
                if local_start_ms
                    <= last_interval
                        .end_ms
                        .saturating_add(BUFF_INTERVAL_MERGE_TOLERANCE_MS)
                {
                    last_interval.end_ms = last_interval.end_ms.max(local_end_ms);
                } else {
                    skill_intervals.push_back(BuffInterval {
                        start_ms: local_start_ms,
                        end_ms: local_end_ms,
                    });
                }
            } else {
                skill_intervals.push_back(BuffInterval {
                    start_ms: local_start_ms,
                    end_ms: local_end_ms,
                });
            }
            while skill_intervals.len() > BUFF_INTERVALS_PER_SKILL_CAPACITY {
                skill_intervals.pop_front();
            }
            inner
                .buff_instances
                .insert((target_id, instance_id), BuffInstance { actor_id, skill_code });
        }

        BuffSummary {
            target_id,
            actor_id,
            skill_code,
            coverage: 0.0,
            active: local_end_ms > current_timestamp_millis(),
            last_start_ms: local_start_ms,
            last_end_ms: local_end_ms,
        }
    }

    /// The game ended a buff instance before its duration ran out. Its
    /// interval stops now, unless another instance of the same buff on the
    /// same target is still running.
    pub fn end_buff(&self, target_id: u32, instance_id: u32) {
        let ended_at_ms = current_timestamp_millis();
        let mut inner = self.inner.write().unwrap();
        let Some(ended) = inner.buff_instances.remove(&(target_id, instance_id)) else {
            return;
        };
        let another_instance_runs = inner.buff_instances.map.iter().any(|((target, _), other)| {
            *target == target_id
                && other.actor_id == ended.actor_id
                && other.skill_code == ended.skill_code
        });
        if another_instance_runs {
            return;
        }
        let Some(last_interval) = inner
            .use_buffs_by_target
            .get_mut(&target_id)
            .and_then(|by_actor| by_actor.get_mut(&ended.actor_id))
            .and_then(|by_skill| by_skill.get_mut(&ended.skill_code))
            .and_then(|intervals| intervals.back_mut())
        else {
            return;
        };
        last_interval.end_ms = last_interval.end_ms.min(ended_at_ms);
    }

    pub fn append_damage_at(&self, packet: ParsedDamagePacket, timestamp: f64) {
        let config = self.config.read().unwrap().clone();
        let mut inner = self.inner.write().unwrap();

        if self.healing_skill_codes.contains(&packet.skill_code) {
            let skill_code = if (10_000_000..=99_999_999).contains(&packet.ori_skill_code) {
                packet.skill_code
            } else {
                packet.ori_skill_code
            };
            record_heal(
                &mut inner,
                Heal {
                    at: timestamp,
                    healer_id: packet.actor_id,
                    target_id: packet.target_id,
                    skill_code,
                    kind: HealKind::Direct,
                    amount: packet.damage,
                    critical: packet.is_crit,
                },
            );
            return;
        }

        if packet.actor_id == 0 {
            return;
        }

        // let actor_id = packet.actor_id;
        // A summon's damage is attributed to whoever summoned it.
        let mut actor_id = packet.actor_id;
        if let Some(owner_id) = inner.summon_owner_map.get(&actor_id) {
            actor_id = *owner_id;
        }
        if actor_id == 0 {
            return;
        }

        let target_mob_code = inner.mob_id_code_map.get(&packet.target_id).copied();
        let is_target_boss = target_mob_code.is_some_and(|mob_code| self.mobs.is_boss(mob_code));
        let is_target_player = inner.actor_id_name_map.contains_key(&packet.target_id)
            && !inner.mob_id_code_map.contains_key(&packet.target_id);

        if config.boss_only && !is_target_boss && !(config.pvp_mode_on && is_target_player) {
            self.boss_only_filtered.fetch_add(1, Ordering::Relaxed);
            return;
        }

        if config.my_muzhuang_only
            && target_mob_code.is_some_and(|mob_code| TRAINING_DUMMY_MOB_CODE.contains(&mob_code))
            && inner.main_actor_id != Some(actor_id)
        {
            return;
        }

        if config.pvp_mode_on && is_target_player {
            let actor = pvp_player_key(&inner, actor_id);
            let target = pvp_player_key(&inner, packet.target_id);
            if let (Some(actor), Some(target)) = (actor, target) {
                if actor != target {
                    *inner
                        .pvp_attackers_by_target
                        .entry(packet.target_id)
                        .or_default()
                        .entry(actor.clone())
                        .or_default() += packet.damage;
                    inner
                        .pvp_last_attacker_by_target
                        .insert(packet.target_id, actor);
                }
            }
        }

        if packet.is_dot && !inner.dot_skill_list.contains(&packet.skill_code) {
            inner.dot_skill_list.push(packet.skill_code);
        }

        let mut own_class_learned = false;
        if !inner.actor_id_class_map.contains_key(&actor_id) {
            if let Some(actor_class) = infer_actor_class(packet.skill_code) {
                inner.actor_id_class_map.insert(actor_id, actor_class);
                own_class_learned = inner.own_name_pending != OwnNamePending::No && inner.main_actor_id == Some(actor_id);
            }
        }

        if inner.start_time.is_none() {
            inner.start_time = Some(timestamp);
        }

        inner
            .actor_id_skill_spec_map
            .get_mut_or_insert_with(actor_id, HashMap::new)
            .entry(packet.skill_code)
            .or_insert_with(|| infer_specialty_slots(packet.ori_skill_code));

        inner
            .start_time_by_target
            .entry(packet.target_id)
            .or_default()
            .entry(actor_id)
            .or_insert(timestamp);
        inner
            .last_time_by_target
            .entry(packet.target_id)
            .or_default()
            .insert(actor_id, timestamp);

        let mut special_counts = HashMap::new();
        for special in &packet.specials {
            *special_counts.entry(special.clone()).or_insert(0) += 1;
        }
        if packet.multi_hit_count > 0 {
            special_counts.insert("MULTIHIT".to_string(), 1);
            special_counts.insert(format!("MULTIHIT{}", packet.multi_hit_count), 1);
        } else {
            special_counts.insert("MULTIHIT".to_string(), 0);
        }
        special_counts.insert("MULTIHITDMG".to_string(), packet.multi_hit_damage as u32);

        let total_damage = packet.damage;

        // An 8-digit skill id is a regular skill, so use the normalised code.
        // Longer ids -- usually 10 digits -- are DoT skills, and keep their
        // original code so the two stay distinguishable.
        let mut stats_skill_code = if (10_000_000..=99_999_999).contains(&packet.ori_skill_code) {
            packet.skill_code
        } else {
            packet.ori_skill_code
        };
        // Fighter's rage variants are folded back onto the base skill.
        if let Some((_, mapped_skill_code)) = FIGHTER_SKILLID_MAP
            .iter()
            .find(|(from_skill_code, _)| *from_skill_code == stats_skill_code)
        {
            stats_skill_code = *mapped_skill_code;
        }
        // Elementalist summon basic attacks collapse onto one code.
        if (100_010..=100_059).contains(&stats_skill_code)
            && (10001..=10005).contains(&(stats_skill_code / 10))
        {
            stats_skill_code = (stats_skill_code / 10) * 10;
        }

        let skill_stats = inner
            .dps_stats
            .get_mut_or_insert_with(packet.target_id, Default::default)
            .entry(actor_id)
            .or_default()
            .entry(stats_skill_code)
            .or_insert_with(SkillStats::new);

        skill_stats.counts += 1;
        skill_stats.total_damage += total_damage;
        skill_stats.max_damage = skill_stats.max_damage.max(total_damage);
        skill_stats.min_damage = skill_stats.min_damage.min(total_damage);
        for (special_name, count) in &special_counts {
            *skill_stats
                .special_counts
                .entry(special_name.clone())
                .or_insert(0) += count;
        }

        if !inner.mob_id_code_map.contains_key(&packet.actor_id) {
            for (kind, amount) in [
                (HealKind::Drain, packet.drain_amount),
                (HealKind::Regeneration, packet.regeneration_amount),
            ] {
                record_heal(
                    &mut inner,
                    Heal {
                        at: timestamp,
                        healer_id: actor_id,
                        target_id: actor_id,
                        skill_code: stats_skill_code,
                        kind,
                        amount,
                        critical: false,
                    },
                );
            }
        }

        let hit_on_own_summon = inner.summon_owner_map.get(&packet.target_id) == Some(&actor_id);
        if actor_id != packet.target_id && !hit_on_own_summon {
            inner.last_target = Some(packet.target_id);
            if inner.main_actor_id == Some(actor_id) {
                inner.last_target_by_main_actor = Some(packet.target_id);
                if is_target_player {
                    inner.last_player_target_by_main_actor = Some(packet.target_id);
                }
            }
        }

        if is_own_fight(&inner, actor_id, packet.target_id) {
            inner.activity_at = Some(timestamp);
        }

        drop(inner);
        if own_class_learned {
            self.resolve_own_name();
        }
    }

    /// A hit a monster's skill landed on `victim_id`, by a record or a damage
    /// tick. It is not damage dealt: it moves no target and no stats.
    pub fn record_incoming_hit(&self, victim_id: u32, source_id: u32, skill_code: u32, amount: u64) {
        self.record_incoming_hit_at(victim_id, source_id, skill_code, amount, current_timestamp_seconds());
    }

    pub fn record_incoming_hit_at(&self, victim_id: u32, source_id: u32, skill_code: u32, amount: u64, at: f64) {
        let mut inner = self.inner.write().unwrap();
        let from_a_player = inner.actor_id_name_map.contains_key(&source_id) && !inner.mob_id_code_map.contains_key(&source_id);
        if amount == 0
            || victim_id == source_id
            || from_a_player
            || inner.mob_id_code_map.contains_key(&victim_id)
            || is_summon_of_a_player(&inner, source_id)
        {
            return;
        }
        let source_mob_code = inner.mob_id_code_map.get(&source_id).copied();
        inner.combat_events.record_hit(IncomingHit { at, victim_id, source_id, source_mob_code, skill_code, amount });
    }

    /// A tick of a heal over time.
    pub fn record_heal_tick(&self, healer_id: u32, target_id: u32, skill_code: u32, amount: u64) {
        self.record_heal_tick_at(healer_id, target_id, skill_code, amount, current_timestamp_seconds());
    }

    pub fn record_heal_tick_at(&self, healer_id: u32, target_id: u32, skill_code: u32, amount: u64, at: f64) {
        let heal = Heal { at, healer_id, target_id, skill_code, kind: HealKind::OverTime, amount, critical: false };
        record_heal(&mut self.inner.write().unwrap(), heal);
    }

    /// The game's record of a player's death. `killer_name` is empty unless the
    /// killer is a player; `killer_mob_code` is 0 unless it names a monster.
    pub fn record_player_death(
        &self,
        victim_id: u32,
        skill_code: u32,
        killer_id: u32,
        killer_name: &str,
        killer_mob_code: u32,
    ) {
        self.record_player_death_at(victim_id, skill_code, killer_id, killer_name, killer_mob_code, current_timestamp_seconds());
    }

    pub fn record_player_death_at(
        &self,
        victim_id: u32,
        skill_code: u32,
        killer_id: u32,
        killer_name: &str,
        killer_mob_code: u32,
        at: f64,
    ) {
        let mut inner = self.inner.write().unwrap();
        if inner.mob_id_code_map.contains_key(&victim_id) {
            return;
        }
        let mob_code = if killer_name.is_empty() {
            Some(killer_mob_code)
                .filter(|code| *code != 0)
                .or_else(|| inner.mob_id_code_map.get(&killer_id).copied())
        } else {
            None
        };
        let killer = Killer { id: killer_id, name: Some(killer_name.to_string()).filter(|name| !name.is_empty()), mob_code };
        inner.combat_events.record_death(at, victim_id, skill_code, killer);
    }

    /// The hits taken, heals and deaths from `from` to `to`, in seconds since
    /// the epoch.
    pub fn combat_events(&self, from: f64, to: f64) -> CombatEvents {
        self.inner.read().unwrap().combat_events.between(from, to)
    }

    /// Everything a boss fight ledger reads from storage, copied in one short
    /// read lock: the combat events of the window and the buffs on the target
    /// and on the party only, not on every actor ever buffed.
    pub(crate) fn fight_ledger_inputs(
        &self,
        target_id: u32,
        damage_dealers: &[u32],
        (from, to): (f64, f64),
    ) -> FightLedgerInputs {
        let inner = self.inner.read().unwrap();
        let events = inner.combat_events.between(from, to);
        let buff_intervals = fight_party(damage_dealers, &events.heals)
            .into_iter()
            .chain([target_id])
            .filter_map(|buffed| {
                inner.use_buffs_by_target.get(&buffed).map(|casters| (buffed, casters.clone()))
            })
            .collect();
        FightLedgerInputs {
            events,
            names: inner.actor_id_name_map.as_hash_map(),
            servers: inner.actor_id_server_map.as_hash_map(),
            classes: inner.actor_id_class_map.as_hash_map(),
            buff_intervals,
        }
    }

    /// Last time you were in the fight; see `DataStorageInner::activity_at`.
    pub fn activity_at(&self) -> Option<f64> {
        self.inner.read().unwrap().activity_at
    }

    pub fn append_actor(&self, actor_id: u32, actor_name: &str, sid: Option<&str>) {
        let mut inner = self.inner.write().unwrap();
        inner
            .actor_id_name_map
            .insert(actor_id, actor_name.to_string());
        if let Some(sid) = sid {
            inner.actor_id_server_map.insert(actor_id, sid.to_string());
        }
        inner.summon_owner_map.map.remove(&actor_id);
    }

    pub fn set_actor_class(&self, actor_id: u32, actor_class: &str) {
        self.inner
            .write()
            .unwrap()
            .actor_id_class_map
            .insert(actor_id, actor_class.to_string());
    }

    pub fn set_actor_combat_power(&self, actor_id: u32, combat_power: u64) {
        if combat_power == 0 || combat_power > 10_000_000 {
            return;
        }

        self.inner
            .write()
            .unwrap()
            .actor_id_combat_power_map
            .insert(actor_id, combat_power);
    }

    pub fn save_main_actor_combat_power(&self, combat_power: u64) -> bool {
        if combat_power == 0 || combat_power > 10_000_000 {
            return false;
        }

        self.inner.write().unwrap().main_actor_combat_power = Some(combat_power);
        true
    }

    pub fn append_mob(&self, target_id: u32, mob_code: u32) {
        // let config = self.config.read().unwrap().clone();
        // let is_target_boss = self.boss_code_list.contains(&mob_code);
        // if config.boss_only && !is_target_boss {
        //     return;
        // }
        let mut inner = self.inner.write().unwrap();
        inner.mob_id_code_map.insert(target_id, mob_code);
    }

    /// The game reported the mob dead. Its HP goes to zero, the same state an
    /// HP packet of zero leaves, so every consumer of that state sees the
    /// kill. A mob whose HP was never recorded has nothing to zero.
    pub fn mark_mob_dead(&self, target_id: u32) {
        if let Some(hp) = self.inner.write().unwrap().mob_id_hp_map.get_mut(&target_id) {
            hp.0 = 0;
        }
    }

    pub fn set_mob_hp(&self, target_id: u32, current_hp: Option<u64>, max_hp: Option<u64>) {
        let Some(seed) = current_hp.or(max_hp) else {
            return;
        };
        let mut inner = self.inner.write().unwrap();
        let entry = inner
            .mob_id_hp_map
            .get_mut_or_insert_with(target_id, || (seed, seed));
        if let Some(current_hp) = current_hp {
            entry.0 = current_hp;
        }
        if let Some(max_hp) = max_hp {
            entry.1 = max_hp;
        }
        entry.1 = entry.1.max(entry.0);
    }

    pub fn set_player_hp(&self, actor_id: u32, current_hp: Option<u32>, max_hp: Option<u32>) {
        let Some(seed) = current_hp.or(max_hp) else {
            return;
        };
        let mut inner = self.inner.write().unwrap();
        let entry = inner
            .player_hp_map
            .get_mut_or_insert_with(actor_id, || PlayerHpInfo {
                actor_id,
                current_hp: seed,
                max_observed_hp: seed,
            });
        if let Some(current_hp) = current_hp {
            entry.current_hp = current_hp;
        }
        if let Some(max_hp) = max_hp {
            entry.max_observed_hp = max_hp;
        }
        entry.max_observed_hp = entry.max_observed_hp.max(entry.current_hp);
    }

    pub fn append_summon(&self, owner_id: u32, summon_id: u32) {
        if owner_id == 0 {
            return;
        }
        let mut inner = self.inner.write().unwrap();
        if inner.actor_id_name_map.contains_key(&summon_id) {
            return;
        }
        inner.summon_owner_map.insert(summon_id, owner_id);
    }

    pub fn forget_summon_owner(&self, summon_id: u32) {
        self.inner.write().unwrap().summon_owner_map.map.remove(&summon_id);
    }

    pub fn get_mob_code(&self, target_id: u32) -> Option<u32> {
        self.inner
            .read()
            .unwrap()
            .mob_id_code_map
            .get(&target_id)
            .copied()
    }

    pub fn add_possible_boss(&self, mob_code: u32) {
        self.mobs.add_possible_boss(mob_code);
    }

    pub fn is_boss(&self, mob_code: u32) -> bool {
        self.mobs.is_boss(mob_code)
    }

    pub fn mobs(&self) -> &Arc<MobCatalog> {
        &self.mobs
    }

    /// A named actor that is not a mob.
    pub fn is_player(&self, actor_id: u32) -> bool {
        let inner = self.inner.read().unwrap();
        inner.actor_id_name_map.contains_key(&actor_id)
            && !inner.mob_id_code_map.contains_key(&actor_id)
    }

    /// The game's own packet named the local player. It is authoritative: what
    /// the signature and the remembered characters decided gives way to it.
    pub fn set_main_actor(&self, actor_id: u32, actor_name: &str) {
        self.install_main_actor(actor_id, actor_name, OwnSource::Game);
    }

    fn install_main_actor(&self, actor_id: u32, actor_name: &str, source: OwnSource) {
        let (sid, is_new_player, overrode) = {
            let mut inner = self.inner.write().unwrap();
            // Naming the actor already found by its signature is not a
            // different player, so nothing is cleared.
            let is_new_player = inner.main_actor_id != Some(actor_id)
                && main_actor_changed(inner.main_actor_name.as_deref(), actor_name);
            if is_new_player {
                inner.main_actor_combat_power = None;
                inner.own_actor_ids.clear();
            }
            if !inner.own_actor_ids.contains(&actor_id) {
                inner.own_actor_ids.push(actor_id);
            }
            let overrode = (source == OwnSource::Game
                && matches!(inner.own_source, OwnSource::Signature | OwnSource::Remembered)
                && (inner.main_actor_id != Some(actor_id)
                    || inner.main_actor_name.as_deref() != Some(actor_name)))
            .then(|| IdentityDecision::GameOverrode {
                actor_id,
                previous_actor_id: inner.main_actor_id,
                previous_name: inner.main_actor_name.clone(),
            });
            inner.main_actor_id = Some(actor_id);
            inner.main_actor_name = Some(actor_name.to_string());
            inner.own_source = source;
            inner.own_name_pending = OwnNamePending::No;
            (
                inner.actor_id_server_map.get(&actor_id).cloned(),
                is_new_player,
                overrode,
            )
        }; // RwLock released — callback below can safely call clear()

        // Only when the player actually changes. The callback clears the meter,
        // and the game re-sends this packet throughout a session -- ten times in
        // one recorded session -- so firing it every time wiped accumulated
        // damage mid-fight and left the overlay showing nothing.
        if is_new_player {
            if let Some(cb) = self.main_actor_callback.lock().unwrap().as_ref() {
                cb(actor_id, actor_name, sid.clone());
            }
        }

        self.events.emit(MeterEvent::MainActorDetected(MainActorDetectedPayload {
            actor_id,
            actor_name: actor_name.to_string(),
            sid,
        }));
        if let Some(decision) = overrode {
            self.events.emit(MeterEvent::OwnIdentity(decision));
        }
    }

    /// The local player's actor, found from its remain-HP packet at the start
    /// of a capture or of a new game connection (a zone change gives the
    /// character a new id without naming it again). A name already known
    /// carries over to the new id, but only provisionally: the new connection
    /// may be another character's (character select), so once the actor's
    /// skills show a class the name stays only if it is the class that name
    /// had. Without a known name the actor is left unnamed until a remembered
    /// character matches its class or the game names it. Nothing changes when
    /// the id is already the player's or belongs to someone else.
    pub fn bind_own_actor(&self, actor_id: u32) -> bool {
        let carried = {
            let inner = self.inner.read().unwrap();
            if inner.main_actor_id == Some(actor_id)
                || inner.actor_id_name_map.contains_key(&actor_id)
                || inner.mob_id_code_map.contains_key(&actor_id)
            {
                return false;
            }
            inner.main_actor_name.clone().map(|name| {
                let previous = inner.main_actor_id;
                let sid = previous.and_then(|previous| inner.actor_id_server_map.get(&previous).cloned());
                let class = previous.and_then(|previous| inner.actor_id_class_map.get(&previous).cloned());
                (name, sid, class, inner.own_source)
            })
        };

        if let Some((name, sid, class, source)) = carried {
            self.append_actor(actor_id, &name, sid.as_deref());
            self.install_main_actor(actor_id, &name, source);
            self.inner.write().unwrap().own_name_pending = OwnNamePending::Carried { class };
            self.resolve_own_name();
            return true;
        }

        {
            let mut inner = self.inner.write().unwrap();
            inner.main_actor_id = Some(actor_id);
            inner.own_actor_ids = vec![actor_id];
            inner.own_source = OwnSource::Signature;
            inner.own_name_pending = OwnNamePending::Nameless;
        }
        self.events.emit(MeterEvent::MainActorDetected(MainActorDetectedPayload {
            actor_id,
            actor_name: String::new(),
            sid: None,
        }));
        self.events
            .emit(MeterEvent::OwnIdentity(IdentityDecision::SignatureBound { actor_id }));
        self.resolve_own_name();
        true
    }

    /// Settles the name of an own actor found by its signature, once its skills
    /// show a class: a carried name stays if that was its class, otherwise the
    /// most recently seen remembered character of the class names it, else it
    /// stays unnamed. Waits while no skill has shown a class.
    fn resolve_own_name(&self) {
        let (actor_id, class, dropped, remembered, remembered_count) = {
            let mut inner = self.inner.write().unwrap();
            let pending = inner.own_name_pending.clone();
            let Some(actor_id) = inner.main_actor_id.filter(|_| pending != OwnNamePending::No) else {
                return;
            };
            let Some(class) = inner.actor_id_class_map.get(&actor_id).cloned() else {
                return;
            };
            inner.own_name_pending = OwnNamePending::No;
            let dropped = match pending {
                OwnNamePending::Carried { class: carried_class } if carried_class.as_deref() == Some(class.as_str()) => {
                    return;
                }
                OwnNamePending::Carried { class: carried_class } => {
                    inner.own_source = OwnSource::Signature;
                    inner.own_actor_ids = vec![actor_id];
                    inner.actor_id_name_map.remove(&actor_id);
                    inner.actor_id_server_map.remove(&actor_id);
                    inner.main_actor_name.take().map(|name| (name, carried_class))
                }
                OwnNamePending::Nameless | OwnNamePending::No => None,
            };
            let known = self.known_characters.read().unwrap();
            let remembered = character_of_class(&known, &class).cloned();
            if let Some(character) = &remembered {
                inner.actor_id_name_map.insert(actor_id, character.name.clone());
                inner
                    .actor_id_server_map
                    .insert(actor_id, character.server_id.to_string());
                inner.main_actor_name = Some(character.name.clone());
                inner.own_source = OwnSource::Remembered;
            } else {
                inner.own_name_pending = OwnNamePending::Nameless;
            }
            (actor_id, class, dropped, remembered, known.len())
        };

        let name_was_dropped = dropped.is_some();
        if let Some((name, carried_class)) = dropped {
            self.events.emit(MeterEvent::OwnIdentity(IdentityDecision::CarriedRejected {
                actor_id,
                name,
                carried_class,
                class: class.clone(),
            }));
        }
        match remembered {
            Some(character) => {
                self.events.emit(MeterEvent::MainActorDetected(MainActorDetectedPayload {
                    actor_id,
                    actor_name: character.name.clone(),
                    sid: Some(character.server_id.to_string()),
                }));
                self.events.emit(MeterEvent::OwnIdentity(IdentityDecision::RememberedApplied {
                    actor_id,
                    name: character.name,
                    class,
                }));
            }
            None => {
                if name_was_dropped {
                    self.events.emit(MeterEvent::MainActorDetected(MainActorDetectedPayload {
                        actor_id,
                        actor_name: String::new(),
                        sid: None,
                    }));
                }
                self.events.emit(MeterEvent::OwnIdentity(IdentityDecision::RememberedRejected {
                    actor_id,
                    class,
                    remembered: remembered_count,
                }));
            }
        }
    }

    /// The characters earlier runs saw the player as, most recent first.
    pub fn set_known_characters(&self, characters: Vec<KnownCharacter>) {
        *self.known_characters.write().unwrap() = characters;
        self.resolve_own_name();
    }

    /// Tells the app the game named the player's character, so it can be
    /// remembered.
    pub fn character_seen(&self, character: KnownCharacter) {
        self.events.emit(MeterEvent::CharacterSeen(character));
    }

    pub fn get_dps_stats_snapshot(&self) -> HashMap<u32, HashMap<u32, HashMap<u32, SkillStats>>> {
        self.inner.read().unwrap().dps_stats.map.clone()
    }

    pub fn actor_id_name_snapshot(&self) -> HashMap<u32, String> {
        self.inner.read().unwrap().actor_id_name_map.as_hash_map()
    }

    pub fn actor_id_server_snapshot(&self) -> HashMap<u32, String> {
        self.inner.read().unwrap().actor_id_server_map.as_hash_map()
    }

    pub fn actor_id_class_snapshot(&self) -> HashMap<u32, String> {
        self.inner.read().unwrap().actor_id_class_map.as_hash_map()
    }

    pub fn player_hp_snapshot(&self) -> HashMap<u32, PlayerHpInfo> {
        self.inner.read().unwrap().player_hp_map.as_hash_map()
    }

    pub fn actor_id_combat_power_snapshot(&self) -> HashMap<u32, u64> {
        self.inner
            .read()
            .unwrap()
            .actor_id_combat_power_map
            .as_hash_map()
    }

    pub fn main_actor_combat_power(&self) -> Option<u64> {
        self.inner.read().unwrap().main_actor_combat_power
    }

    pub fn actor_skill_spec_snapshot(&self) -> HashMap<u32, HashMap<u32, Vec<u32>>> {
        self.inner
            .read()
            .unwrap()
            .actor_id_skill_spec_map
            .as_hash_map()
    }

    pub fn buff_intervals_snapshot(
        &self,
    ) -> HashMap<u32, HashMap<u32, HashMap<u32, VecDeque<BuffInterval>>>> {
        self.inner.read().unwrap().use_buffs_by_target.as_hash_map()
    }

    pub fn get_pvp_watch_info(&self, names: &[String]) -> PvpWatchInfoResponse {
        let inner = self.inner.read().unwrap();
        let mut watch_info = Vec::new();
        let mut known_players = Vec::new();

        for (actor_id, actor_name) in &inner.actor_id_name_map.map {
            if actor_name.trim().is_empty() || inner.mob_id_code_map.contains_key(actor_id) {
                continue;
            }

            known_players.push(PvpKnownPlayer {
                actor_id: *actor_id,
                actor_name: actor_name.clone(),
                server_id: inner.actor_id_server_map.get(actor_id).cloned(),
                actor_class: inner.actor_id_class_map.get(actor_id).cloned(),
            });
        }
        known_players.sort_by(|a, b| {
            a.actor_name
                .cmp(&b.actor_name)
                .then_with(|| a.server_id.cmp(&b.server_id))
                .then_with(|| a.actor_id.cmp(&b.actor_id))
        });

        for raw_name in names {
            let query_name = raw_name.trim();
            if query_name.is_empty() {
                continue;
            }

            let mut matched = false;
            for (actor_id, actor_name) in &inner.actor_id_name_map.map {
                if actor_name != query_name {
                    continue;
                }

                watch_info.push(build_pvp_watch_info_for_actor(
                    &inner, *actor_id, query_name,
                ));
                matched = true;
            }

            if !matched {
                watch_info.push(PvpWatchInfo {
                    query_name: query_name.to_string(),
                    actor_id: None,
                    actor_name: None,
                    server_id: None,
                    actor_class: None,
                    current_hp: None,
                    max_hp: None,
                });
            }
        }

        PvpWatchInfoResponse {
            watch_info,
            known_players,
            last_dealt_player: inner
                .last_player_target_by_main_actor
                .map(|actor_id| build_pvp_watch_info_for_actor(&inner, actor_id, "last_dealt")),
        }
    }

    pub fn mark_player_dead(&self, entity_id: u32) -> (bool, Option<String>) {
        if !self.config.read().unwrap().pvp_mode_on {
            return (false, None);
        }

        let mut inner = self.inner.write().unwrap();
        let Some(victim) = pvp_player_key(&inner, entity_id) else {
            return (false, None);
        };
        if inner.mob_id_code_map.contains_key(&entity_id)
            || !inner.pvp_dead_players.insert(victim.clone())
        {
            return (false, None);
        }

        let attackers = inner
            .pvp_attackers_by_target
            .remove(&entity_id)
            .unwrap_or_default();
        let killer = inner.pvp_last_attacker_by_target.remove(&entity_id);

        inner
            .pvp_combat_stats
            .entry(victim.clone())
            .or_default()
            .deaths += 1;

        let killer = killer.filter(|player| player != &victim);
        if let Some(killer_player) = &killer {
            inner
                .pvp_combat_stats
                .entry(killer_player.clone())
                .or_default()
                .kills += 1;
        }

        for (attacker, damage) in attackers {
            if attacker == victim {
                continue;
            }
            let stats = inner.pvp_combat_stats.entry(attacker.clone()).or_default();
            stats.damage += damage;
            if killer.as_ref() != Some(&attacker) {
                stats.assists += 1;
            }
        }

        (true, killer.map(|player| player.name))
    }

    pub fn mark_player_alive(&self, entity_id: u32) {
        let mut inner = self.inner.write().unwrap();
        if let Some(player) = pvp_player_key(&inner, entity_id) {
            inner.pvp_dead_players.remove(&player);
        }
    }

    pub fn get_pvp_combat_stats(&self) -> Vec<PvpCombatStatsRow> {
        self.inner
            .read()
            .unwrap()
            .pvp_combat_stats
            .iter()
            .map(|(player, stats)| PvpCombatStatsRow {
                actor_name: player.name.clone(),
                server_id: player.server_id.clone(),
                damage: stats.damage,
                kills: stats.kills,
                assists: stats.assists,
                deaths: stats.deaths,
            })
            .collect()
    }

    pub fn clear_pvp_combat_stats(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.pvp_attackers_by_target.clear();
        inner.pvp_last_attacker_by_target.clear();
        inner.pvp_combat_stats.clear();
        inner.pvp_dead_players.clear();
    }

    pub fn summon_owner_snapshot(&self) -> HashMap<u32, u32> {
        self.inner.read().unwrap().summon_owner_map.as_hash_map()
    }

    pub fn mob_id_code_snapshot(&self) -> HashMap<u32, u32> {
        self.inner.read().unwrap().mob_id_code_map.as_hash_map()
    }

    pub fn mob_id_hp_snapshot(&self) -> HashMap<u32, (u64, u64)> {
        self.inner.read().unwrap().mob_id_hp_map.as_hash_map()
    }

    /// A mob's English name. A lookup rather than a copy of the catalogue:
    /// the snapshot loop asks five times a second, and the catalogue holds
    /// eight thousand names.
    pub fn mob_name(&self, mob_code: u32) -> Option<String> {
        self.mobs.mob_name(mob_code)
    }

    pub fn start_time_by_target_snapshot(&self) -> HashMap<u32, HashMap<u32, f64>> {
        self.inner.read().unwrap().start_time_by_target.clone()
    }

    pub fn last_time_by_target_snapshot(&self) -> HashMap<u32, HashMap<u32, f64>> {
        self.inner.read().unwrap().last_time_by_target.clone()
    }

    pub fn start_time(&self) -> Option<f64> {
        self.inner.read().unwrap().start_time
    }

    pub fn main_actor_id(&self) -> Option<u32> {
        self.inner.read().unwrap().main_actor_id
    }

    pub fn own_actor_ids(&self) -> Vec<u32> {
        self.inner.read().unwrap().own_actor_ids.clone()
    }

    pub fn main_actor_name(&self) -> Option<String> {
        self.inner.read().unwrap().main_actor_name.clone()
    }

    pub fn last_target(&self) -> Option<u32> {
        self.inner.read().unwrap().last_target
    }

    pub fn last_target_by_main_actor(&self) -> Option<u32> {
        self.inner.read().unwrap().last_target_by_main_actor
    }
}

fn infer_specialty_slots(skill_id: u32) -> Vec<u32> {
    let last_4_digits = skill_id % 10000;
    let slot_1 = (last_4_digits / 1000) % 10;
    let slot_2 = (last_4_digits / 100) % 10;
    let slot_3 = (last_4_digits / 10) % 10;

    let mut slots = Vec::new();
    if slot_1 > 0 {
        slots.push(slot_1);
    }
    if slot_2 > 0 {
        slots.push(slot_2);
    }
    if slot_3 > 0 {
        slots.push(slot_3);
    }
    slots.sort_unstable();
    slots
}

pub(crate) fn infer_actor_class(skill_code: u32) -> Option<String> {
    let skill_code = skill_code.to_string();
    if skill_code.len() < 2 {
        return None;
    }
    if ACTOR_CLASS_SKILL_IGNORE_LIST
        .iter()
        .any(|ignored_prefix| skill_code.starts_with(ignored_prefix))
    {
        return None;
    }

    match &skill_code[0..2] {
        "11" => Some("GLADIATOR".to_string()),
        "12" => Some("TEMPLAR".to_string()),
        "13" => Some("ASSASSIN".to_string()),
        "14" => Some("RANGER".to_string()),
        "15" => Some("SORCERER".to_string()),
        "16" => Some("ELEMENTALIST".to_string()),
        "17" => Some("CLERIC".to_string()),
        "18" => Some("CHANTER".to_string()),
        "19" => Some("FIGHTER".to_string()),
        _ => None,
    }
}

fn current_timestamp_seconds() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

/// A heal is kept when a player healed a player: nothing a monster did or took.
fn record_heal(inner: &mut DataStorageInner, heal: Heal) {
    let involves_a_monster = inner.mob_id_code_map.contains_key(&heal.healer_id)
        || inner.mob_id_code_map.contains_key(&heal.target_id);
    if heal.amount > 0 && !involves_a_monster {
        inner.combat_events.record_heal(heal);
    }
}

/// Whether a counted hit belongs to your fight: you dealt it, you took it, or
/// it landed on your current target (a healer's party keeps the boss busy).
/// Before the game has told us who you are, every hit counts.
fn is_own_fight(inner: &DataStorageInner, actor_id: u32, target_id: u32) -> bool {
    match inner.main_actor_id {
        None => true,
        Some(me) => {
            actor_id == me
                || target_id == me
                || inner.last_target_by_main_actor == Some(target_id)
        }
    }
}

fn current_timestamp_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn build_pvp_watch_info_for_actor(
    inner: &DataStorageInner,
    actor_id: u32,
    query_name: &str,
) -> PvpWatchInfo {
    let hp = inner.player_hp_map.get(&actor_id);

    PvpWatchInfo {
        query_name: query_name.to_string(),
        actor_id: Some(actor_id),
        actor_name: inner.actor_id_name_map.get(&actor_id).cloned(),
        server_id: inner.actor_id_server_map.get(&actor_id).cloned(),
        actor_class: inner.actor_id_class_map.get(&actor_id).cloned(),
        current_hp: hp.map(|value| value.current_hp),
        max_hp: hp.map(|value| value.max_observed_hp),
    }
}

fn pvp_player_key(inner: &DataStorageInner, actor_id: u32) -> Option<PvpPlayerKey> {
    Some(PvpPlayerKey {
        name: inner.actor_id_name_map.get(&actor_id)?.clone(),
        server_id: inner
            .actor_id_server_map
            .get(&actor_id)
            .cloned()
            .unwrap_or_default(),
    })
}

/// Whether identifying `next` as the main actor means the player has changed.
///
/// Keyed on the name rather than the actor id on purpose: ids are per-session
/// entity handles that change across zones -- one recording shows the same
/// character as both `15056` and `5492` -- so keying on them would clear the
/// meter every time you zoned.
fn main_actor_changed(current: Option<&str>, next: &str) -> bool {
    current != Some(next)
}

#[cfg(test)]
mod main_actor_tests {
    use super::main_actor_changed;

    #[test]
    fn first_identification_counts_as_a_change() {
        assert!(main_actor_changed(None, "Aldwin"));
    }

    #[test]
    fn re_identifying_the_same_player_does_not() {
        // The game re-sends the own-player packet throughout a session. Treating
        // each one as a change cleared the meter mid-fight.
        assert!(!main_actor_changed(Some("Aldwin"), "Aldwin"));
    }

    #[test]
    fn switching_character_counts_as_a_change() {
        assert!(main_actor_changed(Some("Aldwin"), "Caldris"));
    }
}

#[cfg(test)]
mod bounded_map_tests {
    use super::{BoundedMap, COMBAT_TARGET_CAPACITY};

    #[test]
    fn evicts_the_oldest_key_past_capacity() {
        let mut map: BoundedMap<u32, u32> = BoundedMap::new(3);
        for key in 1..=4 {
            map.insert(key, key * 10);
        }

        assert_eq!(map.get(&1), None, "the oldest entry is dropped");
        assert_eq!(map.get(&4), Some(&40));
        assert_eq!(map.map.len(), 3);
    }

    #[test]
    fn touching_a_key_keeps_it_alive() {
        let mut map: BoundedMap<u32, u32> = BoundedMap::new(2);
        map.insert(1, 10);
        map.insert(2, 20);
        map.insert(1, 11); // re-inserting moves it back to the newest end
        map.insert(3, 30);

        assert_eq!(map.get(&1), Some(&11));
        assert_eq!(map.get(&2), None, "the key untouched for longest goes first");
    }

    #[test]
    fn combat_targets_are_bounded() {
        // The overlay snapshot deep-clones this map several times a second, so
        // an unbounded one makes a long session progressively more expensive.
        let mut map: BoundedMap<u32, u32> = BoundedMap::new(COMBAT_TARGET_CAPACITY);
        for key in 0..(COMBAT_TARGET_CAPACITY as u32 * 3) {
            map.insert(key, key);
        }
        assert_eq!(map.map.len(), COMBAT_TARGET_CAPACITY);
    }
}

#[cfg(test)]
mod own_fight_tests {
    use super::{is_own_fight, DataStorageInner};

    const ME: u32 = 7;
    const PARTY: u32 = 8;
    const STRANGER: u32 = 9;
    const BOSS: u32 = 100;
    const OTHER_MOB: u32 = 200;

    fn inner(main_actor: Option<u32>, my_target: Option<u32>) -> DataStorageInner {
        DataStorageInner {
            main_actor_id: main_actor,
            last_target_by_main_actor: my_target,
            ..DataStorageInner::default()
        }
    }

    #[test]
    fn before_you_are_identified_every_hit_counts() {
        assert!(is_own_fight(&inner(None, None), STRANGER, OTHER_MOB));
    }

    #[test]
    fn your_hits_and_hits_on_you_count() {
        let state = inner(Some(ME), None);
        assert!(is_own_fight(&state, ME, BOSS));
        assert!(is_own_fight(&state, BOSS, ME));
    }

    #[test]
    fn your_party_on_your_target_keeps_the_fight_alive() {
        // A healer can go minutes without a hit of their own on a boss.
        assert!(is_own_fight(&inner(Some(ME), Some(BOSS)), PARTY, BOSS));
    }

    #[test]
    fn someone_else_fighting_nearby_does_not() {
        assert!(!is_own_fight(&inner(Some(ME), Some(BOSS)), STRANGER, OTHER_MOB));
    }
}

#[cfg(test)]
mod unowned_hit_tests {
    use std::sync::{Arc, RwLock};

    use super::DataStorage;
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::mobs::catalog::MobCatalog;
    use crate::models::packet::ParsedDamagePacket;

    struct Silent;
    impl EventSink for Silent {
        fn emit(&self, _: MeterEvent) {}
    }

    const SUMMON: u32 = 500;
    const MOB: u32 = 900;
    const PLAYER: u32 = 7;

    fn storage() -> DataStorage {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        DataStorage::new(Arc::new(Silent), config, mobs)
    }

    fn hit(actor_id: u32) -> ParsedDamagePacket {
        ParsedDamagePacket {
            target_id: MOB,
            actor_id,
            skill_code: 11_000_000,
            ori_skill_code: 11_000_000,
            damage: 407,
            is_dot: false,
            is_crit: false,
            multi_hit_damage: 0,
            multi_hit_count: 0,
            specials: Vec::new(),
            drain_amount: 0,
            regeneration_amount: 0,
        }
    }

    fn attackers(storage: &DataStorage) -> Vec<u32> {
        let mut ids: Vec<u32> = storage
            .get_dps_stats_snapshot()
            .values()
            .flat_map(|actors| actors.keys().copied())
            .collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn a_summon_whose_owner_reads_as_zero_does_not_create_an_actor_zero() {
        let storage = storage();
        storage.append_summon(PLAYER, SUMMON);
        storage.append_summon(0, SUMMON);

        storage.append_damage(hit(SUMMON));

        assert_eq!(attackers(&storage), vec![PLAYER], "a bad owner must not replace the real one");
    }

    #[test]
    fn hitting_your_own_summon_does_not_make_it_the_target() {
        let storage = storage();
        storage.set_main_actor(PLAYER, "Me");
        storage.append_summon(PLAYER, SUMMON);
        storage.append_damage(hit(PLAYER));

        let mut on_summon = hit(PLAYER);
        on_summon.target_id = SUMMON;
        storage.append_damage(on_summon);

        assert_eq!(storage.last_target_by_main_actor(), Some(MOB));
        assert_eq!(storage.last_target(), Some(MOB));
    }

    #[test]
    fn hitting_someone_elses_summon_still_makes_it_the_target() {
        let storage = storage();
        storage.set_main_actor(PLAYER, "Me");
        storage.append_summon(PLAYER + 1, SUMMON);

        let mut on_summon = hit(PLAYER);
        on_summon.target_id = SUMMON;
        storage.append_damage(on_summon);

        assert_eq!(storage.last_target_by_main_actor(), Some(SUMMON));
    }

    #[test]
    fn a_hit_that_resolves_to_actor_zero_is_dropped() {
        let storage = storage();
        storage.inner.write().unwrap().summon_owner_map.insert(SUMMON, 0);

        storage.append_damage(hit(SUMMON));
        storage.append_damage(hit(0));

        assert!(attackers(&storage).is_empty());
    }

    #[test]
    fn a_zone_change_adds_the_new_id_to_the_players_own_ids() {
        let storage = storage();
        storage.set_main_actor(PLAYER, "Me");
        storage.set_actor_class(PLAYER, "ELEMENTALIST");

        storage.bind_own_actor(PLAYER + 10);
        storage.set_actor_class(PLAYER + 10, "ELEMENTALIST");
        storage.resolve_own_name();

        assert_eq!(storage.own_actor_ids(), vec![PLAYER, PLAYER + 10]);
    }

    #[test]
    fn another_character_starts_a_new_list_of_own_ids() {
        let storage = storage();
        storage.set_main_actor(PLAYER, "Me");
        storage.set_main_actor(PLAYER + 1, "Other");

        assert_eq!(storage.own_actor_ids(), vec![PLAYER + 1]);
    }

    #[test]
    fn a_carried_name_the_class_rejects_does_not_keep_the_old_id() {
        let storage = storage();
        storage.set_main_actor(PLAYER, "Me");
        storage.set_actor_class(PLAYER, "ELEMENTALIST");

        storage.bind_own_actor(PLAYER + 10);
        storage.set_actor_class(PLAYER + 10, "CLERIC");
        storage.resolve_own_name();

        assert_eq!(storage.own_actor_ids(), vec![PLAYER + 10]);
    }

    #[test]
    fn a_reset_keeps_the_players_own_ids() {
        let storage = storage();
        storage.set_main_actor(PLAYER, "Me");
        storage.bind_own_actor(PLAYER + 10);

        storage.clear();

        assert_eq!(storage.own_actor_ids(), vec![PLAYER, PLAYER + 10]);
    }
}

#[cfg(test)]
mod combat_event_tests {
    use std::sync::{Arc, RwLock};

    use super::DataStorage;
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::mobs::catalog::MobCatalog;
    use crate::models::packet::ParsedDamagePacket;
    use crate::report::events::HealKind;

    struct Silent;
    impl EventSink for Silent {
        fn emit(&self, _: MeterEvent) {}
    }

    const PLAYER: u32 = 7;
    const HEALER: u32 = 8;
    const BOSS: u32 = 900;
    const ADD: u32 = 901;
    const BOSS_CODE: u32 = 2_300_810;
    const MONSTER_SKILL: u32 = 1_607_630;

    fn storage() -> DataStorage {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = DataStorage::new(Arc::new(Silent), config, mobs);
        storage.append_actor(PLAYER, "Player", None);
        storage.append_actor(HEALER, "Healer", None);
        storage.append_mob(BOSS, BOSS_CODE);
        storage.append_mob(ADD, 2_920_174);
        storage
    }

    fn hit(actor_id: u32, target_id: u32, skill_code: u32, damage: u64) -> ParsedDamagePacket {
        ParsedDamagePacket {
            target_id,
            actor_id,
            skill_code,
            ori_skill_code: skill_code + 1,
            damage,
            is_dot: false,
            is_crit: false,
            multi_hit_damage: 0,
            multi_hit_count: 0,
            specials: Vec::new(),
            drain_amount: 0,
            regeneration_amount: 0,
        }
    }

    #[test]
    fn a_monsters_hit_on_a_player_is_damage_taken_that_moves_no_target_and_no_stats() {
        let storage = storage();

        storage.record_incoming_hit_at(PLAYER, BOSS, MONSTER_SKILL, 900, 10.0);

        let hits = storage.combat_events(0.0, 20.0).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].source_mob_code, hits[0].amount), (Some(BOSS_CODE), 900));
        assert_eq!(storage.last_target(), None);
        assert_eq!(storage.last_target_by_main_actor(), None);
        assert!(storage.get_dps_stats_snapshot().is_empty());
    }

    #[test]
    fn what_is_not_a_monster_hitting_a_player_is_not_damage_taken() {
        let storage = storage();

        storage.record_incoming_hit_at(ADD, BOSS, MONSTER_SKILL, 900, 10.0);
        storage.record_incoming_hit_at(PLAYER, HEALER, MONSTER_SKILL, 900, 10.0);
        storage.record_incoming_hit_at(PLAYER, PLAYER, MONSTER_SKILL, 900, 10.0);
        storage.record_incoming_hit_at(PLAYER, BOSS, MONSTER_SKILL, 0, 10.0);

        assert!(storage.combat_events(0.0, 20.0).hits.is_empty());
    }

    #[test]
    fn a_hit_from_a_summon_of_a_monster_is_damage_taken_at_its_full_amount() {
        let storage = storage();
        storage.append_summon(BOSS, 5_001);

        storage.record_incoming_hit_at(PLAYER, 5_001, MONSTER_SKILL, 18_576, 10.0);

        let hits = storage.combat_events(0.0, 20.0).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].source_id, hits[0].amount), (5_001, 18_576));
    }

    #[test]
    fn a_hit_from_a_summon_of_a_player_is_not_damage_taken() {
        let storage = storage();
        storage.append_summon(HEALER, 5_001);
        storage.append_summon(5_001, 5_002);

        storage.record_incoming_hit_at(PLAYER, 5_001, MONSTER_SKILL, 900, 10.0);
        storage.record_incoming_hit_at(PLAYER, 5_002, MONSTER_SKILL, 900, 10.0);

        assert!(storage.combat_events(0.0, 20.0).hits.is_empty());
    }

    #[test]
    fn a_death_by_a_monsters_summon_has_that_hit_as_its_last_hit() {
        let storage = storage();
        storage.append_summon(BOSS, 5_001);

        storage.record_incoming_hit_at(PLAYER, 5_001, MONSTER_SKILL, 15_963, 10.0);
        storage.record_player_death_at(PLAYER, MONSTER_SKILL, 5_001, "", 0, 10.5);

        let deaths = storage.combat_events(0.0, 20.0).deaths;
        assert_eq!(deaths[0].last_hit.as_ref().map(|hit| hit.amount), Some(15_963));
    }

    #[test]
    fn a_hit_from_a_summon_of_an_owner_not_seen_spawning_is_damage_taken() {
        let storage = storage();
        storage.append_summon(5_000, 5_001);

        storage.record_incoming_hit_at(PLAYER, 5_001, MONSTER_SKILL, 18_576, 10.0);

        assert_eq!(storage.combat_events(0.0, 20.0).hits.len(), 1);
    }

    #[test]
    fn a_hit_from_a_monster_not_yet_seen_spawning_is_still_damage_taken() {
        let storage = storage();

        storage.record_incoming_hit_at(PLAYER, 5_000, MONSTER_SKILL, 900, 10.0);

        assert_eq!(storage.combat_events(0.0, 20.0).hits[0].source_mob_code, None);
    }

    #[test]
    fn a_healing_skills_amount_on_a_player_is_a_heal_and_not_damage() {
        let storage = storage();
        let mut healing_hit = hit(HEALER, PLAYER, 17_800_000, 1_000);
        healing_hit.is_crit = true;

        storage.append_damage_at(healing_hit, 10.0);

        let heals = storage.combat_events(0.0, 20.0).heals;
        assert_eq!(heals.len(), 1);
        assert_eq!(
            (heals[0].healer_id, heals[0].target_id, heals[0].skill_code, heals[0].kind, heals[0].amount, heals[0].critical),
            (HEALER, PLAYER, 17_800_000, HealKind::Direct, 1_000, true)
        );
        assert!(storage.get_dps_stats_snapshot().is_empty());
    }

    #[test]
    fn a_healing_skill_landing_on_a_monster_is_not_a_heal() {
        let storage = storage();

        storage.append_damage_at(hit(HEALER, BOSS, 17_800_000, 1_000), 10.0);

        assert!(storage.combat_events(0.0, 20.0).heals.is_empty());
    }

    #[test]
    fn health_regained_on_a_damaging_hit_is_a_heal_of_the_attacker_and_the_damage_still_counts() {
        let storage = storage();
        let mut regenerating_hit = hit(PLAYER, BOSS, 12_060_000, 3_000);
        regenerating_hit.regeneration_amount = 677;
        regenerating_hit.drain_amount = 12;

        storage.append_damage_at(regenerating_hit, 10.0);

        let mut heals = storage.combat_events(0.0, 20.0).heals;
        heals.sort_by_key(|heal| heal.kind);
        assert_eq!(heals.len(), 2);
        assert_eq!((heals[0].kind, heals[0].amount), (HealKind::Drain, 12));
        assert_eq!((heals[1].kind, heals[1].amount), (HealKind::Regeneration, 677));
        assert_eq!((heals[1].healer_id, heals[1].target_id, heals[1].skill_code), (PLAYER, PLAYER, 12_060_000));
        assert_eq!(storage.last_target(), Some(BOSS));
    }

    #[test]
    fn a_heal_tick_is_an_over_time_heal() {
        let storage = storage();

        storage.record_heal_tick_at(HEALER, PLAYER, 17_090_000, 340, 10.0);

        let heals = storage.combat_events(0.0, 20.0).heals;
        assert_eq!((heals[0].kind, heals[0].amount), (HealKind::OverTime, 340));
    }

    #[test]
    fn a_death_names_a_monster_killer_by_its_entity_and_carries_the_last_hit() {
        let storage = storage();
        storage.record_incoming_hit_at(PLAYER, BOSS, MONSTER_SKILL, 52_300, 10.0);

        storage.record_player_death_at(PLAYER, MONSTER_SKILL, BOSS, "", 0, 11.0);

        let death = &storage.combat_events(0.0, 20.0).deaths[0];
        assert_eq!(death.killer.mob_code, Some(BOSS_CODE));
        assert_eq!(death.killer.name, None);
        assert_eq!(death.last_hit.as_ref().map(|hit| hit.amount), Some(52_300));
    }

    #[test]
    fn a_death_names_a_player_killer_by_name() {
        let storage = storage();

        storage.record_player_death_at(PLAYER, 17_050_000, 8_030, "Veyra", 0, 11.0);

        let death = &storage.combat_events(0.0, 20.0).deaths[0];
        assert_eq!(death.killer.name.as_deref(), Some("Veyra"));
        assert_eq!(death.killer.mob_code, None);
        assert_eq!(death.last_hit, None);
    }

    #[test]
    fn the_death_of_a_monster_is_not_a_player_death() {
        let storage = storage();

        storage.record_player_death_at(ADD, 0, 0, "", 0, 11.0);

        assert!(storage.combat_events(0.0, 20.0).deaths.is_empty());
    }

    #[test]
    fn a_reset_forgets_the_events() {
        let storage = storage();
        storage.record_incoming_hit_at(PLAYER, BOSS, MONSTER_SKILL, 900, 10.0);

        storage.clear();

        assert!(storage.combat_events(0.0, 20.0).hits.is_empty());
    }
}
