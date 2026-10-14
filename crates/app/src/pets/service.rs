use std::collections::VecDeque;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global};
use meter_core::pets::{PetCollection, PetContent, PetReport, PetState, MAX_LEVEL};

use super::data::PetData;
use super::gain::{GainSource, LatestGain};
use super::list::ListChoice;
use super::settings::PetsSettings;
use super::store;
use crate::background_save::{BackgroundSave, Persisted};
use crate::game_data;

/// How many gains are kept for the card to catch up on: a message can carry
/// several (a batch of seven was seen), and the card reads them after the
/// message is applied.
const REMEMBERED_GAINS: usize = 16;

/// The pet collection and pins of every server the player has been on, kept up
/// to date from what the game says whether or not the DPS meter is open. Lives
/// on the UI thread; windows observe it.
pub struct Pets {
    data: PetData,
    save: BackgroundSave<PetData>,
    recent_gains: VecDeque<LatestGain>,
    /// When this run began, in seconds since the Unix epoch: a snapshot from
    /// before it was read at an earlier login.
    opened_at: f64,
}

/// Asks for the soul tracker card with one pet as it is now, from Settings ›
/// Pets' "Show a test card".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowTestCard {
    pub pet_id: u32,
    pub state: PetState,
}

impl EventEmitter<ShowTestCard> for Pets {}

impl Persisted for Pets {
    type Value = PetData;

    fn value(&self) -> &PetData {
        &self.data
    }

    fn background_save(&mut self) -> &mut BackgroundSave<PetData> {
        &mut self.save
    }
}

struct PetsHandle(Entity<Pets>);

impl Global for PetsHandle {}

impl Pets {
    /// Reads `pets.json` from `data_dir`; without a folder the collection lives
    /// in memory only.
    pub fn open(data_dir: Option<&Path>) -> Self {
        let (file, loaded) = match data_dir {
            Some(dir) => {
                let file = store::path_in(dir);
                let loaded = store::load(&file, unix_secs() as u64);
                if let Some(notice) = &loaded.notice {
                    crate::app_log::error(notice.to_string());
                }
                (Some(file), loaded)
            }
            None => (None, store::Loaded { data: PetData::default(), notice: None }),
        };
        Self {
            data: loaded.data,
            save: BackgroundSave::new(file, store::save),
            recent_gains: VecDeque::new(),
            opened_at: unix_secs(),
        }
    }

    /// The server the game last spoke about, kept from the previous run until
    /// it speaks again.
    pub fn current_server(&self) -> Option<u32> {
        self.data.current_server
    }

    /// The collection of the server being played.
    pub fn collection(&self) -> Option<&PetCollection> {
        self.data.servers.get(&self.data.current_server?)
    }

    /// The collection of the server being played was sent at a login during
    /// this run, so it is live; else it is the one an earlier run kept.
    pub fn read_this_run(&self) -> bool {
        self.collection().and_then(|collection| collection.snapshot_at).is_some_and(|at| at >= self.opened_at)
    }

    pub fn list_choice(&self) -> ListChoice {
        self.data.list
    }

    pub fn set_list_choice(&mut self, choice: ListChoice, cx: &mut Context<Self>) {
        if self.data.list == choice {
            return;
        }
        self.data.list = choice;
        self.save.changed(cx);
        cx.notify();
    }

    /// Shows the card with the first pet pinned on the server being played,
    /// or the first pet in the game's order when none is, as it is now;
    /// nothing is added to it.
    pub fn show_test_card(&mut self, cx: &mut Context<Self>) {
        let Some(pet_id) = self.pinned().first().or(game_data::pet_ids().first()).copied() else {
            return;
        };
        let state = self.collection().map(|collection| collection.pet(pet_id)).unwrap_or_default();
        cx.emit(ShowTestCard { pet_id, state });
    }

    /// The pets pinned on the server being played, in the order they were pinned.
    pub fn pinned(&self) -> &[u32] {
        self.data.current_server.map_or(&[], |server_id| self.data.pinned(server_id))
    }

    pub fn is_pinned(&self, pet_id: u32) -> bool {
        self.pinned().contains(&pet_id)
    }

    /// Pins a pet on the server being played. Returns false when nothing
    /// changed: no server heard yet, or the pet was already pinned.
    pub fn pin(&mut self, pet_id: u32, cx: &mut Context<Self>) -> bool {
        self.change_pins(cx, |data, server_id| data.pin(server_id, pet_id))
    }

