/// The key a skill-name table knows a skill under: the exact id first, then the
/// forms the backend normalises skill ids to. A damage-over-time id is its skill
/// id followed by two digits, so it is last tried with them cut off: that is how
/// a nine-digit id such as 160467011 reaches the seven-digit key 1604670.
/// `is_named` says whether the table has a key; when none matches, the first
/// eight digits are returned.
pub fn resolve_skill_key(code: u32, is_named: impl Fn(&str) -> bool) -> String {
    let raw = code.to_string();
    let head = |n: usize| &raw[..raw.len().min(n)];
    let mut candidates = vec![raw.clone(), head(8).to_string()];
    if raw.len() > 8 {
        let mut base = head(8).to_string();
        base.pop();
        base.push('0');
        candidates.push(base);
    }
    if raw.len() > 6 {
        candidates.push(format!("{:0<8}", head(6)));
    }
    if raw.len() > 8 {
        candidates.push((code / 100).to_string());
    }
    candidates
        .into_iter()
        .find(|candidate| is_named(candidate))
        .unwrap_or_else(|| head(8).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: u32, named: &[&str]) -> String {
        resolve_skill_key(code, |candidate| named.contains(&candidate))
    }

    #[test]
    fn a_nine_digit_damage_over_time_id_reaches_its_seven_digit_skill() {
        assert_eq!(key(160_467_011, &["1604670"]), "1604670");
    }

    #[test]
    fn a_ten_digit_id_still_resolves_to_its_eight_digit_skill() {
        assert_eq!(key(1_709_014_011, &["17090140", "170901"]), "17090140");
    }

    #[test]
    fn the_exact_id_and_the_earlier_forms_win_over_the_cut_off_form() {
        assert_eq!(key(160_467_011, &["160467011", "1604670"]), "160467011");
        assert_eq!(key(120_001_611, &["12000100", "1200016"]), "12000100");
    }

    #[test]
    fn an_id_no_form_names_falls_back_to_its_first_eight_digits() {
        assert_eq!(key(160_467_011, &[]), "16046701");
    }
}
