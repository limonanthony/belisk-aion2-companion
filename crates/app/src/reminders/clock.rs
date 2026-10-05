//! The PC's time zone, read from Windows, and the current time.

use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::System::Time::{
    FileTimeToSystemTime, GetDynamicTimeZoneInformation, SystemTimeToTzSpecificLocalTimeEx, DYNAMIC_TIME_ZONE_INFORMATION,
};

use super::schedule::Zone;

/// Milliseconds from 1601-01-01 (the FILETIME epoch) to 1970-01-01.
const EPOCH_GAP_MS: i64 = 11_644_473_600_000;

pub struct LocalZone;

impl Zone for LocalZone {
    fn utc_offset_minutes(&self, unix_ms: i64) -> i32 {
        current_zone()
            .and_then(|zone| offset_minutes(&zone, unix_ms))
            .unwrap_or_else(|error| {
                static REPORTED: Once = Once::new();
                REPORTED.call_once(|| eprintln!("cannot read the PC's time zone, reminders assume UTC: {error}"));
                0
            })
    }
}

/// Unix milliseconds now; 0 if the clock reads before 1970.
pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as i64)
}

const TIME_ZONE_ID_INVALID: u32 = 0xFFFF_FFFF;

fn current_zone() -> Result<DYNAMIC_TIME_ZONE_INFORMATION, String> {
    let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
    if unsafe { GetDynamicTimeZoneInformation(&mut zone) } == TIME_ZONE_ID_INVALID {
        return Err(format!("GetDynamicTimeZoneInformation: {}", windows::core::Error::from_win32()));
    }
    Ok(zone)
}

fn offset_minutes(zone: &DYNAMIC_TIME_ZONE_INFORMATION, unix_ms: i64) -> Result<i32, String> {
    let ticks = (unix_ms + EPOCH_GAP_MS) * 10_000;
    let file_time = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    unsafe {
        FileTimeToSystemTime(&file_time, &mut utc).map_err(|error| format!("FileTimeToSystemTime: {error}"))?;
        SystemTimeToTzSpecificLocalTimeEx(Some(zone), &utc, &mut local)
            .map_err(|error| format!("SystemTimeToTzSpecificLocalTimeEx: {error}"))?;
    }
    let date = |time: &SYSTEMTIME| (time.wYear, time.wMonth, time.wDay);
    let day_shift = match date(&local).cmp(&date(&utc)) {
        std::cmp::Ordering::Less => -1440,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1440,
    };
    let minute_of_day = |time: &SYSTEMTIME| i32::from(time.wHour) * 60 + i32::from(time.wMinute);
    Ok(minute_of_day(&local) - minute_of_day(&utc) + day_shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_of_this_pc_is_a_real_zone_offset_and_the_clock_is_after_2025() {
        let now = now_ms();
        assert!(now > 1_735_689_600_000, "the clock reads after 2025-01-01");
        let offset = LocalZone.utc_offset_minutes(now);
        assert!((-12 * 60..=14 * 60).contains(&offset), "{offset}");
    }

    #[test]
    fn a_whole_year_of_offsets_changes_by_at_most_two_hours_at_a_time() {
        let start = now_ms();
        let offsets: Vec<i32> = (0..366).map(|day| LocalZone.utc_offset_minutes(start + day * 86_400_000)).collect();
        assert!(offsets.windows(2).all(|pair| (pair[1] - pair[0]).abs() <= 120), "{offsets:?}");
    }

    fn installed_zone(key_name: &str) -> DYNAMIC_TIME_ZONE_INFORMATION {
        use windows::Win32::System::Time::EnumDynamicTimeZoneInformation;
        let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
        for index in 0.. {
            if unsafe { EnumDynamicTimeZoneInformation(index, &mut zone) } != 0 {
                break;
            }
            let name_len = zone.TimeZoneKeyName.iter().position(|&unit| unit == 0).unwrap_or(zone.TimeZoneKeyName.len());
            if String::from_utf16_lossy(&zone.TimeZoneKeyName[..name_len]) == key_name {
                return zone;
            }
        }
        panic!("Windows has no time zone {key_name}");
    }

    const JAN_1_2025_UTC_MS: i64 = 1_735_689_600_000;
    const JUL_1_2025_UTC_MS: i64 = 1_751_328_000_000;
    const DEC_31_2025_23_UTC_MS: i64 = 1_767_222_000_000;

    #[test]
    fn sydney_is_utc_plus_11_in_january_and_plus_10_in_july_whatever_this_pc_uses() {
        let sydney = installed_zone("AUS Eastern Standard Time");
        assert_eq!(offset_minutes(&sydney, JAN_1_2025_UTC_MS), Ok(660));
        assert_eq!(offset_minutes(&sydney, JUL_1_2025_UTC_MS), Ok(600));
        assert_eq!(offset_minutes(&sydney, DEC_31_2025_23_UTC_MS), Ok(660));
    }

    #[test]
    fn berlin_is_utc_plus_1_in_january_and_plus_2_in_july_whatever_this_pc_uses() {
        let berlin = installed_zone("W. Europe Standard Time");
        assert_eq!(offset_minutes(&berlin, JAN_1_2025_UTC_MS), Ok(60));
        assert_eq!(offset_minutes(&berlin, JUL_1_2025_UTC_MS), Ok(120));
    }
}
