//! Classes, server names, dungeons and skill and mob names for the meter.
//!
//! The JSON files live in `assets/game/data` and are compiled into the binary.
//! Skill and mob names are English in the core and the saved records; the
//! lookups here answer in the language in force, English where an id has no
//! translation.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::i18n::{self, Language};

const SERVERS_JSON: &str = include_str!("../assets/game/data/servers.json");
const DUNGEONS_JSON: &str = include_str!("../assets/game/data/dungeons.json");
const SKILLS_JSON: &str = include_str!("../assets/game/data/skill_names_en.json");
const SKILLS_FR_JSON: &str = include_str!("../assets/game/data/skill_names_fr.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerClass {
    Assassin,
    Chanter,
    Cleric,
    Elementalist,
    Fighter,
    Gladiator,
    Ranger,
    Sorcerer,
    Templar,
}

impl PlayerClass {
    pub fn parse(name: &str) -> Option<Self> {
        const ALL: [(&str, PlayerClass); 9] = [
            ("assassin", PlayerClass::Assassin),
            ("chanter", PlayerClass::Chanter),
            ("cleric", PlayerClass::Cleric),
            ("elementalist", PlayerClass::Elementalist),
            ("fighter", PlayerClass::Fighter),
            ("gladiator", PlayerClass::Gladiator),
            ("ranger", PlayerClass::Ranger),
            ("sorcerer", PlayerClass::Sorcerer),
            ("templar", PlayerClass::Templar),
        ];
        ALL.iter()
            .find(|(key, _)| name.trim().eq_ignore_ascii_case(key))
            .map(|&(_, class)| class)
    }

    /// The class colour as `0xRRGGBB` (tokens.md, class colours).
    pub fn rgb(self) -> u32 {
        match self {
            Self::Assassin => 0xa855f7,
            Self::Chanter => 0x34d399,
            Self::Cleric => 0xfbbf24,
            Self::Elementalist => 0x22d3ee,
            Self::Fighter => 0xf87171,
            Self::Gladiator => 0xfb923c,
            Self::Ranger => 0xa3e635,
            Self::Sorcerer => 0xe879f9,
            Self::Templar => 0x38bdf8,
        }
    }

    /// Asset path of the class icon, see `crate::assets`.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Assassin => "class/assassin.png",
            Self::Chanter => "class/chanter.png",
            Self::Cleric => "class/cleric.png",
            Self::Elementalist => "class/elementalist.png",
            Self::Fighter => "class/fighter.png",
            Self::Gladiator => "class/gladiator.png",
            Self::Ranger => "class/ranger.png",
            Self::Sorcerer => "class/sorcerer.png",
            Self::Templar => "class/templar.png",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Server {
    server_id: u32,
    server_short_name: String,
}

#[derive(Deserialize)]
struct Dungeon {
    name: Localized,
    difficulty: Option<Localized>,
    #[serde(default)]
    boss_ids: Vec<u32>,
}

#[derive(Deserialize)]
struct Localized {
    en: Option<String>,
    fr: Option<String>,
}

struct DungeonLabel {
    english: String,
    french: String,
}

static SERVERS: LazyLock<HashMap<u32, String>> = LazyLock::new(|| {
    serde_json::from_str::<Vec<Server>>(SERVERS_JSON)
        .expect("bundled servers.json is valid")
        .into_iter()
        .map(|server| (server.server_id, server.server_short_name))
        .collect()
});

static DUNGEONS_BY_BOSS: LazyLock<HashMap<u32, DungeonLabel>> = LazyLock::new(|| {
    let mut by_boss = HashMap::new();
    for dungeon in
        serde_json::from_str::<Vec<Dungeon>>(DUNGEONS_JSON).expect("bundled dungeons.json is valid")
    {
        let Some(english_name) = non_empty(dungeon.name.en.clone()) else {
            continue;
        };
        let french_name = non_empty(dungeon.name.fr).unwrap_or_else(|| english_name.clone());
        let difficulty = dungeon.difficulty.unwrap_or(Localized { en: None, fr: None });
        let english_difficulty = non_empty(difficulty.en);
        let french_difficulty = non_empty(difficulty.fr).or_else(|| english_difficulty.clone());
        let label = DungeonLabel {
            english: join_label(english_name, english_difficulty),
            french: join_label(french_name, french_difficulty),
        };
        for boss in dungeon.boss_ids {
            by_boss.insert(boss, DungeonLabel { english: label.english.clone(), french: label.french.clone() });
        }
    }
    by_boss
});

fn non_empty(text: Option<String>) -> Option<String> {
    text.filter(|text| !text.is_empty())
}

fn join_label(name: String, difficulty: Option<String>) -> String {
    match difficulty {
        Some(difficulty) => format!("{name} · {difficulty}"),
        None => name,
    }
}

static SKILLS: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(SKILLS_JSON).expect("bundled en.json is valid"));

static SKILLS_FR: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(SKILLS_FR_JSON).expect("bundled skill_names_fr.json is valid"));

static MOBS_FR: LazyLock<HashMap<u32, String>> = LazyLock::new(meter_core::storage::loaders::load_npc_names_fr);

