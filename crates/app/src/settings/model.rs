use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use meter_core::config::{CaptureBackendPriority, DpsMeterConfig};
use meter_core::identity::KnownCharacter;
use gpui::{App, Context, Entity, Global, Task};
use serde::{Deserialize, Serialize};

use super::{characters, store};
use crate::changelog::Version;
use crate::i18n::{fl, LanguageChoice};
use crate::meter::OverlaySettings;
use crate::pets::PetsSettings;
use crate::reminders::ReminderSettings;
use crate::stored_file::{LoadNotice, SaveError};
use crate::todos::TodoSettings;
use crate::update::UpdateSettings;

/// A change is written to disk once nothing has changed for this long, so a
/// slider drag causes one write.
const SAVE_AFTER: Duration = Duration::from_millis(400);

/// "End a fight after": the choices the page offers, in seconds, with 0 for
/// never. Any stored value from 60 s to an hour is kept as it is.
pub const END_FIGHT_AFTER_OPTIONS: [u64; 6] = [60, 120, 300, 600, 1800, 0];
const END_FIGHT_AFTER_RANGE: std::ops::RangeInclusive<u64> = 60..=3600;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub overlay: OverlaySettings,
    pub behaviour: BehaviourSettings,
    pub capture: CaptureSettings,
    pub windows: WindowSettings,
    pub reminders: ReminderSettings,
    pub todos: TodoSettings,
    pub pets: PetsSettings,
    pub updates: UpdateSettings,
    /// Characters the game named as the player, most recent first; not a
    /// setting the page shows, and kept when the settings are reset.
    #[serde(deserialize_with = "characters::deserialize_lenient")]
    pub known_characters: Vec<KnownCharacter>,
    /// The newest version this player has run and been told about; none in a
    /// file written before it was remembered. Kept when the settings are reset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_version: Option<Version>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CountMode {
    #[default]
    AllTargets,
    BossOnly,
}

/// What the meter counts, and when a fight ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BehaviourSettings {
    pub count: CountMode,
    /// Treat very high-HP monsters missing from the boss table as bosses.
    pub detect_possible_bosses: bool,
    pub training_dummy_only: bool,
    pub hide_unknown_players: bool,
    /// Seconds without a hit of yours before the fight is saved and the meter
    /// clears; 0 never ends it.
    pub end_fight_after_secs: u64,
}

