use serde::{Deserialize, Serialize};

use super::resets::Resets;
use crate::clock::{now_ms, ClockTime, LocalZone, Zone};

/// What Settings → To-dos changes; the to-dos themselves live in `todos.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TodoSettings {
    /// The hour (0 to 23, the PC's clock) the player pinned the daily reset
    /// to; the weekly reset is Wednesday at the same hour. None follows the
    /// game's own grid, 07:00 UTC, which moves on the PC's clock with
    /// daylight saving.
    pub reset_hour: Option<u8>,
    /// Odyle Energy fills up to 840 instead of 560.
    pub membership: bool,
}

impl TodoSettings {
    pub fn clamped(mut self) -> Self {
        self.reset_hour = self.reset_hour.map(|hour| hour.min(23));
        self
    }

    pub fn resets<'a>(&self, zone: &'a dyn Zone) -> Resets<'a> {
        match self.reset_hour {
            Some(hour) => Resets::new(zone, hour),
            None => Resets::following_the_game(zone),
        }
    }

    /// The daily reset on the PC's clock at `now_ms`.
    pub fn reset_clock(&self, zone: &dyn Zone, now_ms: i64) -> ClockTime {
        self.resets(zone).clock_at(now_ms)
    }

    /// The daily reset on the PC's clock now.
    pub fn reset_clock_now(&self) -> ClockTime {
        self.reset_clock(&LocalZone, now_ms())
    }

    /// What picking `hour` in Settings stores: the hour the game's grid reads
    /// as at `now_ms` follows the game, so the reset keeps tracking daylight
    /// saving; any other hour is pinned.
    pub fn choice_of_hour(hour: u8, zone: &dyn Zone, now_ms: i64) -> Option<u8> {
        let game = Resets::following_the_game(zone).clock_at(now_ms);
        (game != ClockTime { hour, minute: 0 }).then_some(hour)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::testing::{berlin, utc};

    #[test]
    fn the_reset_hour_is_pulled_into_the_day() {
        assert_eq!(TodoSettings { reset_hour: Some(31), membership: true }.clamped().reset_hour, Some(23));
        assert_eq!(TodoSettings::default().clamped(), TodoSettings { reset_hour: None, membership: false });
    }

    #[test]
    fn by_default_the_reset_follows_the_game() {
        assert_eq!(TodoSettings::default().reset_hour, None);
    }

    #[test]
    fn the_games_reset_reads_09_in_summer_and_08_in_winter_in_berlin() {
        let zone = berlin();
        let settings = TodoSettings::default();
        assert_eq!(settings.reset_clock(&zone, utc(2026, 10, 5, 12, 0)).to_string(), "09:00", "CEST");
        assert_eq!(settings.reset_clock(&zone, utc(2026, 12, 5, 12, 0)).to_string(), "08:00", "CET");
    }

    #[test]
    fn a_pinned_hour_is_the_same_on_the_clock_all_year() {
        let zone = berlin();
        let settings = TodoSettings { reset_hour: Some(9), membership: false };
        assert_eq!(settings.reset_clock(&zone, utc(2026, 10, 5, 12, 0)).to_string(), "09:00");
        assert_eq!(settings.reset_clock(&zone, utc(2026, 12, 5, 12, 0)).to_string(), "09:00");
    }

    #[test]
    fn picking_the_hour_the_game_reads_as_follows_the_game_and_any_other_pins_it() {
        let zone = berlin();
        let summer = utc(2026, 10, 5, 12, 0);
        assert_eq!(TodoSettings::choice_of_hour(9, &zone, summer), None);
        assert_eq!(TodoSettings::choice_of_hour(10, &zone, summer), Some(10));
        let winter = utc(2026, 12, 5, 12, 0);
        assert_eq!(TodoSettings::choice_of_hour(8, &zone, winter), None);
        assert_eq!(TodoSettings::choice_of_hour(9, &zone, winter), Some(9));
    }

    #[test]
    fn a_stored_hour_stays_the_players_own() {
        let settings: TodoSettings = serde_json::from_str(r#"{"resetHour":6}"#).unwrap();
        assert_eq!(settings.reset_hour, Some(6));
        let settings: TodoSettings = serde_json::from_str(r#"{"membership":true}"#).unwrap();
        assert_eq!(settings.reset_hour, None);
    }
}
