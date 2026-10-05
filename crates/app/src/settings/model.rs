use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aether_core::config::{CaptureBackendPriority, DpsMeterConfig};
use gpui::{App, Context, Entity, Global, Task};
use serde::{Deserialize, Serialize};

use super::store;
use crate::meter::OverlaySettings;
use crate::reminders::ReminderSettings;

/// A change is written to disk once nothing has changed for this long, so a
/// slider drag causes one write.
const SAVE_AFTER: Duration = Duration::from_millis(400);

/// "End a fight after": the choices the page offers, in seconds, with 0 for
/// never. Any stored value from 60 s to an hour is kept as it is.
pub const END_FIGHT_AFTER_OPTIONS: [(u64, &str); 6] =
    [(60, "1 min"), (120, "2 min"), (300, "5 min"), (600, "10 min"), (1800, "30 min"), (0, "Never")];
const END_FIGHT_AFTER_RANGE: std::ops::RangeInclusive<u64> = 60..=3600;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub overlay: OverlaySettings,
    pub behaviour: BehaviourSettings,
    pub capture: CaptureSettings,
    pub windows: WindowSettings,
    pub reminders: ReminderSettings,
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
            hide_unknown_players: false,
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
}

impl AppSettings {
    /// The values a control or a stored file may have set, pulled into range.
    pub fn clamped(mut self) -> Self {
        self.overlay = self.overlay.clamped();
        self.reminders = self.reminders.clamped();
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
        base.capture_backend_priority = self.capture.backend_priority.clone();
        base.auto_record_unknown_server = self.capture.record_unfamiliar_servers;
        base.collect_mobs = self.capture.collect_mob_data;
        base
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
    save_error: Option<String>,
    load_notice: Option<String>,
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
    /// live in memory only.
    pub fn load(data_dir: Option<&Path>) -> Self {
        let Some(dir) = data_dir else {
            return Self::in_memory(AppSettings::default());
        };
        let file = store::path_in(dir);
        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let loaded = store::load(&file, now_secs);
        if let Some(notice) = &loaded.notice {
            eprintln!("{notice}");
        }
        Self {
            value: loaded.settings,
            file: Some(file),
            save: None,
            saved: true,
            save_error: None,
            load_notice: loaded.notice,
            revision: 0,
            written: Arc::default(),
        }
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
    pub fn load_notice(&self) -> Option<&str> {
        self.load_notice.as_deref()
    }

    /// The folder `settings.json` lives in; none for in-memory settings.
    pub fn data_dir(&self) -> Option<&Path> {
        self.file.as_deref().and_then(Path::parent)
    }

    /// Why the last write failed; cleared by the next one that succeeds.
    pub fn save_error(&self) -> Option<&str> {
        self.save_error.as_deref()
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
        self.update(cx, |settings| *settings = AppSettings::default());
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
                eprintln!("{error}");
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
                        eprintln!("{error}");
                        settings.save_error = Some(error);
                    }
                }
                cx.notify();
            });
        }));
    }
}

fn write_in_order(written: &Mutex<u64>, revision: u64, file: &Path, value: &AppSettings) -> Result<(), String> {
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
}