impl Default for BehaviourSettings {
    fn default() -> Self {
        Self {
            count: CountMode::AllTargets,
            detect_possible_bosses: true,
            training_dummy_only: false,
            hide_unknown_players: true,
            end_fight_after_secs: 300,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CaptureSettings {
    /// Read when the meter starts: a change applies at the next launch.
    pub backend_priority: CaptureBackendPriority,
    /// Record the first two minutes on a server no profile matches.
    pub record_unfamiliar_servers: bool,
    /// Keep the game traffic of every live session in rolling files.
    pub record_every_session: bool,
    /// Record every mob fought into `mobs.json`.
    pub collect_mob_data: bool,
    /// Write the meter's decisions to the log as `[meter]` lines.
    pub decision_log: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            backend_priority: CaptureBackendPriority::default(),
            record_unfamiliar_servers: true,
            record_every_session: false,
            collect_mob_data: false,
            decision_log: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DetailPlacement {
    /// Next to the overlay.
    #[default]
    Beside,
    /// In the middle of the screen.
    Centred,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WindowSettings {
    /// Read each time the detail window opens.
    pub detail_opens: DetailPlacement,
    /// Off: only the toolbar opens at launch.
    pub open_meter_at_launch: bool,
    /// The meter steps aside between fights and comes back at your first hit.
    pub hide_meter_between_fights: bool,
    /// The first-open prompt was answered, either way; it is never asked again.
    pub launch_prompt_answered: bool,
    /// The toolbar's top-left corner in device pixels, where it was left; none
    /// opens it at the top centre of the primary monitor.
    pub toolbar_at: Option<ScreenPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralSettings {
    pub language: LanguageChoice,
}

impl AppSettings {
    /// The values a control or a stored file may have set, pulled into range.
    pub fn clamped(mut self) -> Self {
        self.overlay = self.overlay.clamped();
        self.reminders = self.reminders.clamped();
        self.todos = self.todos.clamped();
        self.known_characters = characters::truncated(self.known_characters);
        if self.behaviour.end_fight_after_secs != 0 {
            self.behaviour.end_fight_after_secs =
                self.behaviour.end_fight_after_secs.clamp(*END_FIGHT_AFTER_RANGE.start(), *END_FIGHT_AFTER_RANGE.end());
        }
        self
    }

    /// `base` with the options the core takes from these settings. Everything
    /// else in `base` is left as it is: the PvP window, for one, switches
    /// `pvp_mode_on` on the running meter.
    pub fn backend_config(&self, mut base: DpsMeterConfig) -> DpsMeterConfig {
        base.boss_only = self.behaviour.count == CountMode::BossOnly;
        base.show_possible_boss = self.behaviour.detect_possible_bosses;
        base.my_muzhuang_only = self.behaviour.training_dummy_only;
        base.hide_unknown_players = self.behaviour.hide_unknown_players;
        base.max_player_count = self.overlay.players_shown;
        base.idle_reset_secs = self.behaviour.end_fight_after_secs;
        base.hide_when_idle = self.windows.hide_meter_between_fights;
        base.capture_backend_priority = self.capture.backend_priority.clone();
        base.auto_record_unknown_server = self.capture.record_unfamiliar_servers;
        base.record_every_session = self.capture.record_every_session;
        base.collect_mobs = self.capture.collect_mob_data;
        base
    }
}

/// A choice of "End a fight after" as the page words it: "5 min", "Never".
/// A stored value outside the offered choices reads as its minutes.
pub fn end_fight_after_label(secs: u64) -> String {
    match secs {
        0 => fl!("settings-end-fight-never"),
        secs if secs % 60 == 0 => {
            let minutes = (secs / 60) as i64;
            fl!("duration-minutes", minutes = minutes)
        }
        secs => fl!("duration-minutes", minutes = format!("{:.1}", secs as f64 / 60.)),
    }
}

/// The settings the whole app reads, and the one place they are changed.
/// Views and the live application observe this entity; a change is clamped,
/// kept, and written to `settings.json` a moment later.
pub struct Settings {
    value: AppSettings,
    file: Option<PathBuf>,
    save: Option<Task<()>>,
    saved: bool,
    save_error: Option<SaveError>,
    load_notice: Option<LoadNotice>,
    /// Counts changes; a write carries the number of the value it holds.
    revision: u64,
    /// The revision last written. Writers take the lock, so an older value,
    /// finishing late, never replaces a newer one on disk.
    written: Arc<Mutex<u64>>,
}

struct SettingsHandle(Entity<Settings>);

impl Global for SettingsHandle {}

impl Settings {
    /// Reads `settings.json` from `data_dir`; without a folder the settings
    /// live in memory only. A first run, with no file yet, is stamped with this
    /// build's version and saved at once, so What's new never greets a new
    /// player with notes about versions they never ran.
    pub fn load(data_dir: Option<&Path>) -> Self {
        let Some(dir) = data_dir else {
            return Self::in_memory(AppSettings::default());
        };
        let file = store::path_in(dir);
        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let loaded = store::load(&file, now_secs);
        if let Some(notice) = &loaded.notice {
            crate::app_log::error(notice.to_string());
        }
        let mut settings = Self {
            value: loaded.settings,
            file: Some(file),
            save: None,
            saved: true,
            save_error: None,
            load_notice: loaded.notice,
            revision: 0,
            written: Arc::default(),
        };
        if loaded.fresh_install {
            settings.value.last_seen_version = Some(Version::current());
            settings.saved = false;
            settings.flush();
        }
        settings
    }

    /// Settings that are never read from or written to disk: the replay and
    /// capture modes, whose output must not depend on, or change, the user's
    /// file.
    pub fn in_memory(value: AppSettings) -> Self {
        Self {
            value: value.clamped(),
            file: None,
            save: None,
            saved: true,
            save_error: None,
            load_notice: None,
            revision: 0,
            written: Arc::default(),
        }
    }

    pub fn get(&self) -> &AppSettings {
        &self.value
    }

    /// Why the stored file was not used at start (it was set aside), if so.
    pub fn load_notice(&self) -> Option<&LoadNotice> {
        self.load_notice.as_ref()
    }

    /// The folder `settings.json` lives in; none for in-memory settings.
    pub fn data_dir(&self) -> Option<&Path> {
        self.file.as_deref().and_then(Path::parent)
    }

    /// Why the last write failed; cleared by the next one that succeeds.
    pub fn save_error(&self) -> Option<&SaveError> {
        self.save_error.as_ref()
    }

    /// Applies `change` to a copy, clamps it, and keeps it if it differs.
    pub fn update(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut AppSettings)) {
        let mut next = self.value.clone();
        change(&mut next);
        let next = next.clamped();
        if next == self.value {
            return;
        }
        self.value = next;
        self.revision += 1;
        self.schedule_save(cx);
        cx.notify();
    }

    /// Everything back to its default (Settings → About → Reset settings).
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        let known_characters = self.value.known_characters.clone();
        let last_seen_version = self.value.last_seen_version;
        self.update(cx, |settings| {
            *settings = AppSettings { known_characters, last_seen_version, ..AppSettings::default() };
        });
    }

    /// Writes any change not yet on disk now, on this thread; used when the
    /// app quits.
    pub fn flush(&mut self) {
        self.save = None;
        let Some(file) = self.file.as_ref().filter(|_| !self.saved) else {
            return;
        };
        match write_in_order(&self.written, self.revision, file, &self.value) {
            Ok(()) => {
                self.saved = true;
                self.save_error = None;
            }
            Err(error) => {
                crate::app_log::error(error.to_string());
                self.save_error = Some(error);
            }
        }
    }

    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.saved = false;
        let Some(file) = self.file.clone() else {
            return;
        };
        self.save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_AFTER).await;
            let Ok(Some((revision, value, written))) = this.update(cx, |settings, _| {
                (!settings.saved).then(|| (settings.revision, settings.value.clone(), Arc::clone(&settings.written)))
            }) else {
                return;
            };
            let result = cx
                .background_executor()
                .spawn(async move { write_in_order(&written, revision, &file, &value) })
                .await;
            let _ = this.update(cx, |settings, cx| {
                match result {
                    Ok(()) => {
                        settings.saved = settings.saved || settings.revision == revision;
                        settings.save_error = None;
                    }
                    Err(error) => {
                        crate::app_log::error(error.to_string());
                        settings.save_error = Some(error);
                    }
                }
                cx.notify();
            });
        }));
    }
}

