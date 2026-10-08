//! What the tracker stores and every change to it. Pure: the clock comes in as a
//! [`Moment`], so the tests run any time zone and any day.
//!
//! Counts, ticks, typed amounts, the hidden set and the player's own to-dos
//! belong to one character. Duty Missions are the exception: the game counts
//! them per account and server, and the meter cannot see accounts, so they are
//! shared by the characters of one server.

use std::collections::{BTreeMap, BTreeSet};

use meter_core::identity::KnownCharacter;
use meter_core::tickets::{TicketContent, TicketReport};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

use super::amount::{self, Standing, TypedAmount};
use super::catalogue::{self, AmountSpec, Kind, Section};
use super::resets::Resets;
use super::tickets::{GameTicket, KnownTicket, ServerTicket};
use crate::clock::Zone;

pub const MAX_OWN_NAME_CHARS: usize = 40;
pub const OWN_COUNT_RANGE: std::ops::RangeInclusive<u32> = 2..=99;

/// The time and settings a change or a reading depends on.
#[derive(Clone, Copy)]
pub struct Moment<'a> {
    pub resets: Resets<'a>,
    pub now_ms: i64,
    pub membership: bool,
    /// When this run of the app began: a game value captured since, on the
    /// character being played, is live.
    pub run_started_ms: i64,
}

impl<'a> Moment<'a> {
    pub fn new(zone: &'a dyn Zone, reset_hour: u8, membership: bool, now_ms: i64) -> Self {
        Self { resets: Resets::new(zone, reset_hour), now_ms, membership, run_started_ms: 0 }
    }

    /// Resets on the game's own grid, 07:00 UTC.
    pub fn following_the_game(zone: &'a dyn Zone, membership: bool, now_ms: i64) -> Self {
        Self { resets: Resets::following_the_game(zone), now_ms, membership, run_started_ms: 0 }
    }

    pub fn running_since(self, run_started_ms: i64) -> Self {
        Self { run_started_ms, ..self }
    }

    pub fn cap_of(&self, spec: &AmountSpec) -> u32 {
        spec.cap_for(self.membership)
    }

    /// An amount of `value` known at `at_ms`, as it stands now.
    pub fn standing(&self, spec: &AmountSpec, value: u32, at_ms: i64) -> Standing {
        amount::standing(spec, self.cap_of(spec), value, at_ms, self.now_ms, &self.resets)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharacterId(pub u32);

/// A row of the list: one of the catalogue, or one the player added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowId {
    Builtin(&'static str),
    Own(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cadence {
    Daily,
    Weekly,
}

impl Cadence {
    pub fn section(self) -> Section {
        match self {
            Self::Daily => Section::Today,
            Self::Weekly => Section::ThisWeek,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OwnKind {
    Tick,
    /// Done at this many (2 to 99).
    Count(u32),
}

impl OwnKind {
    pub fn target(self) -> u32 {
        match self {
            Self::Tick => 1,
            Self::Count(target) => target,
        }
    }
}

/// What the player asks to add.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewOwnTodo {
    pub name: String,
    pub kind: OwnKind,
    pub cadence: Cadence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnTodo {
    pub id: u32,
    pub name: String,
    pub kind: OwnKind,
    pub cadence: Cadence,
    pub done: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddError {
    BlankName,
    CountOutOfRange,
    UnknownCharacter,
}

/// An own to-do taken off the list, with where it was, for the Undo note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovedOwnTodo {
    pub character: CharacterId,
    pub index: usize,
    pub todo: OwnTodo,
}

/// What a tick or a count did to a row's done state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressChange {
    pub was_done: bool,
    pub is_done: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HiddenSummary {
    pub hidden: usize,
    pub characters: usize,
}

/// "The game shows 18 Shugo Festival keys on Mirelle, more than 12."
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolAbovePlainCap {
    pub character: String,
    pub label: &'static str,
    pub count: u32,
    pub cap: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterState {
    pub id: CharacterId,
    pub name: String,
    pub server_id: u32,
    /// The class in the core's spelling, as the game last showed it.
    #[serde(default, deserialize_with = "lenient")]
    pub class: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    pub key: Option<String>,
    /// Count (or 0/1 for a tick) of the built-in rows, by id; absent is 0.
    #[serde(default, deserialize_with = "lenient_map")]
    pub(super) progress: BTreeMap<String, u32>,
    /// What the player typed over a row, by id.
    #[serde(default, deserialize_with = "lenient_map")]
    pub(super) amounts: BTreeMap<String, TypedAmount>,
    #[serde(default, deserialize_with = "lenient_set")]
    pub(super) hidden: BTreeSet<String>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub(super) own: Vec<OwnTodo>,
    #[serde(default, deserialize_with = "lenient")]
    next_own_id: u32,
    /// What the game said about this character's own tickets, by ticket id.
    #[serde(default, deserialize_with = "lenient_map")]
    tickets: BTreeMap<String, GameTicket>,
    /// When the game last listed every ticket on this character (Unix ms); 0 if never.
    #[serde(default, deserialize_with = "lenient")]
    logged_in_ms: i64,
}

impl CharacterState {
    fn new(id: CharacterId, known: &KnownCharacter) -> Self {
        Self {
            id,
            name: known.name.clone(),
            server_id: known.server_id,
            class: known.class.clone(),
            key: known.key.clone(),
            progress: BTreeMap::new(),
            amounts: BTreeMap::new(),
            hidden: catalogue::hidden_by_default().map(str::to_string).collect(),
            own: Vec::new(),
            next_own_id: 1,
            tickets: BTreeMap::new(),
            logged_in_ms: 0,
        }
    }

    /// When the game last listed every ticket on this character; none before the first.
    pub fn logged_in_ms(&self) -> Option<i64> {
        (self.logged_in_ms != 0).then_some(self.logged_in_ms)
    }

    /// What the player typed or counted over rows following `ticket` is gone:
    /// the game has spoken.
    fn drop_manual_values(&mut self, ticket: u32) {
        for builtin in catalogue::rows_of_ticket(ticket) {
            self.amounts.remove(builtin.id);
            self.progress.remove(builtin.id);
        }
    }

    pub fn is_hidden(&self, id: &str) -> bool {
        self.hidden.contains(id)
    }

    pub fn typed_amount(&self, id: &str) -> Option<TypedAmount> {
        self.amounts.get(id).copied()
    }

    pub fn own_todos(&self) -> &[OwnTodo] {
        &self.own
    }

    pub fn matches(&self, known: &KnownCharacter) -> bool {
        match (&self.key, &known.key) {
            (Some(own), Some(other)) => own == other,
            _ => self.name == known.name && self.server_id == known.server_id,
        }
    }

    fn clear_section(&mut self, section: Section) {
        self.progress.retain(|id, _| catalogue::builtin(id).is_none_or(|builtin| builtin.section != section));
        let cadence = match section {
            Section::Today => Some(Cadence::Daily),
            Section::ThisWeek => Some(Cadence::Weekly),
            Section::Recharges => None,
        };
        for todo in self.own.iter_mut().filter(|todo| Some(todo.cadence) == cadence) {
            todo.done = 0;
        }
    }

    fn sanitise(&mut self) {
        self.name = self.name.trim().to_string();
        self.progress.retain(|id, count| {
            let target = match catalogue::builtin(id).map(|builtin| builtin.kind) {
                Some(Kind::Tick) => 1,
                Some(Kind::Count { target }) => target,
                _ => return false,
            };
            *count = (*count).min(target);
            *count > 0 && !catalogue::builtin(id).is_some_and(|builtin| builtin.account_wide)
        });
        self.amounts.retain(|id, _| matches!(catalogue::builtin(id).map(|builtin| builtin.kind), Some(Kind::Amount(_))));
        self.tickets.retain(|id, _| {
            id.parse::<u32>().is_ok_and(|ticket| {
                catalogue::rows_of_ticket(ticket).next().is_some() && !catalogue::is_server_ticket(ticket)
            })
        });
        self.hidden.retain(|id| catalogue::builtin(id).is_some());
        let mut seen = BTreeSet::new();
        self.own.retain_mut(|todo| {
            todo.name = truncated_name(&todo.name);
            if let OwnKind::Count(target) = &mut todo.kind {
                *target = (*target).clamp(*OWN_COUNT_RANGE.start(), *OWN_COUNT_RANGE.end());
            }
            todo.done = todo.done.min(todo.kind.target());
            !todo.name.is_empty() && seen.insert(todo.id)
        });
        let after_last = self.own.iter().map(|todo| todo.id).max().map_or(1, |last| last + 1);
        self.next_own_id = self.next_own_id.max(after_last);
    }
}

fn truncated_name(name: &str) -> String {
    name.trim().chars().take(MAX_OWN_NAME_CHARS).collect::<String>().trim_end().to_string()
}

/// Everything in `todos.json` apart from the version.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TodoData {
    /// The daily and weekly reset last applied (Unix ms); 0 before the first.
    #[serde(deserialize_with = "lenient")]
    daily_reset_ms: i64,
    #[serde(deserialize_with = "lenient")]
    weekly_reset_ms: i64,
    #[serde(deserialize_with = "lenient")]
    next_character_id: u32,
    #[serde(deserialize_with = "lenient_vec")]
    characters: Vec<CharacterState>,
    /// Duty Missions done, by server id.
    #[serde(deserialize_with = "lenient_map")]
    duty_missions: BTreeMap<String, u32>,
    /// What the game said about the tickets it counts for the server.
    #[serde(deserialize_with = "lenient_vec")]
    server_tickets: Vec<ServerTicket>,
    /// Sections folded shut, by [`Section::key`].
    #[serde(deserialize_with = "lenient_set")]
    folded: BTreeSet<String>,
    /// Groups opened, by built-in id.
    #[serde(deserialize_with = "lenient_set")]
    open_groups: BTreeSet<String>,
}

fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    Ok(serde_json::Value::deserialize(deserializer).ok().and_then(|value| serde_json::from_value(value).ok()).unwrap_or_default())
}

fn lenient_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Array(entries) => {
            entries.into_iter().filter_map(|entry| serde_json::from_value(entry).ok()).collect()
        }
        _ => Vec::new(),
    })
}

fn lenient_set<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BTreeSet<String>, D::Error> {
    lenient_vec(deserializer).map(|entries| entries.into_iter().collect())
}

fn lenient_map<'de, D, V>(deserializer: D) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: DeserializeOwned,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Object(entries) => entries
            .into_iter()
            .filter_map(|(key, value)| serde_json::from_value(value).ok().map(|value| (key, value)))
            .collect(),
        _ => BTreeMap::new(),
    })
}

