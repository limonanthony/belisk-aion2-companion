//! What the window shows for one character at a moment: sections, rows with
//! their display state, the counts and the words for them. Built fresh from the
//! stored data each time; nothing here is stored.

use super::catalogue::{self, AmountSpec, Builtin, Kind, Section};
use super::copy;
use super::data::{CharacterId, CharacterState, Moment, OwnKind, OwnTodo, RowId, TodoData};
use super::tickets::GameTicket;
use crate::game_data::{self, PlayerClass};
use crate::i18n::duration_words;

#[derive(Clone, Debug, PartialEq)]
pub struct TodoList {
    pub character: CharacterSummary,
    /// Rows still to do in Today and This week, groups counted as one.
    pub left: usize,
    /// "9 left", or "all done".
    pub left_text: String,
    /// Tracked amounts at their cap.
    pub full: usize,
    /// "2 full" when there is one.
    pub full_text: Option<String>,
    /// Set while nothing is left in Today and This week.
    pub everything_done: Option<EverythingDone>,
    /// Today, Recharges, This week, in that order.
    pub sections: Vec<SectionView>,
    pub footer: Footer,
    /// When the game last listed this character's tickets at login (Unix ms): "logged in at 19:02".
    pub logged_in_ms: Option<i64>,
    /// Where the values come from, for the line under the character selector.
    pub source_line: SourceLine,
}

/// The line under the character selector. Names are the stored ones; the
/// window shows them as it shows names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceLine {
    /// "From the game, live · logged in at 19:02"; no time when the login
    /// came before this run.
    Live { logged_in_at: Option<String> },
    /// "From the game at login, Sun 22:40 · refills since estimated".
    Earlier { at: String },
    /// "Tavrin not seen at login yet · Account rows from Mirelle".
    Shared { from: String },
    /// "No game values yet. Log in with Sefa while the meter runs."
    NoneYet,
}

/// The note "Everything is done on <character>." with, when a tracked amount is
/// still filling, the one that fills first. Build the first sentence with
/// [`copy::everything_done`] and the name as the window shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EverythingDone {
    /// "Odyle Energy is full in 2 d 3 h."
    pub fills_first: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footer {
    /// "09:00".
    pub reset_time: String,
    /// "Weekly on Wednesday".
    pub weekly: String,
}

/// A remembered character, for the header selector and the picker.
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterSummary {
    pub id: CharacterId,
    pub name: String,
    pub server_id: u32,
    /// The server's short name.
    pub server: String,
    /// The class in the core's spelling ("GLADIATOR"); [`Self::class`] parses it.
    pub class_code: String,
    /// "Gladiator".
    pub class_name: String,
    /// The character the game is showing right now.
    pub playing: bool,
    pub left: usize,
    pub full: usize,
    /// What the picker shows at the end: "N full" (warning) when an amount is
    /// full, else "N left" or "all done".
    pub status: String,
    pub status_is_warning: bool,
}

impl CharacterSummary {
    pub fn class(&self) -> Option<PlayerClass> {
        PlayerClass::parse(&self.class_code)
    }

