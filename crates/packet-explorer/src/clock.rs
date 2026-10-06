use std::str::FromStr;

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

pub fn local_datetime(unix_secs: f64) -> DateTime<Local> {
    let millis = (unix_secs * 1000.0).round() as i64;
    Utc.timestamp_millis_opt(millis)
        .single()
        .unwrap_or_default()
        .with_timezone(&Local)
}

pub fn format_full(unix_secs: f64) -> String {
    local_datetime(unix_secs).format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

pub fn format_with_weekday(unix_secs: f64) -> String {
    local_datetime(unix_secs).format("%a %Y-%m-%d %H:%M:%S").to_string()
}

pub fn format_short(unix_secs: f64) -> String {
    local_datetime(unix_secs).format("%m-%d %H:%M:%S").to_string()
}

pub fn format_clock(secs_of_day: u32) -> String {
    format!("{:02}:{:02}:{:02}", secs_of_day / 3600, secs_of_day / 60 % 60, secs_of_day % 60)
}

pub fn format_span(secs: f64) -> String {
    let sign = if secs < 0.0 { "-" } else { "+" };
    let secs = secs.abs();
    if secs < 120.0 {
        return format!("{sign}{secs:.1}s");
    }
    let total = secs.round() as u64;
    let (days, hours, minutes) = (total / 86_400, total / 3600 % 24, total / 60 % 60);
    if days > 0 {
        format!("{sign}{days}d{hours:02}h{minutes:02}m")
    } else if hours > 0 {
        format!("{sign}{hours}h{minutes:02}m")
    } else {
        format!("{sign}{minutes}m{:02}s", total % 60)
    }
}

/// Local wall-clock seconds since the local epoch, so `% 86400` is the local time of day.
pub fn local_seconds(unix_secs: f64) -> f64 {
    unix_secs + f64::from(local_datetime(unix_secs).offset().local_minus_utc())
}

pub fn local_date(unix_secs: f64) -> NaiveDate {
    local_datetime(unix_secs).date_naive()
}

pub fn local_to_unix(date: NaiveDate, secs_of_day: u32) -> Option<f64> {
    let naive = date.and_hms_opt(secs_of_day / 3600, secs_of_day / 60 % 60, secs_of_day % 60)?;
    from_naive(naive)
}

fn from_naive(naive: NaiveDateTime) -> Option<f64> {
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|moment| moment.timestamp() as f64)
}

pub fn parse_time_of_day(text: &str) -> Option<u32> {
    let mut parts = text.split(':');
    let hours: u32 = parts.next()?.parse().ok()?;
    let minutes: u32 = parts.next()?.parse().ok()?;
    let seconds: u32 = parts.next().map_or(Some(0), |part| part.parse().ok())?;
    if parts.next().is_some() || hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    Some(hours * 3600 + minutes * 60 + seconds)
}

fn parse_local_datetime(text: &str) -> Option<f64> {
    let (date, time) = text.split_once([' ', 'T'])?;
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    local_to_unix(date, parse_time_of_day(time)?)
}

/// A point in a recording, written the way a person reads a clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeSpec {
    /// `+90` or `+90s`: seconds after the recording's first packet.
    Offset(f64),
    /// `HH:MM[:SS]`: local time of day; the first such moment at or after the recording start.
    TimeOfDay(u32),
    /// `YYYY-MM-DD HH:MM[:SS]`, local.
    Absolute(f64),
}

impl FromStr for TimeSpec {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if let Some(offset) = text.strip_prefix('+') {
            let secs = offset.trim_end_matches('s').parse::<f64>();
            return secs
                .map(TimeSpec::Offset)
                .map_err(|_| format!("'{text}' is not a +<seconds> offset"));
        }
        if let Some(secs_of_day) = parse_time_of_day(text) {
            return Ok(TimeSpec::TimeOfDay(secs_of_day));
        }
        parse_local_datetime(text)
            .map(TimeSpec::Absolute)
            .ok_or_else(|| format!("'{text}' is not +<seconds>, HH:MM[:SS] or 'YYYY-MM-DD HH:MM[:SS]'"))
    }
}

impl TimeSpec {
    pub fn resolve(self, recording_start: f64) -> f64 {
        match self {
            TimeSpec::Offset(secs) => recording_start + secs,
            TimeSpec::Absolute(unix) => unix,
            TimeSpec::TimeOfDay(secs_of_day) => {
                let date = local_date(recording_start);
                let same_day = local_to_unix(date, secs_of_day).unwrap_or(recording_start);
                if same_day >= recording_start - 0.5 {
                    return same_day;
                }
                date.checked_add_signed(Duration::days(1))
                    .and_then(|next| local_to_unix(next, secs_of_day))
                    .unwrap_or(same_day)
            }
        }
    }

    /// Every moment inside `[start, end]` this spec names, for notes that apply to any day.
    pub fn occurrences(self, start: f64, end: f64) -> Vec<f64> {
        match self {
            TimeSpec::Offset(secs) => vec![start + secs],
            TimeSpec::Absolute(unix) => (start..=end).contains(&unix).then_some(unix).into_iter().collect(),
            TimeSpec::TimeOfDay(secs_of_day) => {
                let mut found = Vec::new();
                let mut date = local_date(start);
                let last = local_date(end);
                while date <= last {
                    if let Some(unix) = local_to_unix(date, secs_of_day) {
                        if (start..=end).contains(&unix) {
                            found.push(unix);
                        }
                    }
                    match date.checked_add_signed(Duration::days(1)) {
                        Some(next) => date = next,
                        None => break,
                    }
                }
                found
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_specs_parse_every_documented_form() {
        assert_eq!("+90".parse(), Ok(TimeSpec::Offset(90.0)));
        assert_eq!("+2.5s".parse(), Ok(TimeSpec::Offset(2.5)));
        assert_eq!("16:05".parse(), Ok(TimeSpec::TimeOfDay(16 * 3600 + 5 * 60)));
        assert_eq!("16:05:09".parse(), Ok(TimeSpec::TimeOfDay(16 * 3600 + 5 * 60 + 9)));
        assert!(matches!("2026-10-07 17:00".parse(), Ok(TimeSpec::Absolute(_))));
        assert!("25:00".parse::<TimeSpec>().is_err());
        assert!("noon".parse::<TimeSpec>().is_err());
    }

    #[test]
    fn a_time_of_day_before_the_recording_start_means_the_next_day() {
        let recording_start = "2026-10-06 12:00:00".parse::<TimeSpec>().unwrap().resolve(0.0);
        let later = TimeSpec::TimeOfDay(13 * 3600).resolve(recording_start);
        let earlier = TimeSpec::TimeOfDay(11 * 3600).resolve(recording_start);
        assert_eq!(later - recording_start, 3600.0);
        assert_eq!(earlier - recording_start, 23.0 * 3600.0);
    }

    #[test]
    fn a_time_of_day_occurs_once_per_day_in_a_multi_day_range() {
        let start = "2026-10-06 12:00:00".parse::<TimeSpec>().unwrap().resolve(0.0);
        let end = start + 3.0 * 86_400.0;
        assert_eq!(TimeSpec::TimeOfDay(16 * 3600).occurrences(start, end).len(), 3);
    }
}