    /// Returns false when nothing changed: no server heard yet, or the pet was not pinned.
    pub fn unpin(&mut self, pet_id: u32, cx: &mut Context<Self>) -> bool {
        self.change_pins(cx, |data, server_id| data.unpin(server_id, pet_id))
    }

    pub fn latest_gain(&self) -> Option<&LatestGain> {
        self.recent_gains.back()
    }

    /// The gains after `sequence` that the card shows under `settings`, oldest
    /// first: the card passes the last sequence it handled.
    pub fn card_gains_after(&self, sequence: u64, settings: &PetsSettings) -> Vec<LatestGain> {
        self.recent_gains
            .iter()
            .filter(|gain| gain.sequence > sequence)
            .filter(|gain| {
                let pinned = self.data.is_pinned(gain.server_id, gain.pet_id);
                settings.admits(gain, pinned)
            })
            .copied()
            .collect()
    }

    fn change_pins(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut PetData, u32) -> bool) -> bool {
        let Some(server_id) = self.data.current_server else {
            return false;
        };
        if !change(&mut self.data, server_id) {
            return false;
        }
        self.save.changed(cx);
        cx.notify();
        true
    }

    fn remember_gain(&mut self, gain: LatestGain) {
        let sequence = self.recent_gains.back().map_or(1, |latest| latest.sequence + 1);
        if self.recent_gains.len() == REMEMBERED_GAINS {
            self.recent_gains.pop_front();
        }
        self.recent_gains.push_back(LatestGain { sequence, ..gain });
    }

    /// Applies what the game said, in the order it came.
    pub fn apply(&mut self, reports: &[PetReport], cx: &mut Context<Self>) {
        let mut save_needed = false;
        let mut gained = false;
        for report in reports {
            let server_id = report.character.server_id;
            save_needed |= self.data.current_server != Some(server_id);
            self.data.current_server = Some(server_id);
            match &report.content {
                PetContent::Snapshot(snapshot) => {
                    self.data.server_mut(server_id).apply_snapshot(snapshot, report.captured_at);
                    save_needed = true;
                }
                PetContent::SoulGains { gains, source } => {
                    for gain in gains {
                        let collection = self.data.server_mut(server_id);
                        let before = collection.pet(gain.pet_id);
                        let now = collection.apply_soul_gain(gain);
                        self.remember_gain(LatestGain {
                            sequence: 0,
                            server_id,
                            pet_id: gain.pet_id,
                            amount: gain.amount,
                            level_before: before.level,
                            level: now.level,
                            souls: now.souls,
                            source: (*source).into(),
                            captured_at: report.captured_at,
                        });
                        save_needed = true;
                        gained = true;
                    }
                }
                PetContent::Unlock(unlock) => {
                    self.data.server_mut(server_id).apply_unlock(unlock);
                    save_needed = true;
                }
                PetContent::CounterGains { looted_mobs, .. } => {
                    let Some(collection) = self.data.servers.get(&server_id) else {
                        continue;
                    };
                    for pet_id in maxed_pets_among(collection, looted_mobs) {
                        self.remember_gain(LatestGain {
                            sequence: 0,
                            server_id,
                            pet_id,
                            amount: 1,
                            level_before: MAX_LEVEL,
                            level: MAX_LEVEL,
                            souls: 0,
                            source: GainSource::MaxedLoot,
                            captured_at: report.captured_at,
                        });
                        gained = true;
                    }
                }
            }
        }
        if save_needed {
            self.save.changed(cx);
        }
        if save_needed || gained {
            cx.notify();
        }
    }

    /// Writes any change not yet on disk now, on this thread; used when the app quits.
    pub fn flush(&mut self) {
        self.save.flush(&self.data);
    }
}

fn unix_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |since| since.as_secs_f64())
}

/// The pets, each once, that the looted mobs drop souls of and that are at
/// their cap in `collection`.
fn maxed_pets_among(collection: &PetCollection, looted_mobs: &[u32]) -> Vec<u32> {
    let mut pets = Vec::new();
    for pet_id in looted_mobs.iter().filter_map(|&mob_code| game_data::pet_of_mob(mob_code)) {
        let at_cap = collection.pet(pet_id) == PetState { level: MAX_LEVEL, souls: 0 };
        if at_cap && !pets.contains(&pet_id) {
            pets.push(pet_id);
        }
    }
    pets
}

