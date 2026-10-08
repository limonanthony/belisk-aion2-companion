//! What the game last said about a ticket, as `todos.json` keeps it. The game
//! sends two numbers per ticket: the regular pool (what the game refills) and
//! the extra pool (items and rewards, never refilled). Both are kept with the
//! moment they were captured; what they are worth later is worked out by
//! `amount.rs` (refills) and `list.rs` (weekly resets).

use serde::{Deserialize, Serialize};

use super::data::CharacterId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameTicket {
    pub count: u32,
    pub charge: u32,
    /// When the packet was captured (Unix ms).
    pub at_ms: i64,
}

/// A ticket the game counts for the server. Whoever logged in last said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerTicket {
    pub server_id: u32,
    pub ticket: u32,
    pub value: GameTicket,
    pub by: CharacterId,
}

/// A game value as one character sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownTicket {
    pub value: GameTicket,
    /// Another character of the server gave it.
    pub shared_from: Option<CharacterId>,
}
