use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use meter_core::events::MeterEvent;
use meter_core::identity::KnownCharacter;
use meter_core::tickets::TicketReport;
use gpui::{App, AppContext as _, Context, Entity, Global, Task};

use super::catalogue::Section;
use super::data::{
    AddError, Cadence, CharacterId, HiddenSummary, Moment, NewOwnTodo, PoolAbovePlainCap, ProgressChange, RemovedOwnTodo,
    RowId, TodoData,
};
use super::list::{self, CharacterSummary, FullResource, TodoList};
use super::settings::TodoSettings;
use super::store;
use crate::clock::{now_ms, ClockTime, LocalZone};
use crate::settings::Settings;

/// A change is written to disk once nothing has changed for this long, so a
/// run of taps on an arrow causes one write.
const SAVE_AFTER: Duration = Duration::from_millis(400);

/// The countdowns are read to the minute: the tick lands just after each one.
const TICK_SLACK: Duration = Duration::from_millis(50);

static LOCAL_ZONE: LocalZone = LocalZone;

/// Who the game says the player is, as far as it said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerRef {
    pub name: String,
    pub server_id: Option<u32>,
    /// The game's character key (12 hex digits), when the packet carried one.
    pub key: Option<String>,
}

/// What a meter event says about the character being played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Identified {
    Named(PlayerRef),
    /// The meter knows the player's actor but not whose it is.
    Unnamed,
}

/// The identity a meter event carries, if any. The bridge calls this for each
/// event of a batch and hands the results to [`follow_player`].
pub fn identified_by(event: &MeterEvent) -> Option<Identified> {
    match event {
        MeterEvent::CharacterSeen(character) => Some(Identified::Named(PlayerRef {
            name: character.name.clone(),
            server_id: Some(character.server_id),
            key: character.key.clone(),
        })),
        MeterEvent::MainActorDetected(actor) if actor.actor_name.is_empty() => Some(Identified::Unnamed),
        MeterEvent::MainActorDetected(actor) => Some(Identified::Named(PlayerRef {
            name: actor.actor_name.clone(),
            server_id: actor.sid.as_deref().and_then(|server| server.trim().parse().ok()),
            key: None,
        })),
        _ => None,
    }
}

/// The to-do tracker: every character's lists, the resets that keep them
/// fresh, and which character the window shows. Lives on the UI thread; the
/// window observes it. All changes come through its methods.
pub struct Todos {
    settings: Entity<Settings>,
    data: TodoData,
    config: TodoSettings,
    remembered: Vec<KnownCharacter>,
    playing: Option<CharacterId>,
    /// The last character the game named; a different one takes the window back from a manual pick.
    last_named: Option<CharacterId>,
    picked: Option<CharacterId>,
    file: Option<PathBuf>,
    save: Option<Task<()>>,
    saved: bool,
    save_error: Option<String>,
    load_notice: Option<String>,
    /// Counts changes; a write carries the number of the value it holds.
    revision: u64,
    /// The revision last written. Writers take the lock, so an older value,
    /// finishing late, never replaces a newer one on disk.
    written: Arc<Mutex<u64>>,
    ticker: Option<Task<()>>,
    /// When this run began: what the game said since, about the character
    /// being played, is live.
    run_started_ms: i64,
}

struct TodosHandle(Entity<Todos>);

impl Global for TodosHandle {}

impl Todos {
    /// Reads `todos.json` from `data_dir`; without a folder the to-dos live in
    /// memory only. `follows_clock` false keeps the minute tick off (replay and
    /// probe runs); resets are still applied when the tracker opens and on
    /// every change.
    pub fn open(settings: &Entity<Settings>, data_dir: Option<&Path>, follows_clock: bool, cx: &mut Context<Self>) -> Self {
        let (file, loaded) = match data_dir {
            Some(dir) => {
                let file = store::path_in(dir);
                let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
                let loaded = store::load(&file, now_secs);
                if let Some(notice) = &loaded.notice {
                    eprintln!("{notice}");
                }
                (Some(file), loaded)
            }
            None => (None, store::Loaded { data: TodoData::default(), notice: None }),
        };
        Self::new(settings, loaded.data, file, loaded.notice, follows_clock, cx)
    }