    /// "Gladiator · EU-Kaelis".
    pub fn class_and_server(&self) -> String {
        format!("{} \u{b7} {}", self.class_name, self.server)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SectionView {
    pub section: Section,
    pub title: String,
    pub folded: bool,
    /// Every row, hidden ones included and flagged; groups hold their rows.
    pub rows: Vec<RowView>,
    /// Hidden rows at the section's level ("N hidden · Show").
    pub hidden_rows: usize,
    /// Rows still to do (Today and This week).
    pub left: usize,
    /// Tracked amounts, and those at their cap (Recharges).
    pub tracked: usize,
    pub full: usize,
    /// After the title: "2 left", "done", "3 tracked" or empty.
    pub count_text: String,
    /// `count_text` is "done": drawn in the good colour.
    pub count_is_done: bool,
    /// "N full" chip (Recharges).
    pub full_text: Option<String>,
    /// The section's right part: "resets in 11 h 12 min", "next full in 1 d 2 h",
    /// "Wed 09:00 · in 1 d 11 h", "amounts optional" or empty.
    pub when_text: String,
    /// Where the countdown, drawn brighter, starts in `when_text`.
    pub when_time_at: Option<usize>,
    /// The reset the right part counts down to (Today and This week).
    pub next_reset_ms: Option<i64>,
}

/// Where a row's figure comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Counted by the player: their own to-dos, Duty Missions.
    Manual,
    /// The game fills this row but has not said yet; any figure is the player's.
    NoneYet,
    /// The player typed it, over the game's value or before the game said one.
    Typed,
    /// The game's value, heard while this character is being played.
    GameLive,
    /// The game's value from an earlier login, with the refills the table
    /// knows since added; `refills` of them, `game_value` the regular pool at login.
    GameEarlier { at_ms: i64, game_value: u32, refills: u32 },
    /// Another character of the server gave it (Account rows).
    Shared { from: String, at_ms: i64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RowView {
    pub id: RowId,
    pub label: String,
    pub sub: Option<String>,
    /// Shown with the "Account" tag: Duty Missions, and what the game counts
    /// for the whole server (Shugo Festival keys, Invasion keys, the Unknown Fissure).
    pub account: bool,
    /// Shown with the "Yours" tag; its × removes instead of hides.
    pub yours: bool,
    pub hidden: bool,
    pub kind: RowKind,
    pub source: Source,
    /// The game's extra pool (items and rewards) beside the regular one; 0 when none.
    pub extra: u32,
    /// When the game last gave this row a value (Unix ms).
    pub last_seen_ms: Option<i64>,
    /// What the figure's tooltip says about where its value comes from.
    pub tip: Option<FigureTip>,
}

/// "125 at your login Sun 22:40." over "+120 since: 8 refills of 15, estimated.".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FigureTip {
    pub text: String,
    pub detail: Option<String>,
}

impl RowView {
    /// "Daily Dungeon, Unknown Fissure": what a screen reader says.
    pub fn accessible_name(&self) -> String {
        match &self.sub {
            Some(sub) => format!("{}, {sub}", self.label),
            None => self.label.clone(),
        }
    }

    pub fn is_done(&self) -> bool {
        match &self.kind {
            RowKind::Tick { done } | RowKind::Count { done, .. } => *done,
            RowKind::Group(group) => group.all_done,
            RowKind::Amount(_) => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RowKind {
    Tick { done: bool },
    Count { count: u32, target: u32, done: bool },
    Amount(AmountView),
    Group(GroupView),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupView {
    pub open: bool,
    /// Counted over the rows not hidden.
    pub done_children: usize,
    pub total_children: usize,
    /// Every row not hidden is done (and there is one).
    pub all_done: bool,
    /// Amount rows at their cap (a group of amounts, like Conquest dungeons).
    pub full_children: usize,
    /// "2 of 6 done"; "2 full" or "3 tracked" for a group of amounts, empty when none is tracked.
    pub summary: String,
    pub children: Vec<RowView>,
    pub hidden_children: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmountView {
    /// What one arrow press adds or removes.
    pub step: u32,
    /// The cap in force: the membership cap when the switch is on.
    pub cap: u32,
    /// The game or the player gave a value; else the row shows "—".
    pub tracked: bool,
    /// The regular pool now: the game's last value plus the refills since, or
    /// what the player typed (0 when not tracked). It may be above the cap.
    pub value: u32,
    /// The regular pool is at or above the cap. Not while the membership
    /// switch is off and the game shows more than the plain cap.
    pub full: bool,
    /// When it reaches the cap, while it is filling.
    pub full_at_ms: Option<i64>,
    /// While full: the refill coming, the one wasted unless spent first.
    pub refill_at_ms: Option<i64>,
    /// "Full in 2 d 16 h" while filling, "Full, being wasted" or "Full, spend by
    /// Tue 16:00" at the cap; none when not tracked.
    pub status_text: Option<String>,
    /// Refills come continuously (Odyle Energy), not on a schedule.
    pub continuous: bool,
    /// What one refill adds.
    pub refill_add: u32,
    /// How far the extra pool goes; none when the game sets no limit.
    pub extra_cap: Option<u32>,
    /// The game's regular pool as it stands now, whatever the player typed over it.
    pub game_value: Option<u32>,
    /// The membership switch is off and the game's regular pool is above the
    /// plain cap: the player probably has the membership.
    pub above_plain_cap: bool,
}

impl AmountView {
    /// The spin button's value text: "425 of 560", or "not tracked".
    pub fn value_text(&self) -> String {
        if self.tracked {
            copy::value_of(self.value, self.cap)
        } else {
            copy::not_tracked()
        }
    }
}

pub fn build(data: &TodoData, character: &CharacterState, playing: bool, moment: &Moment) -> TodoList {
    let sections: Vec<SectionView> =
        Section::ALL.into_iter().map(|section| section_view(data, character, section, playing, moment)).collect();
    let left: usize = sections.iter().map(|section| section.left).sum();
    let full = sections.iter().map(|section| section.full).sum();
    let fills_first = soonest_filling(&sections).map(|(label, at_ms)| copy::amount_full_note(label, at_ms - moment.now_ms));
    TodoList {
        character: summary(character, playing, left, full),
        left,
        left_text: if left > 0 { copy::left(left) } else { copy::all_done() },
        full,
        full_text: (full > 0).then(|| copy::full(full)),
        everything_done: (left == 0).then_some(EverythingDone { fills_first }),
        source_line: source_line(&sections, character, moment),
        sections,
        logged_in_ms: character.logged_in_ms(),
        footer: Footer {
            reset_time: moment.resets.clock_at(moment.now_ms).to_string(),
            weekly: copy::weekly_on_wednesday(),
        },
    }
}

fn source_line(sections: &[SectionView], character: &CharacterState, moment: &Moment) -> SourceLine {
    let zone = moment.resets.zone();
    let mut sources: Vec<&Source> = Vec::new();
    for row in sections.iter().flat_map(|section| &section.rows) {
        sources.push(&row.source);
        if let RowKind::Group(group) = &row.kind {
            sources.extend(group.children.iter().map(|child| &child.source));
        }
    }
    if sources.contains(&&Source::GameLive) {
        let logged_in_at = character
            .logged_in_ms()
            .filter(|&at_ms| at_ms >= moment.run_started_ms)
            .map(|at_ms| crate::clock::clock_time(zone, at_ms).to_string());
        return SourceLine::Live { logged_in_at };
    }
    let last_earlier = sources
        .iter()
        .filter_map(|source| match source {
            Source::GameEarlier { at_ms, .. } => Some(*at_ms),
            _ => None,
        })
        .max();
    if let Some(last_earlier) = last_earlier {
        let at_ms = character.logged_in_ms().unwrap_or(last_earlier);
        return SourceLine::Earlier { at: crate::i18n::day_stamp(zone, at_ms) };
    }
    sources
        .iter()
        .find_map(|source| match source {
            Source::Shared { from, .. } => Some(SourceLine::Shared { from: from.clone() }),
            _ => None,
        })
        .unwrap_or(SourceLine::NoneYet)
}

pub fn summary_of(data: &TodoData, character: &CharacterState, playing: bool, moment: &Moment) -> CharacterSummary {
    build(data, character, playing, moment).character
}

fn summary(character: &CharacterState, playing: bool, left: usize, full: usize) -> CharacterSummary {
    CharacterSummary {
        id: character.id,
        name: character.name.clone(),
        server_id: character.server_id,
        server: game_data::server_name(&character.server_id.to_string()),
        class_code: character.class.clone(),
        class_name: class_name(&character.class),
        playing,
        left,
        full,
        status: if full > 0 {
            copy::full(full)
        } else if left > 0 {
            copy::left(left)
        } else {
            copy::all_done()
        },
        status_is_warning: full > 0,
    }
}

/// "GLADIATOR" as "Gladiator".
fn class_name(code: &str) -> String {
    let mut letters = code.trim().chars();
    letters.next().map(|first| first.to_uppercase().chain(letters.flat_map(char::to_lowercase)).collect()).unwrap_or_default()
}

/// The amount rows of `rows` still shown, the ones inside groups too.
fn shown_amounts(rows: &[RowView]) -> Vec<(&RowView, &AmountView)> {
    let mut found = Vec::new();
    for row in rows.iter().filter(|row| !row.hidden) {
        match &row.kind {
            RowKind::Amount(amount) => found.push((row, amount)),
            RowKind::Group(group) => found.extend(shown_amounts(&group.children)),
            _ => {}
        }
    }
    found
}

fn soonest_filling(sections: &[SectionView]) -> Option<(&str, i64)> {
    sections
        .iter()
        .filter(|section| section.section == Section::Recharges)
        .flat_map(|section| shown_amounts(&section.rows))
        .filter_map(|(row, amount)| amount.full_at_ms.map(|at_ms| (row.label.as_str(), at_ms)))
        .min_by_key(|&(_, at_ms)| at_ms)
}

/// The Recharges section's rows in the window's order, groups holding their
/// rows.
fn recharges(data: &TodoData, character: &CharacterState, playing: bool, moment: &Moment) -> Vec<RowView> {
    section_rows(data, character, Section::Recharges, playing, moment)
}

/// A resource at its cap with a refill coming: the refill is wasted unless
/// the player spends first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullResource {
    /// The built-in row's id.
    pub id: &'static str,
    pub label: String,
    /// The group the row sits in ("Conquest dungeons"), if any.
    pub group: Option<&'static str>,
    pub value: u32,
    pub cap: u32,
    pub extra: u32,
    /// When the refill that would be wasted comes.
    pub refill_at_ms: i64,
}

/// The rows of Recharges that are full, hidden rows left out, in order.
pub fn full_resources(data: &TodoData, character: &CharacterState, playing: bool, moment: &Moment) -> Vec<FullResource> {
    let rows = recharges(data, character, playing, moment);
    shown_amounts(&rows)
        .into_iter()
        .filter(|(_, amount)| amount.full)
        .filter_map(|(row, amount)| {
            let RowId::Builtin(id) = row.id else {
                return None;
            };
            Some(FullResource {
                id,
                label: row.label.clone(),
                group: catalogue::builtin(id)?.group.and_then(catalogue::builtin).map(|group| group.label),
                value: amount.value,
                cap: amount.cap,
                extra: row.extra,
                refill_at_ms: amount.refill_at_ms?,
            })
        })
        .collect()
}

fn section_rows(data: &TodoData, character: &CharacterState, section: Section, playing: bool, moment: &Moment) -> Vec<RowView> {
    let mut rows: Vec<RowView> = catalogue::CATALOGUE
        .iter()
        .filter(|builtin| builtin.section == section && builtin.group.is_none())
        .map(|builtin| builtin_row(data, character, builtin, playing, moment))
        .collect();
    rows.extend(character.own_todos().iter().filter(|todo| todo.cadence.section() == section).map(|todo| own_row(data, character, todo)));
    rows
}

fn section_view(data: &TodoData, character: &CharacterState, section: Section, playing: bool, moment: &Moment) -> SectionView {
    let rows = section_rows(data, character, section, playing, moment);
    let hidden_rows = rows.iter().filter(|row| row.hidden).count();
    let zone = moment.resets.zone();
    let now = moment.now_ms;

    let (left, tracked, full) = match section {
        Section::Recharges => {
            let amounts: Vec<&AmountView> =
                shown_amounts(&rows).into_iter().map(|(_, amount)| amount).filter(|amount| amount.tracked).collect();
            (0, amounts.len(), amounts.iter().filter(|amount| amount.full).count())
        }
        _ => (rows.iter().filter(|row| !row.hidden && needs_doing(row)).count(), 0, 0),
    };
    let next_reset_ms = match section {
        Section::Today => Some(moment.resets.next_daily(now)),
        Section::ThisWeek => Some(moment.resets.next_weekly(now)),
        Section::Recharges => None,
    };
    let soonest_full_at = shown_amounts(&rows).into_iter().filter_map(|(_, amount)| amount.full_at_ms).min();
    let countdown_ms = match (section, next_reset_ms) {
        (Section::Today | Section::ThisWeek, Some(at)) => Some(at - now),
        _ => soonest_full_at.map(|at| at - now),
    };
    let when_text = match (section, next_reset_ms) {
        (Section::Today, Some(at)) => copy::resets_in(at - now),
        (Section::ThisWeek, Some(at)) => copy::weekly_reset_when(zone, at, now),
        _ => match soonest_full_at {
            Some(at) => copy::next_full_in(at - now),
            None if tracked == 0 => copy::amounts_optional(),
            None => String::new(),
        },
    };
    let when_time_at = countdown_ms.and_then(|ms| when_text.strip_suffix(duration_words(ms).as_str()).map(str::len));
    let (count_text, count_is_done) = match section {
        Section::Recharges if tracked > 0 => (copy::tracked(tracked), false),
        Section::Recharges => (String::new(), false),
        _ if left > 0 => (copy::left(left), false),
        _ => (copy::done(), true),
    };
    SectionView {
        section,
        title: section.title(),
        folded: data.is_folded(section),
        rows,
        hidden_rows,
        left,
        tracked,
        full,
        count_text,
        count_is_done,
        full_text: (full > 0).then(|| copy::full(full)),
        when_text,
        when_time_at,
        next_reset_ms,
    }
}

/// A row that still has something to do: not done, and for a group, with a
/// row left to show.
fn needs_doing(row: &RowView) -> bool {
    match &row.kind {
        RowKind::Group(group) => group.total_children > 0 && !group.all_done,
        RowKind::Amount(_) => false,
        _ => !row.is_done(),
    }
}

/// What the game said about a ticket, as the character sees it.
struct Game {
    value: GameTicket,
    /// The name of the other character of the server that gave it.
    shared_from: Option<String>,
}

fn game_of(data: &TodoData, character: &CharacterState, builtin: &Builtin) -> Option<Game> {
    let known = data.game_ticket(character, builtin.ticket?)?;
    let shared_from = known.shared_from.map(|id| data.character(id).map(|giver| giver.name.clone()).unwrap_or_default());
    Some(Game { value: known.value, shared_from })
}

impl Game {
    fn origin(&self, playing: bool, moment: &Moment, refills: u32) -> Origin {
        let at_ms = self.value.at_ms;
        let source = match &self.shared_from {
            Some(name) => Source::Shared { from: name.clone(), at_ms },
            None if playing && at_ms >= moment.run_started_ms => Source::GameLive,
            None => Source::GameEarlier { at_ms, game_value: self.value.count, refills },
        };
        Origin { source, extra: self.value.charge, last_seen_ms: Some(at_ms), typed_over: None }
    }
}

/// What a row shows besides its figure.
struct Origin {
    source: Source,
    extra: u32,
    last_seen_ms: Option<i64>,
    /// The game's value the player typed over, as the row would show it.
    typed_over: Option<u32>,
}

impl Origin {
    fn counted_by_the_player(followed_by_game: bool) -> Self {
        let source = if followed_by_game { Source::NoneYet } else { Source::Manual };
        Self { source, extra: 0, last_seen_ms: None, typed_over: None }
    }
}

fn builtin_row(data: &TodoData, character: &CharacterState, builtin: &'static Builtin, playing: bool, moment: &Moment) -> RowView {
    let id = RowId::Builtin(builtin.id);
    let (kind, origin) = match builtin.kind {
        Kind::Tick => {
            let (done, origin) = progress(data, character, builtin, 1, playing, moment);
            (RowKind::Tick { done: done >= 1 }, origin)
        }
        Kind::Count { target } => {
            let (done, origin) = progress(data, character, builtin, target, playing, moment);
            (RowKind::Count { count: done, target, done: done >= target }, origin)
        }
        Kind::Amount(spec) => {
            let (view, origin) = amount_view(data, character, builtin, &spec, playing, moment);
            (RowKind::Amount(view), origin)
        }
        Kind::Group => (RowKind::Group(group_view(data, character, builtin, playing, moment)), Origin::counted_by_the_player(false)),
    };
    let tip = figure_tip(character, &kind, &origin, moment);
    RowView {
        id,
        label: builtin.label.to_string(),
        sub: builtin.sub.map(catalogue::Sub::text),
        account: builtin.is_account(),
        yours: false,
        hidden: character.is_hidden(builtin.id),
        kind,
        source: origin.source,
        extra: origin.extra,
        last_seen_ms: origin.last_seen_ms,
        tip,
    }
}

/// Where a figure's value comes from, for its tooltip; none for what the
/// player counts alone.
fn figure_tip(character: &CharacterState, kind: &RowKind, origin: &Origin, moment: &Moment) -> Option<FigureTip> {
    let stamp = |at_ms: i64| crate::i18n::day_stamp(moment.resets.zone(), at_ms);
    let (text, detail) = match (&origin.source, kind) {
        (_, RowKind::Tick { .. } | RowKind::Group(_)) => return None,
        (Source::GameLive, RowKind::Amount(amount)) => (copy::tip_live(), (origin.extra > 0).then(|| copy::tip_extra(origin.extra, amount.cap))),
        (Source::GameLive, _) => (copy::tip_live(), (origin.extra > 0).then(|| copy::tip_extra_entries(origin.extra))),
        (&Source::GameEarlier { at_ms, game_value, refills }, RowKind::Amount(amount)) => (
            copy::tip_game_value(game_value, &stamp(at_ms), character.logged_in_ms() == Some(at_ms)),
            (refills > 0).then(|| copy::tip_refills(refills, amount.refill_add)),
        ),
        (&Source::GameEarlier { at_ms, refills, .. }, _) => (copy::tip_game_at(&stamp(at_ms)), (refills > 0).then(copy::tip_restarted)),
        (&Source::Shared { at_ms, .. }, _) => (copy::tip_shared(&stamp(at_ms)), None),
        (Source::Typed, _) => match origin.typed_over {
            Some(game_value) => (copy::tip_typed(game_value), Some(copy::tip_typed_detail())),
            None => return None,
        },
        _ => return None,
    };
    Some(FigureTip { text, detail })
}

fn own_row(data: &TodoData, character: &CharacterState, todo: &OwnTodo) -> RowView {
    let id = RowId::Own(todo.id);
    let done = data.progress_of(character, id);
    let kind = match todo.kind {
        OwnKind::Tick => RowKind::Tick { done: done >= 1 },
        OwnKind::Count(target) => RowKind::Count { count: done, target, done: done >= target },
    };
    let origin = Origin::counted_by_the_player(false);
    RowView {
        id,
        label: todo.name.clone(),
        sub: None,
        account: false,
        yours: true,
        hidden: false,
        kind,
        source: origin.source,
        extra: origin.extra,
        last_seen_ms: origin.last_seen_ms,
        tip: None,
    }
}

/// How much a tick or a count has done. When the game fills the row, the game
/// sends what is left: done is the target minus that, and the row starts over
/// at the weekly reset that came after the game said it.
fn progress(
    data: &TodoData,
    character: &CharacterState,
    builtin: &Builtin,
    target: u32,
    playing: bool,
    moment: &Moment,
) -> (u32, Origin) {
    let Some(game) = game_of(data, character, builtin) else {
        let own = data.progress_of(character, RowId::Builtin(builtin.id));
        return (own, Origin::counted_by_the_player(builtin.ticket.is_some()));
    };
    let restarted = builtin.section == Section::ThisWeek && moment.resets.last_weekly(moment.now_ms) > game.value.at_ms;
    let left = if restarted { game.value.count.max(target) } else { game.value.count };
    let game_done = target.saturating_sub(left);
    let origin = game.origin(playing, moment, u32::from(left > game.value.count));
    match character.progress.get(builtin.id) {
        Some(&typed) => (typed.min(target), Origin { source: Source::Typed, typed_over: Some(game_done), ..origin }),
        None => (game_done, origin),
    }
}

fn group_view(data: &TodoData, character: &CharacterState, group: &'static Builtin, playing: bool, moment: &Moment) -> GroupView {
    let children: Vec<RowView> =
        catalogue::children_of(group.id).map(|builtin| builtin_row(data, character, builtin, playing, moment)).collect();
    let shown = children.iter().filter(|row| !row.hidden);
    let total_children = shown.clone().count();
    let done_children = shown.filter(|row| row.is_done()).count();
    let amounts = shown_amounts(&children);
    let tracked: Vec<&AmountView> = amounts.iter().map(|(_, amount)| *amount).filter(|amount| amount.tracked).collect();
    let full_children = amounts.iter().filter(|(_, amount)| amount.full).count();
    let summary = if amounts.is_empty() {
        copy::done_of(done_children, total_children)
    } else if full_children > 0 {
        copy::full(full_children)
    } else {
        let values = tracked.iter().map(|amount| amount.value);
        match (values.clone().min(), values.max(), tracked.first()) {
            (Some(low), Some(high), Some(first)) => copy::of_each(low, high, first.cap),
            _ => copy::dungeons(total_children),
        }
    };
    GroupView {
        open: data.is_group_open(group.id),
        done_children,
        total_children,
        all_done: total_children > 0 && done_children == total_children,
        full_children,
        summary,
        hidden_children: children.len() - total_children,
        children,
    }
}

fn amount_view(
    data: &TodoData,
    character: &CharacterState,
    builtin: &Builtin,
    spec: &AmountSpec,
    playing: bool,
    moment: &Moment,
) -> (AmountView, Origin) {
    let continuous = spec.refill.is_continuous();
    let game = game_of(data, character, builtin);
    let above_plain_cap = !moment.membership
        && spec.membership_cap.is_some()
        && game.as_ref().is_some_and(|game| game.value.count > spec.cap);
    let game_standing = game.as_ref().map(|game| moment.standing(spec, game.value.count, game.value.at_ms));
    let base = AmountView {
        step: spec.step,
        cap: moment.cap_of(spec),
        tracked: false,
        value: 0,
        full: false,
        full_at_ms: None,
        refill_at_ms: None,
        status_text: None,
        continuous,
        refill_add: spec.refill.add(),
        extra_cap: spec.extra_cap,
        game_value: game_standing.map(|standing| standing.value),
        above_plain_cap,
    };
    let typed = character.typed_amount(builtin.id);
    let (standing, origin, game_is_basis) = match (typed, &game, game_standing) {
        (Some(typed), _, _) => {
            let extra = game.as_ref().map_or(0, |game| game.value.charge);
            let last_seen_ms = game.as_ref().map(|game| game.value.at_ms);
            let origin = Origin { source: Source::Typed, extra, last_seen_ms, typed_over: base.game_value };
            (moment.standing(spec, typed.value, typed.at_ms), origin, false)
        }
        (None, Some(game), Some(standing)) => (standing, game.origin(playing, moment, standing.refills), true),
        _ => return (base, Origin::counted_by_the_player(true)),
    };
    let full = standing.is_full() && !(game_is_basis && above_plain_cap);
    let status_text = match standing.full_at_ms {
        Some(at_ms) => Some(copy::amount_full_in(at_ms - moment.now_ms)),
        None if !full => None,
        None if continuous => Some(copy::full_being_wasted()),
        None => {
            let next = super::amount::next_refill(spec, &moment.resets, moment.now_ms);
            Some(copy::full_spend_by(&crate::i18n::day_stamp(moment.resets.zone(), next)))
        }
    };
    let view = AmountView {
        tracked: true,
        value: standing.value,
        full,
        full_at_ms: standing.full_at_ms,
        refill_at_ms: full.then(|| super::amount::next_refill(spec, &moment.resets, moment.now_ms)),
        status_text,
        ..base
    };
    (view, origin)
}

#[cfg(test)]
mod tests {
    use meter_core::identity::KnownCharacter;

    use super::*;
    use crate::clock::testing::{utc, TestZone};
    use meter_core::capture::parser::ticket_record::{TicketReason, TicketRecord};
    use meter_core::tickets::{TicketContent, TicketReport};
    use crate::todos::data::{Cadence, NewOwnTodo};

    const HOUR: i64 = 3_600_000;

    fn summer() -> TestZone {
        TestZone { base: 120, changes: Vec::new() }
    }

    /// Local time in the fixed summer zone; 2026-10-05 is a Monday.
    fn local(day_of_october: i64, hour: i64, minute: i64) -> i64 {
        utc(2026, 10, day_of_october, hour, minute) - 2 * HOUR
    }

    /// Monday 12:48, 20 h 12 min before the daily reset and 1 d 20 h 12 min before Wednesday's.
    fn monday_noon(zone: &TestZone) -> Moment<'_> {
        Moment::new(zone, 9, false, local(5, 12, 48))
    }

    fn tracked() -> (TodoData, CharacterId) {
        let mut data = TodoData::default();
        data.sync_characters(&[KnownCharacter {
            name: "Alder".to_string(),
            server_id: 1001,
            class: "GLADIATOR".to_string(),
            key: None,
        }]);
        let id = data.characters()[0].id;
        (data, id)
    }

    fn list_of(data: &TodoData, id: CharacterId, moment: &Moment) -> TodoList {
        build(data, data.character(id).unwrap(), false, moment)
    }

    fn row<'a>(list: &'a TodoList, id: &str) -> &'a RowView {
        let wanted = RowId::Builtin(catalogue::builtin(id).unwrap().id);
        let mut rows: Vec<&RowView> = Vec::new();
        for row in list.sections.iter().flat_map(|section| &section.rows) {
            rows.push(row);
            if let RowKind::Group(group) = &row.kind {
                rows.extend(&group.children);
            }
        }
        rows.into_iter().find(|row| row.id == wanted).unwrap_or_else(|| panic!("no row {id}"))
    }

    fn amount<'a>(list: &'a TodoList, id: &str) -> &'a AmountView {
        match &row(list, id).kind {
            RowKind::Amount(amount) => amount,
            other => panic!("{id} is {other:?}"),
        }
    }

    fn hide_everything_to_do(data: &mut TodoData, id: CharacterId) {
        for builtin in catalogue::CATALOGUE.iter().filter(|builtin| builtin.section != Section::Recharges) {
            data.hide(id, builtin.id);
        }
    }

    #[test]
    fn a_fresh_character_has_every_default_row_left_and_duty_commands_hidden() {
        let zone = summer();
        let (data, id) = tracked();
        let list = list_of(&data, id, &monday_noon(&zone));
        let [today, recharges, week] = &list.sections[..] else { panic!("three sections") };
        assert_eq!((today.left, week.left, recharges.left), (1, 10, 0), "groups count as one row each");
        assert_eq!((list.left, list.left_text.as_str()), (11, "11 left"));
        assert_eq!((today.count_text.as_str(), week.count_text.as_str()), ("1 left", "10 left"));
        assert_eq!(week.hidden_rows, 1);
        assert!(row(&list, "dutycmd").hidden);
        assert!(list.everything_done.is_none() && list.full_text.is_none());
        let names: Vec<&str> = week.rows.iter().map(|row| row.label.as_str()).take(4).collect();
        assert_eq!(names, ["Daily Dungeon", "Ascension Trial", "Subjugation", "Sanctuary"]);
    }

    #[test]
    fn the_sections_say_when_they_reset() {
        let zone = summer();
        let (data, id) = tracked();
        let list = list_of(&data, id, &monday_noon(&zone));
        assert_eq!(list.sections[0].when_text, "resets in 20 h 12 min");
        assert_eq!(list.sections[0].next_reset_ms, Some(local(6, 9, 0)));
        assert_eq!(list.sections[2].when_text, "Wed 09:00 \u{b7} in 1 d 20 h");
        assert_eq!(list.sections[2].next_reset_ms, Some(local(7, 9, 0)));
        assert_eq!((list.footer.reset_time.as_str(), list.footer.weekly.as_str()), ("09:00", "Weekly on Wednesday"));
    }

    #[test]
    fn a_section_reads_done_in_the_good_style_once_nothing_is_left() {
        let zone = summer();
        let (mut data, id) = tracked();
        for _ in 0..5 {
            data.adjust_count(id, RowId::Builtin("duty"), 1);
        }
        let list = list_of(&data, id, &monday_noon(&zone));
        let today = &list.sections[0];
        assert_eq!((today.count_text.as_str(), today.count_is_done, today.left), ("done", true, 0));
        assert_eq!(list.left, 10);
        assert!(matches!(row(&list, "duty").kind, RowKind::Count { count: 5, target: 5, done: true }));
        assert!(row(&list, "duty").account);
    }

    #[test]
    fn a_group_is_one_row_that_is_done_when_all_its_shown_rows_are() {
        let zone = summer();
        let (mut data, id) = tracked();
        let sanctuary = |data: &TodoData| match &row(&list_of(data, id, &monday_noon(&zone)), "sanct").kind {
            RowKind::Group(group) => group.clone(),
            other => panic!("{other:?}"),
        };
        data.toggle_tick(id, RowId::Builtin("forgeboss"));
        data.adjust_count(id, RowId::Builtin("forge"), 4);
        let group = sanctuary(&data);
        assert_eq!((group.done_children, group.total_children, group.summary.as_str()), (2, 6, "2 of 6 done"));
        assert!(!group.all_done && !group.open);
        assert_eq!(list_of(&data, id, &monday_noon(&zone)).sections[2].left, 10, "still one row left");

        for (row_id, times) in [("corr", 4), ("corrboss", 2), ("chal", 4), ("chalboss", 2)] {
            data.adjust_count(id, RowId::Builtin(row_id), times);
        }
        assert!(sanctuary(&data).all_done);
        let list = list_of(&data, id, &monday_noon(&zone));
        assert_eq!(list.sections[2].left, 9);
        assert!(row(&list, "sanct").is_done());
    }

    #[test]
    fn hiding_a_row_of_a_group_shrinks_the_group_and_a_group_with_nothing_shown_is_not_left() {
        let zone = summer();
        let (mut data, id) = tracked();
        data.hide(id, "chalboss");
        data.set_group_open("sanct", true);
        let list = list_of(&data, id, &monday_noon(&zone));
        let RowKind::Group(group) = &row(&list, "sanct").kind else { panic!("a group") };
        assert_eq!((group.total_children, group.hidden_children, group.open), (5, 1, true));
        assert_eq!(group.children.len(), 6, "the hidden row is still listed, flagged, for Show");

        for child in ["forge", "forgeboss", "corr", "corrboss", "chal"] {
            data.hide(id, child);
        }
        let list = list_of(&data, id, &monday_noon(&zone));
        assert_eq!(list.sections[2].left, 9, "nothing shown inside, so nothing left to do there");
    }

    #[test]
    fn hidden_rows_do_not_count_and_own_to_dos_follow_the_built_ins() {
        let zone = summer();
        let (mut data, id) = tracked();
        data.hide(id, "trial");
        let daily = data
            .add_own(id, NewOwnTodo { name: "Legion".into(), kind: OwnKind::Tick, cadence: Cadence::Daily })
            .unwrap();
        data.add_own(id, NewOwnTodo { name: "Market".into(), kind: OwnKind::Count(3), cadence: Cadence::Weekly }).unwrap();
        let list = list_of(&data, id, &monday_noon(&zone));
        assert_eq!((list.sections[0].left, list.sections[2].left), (2, 10), "Legion added, Trial hidden, Market added");
        let today: Vec<(&str, bool)> = list.sections[0].rows.iter().map(|row| (row.label.as_str(), row.yours)).collect();
        assert_eq!(today, [("Duty Missions", false), ("Legion", true)]);
        let last = list.sections[2].rows.last().unwrap();
        assert_eq!((last.label.as_str(), last.yours), ("Market", true));
        assert!(matches!(last.kind, RowKind::Count { count: 0, target: 3, done: false }));

        data.toggle_tick(id, RowId::Own(daily));
        assert_eq!(list_of(&data, id, &monday_noon(&zone)).sections[0].left, 1);
    }

    #[test]
    fn the_accessible_name_adds_the_sub_label() {
        let zone = summer();
        let (data, id) = tracked();
        let list = list_of(&data, id, &monday_noon(&zone));
        assert_eq!(row(&list, "fissure").accessible_name(), "Daily Dungeon, Unknown Fissure");
        assert_eq!(row(&list, "trial").accessible_name(), "Ascension Trial");
    }

    #[test]
    fn amounts_are_empty_and_out_of_every_count_until_typed() {
        let zone = summer();
        let (data, id) = tracked();
        let list = list_of(&data, id, &monday_noon(&zone));
        let recharges = &list.sections[1];
        assert_eq!((recharges.when_text.as_str(), recharges.count_text.as_str()), ("amounts optional", ""));
        assert_eq!((recharges.tracked, recharges.full, list.full), (0, 0, 0));
        let odyle = amount(&list, "odyle");
        assert_eq!((odyle.tracked, odyle.status_text.as_deref(), odyle.value_text().as_str()), (false, None, "not tracked"));
        assert_eq!((odyle.step, odyle.cap), (10, 560));
    }

    #[test]
    fn a_typed_amount_says_when_it_is_full_and_the_section_names_the_soonest() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        data.set_amount(id, "odyle", Some(425), &now);
        data.set_amount(id, "shugo", Some(10), &now);
        let list = list_of(&data, id, &now);
        let recharges = &list.sections[1];
        assert_eq!((recharges.tracked, recharges.count_text.as_str()), (2, "2 tracked"));
        let odyle = amount(&list, "odyle");
        assert_eq!(odyle.value, 425);
        assert_eq!(odyle.status_text.as_deref(), Some("Full in 1 d 2 h"), "(560 - 425) / 15 = 9 refills on the 3 h clock, the last one 26 h 12 min away");
        assert_eq!(odyle.value_text(), "425 of 560");
        assert_eq!(amount(&list, "shugo").status_text.as_deref(), Some("Full in 20 h 12 min"), "+3 at 09:00 reaches 12");
        assert_eq!(recharges.when_text, "next full in 20 h 12 min");
    }