fn write_in_order(written: &Mutex<u64>, revision: u64, file: &Path, value: &AppSettings) -> Result<(), SaveError> {
    let mut last = written.lock().unwrap_or_else(PoisonError::into_inner);
    if revision < *last {
        return Ok(());
    }
    store::save(file, value)?;
    *last = revision;
    Ok(())
}

/// Makes `settings` the app's settings, for [`global`].
pub fn install(settings: &Entity<Settings>, cx: &mut App) {
    cx.set_global(SettingsHandle(settings.clone()));
}

pub fn global(cx: &App) -> Entity<Settings> {
    cx.global::<SettingsHandle>().0.clone()
}

/// The game named the player's character: remember it, unless it is already
/// remembered as it is.
pub fn remember_characters(cx: &mut App, seen: Vec<KnownCharacter>) {
    global(cx).update(cx, |settings, cx| {
        settings.update(cx, |settings| {
            for character in seen {
                characters::remember(&mut settings.known_characters, character);
            }
        });
    });
}

/// The player has run this build and been told what it brings (What's new was
/// closed, or there was nothing to show): later notes start after it. An older
/// remembered version is replaced; a newer one, from a build the player went
/// back from, is kept.
pub fn mark_version_seen(cx: &mut App) {
    let current = Version::current();
    global(cx).update(cx, |settings, cx| {
        settings.update(cx, |settings| {
            if settings.last_seen_version.is_none_or(|seen| seen < current) {
                settings.last_seen_version = Some(current);
            }
        });
    });
}