/// The short server name, or the id itself when the server is unknown.
pub fn server_name(server_id: &str) -> String {
    server_id
        .trim()
        .parse::<u32>()
        .ok()
        .and_then(|id| SERVERS.get(&id))
        .cloned()
        .unwrap_or_else(|| server_id.to_string())
}

/// "Dungeon · Difficulty" for a boss, in the language in force with English
/// for any field that has no French name.
pub fn dungeon_of(mob_code: u32) -> Option<String> {
    let label = DUNGEONS_BY_BOSS.get(&mob_code)?;
    let text = if i18n::language() == Language::French { &label.french } else { &label.english };
    Some(text.clone())
}

fn resolve_skill_id(code: u32) -> String {
    meter_core::skill_names::resolve_skill_key(code, |key| SKILLS.contains_key(key))
}

pub fn skill_name(code: u32) -> String {
    let key = resolve_skill_id(code);
    let french = (i18n::language() == Language::French).then(|| SKILLS_FR.get(&key)).flatten();
    french
        .or_else(|| SKILLS.get(&key))
        .cloned()
        .unwrap_or_else(|| format!("Skill {code}"))
}

/// The name to show for a mob: its French one when French is in force and the
/// code has one, else the English name the core recorded.
pub fn mob_name(mob_code: Option<u32>, english: Option<&str>) -> Option<String> {
    let french = (i18n::language() == Language::French)
        .then(|| mob_code.and_then(|code| MOBS_FR.get(&code)))
        .flatten();
    french.cloned().or_else(|| english.map(str::to_string))
}

/// Asset path of a skill or buff icon, when the game data has one: a six-digit
/// id has its own icon, any other id shares the icon of its first four digits.
pub fn skill_icon(code: u32) -> Option<String> {
    let base = resolve_skill_id(code);
    let file = if base.len() == 6 { base } else { base[..base.len().min(4)].to_string() };
    let path = format!("skills/{file}.png");
    crate::assets::has(&path).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_parse_whatever_the_case() {
        assert_eq!(PlayerClass::parse("Ranger"), Some(PlayerClass::Ranger));
        assert_eq!(PlayerClass::parse("SORCERER"), Some(PlayerClass::Sorcerer));
        assert_eq!(PlayerClass::parse(""), None);
        assert_eq!(PlayerClass::parse("Bard"), None);
    }

    #[test]
    fn every_class_icon_is_embedded() {
        use gpui::AssetSource as _;
        for name in ["assassin", "chanter", "cleric", "elementalist", "fighter", "gladiator", "ranger", "sorcerer", "templar"] {
            let icon = PlayerClass::parse(name).unwrap().icon();
            assert!(crate::assets::Assets.load(icon).unwrap().is_some_and(|bytes| !bytes.is_empty()), "{icon}");
        }
    }

    #[test]
    fn bundled_data_resolves_names() {
        assert_eq!(server_name("1001"), "SIE");
        assert_eq!(server_name("99999"), "99999");
        i18n::set_language(Language::English);
        assert_eq!(dungeon_of(2300243).as_deref(), Some("Azure Breath Island · Hard"));
        assert_eq!(skill_name(100001), "Basic Attack");
        assert_eq!(skill_name(4_000_000), "Skill 4000000");
        assert_eq!(skill_name(160_467_011), "Attack");
    }

    #[test]
    fn names_follow_the_language_and_fall_back_to_english_without_a_translation() {
        const TRANSLATED_SKILL: u32 = 100_001;
        const UNTRANSLATED_SKILL: u32 = 100_510;
        const TRANSLATED_MOB: u32 = 2_000_000;
        const UNTRANSLATED_MOB: u32 = 2_300_366;

        i18n::set_language(Language::English);
        assert_eq!(skill_name(TRANSLATED_SKILL), "Basic Attack");
        assert_eq!(dungeon_of(2_301_721).as_deref(), Some("Citadel of the Fallen Daeva · Hard"));
        assert_eq!(mob_name(Some(TRANSLATED_MOB), Some("Punching Bag")).as_deref(), Some("Punching Bag"));

        i18n::set_language(Language::French);
        assert_eq!(skill_name(TRANSLATED_SKILL), "Attaque de base");
        assert_eq!(dungeon_of(2_301_721).as_deref(), Some("Citadelle du Daeva déchu · Difficile"));
        assert_eq!(dungeon_of(2_300_555).as_deref(), Some("Devourer's Research Base · Difficile"));
        assert_eq!(skill_name(UNTRANSLATED_SKILL), "Earth Chain [Frugal]");
        assert_eq!(skill_name(4_000_000), "Skill 4000000");
        assert_eq!(mob_name(Some(TRANSLATED_MOB), Some("Punching Bag")).as_deref(), Some("Mannequin"));
        assert_eq!(
            mob_name(Some(UNTRANSLATED_MOB), Some("Contaminated Shulak Laborer")).as_deref(),
            Some("Contaminated Shulak Laborer")
        );
        assert_eq!(mob_name(None, None), None);
    }

    #[test]
    fn skill_icons_follow_the_six_or_four_digit_rule_and_must_exist() {
        assert_eq!(skill_icon(11_020_150).as_deref(), Some("skills/1102.png"));
        assert_eq!(skill_icon(100_010).as_deref(), Some("skills/100010.png"));
        assert_eq!(skill_icon(4_000_000), None);
    }
}