    #[test]
    fn a_full_amount_is_wasted_or_names_the_refill_it_will_lose() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        data.set_amount(id, "odyle", Some(560), &now);
        data.set_amount(id, "shugo", Some(12), &now);
        data.set_amount(id, "balaur", Some(7), &now);
        let list = list_of(&data, id, &now);
        assert_eq!(amount(&list, "odyle").status_text.as_deref(), Some("Full, being wasted"));
        assert_eq!(amount(&list, "shugo").status_text.as_deref(), Some("Full, spend by Tue 09:00"));
        assert_eq!(amount(&list, "balaur").status_text.as_deref(), Some("Full, spend by Tue 09:00"));
        assert_eq!((list.full, list.full_text.as_deref()), (3, Some("3 full")));
        let recharges = &list.sections[1];
        assert_eq!((recharges.full, recharges.full_text.as_deref(), recharges.when_text.as_str()), (3, Some("3 full"), ""));
    }

    #[test]
    fn a_hidden_amount_is_left_out_of_the_counts() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        data.set_amount(id, "odyle", Some(560), &now);
        data.hide(id, "odyle");
        let list = list_of(&data, id, &now);
        assert_eq!((list.full, list.sections[1].tracked), (0, 0));
        assert!(row(&list, "odyle").hidden);
    }

    #[test]
    fn membership_lifts_the_odyle_cap_in_the_list() {
        let zone = summer();
        let (mut data, id) = tracked();
        let member = Moment::new(&zone, 9, true, local(5, 12, 48));
        data.set_amount(id, "odyle", Some(600), &member);
        let odyle = amount(&list_of(&data, id, &member), "odyle").clone();
        assert_eq!((odyle.value, odyle.cap, odyle.full), (600, 840, false));
    }

    #[test]
    fn everything_done_carries_the_amount_that_fills_first() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        hide_everything_to_do(&mut data, id);
        let list = list_of(&data, id, &now);
        assert_eq!((list.left, list.left_text.as_str()), (0, "all done"));
        assert_eq!(list.everything_done, Some(EverythingDone { fills_first: None }));
        assert_eq!(list.sections[0].count_text, "done");

        data.set_amount(id, "odyle", Some(425), &now);
        let list = list_of(&data, id, &now);
        assert_eq!(
            list.everything_done,
            Some(EverythingDone { fills_first: Some("Odyle Energy is full in 1 d 2 h.".to_string()) })
        );
    }

    #[test]
    fn the_picker_shows_full_before_left_and_all_done_last() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        assert_eq!(list_of(&data, id, &now).character.status, "11 left");
        data.set_amount(id, "odyle", Some(560), &now);
        let summary = list_of(&data, id, &now).character;
        assert_eq!((summary.status.as_str(), summary.status_is_warning), ("1 full", true));
        data.set_amount(id, "odyle", None, &now);
        hide_everything_to_do(&mut data, id);
        assert_eq!(list_of(&data, id, &now).character.status, "all done");
    }

    #[test]
    fn the_summary_names_the_class_and_server() {
        let zone = summer();
        let (data, id) = tracked();
        let summary = list_of(&data, id, &monday_noon(&zone)).character;
        assert_eq!(summary.class_name, "Gladiator");
        assert_eq!(summary.class(), Some(PlayerClass::Gladiator));
        assert_eq!(summary.name, "Alder");
        assert!(summary.class_and_server().starts_with("Gladiator \u{b7} "));
    }

    fn record(id: u32, count: u32, charge: u32) -> TicketRecord {
        TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge }
    }

    fn alder() -> KnownCharacter {
        KnownCharacter { name: "Alder".to_string(), server_id: 1001, class: "GLADIATOR".to_string(), key: None }
    }

    fn login(data: &mut TodoData, who: &KnownCharacter, at_ms: i64, records: Vec<TicketRecord>) {
        let report = TicketReport { character: who.clone(), captured_at: at_ms as f64 / 1000.0, content: TicketContent::Snapshot(records) };
        data.apply_tickets(&report);
    }

    fn change(data: &mut TodoData, who: &KnownCharacter, at_ms: i64, record: TicketRecord) {
        let content = TicketContent::Change { record, reason: TicketReason::Spend, delta: Some(1) };
        data.apply_tickets(&TicketReport { character: who.clone(), captured_at: at_ms as f64 / 1000.0, content });
    }

    #[test]
    fn the_game_value_gains_the_refills_since_the_login_up_to_the_cap_and_never_a_spend() {
        let zone = summer();
        let (mut data, id) = tracked();
        let seen_at = local(4, 22, 40);
        let ticket_list = vec![record(60_000_001, 125, 245), record(1, 3, 3), record(4, 12, 0), record(8, 2, 0), record(102, 0, 0)];
        login(&mut data, &alder(), seen_at, ticket_list);
        let list = list_of(&data, id, &monday_noon(&zone));

        let odyle = amount(&list, "odyle");
        assert_eq!((odyle.value, odyle.game_value, odyle.full, odyle.tracked), (200, Some(200), false, true));
        assert_eq!(row(&list, "odyle").extra, 245);
        assert_eq!(row(&list, "odyle").source, Source::GameEarlier { at_ms: seen_at, game_value: 125, refills: 5 });
        assert_eq!(row(&list, "odyle").last_seen_ms, Some(seen_at));
        assert_eq!(amount(&list, "shugo").value, 6, "3 and the +3 of Monday 09:00");
        assert_eq!(amount(&list, "invasion").value, 3);
        let nightmare = amount(&list, "nightmare");
        assert_eq!((nightmare.value, nightmare.full, nightmare.status_text.as_deref()), (14, true, Some("Full, spend by Tue 09:00")));
        assert_eq!(amount(&list, "balaur").value, 1, "an empty pool is not guessed fuller than the table says");
    }

    #[test]
    fn the_membership_cap_lets_the_projection_go_further() {
        let zone = summer();
        let (mut data, id) = tracked();
        login(&mut data, &alder(), local(2, 12, 0), vec![record(1, 12, 0)]);
        let plain = amount(&list_of(&data, id, &monday_noon(&zone)), "shugo").clone();
        assert_eq!((plain.value, plain.cap, plain.full), (12, 12, true));
        let member = Moment::new(&zone, 9, true, local(5, 12, 48));
        let member = amount(&list_of(&data, id, &member), "shugo").clone();
        assert_eq!((member.value, member.cap, member.full), (21, 21, true), "+3 on Sat, Sun and Mon reaches 21");
    }

    #[test]
    fn weekly_rows_read_done_as_the_target_minus_what_is_left_and_start_over_at_the_reset() {
        let zone = summer();
        let (mut data, id) = tracked();
        let seen_at = local(4, 22, 40);
        let ticket_list = vec![
            record(80_000_001, 1, 1),
            record(90_000_002, 0, 0),
            record(90_000_004, 2, 0),
            record(101, 5, 7),
            record(60_000_008, 28, 0),
            record(60_000_201, 0, 0),
        ];
        login(&mut data, &alder(), seen_at, ticket_list);
        let list = list_of(&data, id, &monday_noon(&zone));

        assert!(matches!(row(&list, "trial").kind, RowKind::Count { count: 2, target: 3, done: false }), "1 left of 3");
        assert_eq!(row(&list, "trial").extra, 1);
        assert!(matches!(row(&list, "forgeboss").kind, RowKind::Tick { done: true }), "the final boss ticks at 0 left");
        assert!(matches!(row(&list, "corrboss").kind, RowKind::Count { count: 0, target: 2, done: false }));
        assert!(matches!(row(&list, "fissure").kind, RowKind::Count { count: 9, target: 14, done: false }), "5 left of 14");
        assert_eq!(row(&list, "fissure").extra, 7);
        assert!(row(&list, "fissure").account, "the server's, not the character's");
        assert!(matches!(row(&list, "transc").kind, RowKind::Count { count: 0, target: 28, done: false }));

        change(&mut data, &alder(), local(5, 12, 30), record(80_000_001, 0, 1));
        let list = list_of(&data, id, &monday_noon(&zone));
        assert!(matches!(row(&list, "trial").kind, RowKind::Count { count: 3, target: 3, done: true }), "done at 0 left");

        let after_the_reset = Moment::new(&zone, 9, false, local(7, 9, 30));
        let list = list_of(&data, id, &after_the_reset);
        assert!(matches!(row(&list, "trial").kind, RowKind::Count { count: 0, target: 3, done: false }), "Wednesday's reset");
        assert!(matches!(row(&list, "forgeboss").kind, RowKind::Tick { done: false }));
        assert!(matches!(row(&list, "fissure").kind, RowKind::Count { count: 0, target: 14, .. }));
        assert_eq!(row(&list, "trial").extra, 1, "the extra pool does not start over");
        assert_eq!(row(&list, "trial").source, Source::GameEarlier { at_ms: local(5, 12, 30), game_value: 0, refills: 1 });
    }

    #[test]
    fn a_value_heard_while_the_character_is_played_is_live() {
        let zone = summer();
        let (mut data, id) = tracked();
        login(&mut data, &alder(), local(5, 12, 30), vec![record(4, 6, 0)]);
        let now = Moment::new(&zone, 9, false, local(5, 12, 48)).running_since(local(5, 12, 0));
        let character = data.character(id).unwrap();
        assert_eq!(build(&data, character, true, &now).sections[1].rows[3].source, Source::GameLive);
        let earlier_run = now.running_since(local(5, 12, 40));
        assert!(matches!(build(&data, character, true, &earlier_run).sections[1].rows[3].source, Source::GameEarlier { .. }));
        assert!(matches!(build(&data, character, false, &now).sections[1].rows[3].source, Source::GameEarlier { .. }));
    }

    #[test]
    fn what_is_typed_over_the_game_stands_until_the_games_next_value_for_that_ticket() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        login(&mut data, &alder(), local(5, 12, 0), vec![record(1, 6, 3), record(4, 5, 0)]);
        data.set_amount(id, "shugo", Some(24), &now);

        let list = list_of(&data, id, &now);
        let shugo = amount(&list, "shugo");
        assert_eq!((shugo.value, shugo.game_value, shugo.full, shugo.cap), (24, Some(6), true, 12), "above the cap is normal");
        assert_eq!(row(&list, "shugo").source, Source::Typed);
        assert_eq!(row(&list, "shugo").extra, 3, "the extra pool stays the game's");

        change(&mut data, &alder(), local(5, 12, 50), record(4, 4, 0));
        assert_eq!(amount(&list_of(&data, id, &now), "shugo").value, 24, "another ticket's value does not replace it");

        change(&mut data, &alder(), local(5, 12, 55), record(1, 5, 3));
        let list = list_of(&data, id, &now);
        assert_eq!((amount(&list, "shugo").value, amount(&list, "shugo").full), (5, false));
        assert!(matches!(row(&list, "shugo").source, Source::GameEarlier { .. }));

        data.set_amount(id, "shugo", Some(24), &now);
        data.set_amount(id, "shugo", None, &now);
        assert_eq!(amount(&list_of(&data, id, &now), "shugo").value, 5, "clearing it goes back to the game's value");
    }

    #[test]
    fn before_the_game_says_a_typed_number_is_the_total_and_the_row_follows_its_source() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        assert_eq!(row(&list_of(&data, id, &now), "shugo").source, Source::NoneYet);
        assert_eq!(row(&list_of(&data, id, &now), "duty").source, Source::Manual);
        data.set_amount(id, "shugo", Some(12), &now);
        let list = list_of(&data, id, &now);
        assert_eq!((amount(&list, "shugo").full, amount(&list, "shugo").game_value), (true, None));
        assert_eq!(row(&list, "shugo").source, Source::Typed);
        assert_eq!(row(&list, "shugo").last_seen_ms, None);
    }

    #[test]
    fn with_the_membership_off_a_pool_above_the_plain_cap_is_not_full_and_says_so() {
        let zone = summer();
        let (mut data, id) = tracked();
        login(&mut data, &alder(), local(5, 12, 30), vec![record(1, 18, 0), record(60_000_001, 560, 0)]);
        let list = list_of(&data, id, &monday_noon(&zone));
        let shugo = amount(&list, "shugo");
        assert_eq!((shugo.value, shugo.full, shugo.above_plain_cap, shugo.status_text.as_deref()), (18, false, true, None));
        assert_eq!(shugo.refill_at_ms, None);
        assert!(amount(&list, "odyle").full, "560 is the plain cap itself");
        assert_eq!(list.full, 1);

        let member = Moment::new(&zone, 9, true, local(5, 12, 48));
        let shugo = amount(&list_of(&data, id, &member), "shugo").clone();
        assert_eq!((shugo.value, shugo.cap, shugo.full, shugo.above_plain_cap), (18, 21, false, false), "with the membership 18 is just a count below 21");
        assert_eq!(shugo.full_at_ms, Some(local(6, 9, 0)), "the +3 of Tuesday 09:00");
    }

    #[test]
    fn account_rows_come_from_another_character_of_the_server() {
        let zone = summer();
        let mut data = TodoData::default();
        let birch = KnownCharacter { name: "Birch".to_string(), server_id: 1001, class: "RANGER".to_string(), key: None };
        let cedar = KnownCharacter { name: "Cedar".to_string(), server_id: 2001, class: "RANGER".to_string(), key: None };
        data.sync_characters(&[alder(), birch.clone(), cedar.clone()]);
        let (alder_id, birch_id, cedar_id) = (data.characters()[0].id, data.characters()[1].id, data.characters()[2].id);
        login(&mut data, &alder(), local(5, 12, 30), vec![record(8, 2, 0), record(4, 7, 0)]);

        let list = list_of(&data, birch_id, &monday_noon(&zone));
        assert_eq!(row(&list, "invasion").source, Source::Shared { from: "Alder".to_string(), at_ms: local(5, 12, 30) });
        assert_eq!(amount(&list, "invasion").value, 2);
        assert!(row(&list, "invasion").account);
        assert_eq!(row(&list, "nightmare").source, Source::NoneYet, "Nightmare is each character's own");
        assert_eq!(row(&list_of(&data, cedar_id, &monday_noon(&zone)), "invasion").source, Source::NoneYet);
        assert!(matches!(row(&list_of(&data, alder_id, &monday_noon(&zone)), "invasion").source, Source::GameEarlier { .. }));
    }

    #[test]
    fn conquest_dungeons_are_a_group_of_amounts_and_the_full_ones_are_listed_for_the_warning() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        login(
            &mut data,
            &alder(),
            local(5, 12, 30),
            vec![record(60_000_101, 14, 0), record(60_001_201, 6, 0), record(1, 12, 0), record(60_005_301, 4, 0)],
        );
        let list = list_of(&data, id, &now);
        let RowKind::Group(group) = &row(&list, "conquest").kind else {
            panic!("Conquest dungeons is a group");
        };
        assert_eq!(group.children.len(), 6);
        assert_eq!((group.full_children, group.summary.as_str()), (1, "1 full"));
        assert_eq!(amount(&list, "c-urugugu").value, 6);
        assert_eq!(list.sections[1].tracked, 4);

        let character = data.character(id).unwrap();
        let full = full_resources(&data, character, false, &now);
        let labels: Vec<(&str, Option<&str>)> = full.iter().map(|resource| (resource.label.as_str(), resource.group)).collect();
        assert_eq!(labels, [("Shugo Festival keys", None), ("Krao Cave", Some("Conquest dungeons")), ("Deus Research Base", None)]);
        assert!(full.iter().all(|resource| resource.refill_at_ms == local(6, 9, 0)));
        assert_eq!((full[1].value, full[1].cap), (14, 14));
        let ids: Vec<RowId> = recharges(&data, character, false, &now).iter().map(|row| row.id).collect();
        assert_eq!(ids[..2], [RowId::Builtin("odyle"), RowId::Builtin("shugo")]);

        data.hide(id, "c-krao");
        let character = data.character(id).unwrap();
        assert_eq!(full_resources(&data, character, false, &now).len(), 2, "a hidden row is not warned about");
        let list = list_of(&data, id, &now);
        let RowKind::Group(group) = &row(&list, "conquest").kind else {
            panic!("Conquest dungeons is a group");
        };
        assert_eq!(group.summary, "6 of 14 each", "the tracked dungeons' spread once none is full");
    }

    #[test]
    fn a_group_of_amounts_reads_the_spread_of_its_dungeons_or_how_many_there_are() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        let summary = |data: &TodoData| match &row(&list_of(data, id, &now), "conquest").kind {
            RowKind::Group(group) => group.summary.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(summary(&data), "6 dungeons");
        login(&mut data, &alder(), local(5, 12, 30), vec![record(60_000_101, 10, 0), record(60_001_201, 13, 0)]);
        assert_eq!(summary(&data), "10\u{2013}13 of 14 each");
    }

    #[test]
    fn the_source_line_says_where_the_values_come_from() {
        let zone = summer();
        let mut data = TodoData::default();
        let birch = KnownCharacter { name: "Birch".to_string(), server_id: 1001, class: "RANGER".to_string(), key: None };
        data.sync_characters(&[alder(), birch]);
        let (alder_id, birch_id) = (data.characters()[0].id, data.characters()[1].id);
        let now = monday_noon(&zone);
        assert_eq!(list_of(&data, alder_id, &now).source_line, SourceLine::NoneYet);

        login(&mut data, &alder(), local(4, 22, 40), vec![record(60_000_001, 125, 245), record(8, 2, 0)]);
        assert_eq!(list_of(&data, alder_id, &now).source_line, SourceLine::Earlier { at: "Sun 22:40".to_string() });
        assert_eq!(list_of(&data, birch_id, &now).source_line, SourceLine::Shared { from: "Alder".to_string() });

        login(&mut data, &alder(), local(5, 12, 30), vec![record(60_000_001, 300, 245)]);
        let character = data.character(alder_id).unwrap();
        let this_run = Moment::new(&zone, 9, false, local(5, 12, 48)).running_since(local(5, 12, 0));
        let live = build(&data, character, true, &this_run).source_line;
        assert_eq!(live, SourceLine::Live { logged_in_at: Some("12:30".to_string()) });
    }

    #[test]
    fn a_figures_tooltip_says_where_its_value_comes_from() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        assert_eq!(row(&list_of(&data, id, &now), "duty").tip, None, "Duty Missions are counted by hand");
        login(&mut data, &alder(), local(4, 22, 40), vec![record(60_000_001, 125, 245), record(1, 3, 3)]);
        let list = list_of(&data, id, &now);
        let odyle = row(&list, "odyle").tip.clone().expect("a tooltip");
        assert_eq!(odyle.text, "125 at your login Sun 22:40.");
        assert_eq!(odyle.detail.as_deref(), Some("+75 since: 5 refills of 15, estimated."));

        let character = data.character(id).unwrap();
        let this_run = Moment::new(&zone, 9, false, local(5, 12, 48)).running_since(local(4, 22, 0));
        let live = build(&data, character, true, &this_run);
        let shugo = row(&live, "shugo").tip.clone().expect("a tooltip");
        assert_eq!(shugo.text, "From the game, live.");
        assert_eq!(shugo.detail.as_deref(), Some("+3 extra from items: they never refill and are not part of the 12."));

        data.set_amount(id, "shugo", Some(9), &now);
        let typed = row(&list_of(&data, id, &now), "shugo").tip.clone().expect("a tooltip");
        assert_eq!(typed.text, "Typed by you. The game said 6.");
    }

    #[test]
    fn a_weekly_row_typed_over_reads_the_typed_count_until_the_game_says_again() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        login(&mut data, &alder(), local(5, 12, 30), vec![record(80_000_001, 1, 1)]);
        data.set_progress(id, "trial", Some(0));
        let list = list_of(&data, id, &now);
        assert!(matches!(row(&list, "trial").kind, RowKind::Count { count: 0, target: 3, done: false }));
        assert_eq!(row(&list, "trial").source, Source::Typed);
        assert_eq!(row(&list, "trial").extra, 1, "the extra entries stay the game's");
        assert_eq!(row(&list, "trial").tip.as_ref().map(|tip| tip.text.as_str()), Some("Typed by you. The game said 2."));
        change(&mut data, &alder(), local(5, 12, 40), record(80_000_001, 0, 1));
        assert!(matches!(row(&list_of(&data, id, &now), "trial").kind, RowKind::Count { count: 3, done: true, .. }));
    }
}
