//! The timed events a reminder can announce, as data. Times are the
//! game's schedule tables as MetaBot reads them (client build 0.0.4387.0,
//! `https://metabot.gg/en/aion-2/events`), in server time. The EU servers run on
//! Berlin time, which is the clock of an EU player's PC, so the rules are read
//! on the PC's local clock.

use crate::i18n::fl;

/// Day of the week, Monday first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// The weekday of a day counted in days since 1970-01-01 (a Thursday).
    pub fn of_day(days_since_epoch: i64) -> Self {
        match (days_since_epoch + 3).rem_euclid(7) {
            0 => Self::Mon,
            1 => Self::Tue,
            2 => Self::Wed,
            3 => Self::Thu,
            4 => Self::Fri,
            5 => Self::Sat,
            _ => Self::Sun,
        }
    }
}

/// When an event starts. Every rule is a set of local wall-clock times.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Every hour at `minute` past.
    Hourly { minute: u8 },
    /// On the hour, every `hours` hours counted from `from_hour` each day
    /// (3 from 2 gives 02, 05, 08, 11, 14, 17, 20, 23).
    EveryHours { hours: u8, from_hour: u8 },
    /// On these weekdays at `hour`:`minute`.
    Weekly { days: &'static [Weekday], hour: u8, minute: u8 },
    /// Every day at each of these `(hour, minute)` times, in ascending order.
    Daily { times: &'static [(u8, u8)] },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventId {
    ShugoFestival,
    DimensionalInvasion,
    RiftPortals,
    WatcherKaira,
    ArtifactSiege,
    AbyssFieldBosses,
    GuardianLordNahma,
    ArenaOfTactics,
    RewardKeys,
}

pub struct EventInfo {
    pub id: EventId,
    pub rule: Rule,
}

/// Every event, in the order Settings lists them.
pub const EVENTS: [EventInfo; 9] = [
    EventInfo { id: EventId::ShugoFestival, rule: Rule::Hourly { minute: 0 } },
    EventInfo { id: EventId::DimensionalInvasion, rule: Rule::Hourly { minute: 30 } },
    EventInfo { id: EventId::RiftPortals, rule: Rule::EveryHours { hours: 3, from_hour: 2 } },
    EventInfo { id: EventId::WatcherKaira, rule: Rule::EveryHours { hours: 3, from_hour: 1 } },
    EventInfo {
        id: EventId::ArtifactSiege,
        rule: Rule::Weekly { days: &[Weekday::Mon, Weekday::Thu, Weekday::Sat], hour: 21, minute: 0 },
    },
    EventInfo {
        id: EventId::AbyssFieldBosses,
        rule: Rule::Weekly { days: &[Weekday::Mon, Weekday::Thu, Weekday::Sat], hour: 21, minute: 30 },
    },
    EventInfo {
        id: EventId::GuardianLordNahma,
        rule: Rule::Weekly { days: &[Weekday::Fri, Weekday::Sun], hour: 21, minute: 0 },
    },
    EventInfo { id: EventId::ArenaOfTactics, rule: Rule::Daily { times: &[(11, 0), (19, 0)] } },
    EventInfo { id: EventId::RewardKeys, rule: Rule::Daily { times: &[(16, 0)] } },
];

impl EventId {
    pub fn info(self) -> &'static EventInfo {
        &EVENTS[self as usize]
    }

    pub fn name(self) -> String {
        match self {
            Self::ShugoFestival => fl!("reminder-event-shugo-festival"),
            Self::DimensionalInvasion => fl!("reminder-event-dimensional-invasion"),
            Self::RiftPortals => fl!("reminder-event-rift-portals"),
            Self::WatcherKaira => fl!("reminder-event-watcher-kaira"),
            Self::ArtifactSiege => fl!("reminder-event-artifact-siege"),
            Self::AbyssFieldBosses => fl!("reminder-event-abyss-field-bosses"),
            Self::GuardianLordNahma => fl!("reminder-event-guardian-lord-nahma"),
            Self::ArenaOfTactics => fl!("reminder-event-arena-of-tactics"),
            Self::RewardKeys => fl!("reminder-event-reward-keys"),
        }
    }
}