impl TodoData {
    /// Drops what the catalogue does not know and pulls the rest into range,
    /// after reading a file.
    pub fn sanitised(mut self) -> Self {
        let mut seen = BTreeSet::new();
        self.characters.retain_mut(|character| {
            character.sanitise();
            !character.name.is_empty() && seen.insert(character.id)
        });
        let after_last = self.characters.iter().map(|character| character.id.0).max().map_or(1, |last| last + 1);
        self.next_character_id = self.next_character_id.max(after_last);
        let target = catalogue::builtin(catalogue::DUTY_MISSIONS).map_or(0, |duty| match duty.kind {
            Kind::Count { target } => target,
            _ => 0,
        });
        self.duty_missions.retain(|server, count| {
            *count = (*count).min(target);
            server.parse::<u32>().is_ok() && *count > 0
        });
        let mut seen = BTreeSet::new();
        self.server_tickets.retain(|known| {
            catalogue::is_server_ticket(known.ticket)
                && catalogue::rows_of_ticket(known.ticket).next().is_some()
                && seen.insert((known.server_id, known.ticket))
        });
        self.folded.retain(|key| Section::from_key(key).is_some());
        self.open_groups.retain(|id| catalogue::builtin(id).is_some_and(|builtin| builtin.kind == Kind::Group));
        self
    }

    #[cfg(test)]
    pub fn characters(&self) -> &[CharacterState] {
        &self.characters
    }

    pub fn character(&self, id: CharacterId) -> Option<&CharacterState> {
        self.characters.iter().find(|character| character.id == id)
    }

    pub fn find(&self, known: &KnownCharacter) -> Option<CharacterId> {
        self.characters.iter().find(|character| character.matches(known)).map(|character| character.id)
    }

    fn character_mut(&mut self, id: CharacterId) -> Option<&mut CharacterState> {
        self.characters.iter_mut().find(|character| character.id == id)
    }

