//! How the meter writes its figures, as the approved mockup formats them.

use crate::i18n::fl;

pub fn grouped(value: f64) -> String {
    let digits = format!("{:.0}", value.max(0.0));
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// 9,840, then 61.2K, then 1.24M: DPS and party figures at a glance.
pub fn compact(value: f64) -> String {
    let value = value.max(0.0);
    if value < 10_000.0 {
        grouped(value)
    } else if value < 1_000_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{:.2}M", value / 1_000_000.0)
    }
}

/// Total damage, which runs into billions over a long fight.
pub fn damage(value: u64) -> String {
    let n = value as f64;
    match value {
        0 => "--".to_string(),
        1..=9_999 => value.to_string(),
        10_000..=999_999 => format!("{:.1}K", n / 1e3),
        1_000_000..=999_999_999 => format!("{:.2}M", n / 1e6),
        _ => fl!("unit-damage-billions", value = format!("{:.2}", n / 1e9)),
    }
}

/// A fraction as a percentage with one decimal; anything that rounds to 100
/// reads as a plain "100%".
pub fn percent(fraction: f64) -> String {
    let pct = fraction * 100.0;
    if pct >= 99.95 {
        "100%".to_string()
    } else {
        format!("{:.1}%", pct.max(0.0))
    }
}

pub fn whole_percent(fraction: f64) -> String {
    format!("{:.0}%", (fraction * 100.0).clamp(0.0, 100.0))
}

pub fn duration(secs: f64) -> String {
    let total = if secs.is_finite() { secs.max(0.0) as u64 } else { 0 };
    let (hours, minutes, seconds) = (total / 3600, total % 3600 / 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// "+4.2%" ahead of a personal best, "−6.8%" behind it.
pub fn pace(pace: f64) -> String {
    let sign = if pace >= 0.0 { '+' } else { '\u{2212}' };
    format!("{sign}{:.1}%", (pace * 100.0).abs())
}

/// The label of a player the game has not named: "You" on the local player's
/// own row, whose actor id the meter knows without a name, else the actor id.
pub fn unnamed_player(actor_id: u32, is_you: bool) -> String {
    if is_you {
        fl!("common-you")
    } else {
        format!("ID:{actor_id}")
    }
}

/// Keeps the first and last character: "Mirelle" becomes "M*****e".
pub fn mask(name: &str) -> String {
    let chars: Vec<char> = name.trim().chars().collect();
    match chars.len() {
        0 => String::new(),
        1 => "*".to_string(),
        2 => format!("{}*", chars[0]),
        n => format!("{}{}{}", chars[0], "*".repeat(n - 2), chars[n - 1]),
    }
}

pub fn memory(megabytes: f64) -> Option<String> {
    if !megabytes.is_finite() || megabytes <= 0.0 {
        return None;
    }
    Some(if megabytes >= 1024.0 {
        fl!("unit-gigabytes", value = format!("{:.1}", megabytes / 1024.0))
    } else {
        fl!("unit-megabytes", value = format!("{megabytes:.0}"))
    })
}

/// The report's own duration form: "3m41s".
pub fn report_duration(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{}m{:02}s", total / 60, total % 60)
}

/// A file size as the storage summary shows it: "6 KB", "2.1 MB".
pub fn file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let size = bytes as f64;
    if size < KB {
        fl!("unit-bytes", value = bytes.to_string())
    } else if size < KB * KB {
        fl!("unit-kilobytes", value = format!("{:.0}", size / KB))
    } else if size < KB * KB * KB {
        fl!("unit-megabytes", value = format!("{:.1}", size / (KB * KB)))
    } else {
        fl!("unit-gigabytes", value = format!("{:.1}", size / (KB * KB * KB)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_are_grouped_in_threes() {
        assert_eq!(grouped(0.0), "0");
        assert_eq!(grouped(999.4), "999");
        assert_eq!(grouped(1_234_567.0), "1,234,567");
        assert_eq!(grouped(-5.0), "0");
    }

    #[test]
    fn compact_figures_switch_unit_at_ten_thousand_and_a_million() {
        assert_eq!(compact(9_840.0), "9,840");
        assert_eq!(compact(61_240.0), "61.2K");
        assert_eq!(compact(1_240_000.0), "1.24M");
        assert_eq!(compact(-3.0), "0");
    }

    #[test]
    fn damage_reaches_billions_and_shows_nothing_as_dashes() {
        assert_eq!(damage(0), "--");
        assert_eq!(damage(9_999), "9999");
        assert_eq!(damage(13_533_040), "13.53M");
        assert_eq!(damage(2_500_000_000), "2.50B");
    }

    #[test]
    fn percentages_round_to_a_plain_hundred() {
        assert_eq!(percent(0.4123), "41.2%");
        assert_eq!(percent(0.9996), "100%");
        assert_eq!(whole_percent(0.624), "62%");
        assert_eq!(whole_percent(1.4), "100%");
    }

    #[test]
    fn durations_gain_hours_only_when_needed() {
        assert_eq!(duration(0.0), "0:00");
        assert_eq!(duration(221.7), "3:41");
        assert_eq!(duration(3_725.0), "1:02:05");
        assert_eq!(duration(f64::NAN), "0:00");
        assert_eq!(report_duration(221.0), "3m41s");
    }

    #[test]
    fn pace_carries_its_sign() {
        assert_eq!(pace(0.042), "+4.2%");
        assert_eq!(pace(-0.068), "\u{2212}6.8%");
        assert_eq!(pace(0.0), "+0.0%");
    }

    #[test]
    fn masking_keeps_the_ends() {
        assert_eq!(mask("Mirelle"), "M*****e");
        assert_eq!(mask("Jo"), "J*");
        assert_eq!(mask(" A "), "*");
        assert_eq!(mask(""), "");
    }

    #[test]
    fn an_unnamed_player_is_you_only_on_your_own_row() {
        assert_eq!(unnamed_player(7, true), "You");
        assert_eq!(unnamed_player(7, false), "ID:7");
    }

    #[test]
    fn memory_switches_to_gigabytes() {
        assert_eq!(memory(800.0).as_deref(), Some("800 MB"));
        assert_eq!(memory(11_468.8).as_deref(), Some("11.2 GB"));
        assert_eq!(memory(0.0), None);
    }

    #[test]
    fn file_sizes_pick_the_unit_that_reads_best() {
        assert_eq!(file_size(0), "0 B");
        assert_eq!(file_size(6_144), "6 KB");
        assert_eq!(file_size(2_200_000), "2.1 MB");
        assert_eq!(file_size(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn words_and_units_follow_the_language_and_figures_do_not() {
        crate::i18n::set_language(crate::i18n::Language::French);
        assert_eq!(unnamed_player(7, true), "Vous");
        assert_eq!(file_size(0), "0 o");
        assert_eq!(file_size(6_144), "6 Ko");
        assert_eq!(file_size(2_200_000), "2.1 Mo");
        assert_eq!(memory(11_468.8).as_deref(), Some("11.2 Go"));
        assert_eq!(compact(48_100_000.0), "48.10M");
        assert_eq!(damage(2_500_000_000), "2.50Md");
        assert_eq!(grouped(9_840.0), "9,840");
    }
}