impl Rule {
    /// The first start at or after `wall`, in minutes since 1970-01-01 00:00
    /// on the local wall clock. An hourly rule has none: it follows the real
    /// hour, which a wall time cannot name on a daylight-saving changeover
    /// night.
    pub fn first_at_or_after(self, wall: i64) -> Option<i64> {
        const DAY: i64 = 1440;
        let day_start = wall.div_euclid(DAY) * DAY;
        let minute_of_day = wall - day_start;
        match self {
            Rule::Hourly { .. } => None,
            Rule::EveryHours { hours, from_hour } => {
                let (hours, from_hour) = (i64::from(hours), i64::from(from_hour));
                let slot = |index: i64| from_hour * 60 + index * hours * 60;
                let index = (minute_of_day - from_hour * 60).div_euclid(hours * 60);
                let candidates = [index, index + 1].map(slot);
                match candidates.into_iter().find(|&at| at >= minute_of_day && at < DAY) {
                    Some(at) => Some(day_start + at),
                    None => Some(day_start + DAY + from_hour * 60),
                }
            }
            Rule::Weekly { days, hour, minute } => {
                let at = i64::from(hour) * 60 + i64::from(minute);
                let today = day_start.div_euclid(DAY);
                (0..=7)
                    .map(|ahead| today + ahead)
                    .filter(|&day| days.contains(&Weekday::of_day(day)))
                    .map(|day| day * DAY + at)
                    .find(|&start| start >= wall)
            }
            Rule::Daily { times } => {
                let today = times.iter().map(|&(hour, minute)| i64::from(hour) * 60 + i64::from(minute));
                match today.clone().find(|&at| at >= minute_of_day) {
                    Some(at) => Some(day_start + at),
                    None => today.min().map(|first| day_start + DAY + first),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-05 is a Monday: 20_731 days after the epoch.
    const MONDAY: i64 = 20_731 * 1440;

    fn at(day: i64, hour: i64, minute: i64) -> i64 {
        (MONDAY / 1440 + day) * 1440 + hour * 60 + minute
    }

    fn first(rule: Rule, wall: i64) -> i64 {
        rule.first_at_or_after(wall).expect("a wall-clock rule")
    }

    #[test]
    fn the_catalogue_lists_every_event_once_and_finds_it_by_id() {
        for (index, info) in EVENTS.iter().enumerate() {
            assert_eq!(info.id as usize, index, "EVENTS is in EventId declaration order");
            assert!(!info.id.name().is_empty());
            assert_eq!(EVENTS.iter().filter(|other| other.id == info.id).count(), 1, "row {index}");
        }
    }

    #[test]
    fn an_hourly_rule_has_no_wall_time_because_it_follows_the_real_hour() {
        assert_eq!(Rule::Hourly { minute: 30 }.first_at_or_after(at(0, 10, 0)), None);
    }

    #[test]
    fn the_three_hour_rules_follow_their_anchor_each_day() {
        let rifts = Rule::EveryHours { hours: 3, from_hour: 2 };
        let hours: Vec<i64> = (0..8).map(|i| (first(rifts, at(0, 2 + i * 3 - 1, 1)) - at(0, 0, 0)) / 60).collect();
        assert_eq!(hours, vec![2, 5, 8, 11, 14, 17, 20, 23]);
        assert_eq!(first(rifts, at(0, 23, 1)), at(1, 2, 0));
        assert_eq!(first(rifts, at(0, 0, 0)), at(0, 2, 0));
        assert_eq!(first(rifts, at(0, 2, 0)), at(0, 2, 0));

        let kaira = Rule::EveryHours { hours: 3, from_hour: 1 };
        let hours: Vec<i64> = (0..8).map(|i| (first(kaira, at(0, i * 3, 1)) - at(0, 0, 0)) / 60).collect();
        assert_eq!(hours, vec![1, 4, 7, 10, 13, 16, 19, 22]);
        assert_eq!(first(kaira, at(0, 22, 1)), at(1, 1, 0));
    }

    #[test]
    fn a_weekly_rule_waits_for_its_weekday_and_wraps_the_week() {
        let siege = Rule::Weekly { days: &[Weekday::Mon, Weekday::Thu, Weekday::Sat], hour: 21, minute: 0 };
        assert_eq!(first(siege, at(0, 20, 0)), at(0, 21, 0), "Monday before the start");
        assert_eq!(first(siege, at(0, 21, 1)), at(3, 21, 0), "Monday after it: Thursday");
        assert_eq!(first(siege, at(5, 21, 1)), at(7, 21, 0), "Saturday after it: next Monday");
        assert_eq!(first(siege, at(6, 0, 0)), at(7, 21, 0), "Sunday");

        let nahma = Rule::Weekly { days: &[Weekday::Fri, Weekday::Sun], hour: 21, minute: 0 };
        assert_eq!(first(nahma, at(0, 0, 0)), at(4, 21, 0));
        assert_eq!(first(nahma, at(4, 21, 1)), at(6, 21, 0));
        assert_eq!(first(nahma, at(6, 21, 1)), at(11, 21, 0));
    }

    #[test]
    fn a_daily_rule_picks_the_next_listed_time_or_tomorrows_first() {
        let arena = Rule::Daily { times: &[(11, 0), (19, 0)] };
        assert_eq!(first(arena, at(0, 0, 0)), at(0, 11, 0));
        assert_eq!(first(arena, at(0, 11, 1)), at(0, 19, 0));
        assert_eq!(first(arena, at(0, 19, 1)), at(1, 11, 0));
    }
}
