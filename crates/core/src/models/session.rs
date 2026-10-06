use serde::{Deserialize, Serialize};

/// One player's share of the session: everything they dealt to mobs since the
/// last reset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPlayerStat {
    pub actor_id: u32,
    pub actor_name: String,
    pub actor_server_id: String,
    pub actor_class: String,
    pub combat_power: Option<u64>,
    pub total_damage: u64,
    /// Seconds this player spent fighting; see `engine::session` for the
    /// definition. Not the wall time since the reset.
    pub combat_time_secs: f64,
    /// `total_damage / combat_time_secs`.
    pub dps: f64,
    /// Fraction of the whole session's damage, hidden players included.
    pub damage_share: f64,
}

/// Every mob fought since the last reset, folded into one board.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub total_damage: u64,
    /// Mobs that took damage, counted per spawn: a pack of thirty is thirty.
    pub target_count: u32,
    /// Seconds with at least one fight in progress (the union of every
    /// target's fight window), for the header clock.
    pub combat_time_secs: f64,
    /// Sorted by damage, after the hide-unknown and max-player filters.
    pub players: Vec<SessionPlayerStat>,
}
