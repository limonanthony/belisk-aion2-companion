//! When a record was saved, as the approved history page writes it: "Today
//! 21:42", "Yesterday 22:07", the weekday within the last week ("Mon 19:12"),
//! then the date ("3 Oct 20:42"), in the computer's time zone.

use std::time::{SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

use crate::i18n::{fl, month_short, weekday_short};

/// Milliseconds from 1601-01-01 (the FILETIME epoch) to 1970-01-01.
const EPOCH_GAP_MS: u64 = 11_644_473_600_000;

/// A record's time against the current day, or the unknown-time mark when
/// Windows cannot convert it.
pub fn record_time(unix_ms: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64);
    match (local_time(unix_ms), local_time(now)) {
        (Some(saved), Some(today)) => label(saved, today.date()),
        _ => fl!("common-unknown-time"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LocalTime {
    year: i32,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
}

impl LocalTime {
    fn date(self) -> (i32, u16, u16) {
        (self.year, self.month, self.day)
    }
}

fn label(saved: LocalTime, today: (i32, u16, u16)) -> String {
    let time = format!("{:02}:{:02}", saved.hour, saved.minute);
    let saved_day = days_from_civil(saved.date());
    match days_from_civil(today) - saved_day {
        0 => fl!("history-when-today", time = time),
        1 => fl!("history-when-yesterday", time = time),
        2..=6 => fl!("history-when-weekday", weekday = weekday_short(weekday(saved_day)), time = time),
        _ => fl!(
            "history-when-date",
            day = i64::from(saved.day),
            month = month_short(saved.month.clamp(1, 12) as u8),
            time = time
        ),
    }
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil((year, month, day): (i32, u16, u16)) -> i64 {
    let (month, day) = (i64::from(month), i64::from(day));
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// 0 for Monday; 1970-01-01 was a Thursday.
fn weekday(days: i64) -> usize {
    (days + 3).rem_euclid(7) as usize
}

fn local_time(unix_ms: u64) -> Option<LocalTime> {
    let ticks = (unix_ms + EPOCH_GAP_MS) * 10_000;
    let file_time = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    unsafe {
        FileTimeToSystemTime(&file_time, &mut utc).ok()?;
        SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).ok()?;
    }
    Some(LocalTime {
        year: i32::from(local.wYear),
        month: local.wMonth,
        day: local.wDay,
        hour: local.wHour,
        minute: local.wMinute,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i32, month: u16, day: u16, hour: u16, minute: u16) -> LocalTime {
        LocalTime { year, month, day, hour, minute }
    }

    /// Saturday 4 October 2025.
    const TODAY: (i32, u16, u16) = (2025, 10, 4);

    #[test]
    fn today_runs_from_midnight_and_yesterday_ends_at_it() {
        assert_eq!(label(at(2025, 10, 4, 0, 0), TODAY), "Today 00:00");
        assert_eq!(label(at(2025, 10, 4, 23, 59), TODAY), "Today 23:59");
        assert_eq!(label(at(2025, 10, 3, 23, 59), TODAY), "Yesterday 23:59");
        assert_eq!(label(at(2025, 10, 3, 0, 0), TODAY), "Yesterday 00:00");
    }

    #[test]
    fn the_rest_of_the_week_shows_the_weekday_then_the_date() {
        assert_eq!(label(at(2025, 10, 2, 21, 5), TODAY), "Thu 21:05");
        assert_eq!(label(at(2025, 9, 28, 19, 12), TODAY), "Sun 19:12");
        assert_eq!(label(at(2025, 9, 27, 20, 42), TODAY), "27 Sep 20:42", "seven days back is the same weekday as today");
        assert_eq!(label(at(2024, 10, 3, 20, 42), TODAY), "3 Oct 20:42");
    }

    #[test]
    fn days_cross_month_and_year_ends() {
        assert_eq!(label(at(2025, 12, 31, 22, 0), (2026, 1, 1)), "Yesterday 22:00");
        assert_eq!(label(at(2024, 2, 29, 8, 0), (2024, 3, 1)), "Yesterday 08:00");
        assert_eq!(days_from_civil((1970, 1, 1)), 0);
        assert_eq!(weekday(days_from_civil(TODAY)), 5);
    }

    #[test]
    fn a_saved_time_after_now_shows_its_date() {
        assert_eq!(label(at(2025, 10, 5, 9, 30), TODAY), "5 Oct 09:30");
    }

    #[test]
    fn a_recent_timestamp_converts_to_some_local_time() {
        assert_ne!(record_time(1_791_052_949_892), "--");
    }
}