/// Starts the collection and makes it the app's, for [`global`]. It is stored
/// beside `settings.json`, or kept in memory when there is no data folder (a
/// replay or a capture).
pub fn install(cx: &mut App) {
    let data_dir = crate::data_folder::data_dir();
    let pets = cx.new(|_| Pets::open(data_dir.as_deref()));
    cx.on_app_quit({
        let pets = pets.clone();
        move |cx| {
            pets.update(cx, |pets, _| pets.flush());
            async {}
        }
    })
    .detach();
    cx.set_global(PetsHandle(pets));
}

pub fn global(cx: &App) -> Entity<Pets> {
    cx.global::<PetsHandle>().0.clone()
}

/// Hands the collection what the game said about the pets, in the order the
/// events came; the bridge calls it for every batch, whether or not the meter
/// is open.
pub fn record(cx: &mut App, reports: Vec<PetReport>) {
    global(cx).update(cx, |pets, cx| pets.apply(&reports, cx));
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use gpui::TestAppContext;
    use meter_core::capture::parser::pet_record::{
        CounterGain, PetLevel, PetProgress, PetSnapshot, PetUnlock, SoulGain,
    };
    use meter_core::pet_sources::SoulSource;
    use meter_core::identity::KnownCharacter;
    use meter_core::pets::PetState;

    use super::*;
    use crate::background_save::SAVE_AFTER;

    fn report(server_id: u32, captured_at: f64, content: PetContent) -> PetReport {
        PetReport {
            character: KnownCharacter { name: "Alder".to_string(), server_id, class: "RANGER".to_string(), key: None },
            captured_at,
            content,
        }
    }

    fn snapshot(levels: &[(u32, u32)], progress: &[(u32, u32)]) -> PetContent {
        PetContent::Snapshot(PetSnapshot {
            levels: levels.iter().map(|&(pet_id, level)| PetLevel { pet_id, level }).collect(),
            progress: progress.iter().map(|&(pet_id, progress)| PetProgress { pet_id, progress }).collect(),
            counters: Vec::new(),
        })
    }

    fn gains(gains: &[(u32, u32)]) -> PetContent {
        gains_from(SoulSource::Other, gains)
    }

    fn gains_from(source: SoulSource, gains: &[(u32, u32)]) -> PetContent {
        PetContent::SoulGains {
            gains: gains.iter().map(|&(pet_id, amount)| SoulGain { pet_id, amount }).collect(),
            source,
        }
    }

    fn counter_gains(looted_mobs: &[u32]) -> PetContent {
        PetContent::CounterGains { gains: vec![CounterGain { kind: 4, amount: 1 }], looted_mobs: looted_mobs.to_vec() }
    }

    const FUNGEN_MOB: u32 = 2_100_456;
    const ARMADON_MOB: u32 = 2_100_453;
    const KALGOLEM_MOB: u32 = 2_100_454;

    fn recent_gains(pets: &Entity<Pets>, after: u64, settings: &PetsSettings, cx: &mut TestAppContext) -> Vec<LatestGain> {
        cx.update(|cx| pets.read(cx).card_gains_after(after, settings))
    }

    fn pin(pets: &Entity<Pets>, pet_id: u32, cx: &mut TestAppContext) -> bool {
        cx.update(|cx| pets.update(cx, |pets, cx| pets.pin(pet_id, cx)))
    }

    fn unpin(pets: &Entity<Pets>, pet_id: u32, cx: &mut TestAppContext) -> bool {
        cx.update(|cx| pets.update(cx, |pets, cx| pets.unpin(pet_id, cx)))
    }

    fn pinned(pets: &Entity<Pets>, cx: &mut TestAppContext) -> Vec<u32> {
        cx.update(|cx| pets.read(cx).pinned().to_vec())
    }

    fn open(data_dir: Option<&Path>, cx: &mut TestAppContext) -> Entity<Pets> {
        let data_dir = data_dir.map(Path::to_path_buf);
        cx.update(|cx| cx.new(|_| Pets::open(data_dir.as_deref())))
    }

    fn apply(pets: &Entity<Pets>, reports: Vec<PetReport>, cx: &mut TestAppContext) {
        cx.update(|cx| pets.update(cx, |pets, cx| pets.apply(&reports, cx)));
    }

    fn pet(pets: &Entity<Pets>, pet_id: u32, cx: &mut TestAppContext) -> PetState {
        cx.update(|cx| pets.read(cx).collection().map(|collection| collection.pet(pet_id)).unwrap_or_default())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("aion-pets-service-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[gpui::test]
    fn a_snapshot_then_a_gain_gives_the_pet_and_the_latest_gain(cx: &mut TestAppContext) {
        let pets = open(None, cx);

        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[(1086, 27)]))], cx);
        apply(&pets, vec![report(1302, 114.5, gains(&[(1086, 1)]))], cx);

        assert_eq!(pet(&pets, 1086, cx), PetState { level: 2, souls: 28 });
        let latest = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();
        assert_eq!(
            latest,
            LatestGain {
                sequence: 1,
                server_id: 1302,
                pet_id: 1086,
                amount: 1,
                level_before: 2,
                level: 2,
                souls: 28,
                source: GainSource::Other,
                captured_at: 114.5
            }
        );
    }

    #[gpui::test]
    fn a_gain_that_levels_a_pet_up_carries_the_extra_souls_and_reports_the_new_level(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1012, 1)], &[(1012, 21)]))], cx);

        apply(&pets, vec![report(1302, 101.0, gains(&[(1012, 5)]))], cx);

        let latest = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();
        assert_eq!((latest.level, latest.souls), (2, 1));
    }

    #[gpui::test]
    fn a_batch_applies_every_gain_and_the_latest_is_the_last(cx: &mut TestAppContext) {
        let pets = open(None, cx);

        apply(&pets, vec![report(1302, 100.0, gains(&[(1182, 5), (1041, 5), (1182, 1)]))], cx);

        assert_eq!(pet(&pets, 1182, cx), PetState { level: 1, souls: 1 });
        assert_eq!(pet(&pets, 1041, cx), PetState { level: 1, souls: 0 });
        let latest = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();
        assert_eq!((latest.pet_id, latest.sequence), (1182, 3));
    }

    #[gpui::test]
    fn a_pet_at_its_cap_keeps_the_cap_and_each_gain_still_counts_as_a_new_one(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1214, 3)], &[]))], cx);

        apply(&pets, vec![report(1302, 101.0, gains(&[(1214, 1)]))], cx);
        let first = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();
        apply(&pets, vec![report(1302, 102.0, gains(&[(1214, 1)]))], cx);
        let second = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();

        assert_eq!((first.level, first.souls, second.level, second.souls), (3, 0, 3, 0));
        assert_ne!(first.sequence, second.sequence);
    }

    #[gpui::test]
    fn each_server_keeps_its_own_collection_and_the_current_one_follows_the_game(cx: &mut TestAppContext) {
        let pets = open(None, cx);

        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[]))], cx);
        apply(&pets, vec![report(1001, 200.0, snapshot(&[(1086, 1)], &[]))], cx);
        assert_eq!(cx.update(|cx| pets.read(cx).current_server()), Some(1001));
        assert_eq!(pet(&pets, 1086, cx).level, 1);

        apply(&pets, vec![report(1302, 300.0, gains(&[(1086, 1)]))], cx);
        assert_eq!(cx.update(|cx| pets.read(cx).current_server()), Some(1302));
        assert_eq!(pet(&pets, 1086, cx), PetState { level: 2, souls: 1 });
    }

    #[gpui::test]
    fn counter_gains_without_a_looted_pet_at_its_cap_change_nothing_and_set_no_latest_gain(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2), (1110, 3)], &[(1086, 27)]))], cx);

        apply(&pets, vec![report(1302, 101.0, counter_gains(&[]))], cx);
        apply(&pets, vec![report(1302, 102.0, counter_gains(&[1]))], cx);
        apply(&pets, vec![report(1302, 103.0, counter_gains(&[FUNGEN_MOB]))], cx);

        assert_eq!(pet(&pets, 1086, cx), PetState { level: 2, souls: 27 });
        assert!(cx.update(|cx| pets.read(cx).latest_gain().is_none()), "no mob, an unknown mob, a pet below the cap");
    }

    #[gpui::test]
    fn a_counter_gain_after_the_loot_of_a_pet_at_its_cap_is_a_maxed_loot_gain(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2), (1110, 3)], &[(1086, 27)]))], cx);

        apply(&pets, vec![report(1302, 101.0, counter_gains(&[KALGOLEM_MOB]))], cx);

        let latest = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();
        assert_eq!(
            latest,
            LatestGain {
                sequence: 1,
                server_id: 1302,
                pet_id: 1110,
                amount: 1,
                level_before: 3,
                level: 3,
                souls: 0,
                source: GainSource::MaxedLoot,
                captured_at: 101.0
            }
        );
        assert_eq!(pet(&pets, 1110, cx), PetState { level: 3, souls: 0 });
    }

    #[gpui::test]
    fn a_counter_gain_names_each_maxed_pet_it_looted_once_and_skips_the_others(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 3), (1110, 3), (1047, 1)], &[]))], cx);

        apply(&pets, vec![report(1302, 101.0, counter_gains(&[KALGOLEM_MOB, ARMADON_MOB, FUNGEN_MOB, KALGOLEM_MOB]))], cx);

        let gains = recent_gains(&pets, 0, &PetsSettings::default(), cx);
        assert_eq!(gains.iter().map(|gain| (gain.sequence, gain.pet_id)).collect::<Vec<_>>(), vec![(1, 1110), (2, 1086)]);
    }

    #[gpui::test]
    fn a_counter_gain_on_a_server_with_no_collection_yet_sets_nothing(cx: &mut TestAppContext) {
        let pets = open(None, cx);

        apply(&pets, vec![report(1302, 101.0, counter_gains(&[KALGOLEM_MOB]))], cx);

        assert!(cx.update(|cx| pets.read(cx).latest_gain().is_none()));
    }

    #[gpui::test]
    fn a_soul_gain_carries_its_source_and_whether_the_pet_was_already_at_its_cap(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1214, 3), (1012, 2)], &[(1012, 74)]))], cx);

        apply(&pets, vec![report(1302, 101.0, gains_from(SoulSource::BlackMarket, &[(1214, 5), (1012, 1)]))], cx);
        apply(&pets, vec![report(1302, 102.0, gains_from(SoulSource::Loot, &[(1012, 1)]))], cx);

        let gains = recent_gains(&pets, 0, &PetsSettings::default(), cx);
        let summary: Vec<_> = gains.iter().map(|gain| (gain.pet_id, gain.source, gain.already_maxed(), gain.level)).collect();
        assert_eq!(
            summary,
            vec![
                (1214, GainSource::BlackMarket, true, 3),
                (1012, GainSource::BlackMarket, false, 3),
                (1012, GainSource::Loot, true, 3),
            ],
            "a pet that reaches the cap with a gain was not at it before; the next gain finds it there"
        );
    }

    #[gpui::test]
    fn the_card_gets_every_gain_of_a_batch_after_the_sequence_it_handled(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, gains(&[(1182, 5), (1041, 5)]))], cx);
        apply(&pets, vec![report(1302, 101.0, gains(&[(1039, 5)]))], cx);

        let settings = PetsSettings::default();
        let mut pet_ids = |after| recent_gains(&pets, after, &settings, cx).iter().map(|gain| gain.pet_id).collect::<Vec<_>>();
        assert_eq!(pet_ids(0), vec![1182, 1041, 1039]);
        assert_eq!(pet_ids(2), vec![1039]);
        assert!(pet_ids(3).is_empty());
    }

    #[gpui::test]
    fn the_card_filter_follows_the_four_settings(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1214, 3)], &[]))], cx);
        assert!(pin(&pets, 1041, cx));
        apply(&pets, vec![report(1302, 101.0, gains_from(SoulSource::Loot, &[(1182, 1)]))], cx);
        apply(&pets, vec![report(1302, 102.0, gains_from(SoulSource::BlackMarket, &[(1041, 5)]))], cx);
        apply(&pets, vec![report(1302, 103.0, counter_gains(&[]))], cx);
        apply(&pets, vec![report(1302, 104.0, gains_from(SoulSource::Loot, &[(1214, 1)]))], cx);

        let shown = |settings: PetsSettings, cx: &mut TestAppContext| {
            recent_gains(&pets, 0, &settings, cx).iter().map(|gain| gain.pet_id).collect::<Vec<_>>()
        };
        let defaults = PetsSettings::default();
        assert_eq!(shown(defaults.clone(), cx), vec![1182, 1041, 1214]);
        assert!(shown(PetsSettings { soul_tracker: false, ..defaults.clone() }, cx).is_empty());
        assert_eq!(shown(PetsSettings { only_pinned: true, ..defaults.clone() }, cx), vec![1041]);
        assert_eq!(shown(PetsSettings { maxed_pets: false, ..defaults.clone() }, cx), vec![1182, 1041]);
        assert_eq!(shown(PetsSettings { black_market: false, ..defaults }, cx), vec![1182, 1214]);
    }

    #[gpui::test]
    fn pins_belong_to_the_server_being_played_and_keep_the_order_they_were_made_in(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        assert!(!pin(&pets, 1086, cx), "no server is known yet");
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[]))], cx);

        assert!(pin(&pets, 1214, cx));
        assert!(pin(&pets, 1086, cx));
        assert!(!pin(&pets, 1214, cx), "already pinned");
        assert_eq!(pinned(&pets, cx), vec![1214, 1086]);
        assert!(cx.update(|cx| pets.read(cx).is_pinned(1086)));

        apply(&pets, vec![report(1001, 200.0, snapshot(&[(1086, 1)], &[]))], cx);
        assert!(pinned(&pets, cx).is_empty(), "another server has its own pins");
        assert!(pin(&pets, 1119, cx));
        apply(&pets, vec![report(1302, 300.0, gains(&[(1086, 1)]))], cx);
        assert_eq!(pinned(&pets, cx), vec![1214, 1086], "back on the first server");

        assert!(unpin(&pets, 1214, cx));
        assert!(!unpin(&pets, 1214, cx), "not pinned any more");
        assert_eq!(pinned(&pets, cx), vec![1086]);
    }

    #[gpui::test]
    fn a_snapshot_keeps_the_pins_of_its_server(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[]))], cx);
        pin(&pets, 1086, cx);

        apply(&pets, vec![report(1302, 200.0, snapshot(&[(1086, 3)], &[]))], cx);

        assert_eq!(pinned(&pets, cx), vec![1086]);
        assert_eq!(pet(&pets, 1086, cx).level, 3);
    }

    #[gpui::test]
    fn pins_are_written_with_the_collection_and_read_back_per_server(cx: &mut TestAppContext) {
        let dir = temp_dir("pins");
        let pets = open(Some(&dir), cx);
        apply(&pets, vec![report(1001, 100.0, snapshot(&[(1010, 1)], &[]))], cx);
        pin(&pets, 1010, cx);
        apply(&pets, vec![report(1302, 200.0, snapshot(&[(1086, 2)], &[]))], cx);
        pin(&pets, 1214, cx);
        pin(&pets, 1086, cx);
        cx.update(|cx| pets.update(cx, |pets, _| pets.flush()));

        let reopened = open(Some(&dir), cx);

        assert_eq!(pinned(&reopened, cx), vec![1214, 1086]);
        apply(&reopened, vec![report(1001, 300.0, gains(&[(1010, 1)]))], cx);
        assert_eq!(pinned(&reopened, cx), vec![1010]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn unlock(pet_id: u32) -> PetContent {
        PetContent::Unlock(PetUnlock { pet_id })
    }

    #[gpui::test]
    fn an_unlock_puts_the_pet_at_level_one_and_sets_no_latest_gain(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[(1192, 3)]))], cx);

        apply(&pets, vec![report(1302, 101.0, unlock(1192)), report(1302, 101.0, unlock(1206))], cx);

        assert_eq!(pet(&pets, 1192, cx), PetState { level: 1, souls: 3 });
        assert_eq!(pet(&pets, 1206, cx), PetState { level: 1, souls: 0 });
        assert!(cx.update(|cx| pets.read(cx).latest_gain().is_none()), "the card follows soul gains, not unlocks");
    }

    #[gpui::test]
    fn an_unlock_after_the_gain_that_unlocked_the_pet_counts_nothing_twice_and_never_lowers_a_pet(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[(1108, 4), (1086, 27)]))], cx);

        apply(&pets, vec![report(1302, 101.0, gains(&[(1108, 1)])), report(1302, 101.0, unlock(1108))], cx);
        apply(&pets, vec![report(1302, 102.0, unlock(1086))], cx);

        assert_eq!(pet(&pets, 1108, cx), PetState { level: 1, souls: 0 });
        assert_eq!(pet(&pets, 1086, cx), PetState { level: 2, souls: 27 });
        let latest = cx.update(|cx| pets.read(cx).latest_gain().copied()).unwrap();
        assert_eq!((latest.pet_id, latest.level, latest.souls, latest.sequence), (1108, 1, 0, 1));
    }

    #[gpui::test]
    fn the_collection_is_written_after_the_pause_and_read_back(cx: &mut TestAppContext) {
        let dir = temp_dir("debounce");
        let pets = open(Some(&dir), cx);

        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[(1086, 27)]))], cx);
        assert!(!store::path_in(&dir).exists(), "nothing is written before the pause");
        cx.executor().advance_clock(SAVE_AFTER * 2);
        cx.run_until_parked();

        assert!(store::path_in(&dir).exists());
        let reopened = open(Some(&dir), cx);
        assert_eq!(pet(&reopened, 1086, cx), PetState { level: 2, souls: 27 });
        assert_eq!(cx.update(|cx| reopened.read(cx).current_server()), Some(1302));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn flush_writes_at_once_and_a_bad_file_is_set_aside_on_the_next_open(cx: &mut TestAppContext) {
        let dir = temp_dir("flush");
        let pets = open(Some(&dir), cx);
        apply(&pets, vec![report(1302, 100.0, gains(&[(1119, 3)]))], cx);
        cx.update(|cx| pets.update(cx, |pets, _| pets.flush()));
        assert_eq!(pet(&open(Some(&dir), cx), 1119, cx), PetState { level: 0, souls: 3 });

        std::fs::write(store::path_in(&dir), "not json {").unwrap();
        let reopened = open(Some(&dir), cx);

        assert!(cx.update(|cx| reopened.read(cx).collection().is_none()));
        assert!(!store::path_in(&dir).exists());
        assert!(dir.read_dir().unwrap().any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with("pets.json.bad-")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn a_snapshot_of_this_run_is_live_and_one_kept_from_an_earlier_run_is_not(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        assert!(!cx.update(|cx| pets.read(cx).read_this_run()), "nothing read yet");

        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2)], &[]))], cx);
        assert!(!cx.update(|cx| pets.read(cx).read_this_run()), "captured before this run began");

        apply(&pets, vec![report(1302, unix_secs() + 1.0, snapshot(&[(1086, 2)], &[]))], cx);
        assert!(cx.update(|cx| pets.read(cx).read_this_run()));
    }

    #[gpui::test]
    fn the_window_menu_choice_is_written_and_read_back(cx: &mut TestAppContext) {
        use crate::pets::list::{Order, Show};
        let dir = temp_dir("list-choice");
        let pets = open(Some(&dir), cx);
        let choice = ListChoice { show: Show::NotMaxed, order: Order::ClosestToNextLevel };

        cx.update(|cx| pets.update(cx, |pets, cx| pets.set_list_choice(choice, cx)));
        cx.update(|cx| pets.update(cx, |pets, _| pets.flush()));

        let reopened = open(Some(&dir), cx);
        assert_eq!(cx.update(|cx| reopened.read(cx).list_choice()), choice);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn the_test_card_shows_the_first_pinned_pet_as_it_is_or_else_the_first_pet_in_game_order(cx: &mut TestAppContext) {
        let pets = open(None, cx);
        let shown = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = shown.clone();
        cx.update(|cx| cx.subscribe(&pets, move |_, request: &ShowTestCard, _| seen.borrow_mut().push(*request)).detach());

        cx.update(|cx| pets.update(cx, |pets, cx| pets.show_test_card(cx)));
        apply(&pets, vec![report(1302, 100.0, snapshot(&[(1086, 2), (1214, 3)], &[(1086, 27)]))], cx);
        pin(&pets, 1086, cx);
        pin(&pets, 1214, cx);
        cx.update(|cx| pets.update(cx, |pets, cx| pets.show_test_card(cx)));

        let first = game_data::pet_ids()[0];
        assert_eq!(
            *shown.borrow(),
            vec![
                ShowTestCard { pet_id: first, state: PetState::default() },
                ShowTestCard { pet_id: 1086, state: PetState { level: 2, souls: 27 } },
            ]
        );
        assert!(cx.update(|cx| pets.read(cx).latest_gain().is_none()), "nothing is added");
    }
}
