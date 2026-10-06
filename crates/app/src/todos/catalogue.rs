//! The built-in to-dos: what resets, what refills and how far. Ids are stable
//! (they are what `todos.json` stores), so a label can change and a row can be
//! added without losing anyone's progress. Labels are the game's own names and
//! stay English; only the words around them are translated.

use crate::i18n::fl;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    Today,
    Recharges,
    ThisWeek,
}

impl Section {
    pub const ALL: [Section; 3] = [Section::Today, Section::Recharges, Section::ThisWeek];

    pub fn title(self) -> String {
        match self {
            Self::Today => fl!("todos-section-today"),
            Self::Recharges => fl!("todos-section-recharges"),
            Self::ThisWeek => fl!("todos-section-this-week"),
        }
    }

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Recharges => "recharges",
            Self::ThisWeek => "thisWeek",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|section| section.key() == key)
    }
}

/// When a refilling amount gains its next step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refill {
    /// Every `hours` real hours, counted from the moment the amount was typed.
    EveryHours { add: u32, hours: u32 },
    /// Once a day at a fixed hour of the PC's clock.
    DailyAt { add: u32, hour: u8 },
    /// Once a day at the daily reset hour.
    AtDailyReset { add: u32 },
}

impl Refill {
    pub fn add(self) -> u32 {
        match self {
            Self::EveryHours { add, .. } | Self::DailyAt { add, .. } | Self::AtDailyReset { add } => add,
        }
    }

