//! The built-in rows: the to-dos of the to-do window (what resets) and the
//! resources of the Resources window (what refills and how far). Ids are
//! stable (they are what `todos.json` stores), so a label can change and a
//! row can be added without losing anyone's progress. Labels are the game's
//! own names and stay English; only the words around them are translated.

use crate::i18n::fl;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    Today,
    ThisWeek,
}

impl Section {
    pub const ALL: [Section; 2] = [Section::Today, Section::ThisWeek];

    pub fn title(self) -> String {
        match self {
            Self::Today => fl!("todos-section-today"),
            Self::ThisWeek => fl!("todos-section-this-week"),
        }
    }

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::ThisWeek => "thisWeek",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|section| section.key() == key)
    }
}

const HOUR_MS: i64 = 3_600_000;

/// How long before a refill a full amount starts to warn that the refill is about to be lost.
const EVERY_HOURS_WARNING_LEAD_MS: i64 = HOUR_MS;
const DAILY_WARNING_LEAD_MS: i64 = 3 * HOUR_MS;
const WEEKLY_WARNING_LEAD_MS: i64 = 24 * HOUR_MS;

/// When a refilling amount gains its next step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refill {
    /// Every `hours` hours on a clock that starts at the daily reset hour.
    EveryHours { add: u32, hours: u32 },
    /// Once a day at the daily reset hour.
    AtDailyReset { add: u32 },
    /// Once a week at the weekly reset (Wednesday, at the reset hour).
    AtWeeklyReset { add: u32 },
}

impl Refill {
    pub fn add(self) -> u32 {
        match self {
            Self::EveryHours { add, .. } | Self::AtDailyReset { add } | Self::AtWeeklyReset { add } => add,
        }
    }

    pub fn is_continuous(self) -> bool {
        matches!(self, Self::EveryHours { .. })
    }

