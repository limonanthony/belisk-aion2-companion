//! Ties the game's per-character messages to the local player's character. The
//! game names the character with `33 36`; the messages that belong to it arrive
//! a few seconds before or after, and a world entry sends a fresh snapshot.

use crate::identity::KnownCharacter;

/// The game names the character about 4 s after the snapshot that belongs to
/// it; this bounds how long a message waits for that name.
const IDENTITY_AFTER_MESSAGE_SECS: f64 = 15.0;
/// A name this close before a snapshot is the one the snapshot belongs to.
const IDENTITY_BEFORE_SNAPSHOT_SECS: f64 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterReport<Content> {
    pub character: KnownCharacter,
    /// When the packet was captured, in seconds since the Unix epoch. A replay
    /// carries the recording's time.
    pub captured_at: f64,
    pub content: Content,
}

struct NamedCharacter {
    character: KnownCharacter,
    naming: Naming,
}

#[derive(Clone, Copy, PartialEq)]
enum Naming {
    /// The game named the character, when it was captured.
    Game { seen_at: f64 },
    /// Taken from the remembered characters by `assume_character`.
    Character,
    /// Only the server was taken from the remembered characters by
    /// `assume_server`; `character` stands in for whoever the player is.
    Server,
}

impl NamedCharacter {
    fn is_assumed(&self) -> bool {
        !matches!(self.naming, Naming::Game { .. })
    }
}

struct Waiting<Content> {
    captured_at: f64,
    content: Content,
}

/// A snapshot belongs to the name that arrives up to 15 s after it or up to 2 s
/// before it; a change belongs to the character named now. Anything that cannot
/// be tied yet waits for the next name, and is dropped once 15 s old or when a
/// new snapshot shows the player entered another world.
pub struct CharacterBinder<Content> {
    named: Option<NamedCharacter>,
    waiting: Vec<Waiting<Content>>,
    assumed_messages: usize,
}

impl<Content> Default for CharacterBinder<Content> {
    fn default() -> Self {
        Self { named: None, waiting: Vec::new(), assumed_messages: 0 }
    }
}

impl<Content> CharacterBinder<Content> {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn character_named(&mut self, character: KnownCharacter, captured_at: f64) -> Vec<CharacterReport<Content>> {
        let waiting = std::mem::take(&mut self.waiting);
        let reports = waiting
            .into_iter()
            .filter(|message| captured_at - message.captured_at <= IDENTITY_AFTER_MESSAGE_SECS)
            .map(|message| CharacterReport {
                character: character.clone(),
                captured_at: message.captured_at,
                content: message.content,
            })
            .collect();
        self.named = Some(NamedCharacter { character, naming: Naming::Game { seen_at: captured_at } });
        self.assumed_messages = 0;
        reports
    }

    /// Takes `character` as the one the messages belong to when the game has
    /// not named anyone since the binder started (the meter was opened after
    /// login, and the character comes from the remembered ones). The game's own
    /// name, when it comes, replaces it; one already given is never replaced.
    /// Returns whether the assumption took.
    pub fn assume_character(&mut self, character: KnownCharacter) -> bool {
        let unnamed = self.named.is_none();
        if unnamed {
            self.named = Some(NamedCharacter { character, naming: Naming::Character });
            self.assumed_messages = 0;
        }
        unnamed
    }

    /// Takes the server of `stand_in` as the one the messages belong to when
    /// the game has not named anyone since the binder started; for messages
    /// that matter per server, not per character. `stand_in` is whoever of
    /// the remembered characters the report carries meanwhile. The game's own
    /// name, when it comes, replaces it; one already given is never replaced.
    /// Returns whether the assumption took.
    pub fn assume_server(&mut self, stand_in: KnownCharacter) -> bool {
        let unnamed = self.named.is_none();
        if unnamed {
            self.named = Some(NamedCharacter { character: stand_in, naming: Naming::Server });
            self.assumed_messages = 0;
        }
        unnamed
    }

    /// The character taken as the player's by `assume_character` while the game
    /// has not named anyone, and how many changes were bound to it meanwhile.
    pub fn assumed(&self) -> Option<(&KnownCharacter, usize)> {
        self.named
            .as_ref()
            .filter(|named| named.naming == Naming::Character)
            .map(|named| (&named.character, self.assumed_messages))
    }

    /// The server taken by `assume_server` while the game has not named
    /// anyone, and how many changes were bound to it meanwhile.
    pub fn assumed_server(&self) -> Option<(u32, usize)> {
        self.named
            .as_ref()
            .filter(|named| named.naming == Naming::Server)
            .map(|named| (named.character.server_id, self.assumed_messages))
    }

    pub fn snapshot(&mut self, content: Content, captured_at: f64) -> Option<CharacterReport<Content>> {
        self.waiting.clear();
        let named_just_before = self.named.take().filter(|named| match named.naming {
            Naming::Game { seen_at } => (0.0..=IDENTITY_BEFORE_SNAPSHOT_SECS).contains(&(captured_at - seen_at)),
            Naming::Character | Naming::Server => false,
        });
        match named_just_before {
            Some(named) => {
                let report = CharacterReport { character: named.character.clone(), captured_at, content };
                self.named = Some(named);
                Some(report)
            }
            None => {
                self.waiting.push(Waiting { captured_at, content });
                None
            }
        }
    }

    pub fn change(&mut self, content: Content, captured_at: f64) -> Option<CharacterReport<Content>> {
        match &self.named {
            Some(named) => {
                if named.is_assumed() {
                    self.assumed_messages += 1;
                }
                Some(CharacterReport { character: named.character.clone(), captured_at, content })
            }
            None => {
                self.waiting.retain(|message| captured_at - message.captured_at <= IDENTITY_AFTER_MESSAGE_SECS);
                self.waiting.push(Waiting { captured_at, content });
                None
            }
        }
    }
}
