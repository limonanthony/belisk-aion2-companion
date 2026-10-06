//! The characters the game has named as the local player, kept in
//! `settings.json` so a later run can name the player's own row before the game
//! does (the core matches one by the class the player's skills show). Most
//! recent first, bounded, and read leniently: an entry that is not a character
//! costs that entry only.

use meter_core::identity::KnownCharacter;
use serde::{Deserialize, Deserializer};

pub const MAX_REMEMBERED: usize = 20;
const MAX_NAME_CHARS: usize = 36;
const KEY_DIGITS: usize = 12;

/// Puts `seen` first. A character is the same one when both carry the same key
/// or, without keys, the same name on the same server; its entry is replaced,
/// keeping a key the new one lacks. Returns whether the list changed.
pub fn remember(list: &mut Vec<KnownCharacter>, mut seen: KnownCharacter) -> bool {
    if !is_character(&seen) {
        return false;
    }
    let before = list.clone();
    if let Some(index) = list.iter().position(|known| is_same_character(known, &seen)) {
        let known = list.remove(index);
        seen.key = seen.key.or(known.key);
    }
    list.insert(0, seen);
    list.truncate(MAX_REMEMBERED);
    *list != before
}

fn is_same_character(known: &KnownCharacter, seen: &KnownCharacter) -> bool {
    match (&known.key, &seen.key) {
        (Some(known_key), Some(seen_key)) => known_key == seen_key,
        _ => known.name == seen.name && known.server_id == seen.server_id,
    }
}

fn is_character(character: &KnownCharacter) -> bool {
    let name = character.name.trim();
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && !character.class.is_empty()
        && character
            .key
            .as_deref()
            .is_none_or(|key| key.len() == KEY_DIGITS && key.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub fn truncated(mut list: Vec<KnownCharacter>) -> Vec<KnownCharacter> {
    list.truncate(MAX_REMEMBERED);
    list
}

pub fn deserialize_lenient<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<KnownCharacter>, D::Error> {
    let entries = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(truncated(
        entries
            .into_iter()
            .filter_map(|entry| serde_json::from_value::<KnownCharacter>(entry).ok())
            .filter(is_character)
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(name: &str, class: &str, key: Option<&str>) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id: 1001, class: class.to_string(), key: key.map(str::to_string) }
    }

    #[test]
    fn the_newest_character_goes_first_and_a_repeat_changes_nothing() {
        let mut list = Vec::new();
        assert!(remember(&mut list, character("Alder", "RANGER", None)));
        assert!(remember(&mut list, character("Birch", "CLERIC", None)));
        assert!(remember(&mut list, character("Alder", "RANGER", None)));
        assert!(!remember(&mut list, character("Alder", "RANGER", None)));

        let names: Vec<_> = list.iter().map(|known| known.name.as_str()).collect();
        assert_eq!(names, ["Alder", "Birch"]);
    }

    #[test]
    fn a_character_is_the_same_by_key_even_when_renamed_and_keeps_its_key() {
        let mut list = vec![character("Alder", "RANGER", Some("3AA503000000"))];

        assert!(remember(&mut list, character("Renamed", "RANGER", Some("3AA503000000"))));
        assert!(!remember(&mut list, character("Renamed", "RANGER", None)), "a repeat without the key changes nothing");

        assert_eq!(list, vec![character("Renamed", "RANGER", Some("3AA503000000"))]);
    }

    #[test]
    fn the_list_is_bounded_and_drops_the_oldest() {
        let mut list = Vec::new();
        for index in 0..MAX_REMEMBERED + 3 {
            remember(&mut list, character(&format!("Hero{index}"), "RANGER", None));
        }

        assert_eq!(list.len(), MAX_REMEMBERED);
        assert_eq!(list[0].name, format!("Hero{}", MAX_REMEMBERED + 2));
        assert_eq!(list.last().unwrap().name, "Hero3");
    }

    #[test]
    fn something_that_is_not_a_character_is_not_remembered() {
        let mut list = Vec::new();

        assert!(!remember(&mut list, character("", "RANGER", None)));
        assert!(!remember(&mut list, character("Alder", "", None)));
        assert!(!remember(&mut list, character("Alder", "RANGER", Some("not hex"))));
        assert!(list.is_empty());
    }

    #[test]
    fn reading_keeps_the_good_entries_and_the_bound() {
        let stored = serde_json::json!([
            { "name": "Alder", "serverId": 1001, "class": "RANGER", "key": "3AA503000000" },
            { "name": "Broken", "serverId": "nope", "class": "RANGER" },
            "not an object",
            { "name": "", "serverId": 1001, "class": "RANGER" },
            { "name": "Birch", "serverId": 1002, "class": "CLERIC" },
        ]);

        let read = deserialize_lenient(stored).unwrap();

        let birch = KnownCharacter { server_id: 1002, ..character("Birch", "CLERIC", None) };
        assert_eq!(read, vec![character("Alder", "RANGER", Some("3AA503000000")), birch]);
    }
}