    pub fn new(
        settings: &Entity<Settings>,
        data: TodoData,
        file: Option<PathBuf>,
        load_notice: Option<String>,
        follows_clock: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(settings, |this, settings, cx| this.follow_settings(&settings, cx)).detach();
        let (config, remembered) = {
            let all = settings.read(cx).get();
            (all.todos.clone(), all.known_characters.clone())
        };
        let mut todos = Self {
            settings: settings.clone(),
            data,
            config,
            remembered,
            playing: None,
            last_named: None,
            picked: None,
            file,
            save: None,
            saved: true,
            save_error: None,
            load_notice,
            revision: 0,
            written: Arc::default(),
            ticker: None,
            run_started_ms: now_ms(),
        };
        let moment = todos.moment();
        let synced = todos.data.sync_characters(&todos.remembered);
        let reset = todos.data.apply_resets(&moment);
        if synced || reset {
            todos.changed(cx);
        }
        if follows_clock {
            todos.ticker = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(until_next_minute()).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }));
        }
        todos
    }

    fn moment(&self) -> Moment<'static> {
        let now = now_ms();
        let moment = match self.config.reset_hour {
            Some(hour) => Moment::new(&LOCAL_ZONE, hour, self.config.membership, now),
            None => Moment::following_the_game(&LOCAL_ZONE, self.config.membership, now),
        };
        moment.running_since(self.run_started_ms)
    }

    /// Applies the resets that are due, then notifies so countdowns redraw.
    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.data.apply_resets(&self.moment()) {
            self.changed(cx);
        } else {
            cx.notify();
        }
    }

    fn follow_settings(&mut self, settings: &Entity<Settings>, cx: &mut Context<Self>) {
        let all = settings.read(cx).get();
        let config = (all.todos != self.config).then(|| all.todos.clone());
        let remembered = (all.known_characters != self.remembered).then(|| all.known_characters.clone());
        let mut dirty = false;
        let mut seen = false;
        if let Some(remembered) = remembered {
            dirty |= self.data.sync_characters(&remembered);
            self.remembered = remembered;
            seen = true;
        }
        if let Some(config) = config {
            let previous = std::mem::replace(&mut self.config, config);
            if previous.reset_hour != self.config.reset_hour {
                dirty |= self.data.rebase_resets(&self.moment());
            }
            seen = true;
        }
        if dirty {
            self.changed(cx);
        } else if seen {
            cx.notify();
        }
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.revision += 1;
        self.schedule_save(cx);
        cx.notify();
    }

    /// Applies the resets that are due, runs `change` on the data and keeps the
    /// result; `change` says whether it changed anything.
    fn edit<R>(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut TodoData, &Moment) -> (R, bool)) -> R {
        let moment = self.moment();
        let reset = self.data.apply_resets(&moment);
        let (result, changed) = change(&mut self.data, &moment);
        if reset || changed {
            self.changed(cx);
        }
        result
    }

    // --- Which character ---

    /// The list the window shows: the character picked by hand, else the one
    /// being played, else the last one played; none when no character is
    /// remembered.
    pub fn selected(&self) -> Option<CharacterId> {
        [self.picked, self.playing, self.last_named]
            .into_iter()
            .flatten()
            .find(|&id| self.data.character(id).is_some())
            .or_else(|| self.remembered.first().and_then(|known| self.data.find(known)))
    }

    /// False while a character was picked by hand.
    #[cfg(test)]
    pub fn follows_the_game(&self) -> bool {
        self.picked.is_none()
    }

    /// The remembered characters, most recent first, for the picker.
    pub fn characters(&self) -> Vec<CharacterSummary> {
        let moment = self.moment();
        let mut seen = Vec::new();
        self.remembered
            .iter()
            .filter_map(|known| self.data.find(known))
            .filter(|id| {
                let first = !seen.contains(id);
                seen.push(*id);
                first
            })
            .filter_map(|id| self.data.character(id))
            .map(|character| list::summary_of(&self.data, character, Some(character.id) == self.playing, &moment))
            .collect()
    }

    /// Shows another character's list until the game names the character
    /// being played again.
    pub fn select(&mut self, id: CharacterId, cx: &mut Context<Self>) {
        if self.data.character(id).is_none() || self.picked == Some(id) {
            return;
        }
        self.picked = Some(id);
        cx.notify();
    }

    /// Back to following the character in game.
    pub fn follow_the_game(&mut self, cx: &mut Context<Self>) {
        if self.picked.take().is_some() {
            cx.notify();
        }
    }

    /// The meter learned who is being played. A character that is not
    /// remembered (or an `Unnamed` player) leaves the list on the last one
    /// played. A different character than before drops a manual pick.
    pub fn player_identified(&mut self, identified: &Identified, cx: &mut Context<Self>) {
        let now_playing = match identified {
            Identified::Named(player) => self.remembered_character(player).and_then(|known| self.data.find(known)),
            Identified::Unnamed => None,
        };
        if now_playing == self.playing {
            return;
        }
        self.playing = now_playing;
        if now_playing.is_some() && now_playing != self.last_named {
            self.last_named = now_playing;
            self.picked = None;
        }
        cx.notify();
    }

    fn remembered_character(&self, player: &PlayerRef) -> Option<&KnownCharacter> {
        self.remembered.iter().find(|known| match (&player.key, &known.key) {
            (Some(own), Some(other)) => own == other,
            _ => {
                known.name == player.name && player.server_id.is_none_or(|server_id| server_id == known.server_id)
            }
        })
    }

    // --- Reading ---

    /// The selected character's list; none when no character is remembered.
    pub fn list(&self) -> Option<TodoList> {
        self.list_of(self.selected()?)
    }

    pub fn list_of(&self, id: CharacterId) -> Option<TodoList> {
        let character = self.data.character(id)?;
        Some(list::build(&self.data, character, Some(id) == self.playing, &self.moment()))
    }

    /// When the game last listed the character's tickets at login (Unix ms).
    pub fn logged_in_ms(&self, id: CharacterId) -> Option<i64> {
        self.data.character(id)?.logged_in_ms()
    }

    /// What the selected character has at its cap with a refill coming, in
    /// the Resources window's order, for the toolbar's warning.
    pub fn full_resources(&self) -> Vec<FullResource> {
        let Some(id) = self.selected() else {
            return Vec::new();
        };
        let Some(character) = self.data.character(id) else {
            return Vec::new();
        };
        list::full_resources(&self.data, character, Some(id) == self.playing, &self.moment())
    }

    /// The game shows more than the plain cap in a pool the membership raises
    /// while the membership switch is off: Settings asks if the player has it.
    pub fn membership_hint(&self) -> Option<PoolAbovePlainCap> {
        if self.config.membership {
            return None;
        }
        self.data.pool_above_plain_cap()
    }

    /// The daily reset on the PC's clock now.
    pub fn reset_clock(&self) -> ClockTime {
        self.config.reset_clock_now()
    }

    pub fn membership(&self) -> bool {
        self.config.membership
    }

    pub fn hidden_summary(&self) -> HiddenSummary {
        self.data.hidden_summary()
    }

    pub fn own_total(&self) -> usize {
        self.data.own_total()
    }

    pub fn load_notice(&self) -> Option<&str> {
        self.load_notice.as_deref()
    }

    pub fn save_error(&self) -> Option<&str> {
        self.save_error.as_deref()
    }

    // --- Changing a character's list ---

    pub fn toggle_tick(&mut self, id: CharacterId, row: RowId, cx: &mut Context<Self>) -> Option<ProgressChange> {
        self.edit(cx, |data, _| {
            let change = data.toggle_tick(id, row);
            (change, change.is_some())
        })
    }

    /// Adds `delta` to a count, clamped to 0 and its target; `None` when nothing changed.
    pub fn adjust_count(&mut self, id: CharacterId, row: RowId, delta: i32, cx: &mut Context<Self>) -> Option<ProgressChange> {
        self.edit(cx, |data, _| {
            let change = data.adjust_count(id, row, delta);
            (change, change.is_some())
        })
    }

    /// Types what the character has (`Some`) or stops tracking it (`None`).
    pub fn set_amount(&mut self, id: CharacterId, row: &'static str, value: Option<u32>, cx: &mut Context<Self>) -> bool {
        self.edit(cx, |data, moment| {
            let changed = data.set_amount(id, row, value, moment);
            (changed, changed)
        })
    }

    /// Types how much of a row the game fills is done (`Some`), or goes back
    /// to the game's value (`None`).
    pub fn set_progress(&mut self, id: CharacterId, row: &'static str, value: Option<u32>, cx: &mut Context<Self>) -> bool {
        self.edit(cx, |data, _| {
            let changed = data.set_progress(id, row, value);
            (changed, changed)
        })
    }

    /// One arrow step (`direction` above 0 is up) from what the amount is worth now.
    pub fn step_amount(&mut self, id: CharacterId, row: &'static str, direction: i32, cx: &mut Context<Self>) -> bool {
        self.edit(cx, |data, moment| {
            let changed = data.step_amount(id, row, direction, moment);
            (changed, changed)
        })
    }

    pub fn hide(&mut self, id: CharacterId, row: &'static str, cx: &mut Context<Self>) -> bool {
        self.edit(cx, |data, _| {
            let changed = data.hide(id, row);
            (changed, changed)
        })
    }

    /// Puts a hidden built-in back.
    pub fn show(&mut self, id: CharacterId, row: &'static str, cx: &mut Context<Self>) -> bool {
        self.edit(cx, |data, _| {
            let changed = data.show(id, row);
            (changed, changed)
        })
    }

    /// Adds a to-do of the player's own to the end of Today (daily) or This
    /// week (weekly) and unfolds that section.
    pub fn add_own(&mut self, id: CharacterId, request: NewOwnTodo, cx: &mut Context<Self>) -> Result<u32, AddError> {
        let section = match request.cadence {
            Cadence::Daily => Section::Today,
            Cadence::Weekly => Section::ThisWeek,
        };
        self.edit(cx, |data, _| {
            let added = data.add_own(id, request);
            if added.is_ok() {
                data.set_folded(section, false);
            }
            let changed = added.is_ok();
            (added, changed)
        })
    }

    pub fn remove_own(&mut self, id: CharacterId, own_id: u32, cx: &mut Context<Self>) -> Option<RemovedOwnTodo> {
        self.edit(cx, |data, _| {
            let removed = data.remove_own(id, own_id);
            let changed = removed.is_some();
            (removed, changed)
        })
    }

    /// The "Undo" of a removal.
    pub fn restore_own(&mut self, removed: RemovedOwnTodo, cx: &mut Context<Self>) -> bool {
        self.edit(cx, |data, _| {
            let restored = data.restore_own(removed);
            (restored, restored)
        })
    }

    // --- What the game says ---

    /// Keeps the tickets the game listed or changed, each on the character it
    /// named (and on every character of the server for a server ticket). Runs
    /// on the UI thread with the batch the bridge drained.
    pub fn apply_tickets(&mut self, reports: &[TicketReport], cx: &mut Context<Self>) {
        self.edit(cx, |data, _| {
            let changed = reports.iter().fold(false, |changed, report| data.apply_tickets(report) | changed);
            ((), changed)
        });
    }

    // --- View state, kept across runs ---

    pub fn set_section_folded(&mut self, section: Section, folded: bool, cx: &mut Context<Self>) {
        if self.data.set_folded(section, folded) {
            self.changed(cx);
        }
    }

    // --- Settings → To-dos ---

    pub fn set_reset_hour(&mut self, hour: u8, cx: &mut Context<Self>) {
        let choice = TodoSettings::choice_of_hour(hour, &LOCAL_ZONE, now_ms());
        self.settings.update(cx, |settings, cx| settings.update(cx, |all| all.todos.reset_hour = choice));
    }

    pub fn set_membership(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.update(cx, |settings, cx| settings.update(cx, |all| all.todos.membership = on));
    }

    /// Un-hides every built-in on every character; own to-dos, counts and
    /// amounts stay. Returns how many came back.
    pub fn restore_defaults(&mut self, cx: &mut Context<Self>) -> usize {
        self.edit(cx, |data, _| {
            let restored = data.restore_defaults();
            (restored, restored > 0)
        })
    }

    /// On every character: own to-dos removed, everything shown, every count,
    /// tick and amount emptied. The reset hour and the membership stay.
    pub fn reset_completely(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |data, _| ((), data.reset_completely()));
    }

    // --- Saving ---

    /// Writes any change not yet on disk now, on this thread; used when the app quits.
    pub fn flush(&mut self) {
        self.save = None;
        let Some(file) = self.file.as_ref().filter(|_| !self.saved) else {
            return;
        };
        match write_in_order(&self.written, self.revision, file, &self.data) {
            Ok(()) => {
                self.saved = true;
                self.save_error = None;
            }
            Err(error) => {
                eprintln!("{error}");
                self.save_error = Some(error);
            }
        }
    }

    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.saved = false;
        let Some(file) = self.file.clone() else {
            return;
        };
        self.save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_AFTER).await;
            let Ok(Some((revision, value, written))) = this.update(cx, |todos, _| {
                (!todos.saved).then(|| (todos.revision, todos.data.clone(), Arc::clone(&todos.written)))
            }) else {
                return;
            };
            let result = cx
                .background_executor()
                .spawn(async move { write_in_order(&written, revision, &file, &value) })
                .await;
            let _ = this.update(cx, |todos, cx| {
                match result {
                    Ok(()) => {
                        todos.saved = todos.saved || todos.revision == revision;
                        todos.save_error = None;
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        todos.save_error = Some(error);
                    }
                }
                cx.notify();
            });
        }));
    }

    #[cfg(test)]
    fn data(&self) -> &TodoData {
        &self.data
    }
}

