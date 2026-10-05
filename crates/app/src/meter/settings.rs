use serde::{Deserialize, Serialize};

/// How many players the boards can list; also the core's player limit.
pub const PLAYERS_SHOWN_RANGE: std::ops::RangeInclusive<usize> = 5..=20;

/// The meter overlay's options, with the defaults of `aion2.overlay` in
/// `src/hooks/use-settings.ts`. Stored in `settings.json` by the settings
/// layer, which applies a change to the live overlay at once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    /// Card background opacity, 0–255.
    pub background_alpha: u8,
    /// 0.7–1.5; see [`OverlaySettings::scale`].
    pub content_scale: f32,
    pub layout: Layout,
    pub pct_mode: PctMode,
    pub players_shown: usize,
    pub show_player_name: bool,
    pub show_server: bool,
    pub show_damage: bool,
    pub show_dps: bool,
    pub show_combat_power: bool,
    pub show_boss_hp: bool,
    pub mask_nicknames: bool,
    pub show_fight_summary: bool,
    pub show_personal_best: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    Full,
    /// One line that opens into the full card under the pointer.
    Capsule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PctMode {
    /// Share of the damage the target took, from everyone.
    Contribution,
    /// Share of the damage the listed players dealt.
    Share,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            background_alpha: 150,
            content_scale: 1.0,
            layout: Layout::Full,
            pct_mode: PctMode::Contribution,
            players_shown: 10,
            show_player_name: true,
            show_server: false,
            show_damage: false,
            show_dps: true,
            show_combat_power: true,
            show_boss_hp: true,
            mask_nicknames: false,
            show_fight_summary: true,
            show_personal_best: true,
        }
    }
}

impl Layout {
    pub fn toggled(self) -> Self {
        match self {
            Self::Full => Self::Capsule,
            Self::Capsule => Self::Full,
        }
    }
}

impl OverlaySettings {
    pub fn scale(&self) -> f32 {
        if self.content_scale.is_finite() {
            self.content_scale.clamp(0.7, 1.5)
        } else {
            1.0
        }
    }

    /// The values a control or a stored file may have set, pulled into range.
    pub fn clamped(mut self) -> Self {
        self.content_scale = self.scale();
        self.players_shown = self.players_shown.clamp(*PLAYERS_SHOWN_RANGE.start(), *PLAYERS_SHOWN_RANGE.end());
        self
    }

    /// Whether `other` differs from these only in how the card is painted and
    /// fitted (opacity, scale), which changes nothing the boards prepared.
    pub fn same_boards_as(&self, other: &Self) -> bool {
        let mut painted_alike = self.clone();
        painted_alike.background_alpha = other.background_alpha;
        painted_alike.content_scale = other.content_scale;
        painted_alike == *other
    }

    pub fn display_name(&self, name: &str) -> String {
        if self.mask_nicknames {
            crate::format::mask(name)
        } else {
            name.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_clamped_to_the_settings_range() {
        let mut settings = OverlaySettings::default();
        assert_eq!(settings.scale(), 1.0);
        settings.content_scale = 3.0;
        assert_eq!(settings.scale(), 1.5);
        settings.content_scale = 0.2;
        assert_eq!(settings.scale(), 0.7);
        settings.content_scale = f32::NAN;
        assert_eq!(settings.scale(), 1.0);
    }
}
