//! Who the local player is, when the game has not said. The types the core and
//! the app share: a character the player was seen as, and what the core decided
//! about the player's own actor.

use serde::{Deserialize, Serialize};

/// A character the game named as the local player's, kept by the app so a later
/// run can name the player before the game does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownCharacter {
    pub name: String,
    pub server_id: u32,
    /// The class in the core's own spelling, as inferred from skill codes
    /// (`"GLADIATOR"`), so it compares with what the player's skills show.
    pub class: String,
    /// The game's stable id of the character: 6 bytes as 12 hex digits. Absent
    /// when the packet did not carry one that could be told apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// What the core decided about the local player's actor, for the decision log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityDecision {
    /// The remain-HP signature named the actor; its name is not known yet.
    SignatureBound { actor_id: u32 },
    /// A name carried over from an earlier connection was dropped: the class the
    /// new actor's skills show is not the class that name had (another
    /// character was picked at character select), or that class is unknown.
    CarriedRejected { actor_id: u32, name: String, carried_class: Option<String>, class: String },
    /// A remembered character's class matched the one the actor's skills show.
    RememberedApplied { actor_id: u32, name: String, class: String },
    /// No remembered character has the class the actor's skills show.
    RememberedRejected { actor_id: u32, class: String, remembered: usize },
    /// The game's own name packet replaced what the signature and the
    /// remembered characters had decided.
    GameOverrode { actor_id: u32, previous_actor_id: Option<u32>, previous_name: Option<String> },
}

/// The remembered character whose class is `class`. The list is most recent
/// first, so with several the most recently seen wins.
pub fn character_of_class<'a>(known: &'a [KnownCharacter], class: &str) -> Option<&'a KnownCharacter> {
    known.iter().find(|character| character.class == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(name: &str, class: &str) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id: 1001, class: class.to_string(), key: None }
    }

    #[test]
    fn the_most_recently_seen_character_of_the_class_wins() {
        let known = [character("Newer", "RANGER"), character("Other", "CLERIC"), character("Older", "RANGER")];
        assert_eq!(character_of_class(&known, "RANGER").map(|c| c.name.as_str()), Some("Newer"));
        assert_eq!(character_of_class(&known, "CLERIC").map(|c| c.name.as_str()), Some("Other"));
    }

    #[test]
    fn no_character_matches_a_class_nobody_plays() {
        assert!(character_of_class(&[character("Solo", "RANGER")], "TEMPLAR").is_none());
        assert!(character_of_class(&[], "RANGER").is_none());
    }
}