/// The meter's capsule button.
pub fn toggle_layout(cx: &mut App) {
    global(cx).update(cx, |settings, cx| {
        settings.update(cx, |settings| settings.overlay.layout = settings.overlay.layout.toggled());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_gets_the_counting_options_and_keeps_the_rest() {
        let mut settings = AppSettings::default();
        settings.behaviour.count = CountMode::BossOnly;
        settings.behaviour.detect_possible_bosses = false;
        settings.behaviour.end_fight_after_secs = 0;
        settings.overlay.players_shown = 15;
        settings.capture.collect_mob_data = true;
        let running = DpsMeterConfig { pvp_mode_on: true, dps_snapshot_interval_ms: 200, ..DpsMeterConfig::default() };

        let config = settings.backend_config(running);

        assert!(config.boss_only && !config.show_possible_boss && config.collect_mobs);
        assert_eq!((config.idle_reset_secs, config.max_player_count), (0, 15));
        assert!(config.pvp_mode_on, "the PvP window's switch survives");
        assert_eq!(config.dps_snapshot_interval_ms, 200);
    }

    #[test]
    fn the_defaults_keep_the_boss_rule_on_and_the_collection_off() {
        let config = AppSettings::default().backend_config(DpsMeterConfig::default());
        assert!(config.show_possible_boss);
        assert!(!config.collect_mobs && !config.boss_only && !config.my_muzhuang_only);
        assert_eq!(config.idle_reset_secs, 300);
        assert_eq!(config.max_player_count, 10);
    }

    #[test]
    fn the_meter_stays_on_screen_between_fights_unless_asked_and_the_core_is_told() {
        assert!(!AppSettings::default().windows.hide_meter_between_fights);
        let running = DpsMeterConfig { hide_when_idle: true, ..DpsMeterConfig::default() };
        assert!(!AppSettings::default().backend_config(running).hide_when_idle);

        let mut settings = AppSettings::default();
        settings.windows.hide_meter_between_fights = true;

        assert!(settings.backend_config(DpsMeterConfig::default()).hide_when_idle);
    }

    #[test]
    fn recording_every_session_is_off_by_default_and_the_core_is_told_when_it_is_on() {
        assert!(!AppSettings::default().capture.record_every_session);
        assert!(!AppSettings::default().backend_config(DpsMeterConfig::default()).record_every_session);

        let mut settings = AppSettings::default();
        settings.capture.record_every_session = true;

        assert!(settings.backend_config(DpsMeterConfig::default()).record_every_session);
    }

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("aion-last-seen-{name}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_first_run_is_stamped_with_this_version_and_saved_at_once() {
        let dir = TestDir::new("fresh");
        let settings = Settings::load(Some(&dir.0));
        assert_eq!(settings.get().last_seen_version, Some(Version::current()));

        let stored = std::fs::read_to_string(store::path_in(&dir.0)).unwrap();
        assert!(stored.contains(&format!("\"lastSeenVersion\": \"{}\"", Version::current())), "{stored}");
        assert_eq!(Settings::load(Some(&dir.0)).get().last_seen_version, Some(Version::current()));
    }

    #[test]
    fn a_file_from_before_the_key_has_no_last_seen_version_and_stays_that_way() {
        let dir = TestDir::new("old");
        std::fs::write(store::path_in(&dir.0), r#"{ "version": 2, "general": { "language": "french" } }"#).unwrap();

        let settings = Settings::load(Some(&dir.0));

        assert_eq!(settings.get().last_seen_version, None);
        assert_eq!(settings.get().general.language, LanguageChoice::French);
        let stored = std::fs::read_to_string(store::path_in(&dir.0)).unwrap();
        assert!(!stored.contains("lastSeenVersion"), "an unchanged old file is not rewritten");
    }

    #[test]
    fn the_last_seen_version_survives_a_round_trip_and_an_unreadable_value_is_dropped() {
        let mut settings = AppSettings::default();
        let encoded = crate::stored_file::encode_versioned(&settings, store::SCHEMA_VERSION).unwrap();
        assert!(!String::from_utf8(encoded).unwrap().contains("lastSeenVersion"));
        settings.last_seen_version = Some(Version::new(1, 2, 3));
        let dir = TestDir::new("round-trip");
        store::save(&store::path_in(&dir.0), &settings).unwrap();
        assert_eq!(Settings::load(Some(&dir.0)).get().last_seen_version, Some(Version::new(1, 2, 3)));

        std::fs::write(store::path_in(&dir.0), r#"{ "version": 2, "lastSeenVersion": "one" }"#).unwrap();
        assert_eq!(Settings::load(Some(&dir.0)).get().last_seen_version, None);
    }

    #[gpui::test]
    fn marking_a_version_seen_moves_it_forward_only_and_a_reset_keeps_it(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;

        let settings = cx.new(|_| Settings::in_memory(AppSettings::default()));
        cx.update(|cx| install(&settings, cx));
        let seen = |cx: &mut gpui::TestAppContext| settings.read_with(cx, |settings, _| settings.get().last_seen_version);
        assert_eq!(seen(cx), None);

        cx.update(mark_version_seen);
        assert_eq!(seen(cx), Some(Version::current()));

        let newer = Version::new(99, 0, 0);
        cx.update(|cx| settings.update(cx, |settings, cx| settings.update(cx, |all| all.last_seen_version = Some(newer))));
        cx.update(mark_version_seen);
        assert_eq!(seen(cx), Some(newer), "going back to an older build does not rewind it");

        cx.update(|cx| settings.update(cx, |settings, cx| settings.reset(cx)));
        assert_eq!(seen(cx), Some(newer), "Reset settings does not bring What's new back");
    }
}