    pub fn is_continuous(self) -> bool {
        matches!(self, Self::EveryHours { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmountSpec {
    pub cap: u32,
    /// The cap with the membership switch on; none when membership changes nothing.
    pub membership_cap: Option<u32>,
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
    Tick,
    /// Done at `target`; the figure reads "X/target".
    Count { target: u32 },
    Amount(AmountSpec),
    /// Opens to show the built-ins whose `group` is its id.
    Group,
}

/// The small line under a row's label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    /// A game term, shown as it is.
    Game(&'static str),
    Attempts,
    FinalBoss,
    FinalBossKills,
    RunsPerDungeon,
}

impl Sub {
    pub fn text(self) -> String {
        match self {
            Self::Game(term) => term.to_string(),
            Self::Attempts => fl!("todos-sub-attempts"),
            Self::FinalBoss => fl!("todos-sub-final-boss"),
            Self::FinalBossKills => fl!("todos-sub-final-boss-kills"),
            Self::RunsPerDungeon => fl!("todos-sub-runs-per-dungeon"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Builtin {
    pub id: &'static str,
    pub section: Section,
    pub label: &'static str,
    pub sub: Option<Sub>,
    pub kind: Kind,
    /// The id of the group this row sits in.
    pub group: Option<&'static str>,
    /// Shared by the characters of one server (the game counts it per account).
    pub account_wide: bool,
    pub hidden_by_default: bool,
}

const fn row(id: &'static str, section: Section, label: &'static str, sub: Option<Sub>, kind: Kind) -> Builtin {
    Builtin { id, section, label, sub, kind, group: None, account_wide: false, hidden_by_default: false }
}

const fn today(id: &'static str, label: &'static str, target: u32) -> Builtin {
    row(id, Section::Today, label, None, Kind::Count { target })
}

const fn weekly(id: &'static str, label: &'static str, sub: Option<Sub>, target: u32) -> Builtin {
    row(id, Section::ThisWeek, label, sub, Kind::Count { target })
}

const fn in_group(group: &'static str, mut builtin: Builtin) -> Builtin {
    builtin.group = Some(group);
    builtin
}

const fn recharge(id: &'static str, label: &'static str, spec: AmountSpec) -> Builtin {
    row(id, Section::Recharges, label, None, Kind::Amount(spec))
}

const fn spec(cap: u32, step: u32, refill: Refill) -> AmountSpec {
    AmountSpec { cap, membership_cap: None, step, refill }
}

pub const DUTY_MISSIONS: &str = "duty";
pub const ODYLE: &str = "odyle";

const ODYLE_SPEC: AmountSpec = AmountSpec {
    cap: 560,
    membership_cap: Some(840),
    step: 10,
    refill: Refill::EveryHours { add: 15, hours: 3 },
};

const GROUP_EXPLORATION: &str = "explore";
const GROUP_SANCTUARY: &str = "sanct";

pub const CATALOGUE: &[Builtin] = &[
    Builtin { account_wide: true, ..today(DUTY_MISSIONS, "Duty Missions", 5) },
    recharge(ODYLE, "Odyle Energy", ODYLE_SPEC),
    recharge("shugo", "Shugo Festival keys", spec(28, 1, Refill::DailyAt { add: 4, hour: 16 })),
    recharge("invasion", "Invasion keys", spec(7, 1, Refill::DailyAt { add: 1, hour: 16 })),
    recharge("nightmare", "Nightmare", spec(14, 1, Refill::AtDailyReset { add: 2 })),
    recharge("conquest", "Conquest dungeons", spec(14, 1, Refill::AtDailyReset { add: 2 })),
    recharge("deus", "Deus Research Base", spec(14, 1, Refill::AtDailyReset { add: 2 })),
    recharge("arkanis", "Shattered Arkanis", spec(14, 1, Refill::AtDailyReset { add: 2 })),
    recharge("balaur", "Abandoned Balaur Fortress", spec(7, 1, Refill::AtDailyReset { add: 1 })),
    recharge("fafnium", "Fafnium Laboratory", spec(7, 1, Refill::AtDailyReset { add: 1 })),
    weekly("fissure", "Daily Dungeon", Some(Sub::Game("Unknown Fissure")), 14),
    weekly("trial", "Ascension Trial", None, 3),
    weekly("subj", "Subjugation", None, 3),
    row(GROUP_SANCTUARY, Section::ThisWeek, "Sanctuary", None, Kind::Group),
    in_group(GROUP_SANCTUARY, weekly("forge", "Abyssal Forge", Some(Sub::Attempts), 4)),
    in_group(
        GROUP_SANCTUARY,
        row("forgeboss", Section::ThisWeek, "Abyssal Forge", Some(Sub::FinalBoss), Kind::Tick),
    ),
    in_group(GROUP_SANCTUARY, weekly("corr", "Corroded Facility", Some(Sub::Attempts), 4)),
    in_group(GROUP_SANCTUARY, weekly("corrboss", "Corroded Facility", Some(Sub::FinalBoss), 2)),
    in_group(GROUP_SANCTUARY, weekly("chal", "Chalice of Muspel", Some(Sub::Attempts), 4)),
    in_group(GROUP_SANCTUARY, weekly("chalboss", "Chalice of Muspel", Some(Sub::FinalBoss), 2)),
    weekly("transc", "Transcendence", Some(Sub::FinalBossKills), 28),
    weekly("conqk", "Conquest", Some(Sub::FinalBossKills), 35),
    weekly("cradle", "Cradle of Nihility", Some(Sub::Game("Conquest")), 7),
    weekly("dramata", "Dying Dramata's Nest", Some(Sub::Game("Conquest")), 7),
    weekly("ordeal", "Ordeal", Some(Sub::FinalBossKills), 7),
    row(GROUP_EXPLORATION, Section::ThisWeek, "Exploration", Some(Sub::RunsPerDungeon), Kind::Group),
    in_group(GROUP_EXPLORATION, weekly("x-azure", "Azure Breath Island", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-cradle", "Cradle of Nihility", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-draupnir", "Draupnir", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-dramata", "Dying Dramata's Nest", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-horn", "Ferocious Horn Den", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-fire", "Fire Temple", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-illusion", "Hall of Illusion", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-krao", "Krao Cave", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-urugugu", "Urugugu Canyon", None, 7)),
    in_group(GROUP_EXPLORATION, weekly("x-vakron", "Vakron Sky Island", None, 7)),
    Builtin { hidden_by_default: true, ..weekly("dutycmd", "Duty Commands", None, 20) },
];

pub fn builtin(id: &str) -> Option<&'static Builtin> {
    CATALOGUE.iter().find(|builtin| builtin.id == id)
}

/// The ids hidden on a character the tracker has just met.
pub fn hidden_by_default() -> impl Iterator<Item = &'static str> {
    CATALOGUE.iter().filter(|builtin| builtin.hidden_by_default).map(|builtin| builtin.id)
}

pub fn children_of(group: &'static str) -> impl Iterator<Item = &'static Builtin> {
    CATALOGUE.iter().filter(move |builtin| builtin.group == Some(group))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn ids_are_unique_and_groups_hold_rows_of_their_own_section() {
        let mut seen = HashSet::new();
        for builtin in CATALOGUE {
            assert!(seen.insert(builtin.id), "duplicate id {}", builtin.id);
            if let Some(group) = builtin.group {
                let parent = super::builtin(group).expect("a group row exists");
                assert_eq!(parent.kind, Kind::Group);
                assert_eq!(parent.section, builtin.section);
            }
        }
    }

    #[test]
    fn only_duty_commands_start_hidden() {
        assert_eq!(hidden_by_default().collect::<Vec<_>>(), vec!["dutycmd"]);
    }

    #[test]
    fn the_catalogue_lists_every_section_in_order_with_groups_before_their_rows() {
        let sections: Vec<Section> = CATALOGUE.iter().map(|builtin| builtin.section).collect();
        assert!(sections.windows(2).all(|pair| pair[0] <= pair[1]), "sections are listed Today, Recharges, This week");
        for builtin in CATALOGUE.iter().filter(|builtin| builtin.group.is_some()) {
            let group_at = CATALOGUE.iter().position(|row| Some(row.id) == builtin.group).unwrap();
            let row_at = CATALOGUE.iter().position(|row| row.id == builtin.id).unwrap();
            assert!(group_at < row_at);
        }
    }

    #[test]
    fn membership_changes_only_the_odyle_cap() {
        let capped: Vec<&str> = CATALOGUE
            .iter()
            .filter(|builtin| matches!(builtin.kind, Kind::Amount(spec) if spec.membership_cap.is_some()))
            .map(|builtin| builtin.id)
            .collect();
        assert_eq!(capped, vec![ODYLE]);
        assert_eq!(ODYLE_SPEC.cap_for(false), 560);
        assert_eq!(ODYLE_SPEC.cap_for(true), 840);
    }
}
