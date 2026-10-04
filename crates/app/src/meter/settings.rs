/// The meter overlay's options, with the defaults of `aion2.overlay` in
/// `src/hooks/use-settings.ts`. Not persisted and not editable yet.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlaySettings {
    /// Card background opacity, 0–255.
    pub background_alpha: u8,
    /// 0.7–1.5; see [`OverlaySettings::scale`].
    pub content_scale: f32,
    pub layout: Layout,
    pub pct_mode: PctMode,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Full,
    /// One line that opens into the full card under the pointer.
    Capsule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl OverlaySettings {
    pub fn scale(&self) -> f32 {
        if self.content_scale.is_finite() {
            self.content_scale.clamp(0.7, 1.5)
        } else {
            1.0
        }
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
