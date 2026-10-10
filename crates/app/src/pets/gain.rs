use meter_core::pet_sources::SoulSource;
use meter_core::pets::MAX_LEVEL;

/// Where a gain came from, as far as the traffic proves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GainSource {
    Loot,
    BlackMarket,
    Other,
    /// A soul looted from a mob of a pet already at its cap: the game reports
    /// those only as counter gains, and the looted mob names the pet.
    MaxedLoot,
}

impl From<SoulSource> for GainSource {
    fn from(source: SoulSource) -> Self {
        match source {
            SoulSource::Loot => Self::Loot,
            SoulSource::BlackMarket => Self::BlackMarket,
            SoulSource::Other => Self::Other,
        }
    }
}

/// A soul gain the game sent, with the pet as it is after it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatestGain {
    /// Counts the gains applied in this run, so a gain that leaves the pet
    /// exactly as the one before (a pet at its cap) still reads as a new one.
    pub sequence: u64,
    pub server_id: u32,
    pub pet_id: u32,
    /// Souls gained; a maxed pet's loot counts as one.
    pub amount: u32,
    /// The pet's level before the gain, so the card can say a level was
    /// reached or the pet was unlocked.
    pub level_before: u32,
    pub level: u32,
    /// Souls toward the next level after the gain; 0 at the cap.
    pub souls: u32,
    pub source: GainSource,
    /// When the game's message was captured, in seconds since the Unix epoch.
    pub captured_at: f64,
}

impl LatestGain {
    /// The pet was at its cap before the gain (the gain that takes a pet to
    /// the cap is not this).
    pub fn already_maxed(&self) -> bool {
        self.level_before == MAX_LEVEL
    }

    /// The gain took the pet to a new level.
    pub fn reached_a_level(&self) -> bool {
        self.level > self.level_before
    }
}
