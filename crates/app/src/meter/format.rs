//! How the meter writes its figures. Ported from the Tauri overlay
//! (`overlay/meter/main.js`) and the approved mockup's formatting.

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
        _ => format!("{:.2}B", n / 1e9),
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
        format!("{:.1} GB", megabytes / 1024.0)
    } else {
        format!("{megabytes:.0} MB")
    })
}

/// The report's own duration form: "3m41s".
pub fn report_duration(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{}m{:02}s", total / 60, total % 60)
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
    fn memory_switches_to_gigabytes() {
        assert_eq!(memory(800.0).as_deref(), Some("800 MB"));
        assert_eq!(memory(11_468.8).as_deref(), Some("11.2 GB"));
        assert_eq!(memory(0.0), None);
    }
}