    /// Makes sure each remembered character has a list, and follows a rename or
    /// a key the game has told since. Returns whether anything changed.
    pub fn sync_characters(&mut self, remembered: &[KnownCharacter]) -> bool {
        let mut changed = false;
        for known in remembered {
            match self.characters.iter_mut().find(|character| character.matches(known)) {
                Some(character) => {
                    let seen = (&known.name, known.server_id);
                    if (&character.name, character.server_id) != seen {
                        character.name = known.name.clone();
                        character.server_id = known.server_id;
                        changed = true;
                    }
                    if character.class != known.class {
                        character.class = known.class.clone();
                        changed = true;
                    }
                    if character.key.is_none() && known.key.is_some() {
                        character.key = known.key.clone();
                        changed = true;
                    }
                }
                None => {
                    let id = CharacterId(self.next_character_id.max(1));
                    self.next_character_id = id.0 + 1;
                    self.characters.push(CharacterState::new(id, known));
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn duty_missions_done(&self, server_id: u32) -> u32 {
        self.duty_missions.get(&server_id.to_string()).copied().unwrap_or(0)
    }

    /// What the row has done: a count, or 1 for a ticked row.
    pub fn progress_of(&self, character: &CharacterState, row: RowId) -> u32 {
        match row {
            RowId::Builtin(id) => match catalogue::builtin(id) {
                Some(builtin) if builtin.account_wide => self.duty_missions_done(character.server_id),
                Some(_) => character.progress.get(id).copied().unwrap_or(0),
                None => 0,
            },
            RowId::Own(id) => character.own.iter().find(|todo| todo.id == id).map_or(0, |todo| todo.done),
        }
    }

    /// What the game last said about `ticket`, as `character` sees it: its
    /// own value, or for a ticket the game counts for the server, the value
    /// whichever character of the server gave last.
    pub fn game_ticket(&self, character: &CharacterState, ticket: u32) -> Option<KnownTicket> {
        if catalogue::is_server_ticket(ticket) {
            return self
                .server_tickets
                .iter()
                .find(|known| known.server_id == character.server_id && known.ticket == ticket)
                .map(|known| KnownTicket { value: known.value, shared_from: (known.by != character.id).then_some(known.by) });
        }
        character.tickets.get(&ticket.to_string()).map(|&value| KnownTicket { value, shared_from: None })
    }

    /// A regular pool the game shows above its plain cap, on any character,
    /// for a ticket the membership raises: the player probably has the
    /// membership.
    pub fn pool_above_plain_cap(&self) -> Option<PoolAbovePlainCap> {
        let mut raised = catalogue::CATALOGUE.iter().filter_map(|builtin| match (builtin.kind, builtin.ticket) {
            (Kind::Amount(spec), Some(ticket)) if spec.membership_cap.is_some() => Some((builtin.label, spec.cap, ticket)),
            _ => None,
        });
        raised.find_map(|(label, cap, ticket)| {
            self.characters.iter().find_map(|character| {
                let count = self.game_ticket(character, ticket)?.value.count;
                (count > cap).then(|| PoolAbovePlainCap { character: character.name.clone(), label, count, cap })
            })
        })
    }

    /// Keeps what the game said: a snapshot fills every ticket it lists, a
    /// change replaces one. What the player typed or counted over a ticket's
    /// rows is dropped (on every character of the server for a server ticket).
    /// Returns whether anything changed.
    pub fn apply_tickets(&mut self, report: &TicketReport) -> bool {
        let at_ms = (report.captured_at * 1000.0).round() as i64;
        if self.find(&report.character).is_none() {
            self.sync_characters(std::slice::from_ref(&report.character));
        }
        let Some(id) = self.find(&report.character) else {
            return false;
        };
        match &report.content {
            TicketContent::Snapshot(records) => {
                for record in records {
                    self.store_ticket(id, record.id, record.count, record.charge, at_ms);
                }
                if let Some(character) = self.character_mut(id).filter(|character| character.logged_in_ms <= at_ms) {
                    character.logged_in_ms = at_ms;
                }
                true
            }
            TicketContent::Change { record, .. } => self.store_ticket(id, record.id, record.count, record.charge, at_ms),
        }
    }

    fn store_ticket(&mut self, id: CharacterId, ticket: u32, count: u32, charge: u32, at_ms: i64) -> bool {
        if catalogue::rows_of_ticket(ticket).next().is_none() {
            return false;
        }
        let value = GameTicket { count, charge, at_ms };
        let Some(server_id) = self.character(id).map(|character| character.server_id) else {
            return false;
        };
        if catalogue::is_server_ticket(ticket) {
            let known = self.server_tickets.iter_mut().find(|known| known.server_id == server_id && known.ticket == ticket);
            match known {
                Some(known) if known.value.at_ms > at_ms => return false,
                Some(known) => {
                    known.value = value;
                    known.by = id;
                }
                None => self.server_tickets.push(ServerTicket { server_id, ticket, value, by: id }),
            }
            for character in self.characters.iter_mut().filter(|character| character.server_id == server_id) {
                character.drop_manual_values(ticket);
            }
            return true;
        }
        let Some(character) = self.character_mut(id) else {
            return false;
        };
        if character.tickets.get(&ticket.to_string()).is_some_and(|known| known.at_ms > at_ms) {
            return false;
        }
        character.tickets.insert(ticket.to_string(), value);
        character.drop_manual_values(ticket);
        true
    }

    pub fn is_folded(&self, section: Section) -> bool {
        self.folded.contains(section.key())
    }

    pub fn is_group_open(&self, id: &str) -> bool {
        self.open_groups.contains(id)
    }

    pub fn set_folded(&mut self, section: Section, folded: bool) -> bool {
        if folded {
            self.folded.insert(section.key().to_string())
        } else {
            self.folded.remove(section.key())
        }
    }

    pub fn set_group_open(&mut self, group: &'static str, open: bool) -> bool {
        if catalogue::builtin(group).is_none_or(|builtin| builtin.kind != Kind::Group) {
            return false;
        }
        if open {
            self.open_groups.insert(group.to_string())
        } else {
            self.open_groups.remove(group)
        }
    }

    /// Applies the daily and weekly resets that have come since the last one
    /// applied, even if the meter was closed over them. Returns whether
    /// anything changed, the stamps included.
    pub fn apply_resets(&mut self, moment: &Moment) -> bool {
        let daily = moment.resets.last_daily(moment.now_ms);
        let weekly = moment.resets.last_weekly(moment.now_ms);
        let mut changed = false;
        if self.daily_reset_ms == 0 || daily > self.daily_reset_ms {
            if self.daily_reset_ms != 0 {
                self.clear_section(Section::Today);
                self.duty_missions.clear();
            }
            self.daily_reset_ms = daily;
            changed = true;
        }
        if self.weekly_reset_ms == 0 || weekly > self.weekly_reset_ms {
            if self.weekly_reset_ms != 0 {
                self.clear_section(Section::ThisWeek);
            }
            self.weekly_reset_ms = weekly;
            changed = true;
        }
        changed
    }

    /// Takes the resets already past as applied without clearing anything: the
    /// reset hour changed, which moves the boundaries but starts nothing over.
    pub fn rebase_resets(&mut self, moment: &Moment) -> bool {
        let daily = moment.resets.last_daily(moment.now_ms);
        let weekly = moment.resets.last_weekly(moment.now_ms);
        let changed = (self.daily_reset_ms, self.weekly_reset_ms) != (daily, weekly);
        (self.daily_reset_ms, self.weekly_reset_ms) = (daily, weekly);
        changed
    }

    fn clear_section(&mut self, section: Section) {
        for character in &mut self.characters {
            character.clear_section(section);
        }
    }

    /// Ticks a tick row, or unticks a done one.
    pub fn toggle_tick(&mut self, id: CharacterId, row: RowId) -> Option<ProgressChange> {
        self.change_progress(id, row, |kind| kind == Shape::Tick, |done, target| if done >= target { 0 } else { target })
    }

    /// Adds `delta` to a count row, clamped to 0 and its target.
    pub fn adjust_count(&mut self, id: CharacterId, row: RowId, delta: i32) -> Option<ProgressChange> {
        self.change_progress(id, row, |kind| kind == Shape::Count, |done, target| {
            (i64::from(done) + i64::from(delta)).clamp(0, i64::from(target)) as u32
        })
    }

    /// Types how much of a row the game fills is done (`Some`, up to its
    /// target), or goes back to what the game said (`None`). The game's next
    /// value for the ticket replaces it. Rows the game does not fill are
    /// counted with [`Self::toggle_tick`] and [`Self::adjust_count`].
    pub fn set_progress(&mut self, id: CharacterId, row: &'static str, value: Option<u32>) -> bool {
        let Some(character) = self.character(id) else {
            return false;
        };
        let Some((_, target)) = shape_of(character, RowId::Builtin(row)) else {
            return false;
        };
        let followed = catalogue::builtin(row).and_then(|builtin| builtin.ticket);
        if !followed.is_some_and(|ticket| self.game_ticket(character, ticket).is_some()) {
            return false;
        }
        let Some(character) = self.character_mut(id) else {
            return false;
        };
        match value {
            Some(value) => character.progress.insert(row.to_string(), value.min(target)) != Some(value.min(target)),
            None => character.progress.remove(row).is_some(),
        }
    }

    fn change_progress(
        &mut self,
        id: CharacterId,
        row: RowId,
        allowed: impl FnOnce(Shape) -> bool,
        change: impl FnOnce(u32, u32) -> u32,
    ) -> Option<ProgressChange> {
        let character = self.character(id)?;
        let (shape, target) = shape_of(character, row)?;
        if !allowed(shape) {
            return None;
        }
        if let RowId::Builtin(builtin_id) = row {
            let followed = catalogue::builtin(builtin_id)?.ticket;
            if followed.is_some_and(|ticket| self.game_ticket(character, ticket).is_some()) {
                return None;
            }
        }
        let done = self.progress_of(character, row);
        let next = change(done, target).min(target);
        if next == done {
            return None;
        }
        let server = character.server_id.to_string();
        match row {
            RowId::Builtin(builtin_id) if catalogue::builtin(builtin_id)?.account_wide => {
                set_or_remove(&mut self.duty_missions, server, next);
            }
            RowId::Builtin(builtin_id) => {
                set_or_remove(&mut self.character_mut(id)?.progress, builtin_id.to_string(), next);
            }
            RowId::Own(own_id) => {
                self.character_mut(id)?.own.iter_mut().find(|todo| todo.id == own_id)?.done = next;
            }
        }
        Some(ProgressChange { was_done: done >= target, is_done: next >= target })
    }

    /// Hides a built-in row. Returns whether it was shown before.
    pub fn hide(&mut self, id: CharacterId, row: &'static str) -> bool {
        catalogue::builtin(row).is_some() && self.character_mut(id).is_some_and(|character| character.hidden.insert(row.to_string()))
    }

    pub fn show(&mut self, id: CharacterId, row: &'static str) -> bool {
        self.character_mut(id).is_some_and(|character| character.hidden.remove(row))
    }

    /// Types what the player has (`Some`), or goes back to what the game said
    /// (`None`). A typed amount may be above the cap. The game's next value
    /// for the ticket replaces it.
    pub fn set_amount(&mut self, id: CharacterId, row: &'static str, value: Option<u32>, moment: &Moment) -> bool {
        if amount_spec(row).is_none() {
            return false;
        }
        let Some(character) = self.character_mut(id) else {
            return false;
        };
        match value {
            Some(value) => {
                let typed = TypedAmount { value, at_ms: moment.now_ms };
                character.amounts.insert(row.to_string(), typed) != Some(typed)
            }
            None => character.amounts.remove(row).is_some(),
        }
    }

    /// Moves a tracked amount one arrow step up (`direction` > 0) or down, from
    /// what it is worth now; the refills count from now on. Does nothing on an
    /// amount not tracked or already at the end it moves towards.
    pub fn step_amount(&mut self, id: CharacterId, row: &'static str, direction: i32, moment: &Moment) -> bool {
        let Some(spec) = amount_spec(row) else {
            return false;
        };
        let Some(typed) = self.character(id).and_then(|character| character.typed_amount(row)) else {
            return false;
        };
        let now = moment.standing(&spec, typed.value, typed.at_ms).value;
        let stepped = i64::from(now) + i64::from(direction.signum()) * i64::from(spec.step);
        let next = stepped.clamp(0, i64::from(moment.cap_of(&spec).max(now))) as u32;
        if next == now {
            return false;
        }
        self.set_amount(id, row, Some(next), moment)
    }

    pub fn add_own(&mut self, id: CharacterId, request: NewOwnTodo) -> Result<u32, AddError> {
        let name = truncated_name(&request.name);
        if name.is_empty() {
            return Err(AddError::BlankName);
        }
        if let OwnKind::Count(target) = request.kind {
            if !OWN_COUNT_RANGE.contains(&target) {
                return Err(AddError::CountOutOfRange);
            }
        }
        let character = self.character_mut(id).ok_or(AddError::UnknownCharacter)?;
        let todo_id = character.next_own_id.max(1);
        character.next_own_id = todo_id + 1;
        character.own.push(OwnTodo { id: todo_id, name, kind: request.kind, cadence: request.cadence, done: 0 });
        Ok(todo_id)
    }

    pub fn remove_own(&mut self, id: CharacterId, own_id: u32) -> Option<RemovedOwnTodo> {
        let character = self.character_mut(id)?;
        let index = character.own.iter().position(|todo| todo.id == own_id)?;
        Some(RemovedOwnTodo { character: id, index, todo: character.own.remove(index) })
    }

    /// Puts a removed to-do back where it was, with the progress it had.
    pub fn restore_own(&mut self, removed: RemovedOwnTodo) -> bool {
        let Some(character) = self.character_mut(removed.character) else {
            return false;
        };
        if character.own.iter().any(|todo| todo.id == removed.todo.id) {
            return false;
        }
        let index = removed.index.min(character.own.len());
        character.own.insert(index, removed.todo);
        true
    }

    pub fn hidden_summary(&self) -> HiddenSummary {
        HiddenSummary {
            hidden: self.characters.iter().map(|character| character.hidden.len()).sum(),
            characters: self.characters.iter().filter(|character| !character.hidden.is_empty()).count(),
        }
    }

    pub fn own_total(&self) -> usize {
        self.characters.iter().map(|character| character.own.len()).sum()
    }

    /// Brings back every hidden built-in on every character and nothing else.
    /// Returns how many came back.
    pub fn restore_defaults(&mut self) -> usize {
        self.characters.iter_mut().map(|character| std::mem::take(&mut character.hidden).len()).sum()
    }

    /// Every character starts over: own to-dos removed, everything shown, every
    /// count, tick and amount emptied. The fold state is kept.
    pub fn reset_completely(&mut self) -> bool {
        let before = (self.hidden_summary().hidden, self.own_total(), self.has_progress());
        for character in &mut self.characters {
            character.own.clear();
            character.hidden.clear();
            character.progress.clear();
            character.amounts.clear();
        }
        self.duty_missions.clear();
        before != (0, 0, false)
    }

    fn has_progress(&self) -> bool {
        !self.duty_missions.is_empty()
            || self.characters.iter().any(|character| !character.progress.is_empty() || !character.amounts.is_empty())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Tick,
    Count,
}

/// Whether the row is a tick or a count, and what finishes it.
fn shape_of(character: &CharacterState, row: RowId) -> Option<(Shape, u32)> {
    match row {
        RowId::Builtin(id) => match catalogue::builtin(id)?.kind {
            Kind::Tick => Some((Shape::Tick, 1)),
            Kind::Count { target } => Some((Shape::Count, target)),
            Kind::Amount(_) | Kind::Group => None,
        },
        RowId::Own(id) => character.own.iter().find(|todo| todo.id == id).map(|todo| match todo.kind {
            OwnKind::Tick => (Shape::Tick, 1),
            OwnKind::Count(target) => (Shape::Count, target),
        }),
    }
}

fn set_or_remove(map: &mut BTreeMap<String, u32>, key: String, value: u32) {
    if value == 0 {
        map.remove(&key);
    } else {
        map.insert(key, value);
    }
}

pub fn amount_spec(row: &str) -> Option<AmountSpec> {
    match catalogue::builtin(row)?.kind {
        Kind::Amount(spec) => Some(spec),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use meter_core::capture::parser::ticket_record::{TicketReason, TicketRecord};

    use super::*;
    use crate::clock::testing::{berlin, utc, TestZone};

    const HOUR: i64 = 3_600_000;

    fn summer() -> TestZone {
        TestZone { base: 120, changes: Vec::new() }
    }

    /// Local time in the fixed summer zone; 2026-10-05 is a Monday.
    fn local(day_of_october: i64, hour: i64, minute: i64) -> i64 {
        utc(2026, 10, day_of_october, hour, minute) - 2 * HOUR
    }

    fn character(name: &str, server_id: u32, key: Option<&str>) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id, class: "RANGER".to_string(), key: key.map(str::to_string) }
    }

    fn moment(zone: &TestZone, now_ms: i64) -> Moment<'_> {
        Moment::new(zone, 9, false, now_ms)
    }

    fn two_characters() -> (TodoData, CharacterId, CharacterId) {
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None), character("Birch", 1001, None)]);
        let (first, second) = (data.characters[0].id, data.characters[1].id);
        (data, first, second)
    }