fn until_next_minute() -> Duration {
    let into_minute = now_ms().rem_euclid(60_000) as u64;
    Duration::from_millis(60_000 - into_minute) + TICK_SLACK
}

fn write_in_order(written: &Mutex<u64>, revision: u64, file: &Path, value: &TodoData) -> Result<(), String> {
    let mut last = written.lock().unwrap_or_else(PoisonError::into_inner);
    if revision < *last {
        return Ok(());
    }
    store::save(file, value)?;
    *last = revision;
    Ok(())
}

/// Starts the tracker and makes it the app's, for [`global`]. `live` is a real
/// run: the countdowns tick with the clock. The to-dos are stored beside
/// `settings.json`, or kept in memory when the settings are.
pub fn install(settings: &Entity<Settings>, live: bool, cx: &mut App) {
    let data_dir = settings.read(cx).data_dir().map(Path::to_path_buf);
    let todos = cx.new(|cx| Todos::open(settings, data_dir.as_deref(), live, cx));
    cx.on_app_quit({
        let todos = todos.clone();
        move |cx| {
            todos.update(cx, |todos, _| todos.flush());
            async {}
        }
    })
    .detach();
    cx.set_global(TodosHandle(todos));
}

pub fn global(cx: &App) -> Entity<Todos> {
    cx.global::<TodosHandle>().0.clone()
}

