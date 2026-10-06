use serde::{Deserialize, Serialize};

use super::events::{EventId, EVENTS};

/// "Remind me ... before": the lead times Settings offers, in minutes.
pub const LEAD_MINUTES_OPTIONS: [u8; 4] = [1, 2, 3, 5];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReminderSettings {
    /// The master switch: off, no reminder appears and no sound plays.
    pub enabled: bool,
    pub events: ReminderEvents,
    /// How long before an event starts its reminder appears; one of
    /// [`LEAD_MINUTES_OPTIONS`].
    pub lead_minutes: u8,
    pub sound: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReminderEvents {
    pub shugo_festival: bool,
    pub dimensional_invasion: bool,
    pub rift_portals: bool,
    pub watcher_kaira: bool,
    pub artifact_siege: bool,
    pub abyss_field_bosses: bool,
    pub guardian_lord_nahma: bool,
    pub arena_of_tactics: bool,
    pub reward_keys: bool,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self { enabled: true, events: ReminderEvents::default(), lead_minutes: 1, sound: true }
    }
}

impl Default for ReminderEvents {
    fn default() -> Self {
        Self {
            shugo_festival: true,
            dimensional_invasion: true,
            rift_portals: true,
            watcher_kaira: false,
            artifact_siege: false,
            abyss_field_bosses: false,
            guardian_lord_nahma: false,
            arena_of_tactics: false,
            reward_keys: false,
        }
    }
}

impl ReminderSettings {
    /// The lead time pulled to the nearest offered one (the lower on a tie).
    pub fn clamped(mut self) -> Self {
        self.lead_minutes = LEAD_MINUTES_OPTIONS
            .into_iter()
            .min_by_key(|option| option.abs_diff(self.lead_minutes))
            .unwrap_or(1);
        self
    }

    pub fn event_enabled(&self, id: EventId) -> bool {
        *self.events.switch(id)
    }

    pub fn set_event_enabled(&mut self, id: EventId, on: bool) {
        *self.events.switch_mut(id) = on;
    }

    /// The events to remind about: none while the master switch is off.
    pub fn scheduled_events(&self) -> Vec<EventId> {
        if !self.enabled {
            return Vec::new();
        }
        EVENTS.iter().map(|info| info.id).filter(|&id| self.event_enabled(id)).collect()
    }
}

impl ReminderEvents {
    fn switch(&self, id: EventId) -> &bool {
        match id {
            EventId::ShugoFestival => &self.shugo_festival,
            EventId::DimensionalInvasion => &self.dimensional_invasion,
            EventId::RiftPortals => &self.rift_portals,
            EventId::WatcherKaira => &self.watcher_kaira,
            EventId::ArtifactSiege => &self.artifact_siege,
            EventId::AbyssFieldBosses => &self.abyss_field_bosses,
            EventId::GuardianLordNahma => &self.guardian_lord_nahma,
            EventId::ArenaOfTactics => &self.arena_of_tactics,
            EventId::RewardKeys => &self.reward_keys,
        }
    }

    fn switch_mut(&mut self, id: EventId) -> &mut bool {
        match id {
            EventId::ShugoFestival => &mut self.shugo_festival,
            EventId::DimensionalInvasion => &mut self.dimensional_invasion,
            EventId::RiftPortals => &mut self.rift_portals,
            EventId::WatcherKaira => &mut self.watcher_kaira,
            EventId::ArtifactSiege => &mut self.artifact_siege,
            EventId::AbyssFieldBosses => &mut self.abyss_field_bosses,
            EventId::GuardianLordNahma => &mut self.guardian_lord_nahma,
            EventId::ArenaOfTactics => &mut self.arena_of_tactics,
            EventId::RewardKeys => &mut self.reward_keys,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_remind_about_shugo_invasion_and_rifts_one_minute_ahead_with_sound() {
        let settings = ReminderSettings::default();
        assert!(settings.enabled && settings.sound);
        assert_eq!(settings.lead_minutes, 1);
        assert_eq!(
            settings.scheduled_events(),
            vec![EventId::ShugoFestival, EventId::DimensionalInvasion, EventId::RiftPortals]
        );
    }

    #[test]
    fn every_event_has_its_own_switch() {
        let mut settings = ReminderSettings::default();
        for info in &EVENTS {
            settings.set_event_enabled(info.id, false);
        }
        assert!(settings.scheduled_events().is_empty());
        for info in &EVENTS {
            settings.set_event_enabled(info.id, true);
            assert_eq!(settings.scheduled_events(), vec![info.id], "{}", info.id.name());
            settings.set_event_enabled(info.id, false);
        }
    }

    #[test]
    fn the_master_switch_silences_every_event() {
        let settings = ReminderSettings { enabled: false, ..ReminderSettings::default() };
        assert!(settings.scheduled_events().is_empty());
        assert!(settings.event_enabled(EventId::ShugoFestival), "the per-event choice is kept");
    }

    #[test]
    fn a_lead_time_off_the_list_moves_to_the_nearest_offered_one() {
        let lead = |minutes: u8| {
            ReminderSettings { lead_minutes: minutes, ..ReminderSettings::default() }.clamped().lead_minutes
        };
        assert_eq!([0, 1, 2, 3, 4, 5, 6, 200].map(lead), [1, 1, 2, 3, 3, 5, 5, 5]);
    }
}