    fn count_of(data: &TodoData, id: CharacterId, row: &'static str) -> u32 {
        data.progress_of(data.character(id).unwrap(), RowId::Builtin(row))
    }

    #[test]
    fn a_new_character_starts_with_only_duty_commands_hidden() {
        let (data, first, _) = two_characters();
        let hidden: Vec<&String> = data.character(first).unwrap().hidden.iter().collect();
        assert_eq!(hidden, ["dutycmd"]);
        assert_eq!(data.hidden_summary(), HiddenSummary { hidden: 2, characters: 2 });
    }

    #[test]
    fn syncing_follows_a_key_the_game_told_and_does_not_duplicate() {
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None)]);
        let id = data.characters[0].id;
        assert!(data.sync_characters(&[character("Alder", 1001, Some("3AA503000000"))]));
        assert_eq!(data.characters.len(), 1);
        assert_eq!(data.characters[0].key.as_deref(), Some("3AA503000000"));
        assert!(!data.sync_characters(&[character("Alder", 1001, Some("3AA503000000"))]), "nothing changed");
        assert!(data.sync_characters(&[character("Alder-Renamed", 1001, Some("3AA503000000"))]));
        assert_eq!(data.character(id).unwrap().name, "Alder-Renamed", "the key says it is the same character");
    }

    #[test]
    fn the_same_name_on_another_server_is_another_character() {
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None), character("Alder", 2001, None)]);
        assert_eq!(data.characters.len(), 2);
    }

    #[test]
    fn a_tick_toggles_and_a_count_is_clamped_to_zero_and_its_target() {
        let (mut data, id, _) = two_characters();
        let forge_boss = RowId::Builtin("forgeboss");
        assert_eq!(data.toggle_tick(id, forge_boss), Some(ProgressChange { was_done: false, is_done: true }));
        assert_eq!(data.toggle_tick(id, forge_boss), Some(ProgressChange { was_done: true, is_done: false }));

        let trial = RowId::Builtin("trial");
        assert_eq!(data.adjust_count(id, trial, -1), None, "already at 0");
        for _ in 0..5 {
            data.adjust_count(id, trial, 1);
        }
        assert_eq!(count_of(&data, id, "trial"), 3, "Ascension Trial is 3 a week");
        assert_eq!(data.adjust_count(id, trial, 1), None);
        assert_eq!(data.adjust_count(id, trial, -1), Some(ProgressChange { was_done: true, is_done: false }));
        assert_eq!(data.toggle_tick(id, trial), None, "a count is not a tick");
        assert_eq!(data.adjust_count(id, forge_boss, 1), None, "a tick is not a count");
    }

    #[test]
    fn duty_missions_are_shared_by_the_characters_of_a_server_only() {
        let mut data = TodoData::default();
        data.sync_characters(&[
            character("Alder", 1001, None),
            character("Birch", 1001, None),
            character("Cedar", 2002, None),
        ]);
        let (alder, birch, cedar) = (data.characters[0].id, data.characters[1].id, data.characters[2].id);
        data.adjust_count(alder, RowId::Builtin("duty"), 1);
        data.adjust_count(birch, RowId::Builtin("duty"), 1);
        assert_eq!(count_of(&data, alder, "duty"), 2);
        assert_eq!(count_of(&data, birch, "duty"), 2);
        assert_eq!(count_of(&data, cedar, "duty"), 0);
        data.adjust_count(cedar, RowId::Builtin("duty"), 1);
        assert_eq!(count_of(&data, alder, "duty"), 2, "another server keeps its own count");
    }

    #[test]
    fn other_counts_belong_to_one_character() {
        let (mut data, alder, birch) = two_characters();
        data.adjust_count(alder, RowId::Builtin("fissure"), 3);
        assert_eq!(count_of(&data, alder, "fissure"), 3);
        assert_eq!(count_of(&data, birch, "fissure"), 0);
    }

    #[test]
    fn the_daily_reset_clears_today_and_own_daily_to_dos_and_keeps_the_week() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(5, 12, 0)));
        data.adjust_count(id, RowId::Builtin("duty"), 3);
        data.adjust_count(id, RowId::Builtin("fissure"), 4);
        let daily = data.add_own(id, NewOwnTodo { name: "Legion".into(), kind: OwnKind::Tick, cadence: Cadence::Daily }).unwrap();
        let weekly = data.add_own(id, NewOwnTodo { name: "Market".into(), kind: OwnKind::Count(3), cadence: Cadence::Weekly }).unwrap();
        data.toggle_tick(id, RowId::Own(daily));
        data.adjust_count(id, RowId::Own(weekly), 2);

        assert!(!data.apply_resets(&moment(&zone, local(6, 8, 59))), "before 09:00 nothing happens");
        assert!(data.apply_resets(&moment(&zone, local(6, 9, 0))));

        assert_eq!(count_of(&data, id, "duty"), 0);
        assert_eq!(count_of(&data, id, "fissure"), 4, "This week does not reset on Tuesday");
        let state = data.character(id).unwrap();
        assert_eq!(data.progress_of(state, RowId::Own(daily)), 0);
        assert_eq!(data.progress_of(state, RowId::Own(weekly)), 2);
    }

    #[test]
    fn the_weekly_reset_is_wednesday_at_the_reset_hour() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(5, 12, 0)));
        data.adjust_count(id, RowId::Builtin("fissure"), 6);
        data.adjust_count(id, RowId::Builtin("forge"), 2);
        data.toggle_tick(id, RowId::Builtin("forgeboss"));
        data.adjust_count(id, RowId::Builtin("x-krao"), 1);

        data.apply_resets(&moment(&zone, local(7, 8, 59)));
        assert_eq!(count_of(&data, id, "fissure"), 6);
        data.apply_resets(&moment(&zone, local(7, 9, 0)));
        for row in ["fissure", "forge", "forgeboss", "x-krao"] {
            assert_eq!(count_of(&data, id, row), 0, "{row} starts over, group rows included");
        }
    }

    #[test]
    fn a_meter_closed_over_a_reset_resets_on_the_next_start() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(5, 20, 0)));
        data.adjust_count(id, RowId::Builtin("duty"), 4);
        data.adjust_count(id, RowId::Builtin("trial"), 2);

        let mut reloaded = serde_json::from_value::<TodoData>(serde_json::to_value(&data).unwrap()).unwrap().sanitised();
        assert_eq!(reloaded, data, "what is stored is what is read");
        reloaded.apply_resets(&moment(&zone, local(12, 7, 0)));

        assert_eq!(count_of(&reloaded, id, "duty"), 0);
        assert_eq!(count_of(&reloaded, id, "trial"), 0, "a Wednesday (7 Oct) went by too");
    }

    #[test]
    fn a_first_start_stamps_the_resets_without_inventing_one() {
        let zone = summer();
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None)]);
        let id = data.characters[0].id;
        data.adjust_count(id, RowId::Builtin("duty"), 2);
        data.apply_resets(&moment(&zone, local(6, 12, 0)));
        assert_eq!(count_of(&data, id, "duty"), 2, "no earlier reset to apply");
        assert_eq!(data.daily_reset_ms, local(6, 9, 0));
    }

    #[test]
    fn changing_the_reset_hour_moves_the_boundaries_without_clearing() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(6, 9, 30)));
        data.adjust_count(id, RowId::Builtin("duty"), 2);
        let at_ten = Moment::new(&zone, 10, false, local(6, 10, 30));
        assert!(data.rebase_resets(&at_ten));
        data.apply_resets(&at_ten);
        assert_eq!(count_of(&data, id, "duty"), 2, "10:00 passed a minute ago but the hour was just set to it");
        data.apply_resets(&Moment::new(&zone, 10, false, local(7, 10, 0)));
        assert_eq!(count_of(&data, id, "duty"), 0);
    }

    #[test]
    fn a_clock_set_back_does_not_clear_or_stall_the_resets() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(8, 12, 0)));
        data.adjust_count(id, RowId::Builtin("duty"), 2);
        data.apply_resets(&moment(&zone, local(6, 12, 0)));
        assert_eq!(count_of(&data, id, "duty"), 2);
        data.apply_resets(&moment(&zone, local(9, 9, 0)));
        assert_eq!(count_of(&data, id, "duty"), 0, "the next reset still comes");
    }

    #[test]
    fn a_clock_set_back_and_forward_does_not_clear_the_same_reset_twice() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(5, 12, 0)));
        data.apply_resets(&moment(&zone, local(7, 9, 0)));
        data.adjust_count(id, RowId::Builtin("fissure"), 4);
        data.adjust_count(id, RowId::Builtin("duty"), 2);

        data.apply_resets(&moment(&zone, local(6, 12, 0)));
        data.apply_resets(&moment(&zone, local(7, 12, 0)));
        assert_eq!(count_of(&data, id, "fissure"), 4, "the same Wednesday does not clear again");
        assert_eq!(count_of(&data, id, "duty"), 2, "nor the same day");

        data.apply_resets(&moment(&zone, local(14, 9, 0)));
        assert_eq!(count_of(&data, id, "fissure"), 0, "a later Wednesday still clears");
    }

    #[test]
    fn the_daily_reset_on_a_changeover_day_comes_at_the_wall_hour() {
        let zone = berlin();
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None)]);
        let id = data.characters[0].id;
        let at = |now_ms| Moment::new(&zone, 9, false, now_ms);
        data.apply_resets(&at(utc(2026, 3, 28, 12, 0)));
        data.adjust_count(id, RowId::Builtin("duty"), 1);
        data.apply_resets(&at(utc(2026, 3, 29, 6, 59)));
        assert_eq!(count_of(&data, id, "duty"), 1, "08:59 CEST");
        data.apply_resets(&at(utc(2026, 3, 29, 7, 0)));
        assert_eq!(count_of(&data, id, "duty"), 0, "09:00 CEST, an hour of real time earlier than the day before");
    }

    #[test]
    fn amounts_never_reset() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        data.apply_resets(&moment(&zone, local(5, 12, 0)));
        data.set_amount(id, "shugo", Some(12), &moment(&zone, local(5, 12, 0)));
        data.apply_resets(&moment(&zone, local(14, 12, 0)));
        assert_eq!(data.character(id).unwrap().typed_amount("shugo").map(|typed| typed.value), Some(12));
    }

    #[test]
    fn a_typed_amount_may_be_above_the_cap_and_can_be_cleared() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        let now = moment(&zone, local(5, 12, 0));
        assert!(data.set_amount(id, "odyle", Some(9_999), &now));
        assert_eq!(data.character(id).unwrap().typed_amount("odyle"), Some(TypedAmount { value: 9_999, at_ms: now.now_ms }));
        assert!(!data.set_amount(id, "odyle", Some(9_999), &now), "same value, same time");
        assert!(data.set_amount(id, "odyle", None, &now));
        assert_eq!(data.character(id).unwrap().typed_amount("odyle"), None);
        assert!(!data.set_amount(id, "trial", Some(1), &now), "only recharges hold amounts");
    }

    #[test]
    fn stepping_an_amount_restarts_the_refill_clock_from_the_estimate() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        let typed_at = moment(&zone, local(5, 12, 0));
        data.set_amount(id, "odyle", Some(100), &typed_at);
        let later = moment(&zone, local(5, 18, 30));
        assert!(data.step_amount(id, "odyle", 1, &later));
        assert_eq!(
            data.character(id).unwrap().typed_amount("odyle"),
            Some(TypedAmount { value: 100 + 2 * 15 + 10, at_ms: later.now_ms }),
            "two refills happened, then the +10 step, counted from now"
        );
        assert!(data.step_amount(id, "odyle", -1, &later));
        assert_eq!(data.character(id).unwrap().typed_amount("odyle").map(|typed| typed.value), Some(130));
    }

    #[test]
    fn steps_are_one_except_odyle_and_stop_at_the_ends() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        let now = moment(&zone, local(5, 12, 0));
        assert!(!data.step_amount(id, "shugo", 1, &now), "not tracked: nothing to step");
        data.set_amount(id, "shugo", Some(11), &now);
        assert!(data.step_amount(id, "shugo", 1, &now));
        assert!(!data.step_amount(id, "shugo", 1, &now), "at the cap");
        data.set_amount(id, "shugo", Some(0), &now);
        assert!(!data.step_amount(id, "shugo", -1, &now), "at zero");
        data.set_amount(id, "odyle", Some(555), &now);
        assert!(data.step_amount(id, "odyle", 1, &now));
        assert_eq!(data.character(id).unwrap().typed_amount("odyle").map(|typed| typed.value), Some(560), "555 + 10 stops at 560");
    }

    #[test]
    fn stepping_down_from_above_the_cap_keeps_the_rest() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        let now = moment(&zone, local(5, 12, 0));
        data.set_amount(id, "shugo", Some(24), &now);
        assert!(!data.step_amount(id, "shugo", 1, &now), "above the cap, up does nothing");
        assert!(data.step_amount(id, "shugo", -1, &now));
        assert_eq!(data.character(id).unwrap().typed_amount("shugo").map(|typed| typed.value), Some(23));
    }

    #[test]
    fn turning_the_membership_off_keeps_every_typed_amount() {
        let zone = summer();
        let (mut data, id, _) = two_characters();
        let member = Moment::new(&zone, 9, true, local(5, 12, 0));
        data.set_amount(id, "odyle", Some(800), &member);
        data.apply_resets(&Moment::new(&zone, 9, false, local(5, 13, 0)));
        assert_eq!(data.character(id).unwrap().typed_amount("odyle").map(|typed| typed.value), Some(800));
    }

    #[test]
    fn hide_and_show_move_built_ins_in_and_out_of_the_hidden_set() {
        let (mut data, id, other) = two_characters();
        assert!(data.hide(id, "trial"));
        assert!(!data.hide(id, "trial"));
        assert!(data.character(id).unwrap().is_hidden("trial"));
        assert!(!data.character(other).unwrap().is_hidden("trial"), "per character");
        assert!(data.show(id, "trial"));
        assert!(!data.show(id, "trial"));
    }

    #[test]
    fn own_to_dos_are_added_validated_removed_and_restored_in_place() {
        let (mut data, id, other) = two_characters();
        let add = |data: &mut TodoData, name: &str, kind| {
            data.add_own(id, NewOwnTodo { name: name.into(), kind, cadence: Cadence::Daily })
        };
        assert_eq!(add(&mut data, "   ", OwnKind::Tick), Err(AddError::BlankName));
        assert_eq!(add(&mut data, "Many", OwnKind::Count(1)), Err(AddError::CountOutOfRange));
        assert_eq!(add(&mut data, "Many", OwnKind::Count(100)), Err(AddError::CountOutOfRange));
        let first = add(&mut data, "  Legion check-in  ", OwnKind::Tick).unwrap();
        let second = add(&mut data, "Market", OwnKind::Count(3)).unwrap();
        let third = add(&mut data, "Tower", OwnKind::Tick).unwrap();
        assert_eq!(data.character(id).unwrap().own[0].name, "Legion check-in");
        assert!(data.character(other).unwrap().own.is_empty(), "belongs to the character it was added on");

        data.adjust_count(id, RowId::Own(second), 2);
        let removed = data.remove_own(id, second).unwrap();
        assert_eq!(removed.index, 1);
        let ids: Vec<u32> = data.character(id).unwrap().own.iter().map(|todo| todo.id).collect();
        assert_eq!(ids, [first, third]);

        assert!(data.restore_own(removed.clone()));
        let ids: Vec<u32> = data.character(id).unwrap().own.iter().map(|todo| todo.id).collect();
        assert_eq!(ids, [first, second, third], "back at the same position");
        assert_eq!(data.progress_of(data.character(id).unwrap(), RowId::Own(second)), 2, "with its progress");
        assert!(!data.restore_own(removed), "not twice");
        let fourth = add(&mut data, "Later", OwnKind::Tick).unwrap();
        assert!(fourth > third, "ids are never reused");
    }

    #[test]
    fn a_long_name_is_cut() {
        let (mut data, id, _) = two_characters();
        let todo = data
            .add_own(id, NewOwnTodo { name: "x".repeat(100), kind: OwnKind::Tick, cadence: Cadence::Weekly })
            .unwrap();
        let name = &data.character(id).unwrap().own.iter().find(|own| own.id == todo).unwrap().name;
        assert_eq!(name.chars().count(), MAX_OWN_NAME_CHARS);
    }

    #[test]
    fn restoring_the_defaults_brings_back_hidden_built_ins_everywhere_and_keeps_the_rest() {
        let zone = summer();
        let (mut data, alder, birch) = two_characters();
        data.hide(alder, "trial");
        data.hide(alder, "sanct");
        data.hide(birch, "shugo");
        data.adjust_count(alder, RowId::Builtin("fissure"), 5);
        data.set_amount(birch, "odyle", Some(200), &moment(&zone, local(5, 12, 0)));
        data.add_own(alder, NewOwnTodo { name: "Mine".into(), kind: OwnKind::Tick, cadence: Cadence::Weekly }).unwrap();

        assert_eq!(data.restore_defaults(), 5, "Duty Commands on both, plus three more");
        assert_eq!(data.hidden_summary(), HiddenSummary::default());
        assert_eq!(count_of(&data, alder, "fissure"), 5);
        assert!(data.character(birch).unwrap().typed_amount("odyle").is_some());
        assert_eq!(data.own_total(), 1);
        assert_eq!(data.restore_defaults(), 0);
    }

    #[test]
    fn resetting_completely_empties_every_character_and_keeps_the_view_state() {
        let zone = summer();
        let (mut data, alder, birch) = two_characters();
        data.hide(birch, "trial");
        data.adjust_count(alder, RowId::Builtin("duty"), 2);
        data.adjust_count(birch, RowId::Builtin("fissure"), 5);
        data.set_amount(alder, "shugo", Some(9), &moment(&zone, local(5, 12, 0)));
        data.add_own(alder, NewOwnTodo { name: "Mine".into(), kind: OwnKind::Tick, cadence: Cadence::Daily }).unwrap();
        data.set_folded(Section::Today, true);
        data.set_group_open("sanct", true);

        assert!(data.reset_completely());
        assert_eq!(data.hidden_summary(), HiddenSummary::default(), "Duty Commands comes back too");
        assert_eq!(data.own_total(), 0);
        assert_eq!(count_of(&data, alder, "duty"), 0);
        assert_eq!(count_of(&data, birch, "fissure"), 0);
        assert_eq!(data.character(alder).unwrap().typed_amount("shugo"), None);
        assert_eq!(data.characters.len(), 2, "the characters stay");
        assert!(data.is_folded(Section::Today) && data.is_group_open("sanct"));
        assert!(!data.reset_completely(), "nothing left to clear");
    }

    #[test]
    fn fold_and_group_state_is_kept_and_only_groups_open() {
        let mut data = TodoData::default();
        assert!(data.set_folded(Section::Recharges, true));
        assert!(!data.set_folded(Section::Recharges, true));
        assert!(data.is_folded(Section::Recharges) && !data.is_folded(Section::Today));
        assert!(data.set_group_open("explore", true));
        assert!(!data.set_group_open("trial", true), "a count is not a group");
        assert!(data.is_group_open("explore"));
    }

    #[test]
    fn reading_costs_a_bad_entry_only_that_entry() {
        let stored = serde_json::json!({
            "dailyResetMs": 1234,
            "weeklyResetMs": "soon",
            "characters": [
                { "id": 1, "name": "Alder", "serverId": 1001,
                  "progress": { "trial": 2, "nonsense": 4, "fissure": "many", "duty": 3 },
                  "amounts": { "odyle": { "value": 9999, "atMs": 5 }, "shugo": { "value": "x" }, "trial": { "value": 1, "atMs": 1 } },
                  "hidden": ["trial", "gone", 7],
                  "tickets": {
                    "4": { "count": 3, "charge": 1, "atMs": 100 },
                    "999": { "count": 1, "charge": 0, "atMs": 100 },
                    "8": { "count": 1, "charge": 0, "atMs": 100 },
                    "60000001": { "count": "many" }
                  },
                  "loggedInMs": 77,
                  "own": [
                    { "id": 1, "name": "Fine", "kind": "tick", "cadence": "daily", "done": 0 },
                    { "id": 2, "name": "Broken", "kind": "sideways", "cadence": "daily", "done": 0 },
                    { "id": 3, "name": "Big", "kind": { "count": 500 }, "cadence": "weekly", "done": 900 }
                  ] },
                { "id": "two", "name": "Wrecked" },
                { "id": 3, "name": "  ", "serverId": 1 },
                { "id": 4, "name": "Birch", "serverId": 1002, "progress": 7 }
            ],
            "dutyMissions": { "1001": 9, "oops": 2, "1002": "x" },
            "serverTickets": [
                { "serverId": 1001, "ticket": 8, "value": { "count": 2, "charge": 0, "atMs": 50 }, "by": 1 },
                { "serverId": 1001, "ticket": 4, "value": { "count": 2, "charge": 0, "atMs": 50 }, "by": 1 },
                { "serverId": 1001, "ticket": 8, "value": { "count": 5, "charge": 0, "atMs": 60 }, "by": 1 },
                { "serverId": "x", "ticket": 8 },
                "junk"
            ],
            "folded": ["today", "bogus"],
            "openGroups": ["sanct", "trial"]
        });
        let data = serde_json::from_value::<TodoData>(stored).unwrap().sanitised();

        assert_eq!(data.daily_reset_ms, 1234);
        assert_eq!(data.weekly_reset_ms, 0, "a bad stamp costs the stamp");
        let names: Vec<&str> = data.characters.iter().map(|character| character.name.as_str()).collect();
        assert_eq!(names, ["Alder", "Birch"]);
        let alder = &data.characters[0];
        assert_eq!(alder.progress.iter().map(|(id, count)| (id.as_str(), *count)).collect::<Vec<_>>(), [("trial", 2)]);
        assert_eq!(alder.amounts.keys().collect::<Vec<_>>(), ["odyle"]);
        assert_eq!(alder.amounts["odyle"].value, 9999, "a typed amount above the cap is kept");
        assert_eq!(alder.tickets.iter().map(|(id, ticket)| (id.as_str(), ticket.count)).collect::<Vec<_>>(), [("4", 3)]);
        assert_eq!(alder.logged_in_ms(), Some(77));
        assert_eq!(data.server_tickets.len(), 1);
        assert_eq!((data.server_tickets[0].ticket, data.server_tickets[0].by), (8, CharacterId(1)));
        assert_eq!(alder.hidden.iter().collect::<Vec<_>>(), ["trial"]);
        assert_eq!(alder.own.iter().map(|todo| todo.name.as_str()).collect::<Vec<_>>(), ["Fine", "Big"]);
        assert_eq!(alder.own[1].kind, OwnKind::Count(99));
        assert_eq!(alder.own[1].done, 99);
        assert!(data.characters[1].progress.is_empty());
        assert_eq!(data.duty_missions_done(1001), 5, "clamped to 5 a day");
        assert_eq!(data.duty_missions.len(), 1);
        assert_eq!(data.folded.iter().collect::<Vec<_>>(), ["today"]);
        assert_eq!(data.open_groups.iter().collect::<Vec<_>>(), ["sanct"]);
        assert!(data.next_character_id > 4);
    }

    #[test]
    fn a_new_character_after_loading_gets_an_unused_id() {
        let stored = serde_json::json!({ "characters": [{ "id": 7, "name": "Alder", "serverId": 1001 }] });
        let mut data = serde_json::from_value::<TodoData>(stored).unwrap().sanitised();
        data.sync_characters(&[character("Birch", 1001, None)]);
        assert_eq!(data.characters[1].id, CharacterId(8));
    }

    fn record(id: u32, count: u32, charge: u32) -> TicketRecord {
        TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge }
    }

    fn snapshot(who: &KnownCharacter, at_secs: f64, records: Vec<TicketRecord>) -> TicketReport {
        TicketReport { character: who.clone(), captured_at: at_secs, content: TicketContent::Snapshot(records) }
    }

    fn change(who: &KnownCharacter, at_secs: f64, record: TicketRecord) -> TicketReport {
        let content = TicketContent::Change { record, reason: TicketReason::Spend, delta: Some(1) };
        TicketReport { character: who.clone(), captured_at: at_secs, content }
    }

    fn game_count(data: &TodoData, id: CharacterId, ticket: u32) -> Option<(u32, u32)> {
        let known = data.game_ticket(data.character(id)?, ticket)?;
        Some((known.value.count, known.value.charge))
    }

    #[test]
    fn a_snapshot_fills_every_covered_ticket_and_replaces_what_was_typed() {
        let (mut data, alder, _) = two_characters();
        let zone = summer();
        let now = moment(&zone, local(5, 12, 0));
        data.set_amount(alder, "odyle", Some(425), &now);
        data.adjust_count(alder, RowId::Builtin("trial"), 2);
        data.adjust_count(alder, RowId::Builtin("subj"), 1);

        let listed = vec![record(60_000_001, 0, 245), record(80_000_001, 3, 1), record(4, 0, 3), record(999_999, 5, 5)];
        assert!(data.apply_tickets(&snapshot(&character("Alder", 1001, None), 1_000.0, listed)));

        let state = data.character(alder).unwrap();
        assert_eq!(game_count(&data, alder, 60_000_001), Some((0, 245)));
        assert_eq!(game_count(&data, alder, 4), Some((0, 3)));
        assert_eq!(game_count(&data, alder, 999_999), None, "no row follows it");
        assert_eq!(state.typed_amount("odyle"), None, "the game's value replaces the typed one");
        assert_eq!(count_of(&data, alder, "trial"), 0, "so does a count the player kept");
        assert_eq!(count_of(&data, alder, "subj"), 1, "a ticket the snapshot does not list is left alone");
        assert_eq!(state.logged_in_ms(), Some(1_000_000));
    }

    #[test]
    fn a_change_replaces_one_ticket_and_drops_only_what_was_typed_over_it() {
        let (mut data, alder, _) = two_characters();
        let who = character("Alder", 1001, None);
        data.apply_tickets(&snapshot(&who, 1_000.0, vec![record(60_000_001, 100, 0), record(4, 14, 0)]));
        let zone = summer();
        let now = moment(&zone, local(5, 12, 0));
        data.set_amount(alder, "odyle", Some(300), &now);
        data.set_amount(alder, "nightmare", Some(2), &now);

        assert!(data.apply_tickets(&change(&who, 1_100.0, record(4, 13, 0))));

        assert_eq!(game_count(&data, alder, 4), Some((13, 0)));
        assert_eq!(game_count(&data, alder, 60_000_001), Some((100, 0)), "the other ticket stays");
        assert_eq!(data.character(alder).unwrap().typed_amount("nightmare"), None);
        assert_eq!(data.character(alder).unwrap().typed_amount("odyle").map(|typed| typed.value), Some(300));
        assert_eq!(data.character(alder).unwrap().logged_in_ms(), Some(1_000_000), "a change is not a login");
    }

    #[test]
    fn an_older_snapshot_does_not_move_the_login_back() {
        let (mut data, alder, _) = two_characters();
        let who = character("Alder", 1001, None);
        data.apply_tickets(&snapshot(&who, 2_000.0, vec![record(4, 10, 0)]));

        data.apply_tickets(&snapshot(&who, 1_000.0, vec![record(4, 14, 0)]));

        assert_eq!(data.character(alder).unwrap().logged_in_ms(), Some(2_000_000));
        assert_eq!(game_count(&data, alder, 4), Some((10, 0)), "the older value is ignored too");

        data.apply_tickets(&snapshot(&who, 3_000.0, vec![record(4, 9, 0)]));
        assert_eq!(data.character(alder).unwrap().logged_in_ms(), Some(3_000_000));
    }

    #[test]
    fn a_ticket_older_than_the_stored_one_is_ignored() {
        let (mut data, alder, _) = two_characters();
        let who = character("Alder", 1001, None);
        data.apply_tickets(&change(&who, 2_000.0, record(4, 10, 0)));
        assert!(!data.apply_tickets(&change(&who, 1_000.0, record(4, 14, 0))));
        assert_eq!(game_count(&data, alder, 4), Some((10, 0)));
    }

    #[test]
    fn server_tickets_are_shared_by_the_characters_of_one_server_only() {
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None), character("Birch", 1001, None), character("Cedar", 2001, None)]);
        let (alder, birch, cedar) = (data.characters[0].id, data.characters[1].id, data.characters[2].id);
        let zone = summer();
        let now = moment(&zone, local(5, 12, 0));
        data.set_amount(birch, "shugo", Some(9), &now);

        let listed = vec![record(8, 2, 12), record(1, 6, 0), record(101, 14, 7), record(4, 14, 0)];
        data.apply_tickets(&snapshot(&character("Alder", 1001, None), 1_000.0, listed));

        for ticket in [8, 1, 101] {
            assert_eq!(game_count(&data, alder, ticket), game_count(&data, birch, ticket), "ticket {ticket}");
            assert!(game_count(&data, alder, ticket).is_some());
            assert_eq!(game_count(&data, cedar, ticket), None, "another server");
        }
        assert_eq!(game_count(&data, birch, 8), Some((2, 12)));
        assert_eq!(game_count(&data, birch, 4), None, "a character's own ticket stays its own");
        let known = data.game_ticket(data.character(birch).unwrap(), 8).unwrap();
        assert_eq!(known.shared_from, Some(alder));
        assert_eq!(data.game_ticket(data.character(alder).unwrap(), 8).unwrap().shared_from, None);
        assert_eq!(data.character(birch).unwrap().typed_amount("shugo"), None, "typed over a shared ticket is replaced too");
    }

    #[test]
    fn the_last_character_to_log_in_gives_the_server_tickets() {
        let mut data = TodoData::default();
        data.sync_characters(&[character("Alder", 1001, None), character("Birch", 1001, None)]);
        let (alder, birch) = (data.characters[0].id, data.characters[1].id);
        data.apply_tickets(&snapshot(&character("Alder", 1001, None), 1_000.0, vec![record(8, 2, 0)]));
        data.apply_tickets(&snapshot(&character("Birch", 1001, None), 2_000.0, vec![record(8, 1, 0)]));
        assert_eq!(game_count(&data, alder, 8), Some((1, 0)));
        assert_eq!(data.game_ticket(data.character(alder).unwrap(), 8).unwrap().shared_from, Some(birch));
    }

    #[test]
    fn a_character_the_tracker_has_not_met_is_added() {
        let mut data = TodoData::default();
        assert!(data.apply_tickets(&snapshot(&character("Alder", 1001, None), 1_000.0, vec![record(4, 14, 0)])));
        assert_eq!(data.characters.len(), 1);
        assert_eq!(game_count(&data, data.characters[0].id, 4), Some((14, 0)));
    }

    #[test]
    fn a_count_the_game_fills_is_no_longer_counted_by_hand() {
        let (mut data, alder, _) = two_characters();
        let trial = RowId::Builtin("trial");
        assert!(data.adjust_count(alder, trial, 1).is_some(), "before the game says, the player counts");
        data.apply_tickets(&snapshot(&character("Alder", 1001, None), 1_000.0, vec![record(80_000_001, 3, 0)]));
        assert_eq!(data.adjust_count(alder, trial, 1), None);
        assert_eq!(data.adjust_count(alder, trial, -1), None);
    }

    #[test]
    fn a_count_typed_over_the_game_stays_until_the_games_next_value() {
        let (mut data, alder, _) = two_characters();
        let who = character("Alder", 1001, None);
        let typed = |data: &TodoData| data.character(alder).unwrap().progress.get("trial").copied();
        assert!(!data.set_progress(alder, "trial", Some(2)), "before the game says, the arrows count");
        data.apply_tickets(&snapshot(&who, 1_000.0, vec![record(80_000_001, 3, 0)]));
        assert!(data.set_progress(alder, "trial", Some(0)));
        assert_eq!(typed(&data), Some(0), "a typed 0 is kept");
        assert!(data.set_progress(alder, "trial", Some(9)));
        assert_eq!(typed(&data), Some(3), "never past the target");
        data.apply_tickets(&change(&who, 1_100.0, record(80_000_001, 2, 0)));
        assert_eq!(typed(&data), None, "the game's next value replaces it");
        data.set_progress(alder, "trial", Some(1));
        assert!(data.set_progress(alder, "trial", None));
        assert_eq!(typed(&data), None, "back to the game's value");
    }

    #[test]
    fn the_game_showing_more_than_the_plain_cap_suggests_a_membership() {
        let (mut data, _, _) = two_characters();
        let who = character("Alder", 1001, None);
        data.apply_tickets(&snapshot(&who, 1_000.0, vec![record(1, 12, 0), record(60_000_001, 560, 0), record(4, 14, 0)]));
        assert_eq!(data.pool_above_plain_cap(), None, "at the cap is not above it");
        data.apply_tickets(&change(&who, 1_100.0, record(60_000_001, 600, 0)));
        let hint = PoolAbovePlainCap { character: "Alder".to_string(), label: "Odyle Energy", count: 600, cap: 560 };
        assert_eq!(data.pool_above_plain_cap(), Some(hint));
    }
}
