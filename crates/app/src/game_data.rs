//! Classes, server names, dungeons and skill names for the meter.
//!
//! The JSON files live in `assets/game/data` and are compiled into the binary.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const SERVERS_JSON: &str = include_str!("../assets/game/data/servers.json");
const DUNGEONS_JSON: &str = include_str!("../assets/game/data/dungeons.json");
const SKILLS_JSON: &str = include_str!("../assets/game/data/skill_names_en.json");

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
}

static SERVERS: LazyLock<HashMap<u32, String>> = LazyLock::new(|| {
    serde_json::from_str::<Vec<Server>>(SERVERS_JSON)
        .expect("bundled servers.json is valid")
        .into_iter()
        .map(|server| (server.server_id, server.server_short_name))
        .collect()
});

static DUNGEONS_BY_BOSS: LazyLock<HashMap<u32, String>> = LazyLock::new(|| {
    let mut by_boss = HashMap::new();
    for dungeon in
        serde_json::from_str::<Vec<Dungeon>>(DUNGEONS_JSON).expect("bundled dungeons.json is valid")
    {
        let Some(name) = dungeon.name.en.filter(|name| !name.is_empty()) else {
            continue;
        };
        let label = match dungeon.difficulty.and_then(|difficulty| difficulty.en) {
            Some(difficulty) if !difficulty.is_empty() => format!("{name} · {difficulty}"),
            _ => name,
        };
        for boss in dungeon.boss_ids {
            by_boss.insert(boss, label.clone());
        }
    }
    by_boss
});

static SKILLS: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(SKILLS_JSON).expect("bundled en.json is valid"));

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

/// "Dungeon · Difficulty" for a boss.
pub fn dungeon_of(mob_code: u32) -> Option<&'static str> {
    DUNGEONS_BY_BOSS.get(&mob_code).map(String::as_str)
}

fn resolve_skill_id(code: u32) -> String {
    meter_core::skill_names::resolve_skill_key(code, |key| SKILLS.contains_key(key))
}

pub fn skill_name(code: u32) -> String {
    SKILLS
        .get(&resolve_skill_id(code))
        .cloned()
        .unwrap_or_else(|| format!("Skill {code}"))
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
        assert_eq!(dungeon_of(2300243), Some("Azure Breath Island · Hard"));
        assert_eq!(skill_name(100001), "Basic Attack");
        assert_eq!(skill_name(4_000_000), "Skill 4000000");
        assert_eq!(skill_name(160_467_011), "Attack");
    }

    #[test]
    fn skill_icons_follow_the_six_or_four_digit_rule_and_must_exist() {
        assert_eq!(skill_icon(11_020_150).as_deref(), Some("skills/1102.png"));
        assert_eq!(skill_icon(100_010).as_deref(), Some("skills/100010.png"));
        assert_eq!(skill_icon(4_000_000), None);
    }
}