    /// A full amount warns from this long before its next refill.
    pub fn warning_lead_ms(self) -> i64 {
        match self {
            Self::EveryHours { .. } => EVERY_HOURS_WARNING_LEAD_MS,
            Self::AtDailyReset { .. } => DAILY_WARNING_LEAD_MS,
            Self::AtWeeklyReset { .. } => WEEKLY_WARNING_LEAD_MS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmountSpec {
    pub cap: u32,
    /// The cap with the membership switch on; none when membership changes nothing.
    pub membership_cap: Option<u32>,
    /// How far the extra pool (items and rewards, never refilled) goes; none when the game sets no limit.
    pub extra_cap: Option<u32>,
    /// What one arrow press adds or removes.
    pub step: u32,
    pub refill: Refill,
}

impl AmountSpec {
    pub fn cap_for(&self, membership: bool) -> u32 {
        match self.membership_cap {
            Some(cap) if membership => cap,
            _ => self.cap,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A to-do of the to-do window's `section`, done at `target`; the figure
    /// reads "X/target".
    Count { section: Section, target: u32 },
    /// A resource of the Resources window.
    Amount(AmountSpec),
}

#[derive(Clone, Copy, Debug)]
pub struct Builtin {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    /// Counted by the tracker once for the server's characters, not per character.
    pub account_wide: bool,
    pub hidden_by_default: bool,
    /// The game's ticket this row follows; none for what only the player counts.
    pub ticket: Option<u32>,
}

impl Builtin {
    /// Shown with the "Account" tag: counted for the whole server, by the
    /// tracker or by the game.
    pub fn is_account(&self) -> bool {
        self.account_wide || self.ticket.is_some_and(is_server_ticket)
    }

    /// The to-do window's section; none for a resource.
    pub fn section(&self) -> Option<Section> {
        match self.kind {
            Kind::Count { section, .. } => Some(section),
            Kind::Amount(_) => None,
        }
    }
}

const fn row(id: &'static str, label: &'static str, kind: Kind) -> Builtin {
    Builtin { id, label, kind, account_wide: false, hidden_by_default: false, ticket: None }
}

const fn today(id: &'static str, label: &'static str, target: u32) -> Builtin {
    row(id, label, Kind::Count { section: Section::Today, target })
}

const fn weekly(id: &'static str, label: &'static str, target: u32) -> Builtin {
    row(id, label, Kind::Count { section: Section::ThisWeek, target })
}

const fn backed(ticket: u32, mut builtin: Builtin) -> Builtin {
    builtin.ticket = Some(ticket);
    builtin
}

const fn resource(id: &'static str, label: &'static str, ticket: u32, spec: AmountSpec) -> Builtin {
    backed(ticket, row(id, label, Kind::Amount(spec)))
}

const fn spec(cap: u32, step: u32, refill: Refill) -> AmountSpec {
    AmountSpec { cap, membership_cap: None, extra_cap: None, step, refill }
}

pub const DUTY_MISSIONS: &str = "duty";
pub const ODYLE: &str = "odyle";
pub const SHUGO_KEYS: &str = "shugo";

/// The tickets the game counts for the server, shared by its characters.
const SERVER_TICKETS: [u32; 5] = [1, 8, 15, 101, 60_000_009];

pub fn is_server_ticket(ticket: u32) -> bool {
    SERVER_TICKETS.contains(&ticket)
}

const ODYLE_SPEC: AmountSpec = AmountSpec {
    cap: 560,
    membership_cap: Some(840),
    extra_cap: Some(2000),
    step: 10,
    refill: Refill::EveryHours { add: 15, hours: 3 },
};

const SHUGO_SPEC: AmountSpec = AmountSpec {
    cap: 12,
    membership_cap: Some(21),
    extra_cap: Some(30),
    step: 1,
    refill: Refill::AtDailyReset { add: 3 },
};

const INVASION_SPEC: AmountSpec = AmountSpec { extra_cap: Some(30), ..spec(7, 1, Refill::AtDailyReset { add: 1 }) };
const NIGHTMARE_SPEC: AmountSpec = AmountSpec { extra_cap: Some(30), ..spec(14, 1, Refill::AtDailyReset { add: 2 }) };
const DAILY_DUNGEON_SPEC: AmountSpec = spec(14, 1, Refill::AtWeeklyReset { add: 14 });

pub const CATALOGUE: &[Builtin] = &[
    Builtin { account_wide: true, ..today(DUTY_MISSIONS, "Duty Missions", 5) },
    resource(ODYLE, "Odyle Energy", 60_000_001, ODYLE_SPEC),
    resource(SHUGO_KEYS, "Shugo keys", 1, SHUGO_SPEC),
    resource("invasion", "Invasion keys", 8, INVASION_SPEC),
    resource("nightmare", "Nightmare", 4, NIGHTMARE_SPEC),
    resource("fissure", "Daily Dungeon", 101, DAILY_DUNGEON_SPEC),
    backed(80_000_001, weekly("trial", "Ascension Trial", 3)),
    Builtin { hidden_by_default: true, ..weekly("dutycmd", "Duty Commands", 20) },
];

pub fn builtin(id: &str) -> Option<&'static Builtin> {
    CATALOGUE.iter().find(|builtin| builtin.id == id)
}

/// The ids hidden on a character the tracker has just met.
pub fn hidden_by_default() -> impl Iterator<Item = &'static str> {
    CATALOGUE.iter().filter(|builtin| builtin.hidden_by_default).map(|builtin| builtin.id)
}

/// The rows that follow this ticket.
pub fn rows_of_ticket(ticket: u32) -> impl Iterator<Item = &'static Builtin> {
    CATALOGUE.iter().filter(move |builtin| builtin.ticket == Some(ticket))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for builtin in CATALOGUE {
            assert!(seen.insert(builtin.id), "duplicate id {}", builtin.id);
        }
    }

    #[test]
    fn only_duty_commands_start_hidden() {
        assert_eq!(hidden_by_default().collect::<Vec<_>>(), vec!["dutycmd"]);
    }

    #[test]
    fn the_catalogue_lists_the_to_dos_today_then_this_week() {
        let sections: Vec<Section> = CATALOGUE.iter().filter_map(Builtin::section).collect();
        assert!(sections.windows(2).all(|pair| pair[0] <= pair[1]), "to-dos are listed Today, then This week");
    }

    #[test]
    fn membership_changes_the_odyle_and_shugo_caps() {
        let capped: Vec<&str> = CATALOGUE
            .iter()
            .filter(|builtin| matches!(builtin.kind, Kind::Amount(spec) if spec.membership_cap.is_some()))
            .map(|builtin| builtin.id)
            .collect();
        assert_eq!(capped, vec![ODYLE, "shugo"]);
        assert_eq!((ODYLE_SPEC.cap_for(false), ODYLE_SPEC.cap_for(true)), (560, 840));
        assert_eq!((SHUGO_SPEC.cap_for(false), SHUGO_SPEC.cap_for(true)), (12, 21));
    }

    #[test]
    fn a_ticket_backs_one_row_and_the_server_ones_show_the_account_tag() {
        let mut seen = HashSet::new();
        for ticket in CATALOGUE.iter().filter_map(|builtin| builtin.ticket) {
            assert!(seen.insert(ticket), "ticket {ticket} backs two rows");
        }
        let account: Vec<&str> = CATALOGUE.iter().filter(|builtin| builtin.is_account()).map(|builtin| builtin.id).collect();
        assert_eq!(account, vec![DUTY_MISSIONS, "shugo", "invasion", "fissure"]);
        assert_eq!(rows_of_ticket(1).map(|builtin| builtin.id).collect::<Vec<_>>(), vec!["shugo"]);
        assert_eq!(rows_of_ticket(5).count(), 0);
    }
}
