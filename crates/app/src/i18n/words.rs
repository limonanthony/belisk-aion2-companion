//! Words every area needs in the player's language: durations, weekdays,
//! months, lists.

use super::fl;
use crate::clock::{clock_time, wall_minute, Zone, MINUTE_MS};

const DAY_MINUTES: i64 = 1440;

/// "X d Y h" from a day on, "X h Y min" from an hour, else "X min"
/// (French: "X j Y h"). Rounded up to the minute, so a countdown reads
/// "1 min" until it is due.
pub fn duration_words(ms: i64) -> String {
    let minutes = (ms.max(0) + MINUTE_MS - 1) / MINUTE_MS;
    let (days, hours, minutes_left) = (minutes / DAY_MINUTES, minutes % DAY_MINUTES / 60, minutes % 60);
    if days > 0 {
        fl!("duration-days-hours", days = days, hours = hours)
    } else if hours > 0 {
        fl!("duration-hours-minutes", hours = hours, minutes = minutes_left)
    } else {
        fl!("duration-minutes", minutes = minutes_left)
    }
}

/// "Mon" or "lun.": `days_from_monday` is 0 for Monday to 6 for Sunday.
pub fn weekday_short(days_from_monday: usize) -> String {
    match days_from_monday {
        0 => fl!("weekday-short-mon"),
        1 => fl!("weekday-short-tue"),
        2 => fl!("weekday-short-wed"),
        3 => fl!("weekday-short-thu"),
        4 => fl!("weekday-short-fri"),
        5 => fl!("weekday-short-sat"),
        _ => fl!("weekday-short-sun"),
    }
}

/// "Oct" or "oct.": `month` is 1 for January to 12 for December.
pub fn month_short(month: u8) -> String {
    match month {
        1 => fl!("month-short-1"),
        2 => fl!("month-short-2"),
        3 => fl!("month-short-3"),
        4 => fl!("month-short-4"),
        5 => fl!("month-short-5"),
        6 => fl!("month-short-6"),
        7 => fl!("month-short-7"),
        8 => fl!("month-short-8"),
        9 => fl!("month-short-9"),
        10 => fl!("month-short-10"),
        11 => fl!("month-short-11"),
        _ => fl!("month-short-12"),
    }
}

/// "Tue 16:00" (French: "mar. 16:00") on the local clock.
pub fn day_stamp(zone: &dyn Zone, unix_ms: i64) -> String {
    let days_from_monday = (wall_minute(zone, unix_ms).div_euclid(DAY_MINUTES) + 3).rem_euclid(7) as usize;
    fl!("day-stamp", weekday = weekday_short(days_from_monday), time = clock_time(zone, unix_ms).to_string())
}

/// "A", "A and B", "A, B and C".
pub fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => fl!("list-and", rest = rest.join(", "), last = last.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::testing::TestZone;
    use crate::i18n::{set_language, Language};

    const MINUTE: i64 = MINUTE_MS;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;

    #[test]
    fn durations_round_up_to_the_minute_in_both_languages() {
        let cases = [
            (1, "1 min", "1 min"),
            (59 * MINUTE, "59 min", "59 min"),
            (HOUR, "1 h 0 min", "1 h 0 min"),
            (3 * HOUR + 12 * MINUTE, "3 h 12 min", "3 h 12 min"),
            (DAY + 3 * HOUR, "1 d 3 h", "1 j 3 h"),
            (11 * HOUR + 12 * MINUTE, "11 h 12 min", "11 h 12 min"),
            (2 * DAY + 16 * HOUR + 59 * MINUTE, "2 d 16 h", "2 j 16 h"),
            (0, "0 min", "0 min"),
            (-5, "0 min", "0 min"),
        ];
        for (ms, english, french) in cases {
            set_language(Language::English);
            assert_eq!(duration_words(ms), english);
            set_language(Language::French);
            assert_eq!(duration_words(ms), french);
        }
    }

    #[test]
    fn a_stamp_names_the_weekday_of_the_local_day() {
        let zone = TestZone { base: 0, changes: Vec::new() };
        let stamp = |ms| day_stamp(&zone, ms);
        assert_eq!(stamp(16 * HOUR), "Thu 16:00", "1970-01-01 was a Thursday");
        assert_eq!(stamp(4 * DAY + 9 * HOUR), "Mon 09:00");
        set_language(Language::French);
        assert_eq!(stamp(16 * HOUR), "jeu. 16:00");
        assert_eq!(stamp(4 * DAY + 9 * HOUR), "lun. 09:00");
        assert_eq!(stamp(-DAY + 9 * HOUR), "mer. 09:00");
    }

    #[test]
    fn every_weekday_and_month_has_a_distinct_name() {
        for language in Language::ALL {
            set_language(language);
            let weekdays: Vec<String> = (0..7).map(weekday_short).collect();
            let months: Vec<String> = (1..=12).map(month_short).collect();
            for names in [weekdays, months] {
                assert!(names.iter().all(|name| !name.is_empty()), "{language:?}");
                let distinct: std::collections::HashSet<_> = names.iter().collect();
                assert_eq!(distinct.len(), names.len(), "{language:?}: {names:?}");
            }
        }
    }

    #[test]
    fn lists_join_with_the_languages_and() {
        let names = |items: &[&str]| items.iter().map(|item| item.to_string()).collect::<Vec<_>>();
        assert_eq!(join_and(&[]), "");
        assert_eq!(join_and(&names(&["A"])), "A");
        assert_eq!(join_and(&names(&["A", "B", "C"])), "A, B and C");
        set_language(Language::French);
        assert_eq!(join_and(&names(&["A", "B", "C"])), "A, B et C");
    }
}
