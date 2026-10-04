//! The session board: everyone's damage across every mob since the last reset,
//! prepared for drawing like the target board in `live`.

use aether_core::models::combat::CombatSnapshot;
use aether_core::models::session::SessionPlayerStat;
use gpui::SharedString;

use crate::format;
use super::live::{find_you, player_row, visible_indexes, Lead, PlayerRow, RowFacts};
use super::settings::OverlaySettings;
use super::shades::ShadeBook;

pub struct SessionBoard {
    pub rows: Vec<PlayerRow>,
    pub lead: Lead,
    /// Everyone's damage, for the footer.
    pub damage: SharedString,
    /// Mobs fought.
    pub targets: u32,
    /// The session's combat time, for the header clock.
    pub elapsed: SharedString,
}

impl SessionBoard {
    /// `None` until a mob has taken damage.
    pub fn build(snapshot: &CombatSnapshot, settings: &OverlaySettings, shades: &mut ShadeBook) -> Option<Self> {
        let session = &snapshot.session;
        let players = &session.players;
        let top = players.first()?;
        let you = you_index(snapshot, players);
        let top_damage = top.total_damage.max(1) as f32;

        let rows: Vec<PlayerRow> = visible_indexes(players.len(), you)
            .into_iter()
            .map(|index| {
                let player = &players[index];
                let facts = RowFacts {
                    actor_id: player.actor_id,
                    actor_name: &player.actor_name,
                    actor_server_id: &player.actor_server_id,
                    actor_class: &player.actor_class,
                    combat_power: player.combat_power,
                    total_damage: player.total_damage,
                    dps: player.dps,
                    pct: player.damage_share,
                };
                player_row(&facts, index + 1, Some(index) == you, top_damage, settings, shades)
            })
            .collect();

        let lead = match you {
            Some(index) => {
                let player = &players[index];
                let colour = rows.iter().find(|row| row.is_you).and_then(|row| row.colour);
                Lead::you(index, colour, player.dps, player.total_damage)
            }
            None => Lead::party(players.iter().map(|player| player.dps)),
        };
        Some(Self {
            rows,
            lead,
            damage: format::damage(session.total_damage).into(),
            targets: session.target_count,
            elapsed: format::duration(session.combat_time_secs).into(),
        })
    }
}

fn you_index(snapshot: &CombatSnapshot, players: &[SessionPlayerStat]) -> Option<usize> {
    find_you(snapshot, players.iter().map(|player| (player.actor_id, player.actor_name.as_str())))
        .map(|found| found.index)
}
