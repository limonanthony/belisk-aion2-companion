use serde::{Deserialize, Serialize};

use super::gain::{GainSource, LatestGain};

/// What Settings → Pets changes: which gains the soul tracker card shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PetsSettings {
    /// The master switch: off, no card appears.
    pub soul_tracker: bool,
    pub only_pinned: bool,
    /// Gains of pets that were already at their cap.
    pub maxed_pets: bool,
    pub black_market: bool,
}

impl Default for PetsSettings {
    fn default() -> Self {
        Self { soul_tracker: true, only_pinned: false, maxed_pets: true, black_market: true }
    }
}

impl PetsSettings {
    /// Whether the card shows `gain`; `pet_is_pinned` is whether the gain's pet
    /// is pinned on the gain's server.
    pub fn admits(&self, gain: &LatestGain, pet_is_pinned: bool) -> bool {
        self.soul_tracker
            && (!self.only_pinned || pet_is_pinned)
            && (self.maxed_pets || !gain.already_maxed())
            && (self.black_market || gain.source != GainSource::BlackMarket)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gain(source: GainSource, already_maxed: bool) -> LatestGain {
        LatestGain {
            sequence: 1,
            server_id: 1302,
            pet_id: 1086,
            amount: 1,
            level_before: if already_maxed { 3 } else { 2 },
            level: if already_maxed { 3 } else { 2 },
            souls: if already_maxed { 0 } else { 28 },
            source,
            captured_at: 100.0,
        }
    }

    #[test]
    fn the_defaults_show_everything_but_the_pin_filter() {
        let defaults = PetsSettings::default();
        assert!(defaults.soul_tracker && !defaults.only_pinned && defaults.maxed_pets && defaults.black_market);
        for source in [GainSource::Loot, GainSource::BlackMarket, GainSource::Other, GainSource::MaxedLoot] {
            assert!(defaults.admits(&gain(source, source == GainSource::MaxedLoot), false));
        }
    }

    #[test]
    fn the_master_switch_off_shows_nothing() {
        let settings = PetsSettings { soul_tracker: false, ..PetsSettings::default() };

        assert!(!settings.admits(&gain(GainSource::Loot, false), true));
    }

    #[test]
    fn only_pinned_pets_keeps_the_pinned_ones() {
        let settings = PetsSettings { only_pinned: true, ..PetsSettings::default() };

        assert!(settings.admits(&gain(GainSource::Loot, false), true));
        assert!(!settings.admits(&gain(GainSource::Loot, false), false));
    }

    #[test]
    fn pets_already_at_level_3_can_be_left_out_but_the_gain_that_reaches_it_is_kept() {
        let settings = PetsSettings { maxed_pets: false, ..PetsSettings::default() };
        let reaches_the_cap = LatestGain { level: 3, souls: 0, ..gain(GainSource::Loot, false) };

        assert!(!settings.admits(&gain(GainSource::MaxedLoot, true), false));
        assert!(!settings.admits(&gain(GainSource::Loot, true), false));
        assert!(settings.admits(&reaches_the_cap, false));
    }

    #[test]
    fn black_market_purchases_can_be_left_out_and_other_sources_stay() {
        let settings = PetsSettings { black_market: false, ..PetsSettings::default() };

        assert!(!settings.admits(&gain(GainSource::BlackMarket, false), false));
        assert!(settings.admits(&gain(GainSource::Loot, false), false));
        assert!(settings.admits(&gain(GainSource::Other, false), false));
    }

    #[test]
    fn the_filters_combine() {
        let settings = PetsSettings { only_pinned: true, maxed_pets: false, black_market: false, ..PetsSettings::default() };

        assert!(settings.admits(&gain(GainSource::Loot, false), true));
        assert!(!settings.admits(&gain(GainSource::BlackMarket, false), true));
        assert!(!settings.admits(&gain(GainSource::Loot, true), true));
        assert!(!settings.admits(&gain(GainSource::Loot, false), false));
    }
}