/// Tells the tracker who the meter says is being played, in the order the
/// events came; the bridge calls it after the character is remembered.
pub fn follow_player(cx: &mut App, identified: Vec<Identified>) {
    global(cx).update(cx, |todos, cx| {
        for identified in &identified {
            todos.player_identified(identified, cx);
        }
    });
}

/// Hands the tracker what the game said about the tickets, in the order the
/// events came; the bridge calls it after the character is remembered.
pub fn record_tickets(cx: &mut App, reports: Vec<TicketReport>) {
    global(cx).update(cx, |todos, cx| todos.apply_tickets(&reports, cx));
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;
    use crate::settings::AppSettings;
    use crate::todos::data::OwnKind;

    fn known(name: &str, server_id: u32, key: Option<&str>) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id, class: "RANGER".to_string(), key: key.map(str::to_string) }
    }

    fn settings_with(characters: Vec<KnownCharacter>, cx: &mut TestAppContext) -> Entity<Settings> {
        cx.update(|cx| {
            cx.new(|_| {
                let mut all = AppSettings::default();
                all.known_characters = characters;
                Settings::in_memory(all)
            })
        })
    }

    fn open(settings: &Entity<Settings>, cx: &mut TestAppContext) -> Entity<Todos> {
        cx.update(|cx| cx.new(|cx| Todos::open(settings, None, false, cx)))
    }

    fn named(name: &str) -> Identified {
        Identified::Named(PlayerRef { name: name.to_string(), server_id: Some(1001), key: None })
    }

    fn selected_name(todos: &Entity<Todos>, cx: &mut TestAppContext) -> Option<String> {
        cx.update(|cx| todos.read(cx).list().map(|list| list.character.name))
    }

    #[gpui::test]
    fn no_remembered_character_means_no_list(cx: &mut TestAppContext) {
        let settings = settings_with(Vec::new(), cx);
        let todos = open(&settings, cx);
        assert_eq!(selected_name(&todos, cx), None);
        assert!(cx.update(|cx| todos.read(cx).characters().is_empty()));
    }

    #[gpui::test]
    fn before_the_game_names_anyone_the_last_played_character_shows(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None), known("Birch", 1001, None)], cx);
        let todos = open(&settings, cx);
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Alder"), "the remembered list is most recent first");
        let names: Vec<String> = cx.update(|cx| todos.read(cx).characters().into_iter().map(|c| c.name).collect());
        assert_eq!(names, ["Alder", "Birch"]);
    }

    #[gpui::test]
    fn the_list_follows_the_character_in_game_and_a_pick_lasts_until_it_changes(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None), known("Birch", 1001, None), known("Cedar", 1001, None)], cx);
        let todos = open(&settings, cx);
        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&named("Birch"), cx)));
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Birch"));
        let playing: Vec<bool> = cx.update(|cx| todos.read(cx).characters().into_iter().map(|c| c.playing).collect());
        assert_eq!(playing, [false, true, false]);

        let cedar = cx.update(|cx| todos.read(cx).characters()[2].id);
        cx.update(|cx| todos.update(cx, |todos, cx| todos.select(cedar, cx)));
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Cedar"));
        assert!(!cx.update(|cx| todos.read(cx).follows_the_game()));

        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&named("Birch"), cx)));
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Cedar"), "the same character again changes nothing");

        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&named("Alder"), cx)));
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Alder"), "another character in game takes the window back");
        assert!(cx.update(|cx| todos.read(cx).follows_the_game()));
    }

    #[gpui::test]
    fn a_character_is_matched_by_key_before_name_and_by_server(cx: &mut TestAppContext) {
        let settings = settings_with(
            vec![known("Alder", 1001, Some("3AA503000000")), known("Alder", 2002, None), known("Birch", 1001, None)],
            cx,
        );
        let todos = open(&settings, cx);
        let by = |player: PlayerRef, cx: &mut TestAppContext| {
            cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&Identified::Named(player), cx)));
            cx.update(|cx| todos.read(cx).list().map(|list| (list.character.name, list.character.server_id)))
        };
        let player = |name: &str, server_id, key: Option<&str>| PlayerRef {
            name: name.to_string(),
            server_id,
            key: key.map(str::to_string),
        };
        assert_eq!(by(player("Birch", Some(1001), None), cx), Some(("Birch".to_string(), 1001)));
        assert_eq!(by(player("Alder", Some(2002), None), cx), Some(("Alder".to_string(), 2002)), "the same name on another server");
        assert_eq!(by(player("Whoever", None, Some("3AA503000000")), cx), Some(("Alder".to_string(), 1001)), "by key");
    }

    #[gpui::test]
    fn a_player_nobody_remembers_leaves_the_list_where_it_was(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None), known("Birch", 1001, None)], cx);
        let todos = open(&settings, cx);
        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&named("Birch"), cx)));
        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&named("Stranger"), cx)));
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Birch"), "stays on the last one played");
        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&Identified::Unnamed, cx)));
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Birch"));
    }

    #[gpui::test]
    fn a_character_remembered_later_gets_a_list(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = open(&settings, cx);
        cx.update(|cx| {
            settings.update(cx, |settings, cx| {
                settings.update(cx, |all| all.known_characters.insert(0, known("Birch", 1001, None)))
            })
        });
        assert_eq!(selected_name(&todos, cx).as_deref(), Some("Birch"));
        assert_eq!(cx.update(|cx| todos.read(cx).characters().len()), 2);
    }

    #[gpui::test]
    fn an_edit_reaches_the_window_and_the_list(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = open(&settings, cx);
        let notified = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = notified.clone();
        cx.update(|cx| cx.observe(&todos, move |_, _| counter.set(counter.get() + 1)).detach());
        let id = cx.update(|cx| todos.read(cx).selected()).unwrap();

        let change = cx.update(|cx| todos.update(cx, |todos, cx| todos.adjust_count(id, RowId::Builtin("duty"), 1, cx)));
        assert_eq!(change, Some(ProgressChange { was_done: false, is_done: false }));
        assert_eq!(notified.get(), 1);
        let list = cx.update(|cx| todos.read(cx).list()).unwrap();
        let today = &list.sections[0];
        assert_eq!(today.left, 1);
        let again = cx.update(|cx| todos.update(cx, |todos, cx| todos.adjust_count(id, RowId::Builtin("duty"), -2, cx)));
        assert_eq!(again.map(|change| change.is_done), Some(false));
        let nothing = cx.update(|cx| todos.update(cx, |todos, cx| todos.adjust_count(id, RowId::Builtin("duty"), -1, cx)));
        assert_eq!(nothing, None);
        assert_eq!(notified.get(), 2, "a change that changes nothing notifies nobody");
    }

    #[gpui::test]
    fn restoring_and_resetting_apply_to_every_character(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None), known("Birch", 2002, None)], cx);
        let todos = open(&settings, cx);
        let ids: Vec<CharacterId> = cx.update(|cx| todos.read(cx).characters().into_iter().map(|c| c.id).collect());
        cx.update(|cx| {
            todos.update(cx, |todos, cx| {
                todos.hide(ids[0], "trial", cx);
                todos.hide(ids[1], "duty", cx);
                todos.adjust_count(ids[1], RowId::Builtin("trial"), 3, cx);
                todos
                    .add_own(ids[0], NewOwnTodo { name: "Mine".into(), kind: OwnKind::Tick, cadence: Cadence::Weekly }, cx)
                    .unwrap();
            })
        });
        assert_eq!(cx.update(|cx| todos.read(cx).hidden_summary()), HiddenSummary { hidden: 4, characters: 2 });

        let back = cx.update(|cx| todos.update(cx, |todos, cx| todos.restore_defaults(cx)));
        assert_eq!(back, 4);
        assert_eq!(cx.update(|cx| todos.read(cx).own_total()), 1, "own to-dos stay");
        let trial = cx.update(|cx| todos.read(cx).list_of(ids[1])).unwrap().sections[1].rows.iter().find(|row| row.id == RowId::Builtin("trial")).cloned().unwrap();
        assert!(matches!(trial.kind, crate::todos::RowKind::Count { count: 3, .. }), "progress stays");

        cx.update(|cx| todos.update(cx, |todos, cx| todos.reset_completely(cx)));
        assert_eq!(cx.update(|cx| todos.read(cx).own_total()), 0);
        let list = cx.update(|cx| todos.read(cx).list_of(ids[1])).unwrap();
        assert_eq!(list.sections[1].hidden_rows, 0);
        assert!(list.sections[1].rows.iter().all(|row| !matches!(row.kind, crate::todos::RowKind::Count { count, .. } if count > 0)));
    }

    #[gpui::test]
    fn reset_completely_keeps_the_reset_hour_and_membership(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = open(&settings, cx);
        cx.update(|cx| {
            todos.update(cx, |todos, cx| {
                todos.set_reset_hour(6, cx);
                todos.set_membership(true, cx);
            })
        });
        cx.update(|cx| todos.update(cx, |todos, cx| todos.reset_completely(cx)));
        assert_eq!(
            (cx.update(|cx| todos.read(cx).reset_clock().to_string()), cx.update(|cx| todos.read(cx).membership())),
            ("06:00".to_string(), true)
        );
    }

    #[gpui::test]
    fn turning_membership_off_keeps_what_was_typed(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = open(&settings, cx);
        let id = cx.update(|cx| todos.read(cx).selected()).unwrap();
        cx.update(|cx| todos.update(cx, |todos, cx| todos.set_membership(true, cx)));
        cx.update(|cx| todos.update(cx, |todos, cx| todos.set_amount(id, "odyle", Some(800), cx)));
        cx.update(|cx| todos.update(cx, |todos, cx| todos.set_membership(false, cx)));
        assert_eq!(cx.update(|cx| todos.read(cx).data().character(id).unwrap().typed_amount("odyle")).map(|t| t.value), Some(800));
    }

    #[gpui::test]
    fn the_games_tickets_reach_the_rows_the_resources_and_the_membership_hint(cx: &mut TestAppContext) {
        use meter_core::capture::parser::ticket_record::TicketRecord;
        use meter_core::tickets::TicketContent;

        let record = |id: u32, count: u32, charge: u32| TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge };
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = open(&settings, cx);
        let player = Identified::Named(PlayerRef { name: "Alder".to_string(), server_id: Some(1001), key: None });
        cx.update(|cx| todos.update(cx, |todos, cx| todos.player_identified(&player, cx)));
        let report = TicketReport {
            character: known("Alder", 1001, None),
            captured_at: now_ms() as f64 / 1000.0,
            content: TicketContent::Snapshot(vec![record(60_000_001, 600, 100), record(1, 12, 3), record(4, 0, 3)]),
        };
        cx.update(|cx| todos.update(cx, |todos, cx| todos.apply_tickets(&[report], cx)));

        let resources = |cx: &mut TestAppContext| cx.update(|cx| todos.read(cx).list()).unwrap().resources.rows;
        let rows = resources(cx);
        let amount = |id: &'static str| match &rows.iter().find(|row| row.id == RowId::Builtin(id)).unwrap().kind {
            crate::todos::RowKind::Amount(amount) => amount.clone(),
            other => panic!("{id} is {other:?}"),
        };
        let odyle = amount("odyle");
        assert_eq!((odyle.value, odyle.cap, odyle.full, odyle.above_plain_cap), (600, 560, false, true));
        assert_eq!(rows[0].extra, 100);
        assert_eq!(rows[0].source, crate::todos::Source::GameLive);
        let hint = cx.update(|cx| todos.read(cx).membership_hint()).expect("the membership hint");
        assert_eq!((hint.character.as_str(), hint.label, hint.count, hint.cap), ("Alder", "Odyle Energy", 600, 560));
        assert!(amount("shugo").full, "Shugo keys are at the plain cap");
        let warned: Vec<String> = cx.update(|cx| todos.read(cx).full_resources()).into_iter().map(|resource| resource.label).collect();
        assert!(!warned.contains(&"Odyle Energy".to_string()), "Odyle is above its plain cap, not full");
        let nightmare = amount("nightmare");
        assert_eq!((nightmare.tracked, nightmare.value), (true, 0));

        cx.update(|cx| todos.update(cx, |todos, cx| todos.set_membership(true, cx)));
        assert_eq!(cx.update(|cx| todos.read(cx).membership_hint()), None);
        let odyle = match &resources(cx)[0].kind {
            crate::todos::RowKind::Amount(amount) => amount.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!((odyle.value, odyle.cap, odyle.full, odyle.above_plain_cap), (600, 840, false, false));
        assert!(cx.update(|cx| todos.read(cx).full_resources()).is_empty(), "Shugo keys are 12 of 21 now");
    }

    #[test]
    fn meter_events_say_who_is_played_only_when_they_carry_it() {
        use meter_core::storage::data_storage::MainActorDetectedPayload;

        let seen = MeterEvent::CharacterSeen(known("Alder", 1001, Some("3AA503000000")));
        assert_eq!(
            identified_by(&seen),
            Some(Identified::Named(PlayerRef {
                name: "Alder".to_string(),
                server_id: Some(1001),
                key: Some("3AA503000000".to_string())
            }))
        );
        let detected = |name: &str, sid: Option<&str>| {
            MeterEvent::MainActorDetected(MainActorDetectedPayload {
                actor_id: 1,
                actor_name: name.to_string(),
                sid: sid.map(str::to_string),
            })
        };
        assert_eq!(
            identified_by(&detected("Birch", Some("2002"))),
            Some(Identified::Named(PlayerRef { name: "Birch".to_string(), server_id: Some(2002), key: None }))
        );
        assert_eq!(
            identified_by(&detected("Birch", None)),
            Some(Identified::Named(PlayerRef { name: "Birch".to_string(), server_id: None, key: None }))
        );
        assert_eq!(identified_by(&detected("", None)), Some(Identified::Unnamed));
        assert_eq!(identified_by(&MeterEvent::HistoryUpdated), None);
    }

    #[gpui::test]
    fn the_reset_hour_in_settings_reaches_the_footer(cx: &mut TestAppContext) {
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = open(&settings, cx);
        cx.update(|cx| settings.update(cx, |settings, cx| settings.update(cx, |all| all.todos.reset_hour = Some(21))));
        let list = cx.update(|cx| todos.read(cx).list()).unwrap();
        assert_eq!(list.footer.reset_time, "21:00");
    }

    #[gpui::test]
    fn folds_and_edits_are_written_to_todos_json_and_read_back(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("aion-todos-service-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let settings = settings_with(vec![known("Alder", 1001, None)], cx);
        let todos = cx.update(|cx| cx.new(|cx| Todos::open(&settings, Some(&dir), false, cx)));
        let id = cx.update(|cx| todos.read(cx).selected()).unwrap();
        cx.update(|cx| {
            todos.update(cx, |todos, cx| {
                todos.adjust_count(id, RowId::Builtin("trial"), 2, cx);
                todos.set_section_folded(Section::ThisWeek, true, cx);
                todos.flush();
            })
        });
        let reopened = cx.update(|cx| cx.new(|cx| Todos::open(&settings, Some(&dir), false, cx)));
        let list = cx.update(|cx| reopened.read(cx).list()).unwrap();
        assert!(list.sections[1].folded);
        let trial = list.sections[1].rows.iter().find(|row| row.id == RowId::Builtin("trial")).unwrap();
        assert!(matches!(trial.kind, crate::todos::RowKind::Count { count: 2, .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
