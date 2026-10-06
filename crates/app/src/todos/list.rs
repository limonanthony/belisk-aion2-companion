//! What the window shows for one character at a moment: sections, rows with
//! their display state, the counts and the words for them. Built fresh from the
//! stored data each time; nothing here is stored.

use super::catalogue::{self, AmountSpec, Builtin, Kind, Section};
use super::copy;
use super::data::{CharacterId, CharacterState, Moment, OwnKind, OwnTodo, RowId, TodoData};
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
    pub reset_hour: u8,
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

#[derive(Clone, Debug, PartialEq)]
pub struct RowView {
    pub id: RowId,
    pub label: String,
    pub sub: Option<String>,
    /// Shown with the "Account" tag (Duty Missions).
    pub account: bool,
    /// Shown with the "Yours" tag; its × removes instead of hides.
    pub yours: bool,
    pub hidden: bool,
    pub kind: RowKind,
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
    /// "2 of 6 done".
    pub summary: String,
    pub children: Vec<RowView>,
    pub hidden_children: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmountView {
    /// What one arrow press adds or removes.
    pub step: u32,
    pub cap: u32,
    /// The player typed one; else the row shows "—".
    pub tracked: bool,
    /// What it is worth now (0 when not tracked).
    pub value: u32,
    pub full: bool,
    /// When it reaches the cap, while it is filling.
    pub full_at_ms: Option<i64>,
    /// "Full in 2 d 16 h" while filling, "Full, being wasted" or "Full, spend by
    /// Tue 16:00" at the cap; none when not tracked.
    pub status_text: Option<String>,
    /// Refills come continuously (Odyle Energy), not on a schedule.
    pub continuous: bool,
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
        Section::ALL.into_iter().map(|section| section_view(data, character, section, moment)).collect();
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
        sections,
        footer: Footer {
            reset_hour: moment.resets.hour(),
            reset_time: format!("{:02}:00", moment.resets.hour()),
            weekly: copy::weekly_on_wednesday(),
        },
    }
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

fn soonest_filling(sections: &[SectionView]) -> Option<(&str, i64)> {
    sections
        .iter()
        .filter(|section| section.section == Section::Recharges)
        .flat_map(|section| &section.rows)
        .filter(|row| !row.hidden)
        .filter_map(|row| match &row.kind {
            RowKind::Amount(amount) => amount.full_at_ms.map(|at_ms| (row.label.as_str(), at_ms)),
            _ => None,
        })
        .min_by_key(|&(_, at_ms)| at_ms)
}

fn section_view(data: &TodoData, character: &CharacterState, section: Section, moment: &Moment) -> SectionView {
    let mut rows: Vec<RowView> = catalogue::CATALOGUE
        .iter()
        .filter(|builtin| builtin.section == section && builtin.group.is_none())
        .map(|builtin| builtin_row(data, character, builtin, moment))
        .collect();
    rows.extend(character.own_todos().iter().filter(|todo| todo.cadence.section() == section).map(|todo| own_row(data, character, todo)));

    let hidden_rows = rows.iter().filter(|row| row.hidden).count();
    let visible = rows.iter().filter(|row| !row.hidden);
    let zone = moment.resets.zone();
    let now = moment.now_ms;

    let (left, tracked, full) = match section {
        Section::Recharges => {
            let amounts: Vec<&AmountView> = visible
                .filter_map(|row| match &row.kind {
                    RowKind::Amount(amount) if amount.tracked => Some(amount),
                    _ => None,
                })
                .collect();
            (0, amounts.len(), amounts.iter().filter(|amount| amount.full).count())
        }
        _ => (visible.filter(|row| needs_doing(row)).count(), 0, 0),
    };
    let next_reset_ms = match section {
        Section::Today => Some(moment.resets.next_daily(now)),
        Section::ThisWeek => Some(moment.resets.next_weekly(now)),
        Section::Recharges => None,
    };
    let soonest_full_at = rows
        .iter()
        .filter(|row| !row.hidden)
        .filter_map(|row| match &row.kind {
            RowKind::Amount(amount) => amount.full_at_ms,
            _ => None,
        })
        .min();
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

fn builtin_row(data: &TodoData, character: &CharacterState, builtin: &'static Builtin, moment: &Moment) -> RowView {
    let id = RowId::Builtin(builtin.id);
    let done = data.progress_of(character, id);
    let kind = match builtin.kind {
        Kind::Tick => RowKind::Tick { done: done >= 1 },
        Kind::Count { target } => RowKind::Count { count: done, target, done: done >= target },
        Kind::Amount(spec) => RowKind::Amount(amount_view(character, builtin.id, &spec, moment)),
        Kind::Group => RowKind::Group(group_view(data, character, builtin, moment)),
    };
    RowView {
        id,
        label: builtin.label.to_string(),
        sub: builtin.sub.map(catalogue::Sub::text),
        account: builtin.account_wide,
        yours: false,
        hidden: character.is_hidden(builtin.id),
        kind,
    }
}

fn own_row(data: &TodoData, character: &CharacterState, todo: &OwnTodo) -> RowView {
    let id = RowId::Own(todo.id);
    let done = data.progress_of(character, id);
    let kind = match todo.kind {
        OwnKind::Tick => RowKind::Tick { done: done >= 1 },
        OwnKind::Count(target) => RowKind::Count { count: done, target, done: done >= target },
    };
    RowView { id, label: todo.name.clone(), sub: None, account: false, yours: true, hidden: false, kind }
}

fn group_view(data: &TodoData, character: &CharacterState, group: &'static Builtin, moment: &Moment) -> GroupView {
    let children: Vec<RowView> =
        catalogue::children_of(group.id).map(|builtin| builtin_row(data, character, builtin, moment)).collect();
    let shown = children.iter().filter(|row| !row.hidden);
    let total_children = shown.clone().count();
    let done_children = shown.filter(|row| row.is_done()).count();
    GroupView {
        open: data.is_group_open(group.id),
        done_children,
        total_children,
        all_done: total_children > 0 && done_children == total_children,
        summary: copy::done_of(done_children, total_children),
        hidden_children: children.len() - total_children,
        children,
    }
}

fn amount_view(character: &CharacterState, id: &str, spec: &AmountSpec, moment: &Moment) -> AmountView {
    let cap = moment.cap_of(spec);
    let continuous = spec.refill.is_continuous();
    let base = AmountView {
        step: spec.step,
        cap,
        tracked: false,
        value: 0,
        full: false,
        full_at_ms: None,
        status_text: None,
        continuous,
    };
    let Some(typed) = character.typed_amount(id) else {
        return base;
    };
    let standing = moment.standing(spec, typed);
    let status_text = match standing.full_at_ms {
        Some(at_ms) => copy::amount_full_in(at_ms - moment.now_ms),
        None if continuous => copy::full_being_wasted(),
        None => {
            let next = super::amount::next_refill(spec, &moment.resets, moment.now_ms);
            copy::full_spend_by(&crate::i18n::day_stamp(moment.resets.zone(), next))
        }
    };
    AmountView {
        tracked: true,
        value: standing.value,
        full: standing.is_full(),
        full_at_ms: standing.full_at_ms,
        status_text: Some(status_text),
        ..base
    }
}

#[cfg(test)]
mod tests {
    use meter_core::identity::KnownCharacter;

    use super::*;
    use crate::clock::testing::{utc, TestZone};
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
        data.set_amount(id, "shugo", Some(24), &now);
        let list = list_of(&data, id, &now);
        let recharges = &list.sections[1];
        assert_eq!((recharges.tracked, recharges.count_text.as_str()), (2, "2 tracked"));
        let odyle = amount(&list, "odyle");
        assert_eq!(odyle.value, 425);
        assert_eq!(odyle.status_text.as_deref(), Some("Full in 1 d 3 h"), "(560 - 425) / 15 = 9 refills of 3 h = 27 h");
        assert_eq!(odyle.value_text(), "425 of 560");
        assert_eq!(amount(&list, "shugo").status_text.as_deref(), Some("Full in 3 h 12 min"), "+4 at 16:00 reaches 28");
        assert_eq!(recharges.when_text, "next full in 3 h 12 min");
    }

    #[test]
    fn a_full_amount_is_wasted_or_names_the_refill_it_will_lose() {
        let zone = summer();
        let (mut data, id) = tracked();
        let now = monday_noon(&zone);
        data.set_amount(id, "odyle", Some(560), &now);
        data.set_amount(id, "shugo", Some(28), &now);
        data.set_amount(id, "balaur", Some(7), &now);
        let list = list_of(&data, id, &now);
        assert_eq!(amount(&list, "odyle").status_text.as_deref(), Some("Full, being wasted"));
        assert_eq!(amount(&list, "shugo").status_text.as_deref(), Some("Full, spend by Mon 16:00"));
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
            Some(EverythingDone { fills_first: Some("Odyle Energy is full in 1 d 3 h.".to_string()) })
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
}
