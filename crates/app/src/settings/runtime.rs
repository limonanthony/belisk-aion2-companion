//! What the running meter and the disk say, for the parts of Settings that
//! show state instead of a stored choice: the capture tiles, packet recording,
//! recordings and replay, the capture check, personal bests and storage.
//!
//! The model is created when the Settings window opens and polls the meter's
//! cheap getters once a second until it is dropped with the window; views
//! observe it. Anything slow (the capture check, sizing the data folder) runs
//! on the background executor and shows as a state.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use aether_core::engine::meter::{CaptureBackend, DpsMeter};
use aether_core::preflight;
use aether_core::region;
use gpui::{Context, Entity, Task};

use super::files::{self, StorageSummary};
use super::model::global;
use crate::game_data;
use crate::history::record_time;
use crate::meter::{MeterModel, ReplayState};

const POLL_EVERY: Duration = Duration::from_secs(1);
const REPLAY_SPEED: f64 = 1.0;
/// Recordings the meter starts by itself are named with this prefix.
const AUTOMATIC_PREFIX: &str = "auto-";

/// The four tiles of the Connection group.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CaptureStatus {
    pub meter: MeterTile,
    /// The game's traffic has been found.
    pub game_found: bool,
    /// The game server's address, once found.
    pub game_address: Option<String>,
    /// The character the game named as yours.
    pub character: Option<String>,
    /// The region the traffic looks like, when one matched.
    pub region: Option<&'static str>,
    /// The server names (or ids) seen in the traffic.
    pub servers: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MeterTile {
    /// No meter in this run (a development capture).
    #[default]
    Stopped,
    /// Running; `via` names the capture backend, absent while a recording is
    /// being replayed.
    Running { via: Option<&'static str> },
    /// Capture could not start; the text says why.
    Unavailable(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordingProgress {
    pub packets: u64,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingItem {
    pub path: PathBuf,
    pub name: String,
    pub bytes: u64,
    /// "Today 21:42", as the history lists times.
    pub when: String,
    /// Started by the meter on a server no profile matches.
    pub unfamiliar_server: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ReplayProgress {
    #[default]
    Idle,
    Running { name: String },
    Finished { packets: u64 },
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BestRow {
    pub boss: String,
    pub character: String,
    pub dps: f64,
    pub achieved_at_ms: u64,
    pub when: String,
}

#[derive(Clone, Debug, Default)]
pub enum CaptureCheck {
    #[default]
    NotRun,
    Running,
    Done(preflight::Report),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum StorageState {
    #[default]
    NotLoaded,
    Loading,
    Loaded(StorageSummary),
}

pub struct RuntimeModel {
    meter: Option<Arc<DpsMeter>>,
    meter_model: Entity<MeterModel>,
    data_dir: Option<PathBuf>,
    pub capture: CaptureStatus,
    /// Packet recording progress while one is running.
    pub recording: Option<RecordingProgress>,
    /// Why the last start or stop of a recording failed.
    pub recording_error: Option<String>,
    /// Newest first.
    pub recordings: Vec<RecordingItem>,
    pub replay: ReplayProgress,
    /// Newest first.
    pub bests: Vec<BestRow>,
    pub capture_check: CaptureCheck,
    pub storage: StorageState,
    poll: Option<Task<()>>,
    check: Option<Task<()>>,
    sizing: Option<Task<()>>,
    scan: Option<Task<()>>,
    /// A dev capture draws the page as if a meter ran.
    #[cfg(feature = "probes")]
    pretend_meter: bool,
}

impl RuntimeModel {
    pub fn new(meter_model: Entity<MeterModel>, cx: &mut Context<Self>) -> Self {
        let meter = meter_model.read(cx).meter().cloned();
        let data_dir = global(cx).read(cx).data_dir().map(Path::to_path_buf);
        let mut model = Self {
            meter,
            meter_model,
            data_dir,
            capture: CaptureStatus::default(),
            recording: None,
            recording_error: None,
            recordings: Vec::new(),
            replay: ReplayProgress::Idle,
            bests: Vec::new(),
            capture_check: CaptureCheck::NotRun,
            storage: StorageState::NotLoaded,
            poll: None,
            check: None,
            sizing: None,
            scan: None,
            #[cfg(feature = "probes")]
            pretend_meter: false,
        };
        model.refresh(cx);
        model.poll = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(POLL_EVERY).await;
            if this.update(cx, |model, cx| model.refresh(cx)).is_err() {
                break;
            }
        }));
        model
    }

    /// Whether a recording can be played back: the meter has to be running,
    /// and one replay at a time.
    pub fn can_replay(&self) -> bool {
        self.meter_runs() && !matches!(self.replay, ReplayProgress::Running { .. })
    }

    /// Whether the controls that need the meter can be used at all.
    pub fn has_meter(&self) -> bool {
        #[cfg(feature = "probes")]
        if self.pretend_meter {
            return true;
        }
        self.meter.is_some()
    }

    fn meter_runs(&self) -> bool {
        #[cfg(feature = "probes")]
        if self.pretend_meter {
            return true;
        }
        self.meter.as_ref().is_some_and(|meter| meter.is_running())
    }

    /// For dev captures: stops the poll and the background work, and sets
    /// the state to draw, as if a meter were running.
    #[cfg(feature = "probes")]
    pub fn hold_for_capture(&mut self, cx: &mut Context<Self>, state: impl FnOnce(&mut Self)) {
        self.poll = None;
        self.scan = None;
        self.check = None;
        self.sizing = None;
        self.pretend_meter = true;
        state(self);
        cx.notify();
    }

    pub fn start_recording(&mut self, cx: &mut Context<Self>) {
        let Some(meter) = &self.meter else { return };
        self.recording_error = meter.start_packet_recording().err();
        self.refresh(cx);
    }

    pub fn stop_recording(&mut self, cx: &mut Context<Self>) {
        let Some(meter) = &self.meter else { return };
        self.recording_error = meter.stop_packet_recording().err();
        self.refresh(cx);
    }

    /// Plays a recording through the live meter at the pace it was captured.
    pub fn replay_recording(&mut self, item: &RecordingItem, cx: &mut Context<Self>) {
        let Some(meter) = &self.meter else { return };
        self.meter_model.update(cx, |model, _| model.replay = ReplayState::NotStarted);
        self.replay = match meter.replay_packet_recording_paced(item.path.to_string_lossy().into_owned(), REPLAY_SPEED) {
            Ok(()) => ReplayProgress::Running { name: item.name.clone() },
            Err(error) => ReplayProgress::Failed(error),
        };
        cx.notify();
    }

    pub fn cancel_replay(&mut self, cx: &mut Context<Self>) {
        if let Some(meter) = &self.meter {
            meter.cancel_packet_replay();
        }
        cx.notify();
    }

    /// Clears every personal best. The meter tells its overlay.
    pub fn reset_personal_bests(&mut self, cx: &mut Context<Self>) {
        if let Some(meter) = &self.meter {
            meter.reset_personal_bests();
        }
        self.refresh(cx);
    }

    /// Runs the start-up check again (elevation, Npcap, WinDivert).
    pub fn run_capture_check(&mut self, cx: &mut Context<Self>) {
        if matches!(self.capture_check, CaptureCheck::Running) {
            return;
        }
        self.capture_check = CaptureCheck::Running;
        self.check = Some(cx.spawn(async move |this, cx| {
            let report = cx.background_executor().spawn(async { preflight::run() }).await;
            let _ = this.update(cx, |model, cx| {
                model.capture_check = CaptureCheck::Done(report);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Sizes the data folder; call when the About page opens.
    pub fn load_storage(&mut self, cx: &mut Context<Self>) {
        if self.storage == StorageState::Loading {
            return;
        }
        self.storage = StorageState::Loading;
        let (data_dir, meter) = (self.data_dir.clone(), self.meter.clone());
        self.sizing = Some(cx.spawn(async move |this, cx| {
            let summary = cx
                .background_executor()
                .spawn(async move { files::storage_summary(data_dir.as_deref(), meter.as_deref()) })
                .await;
            let _ = this.update(cx, |model, cx| {
                model.storage = StorageState::Loaded(summary);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let capture = self.read_capture_status(cx);
        let recording = self.meter.as_ref().and_then(|meter| {
            let status = meter.packet_recording_status();
            status.recording.then_some(RecordingProgress { packets: status.packets, bytes: status.bytes })
        });
        self.scan_recordings(cx);
        let bests = self.read_bests();
        let replay = self.read_replay(cx);

        let changed = capture != self.capture
            || recording != self.recording
            || bests != self.bests
            || replay != self.replay;
        self.capture = capture;
        self.recording = recording;
        self.bests = bests;
        self.replay = replay;
        if changed {
            cx.notify();
        }
    }

    fn read_capture_status(&self, cx: &Context<Self>) -> CaptureStatus {
        let Some(meter) = &self.meter else {
            return CaptureStatus {
                meter: self.meter_model.read(cx).unavailable().map_or(MeterTile::Stopped, |why| MeterTile::Unavailable(why.to_string())),
                ..CaptureStatus::default()
            };
        };
        let region = region::status();
        let running = meter.is_running();
        CaptureStatus {
            meter: if running {
                MeterTile::Running {
                    via: meter.capture_backend().map(|backend| match backend {
                        CaptureBackend::Npcap => "Npcap",
                        CaptureBackend::WinDivert => "WinDivert",
                    }),
                }
            } else {
                MeterTile::Stopped
            },
            game_found: running && meter.has_game_traffic(),
            game_address: region.observed_server_ips.first().cloned(),
            character: meter.main_actor_name(),
            region: region.detected.map(|detected| detected.label()),
            servers: region.observed_server_ids.iter().map(|id| game_data::server_name(&id.to_string())).collect(),
        }
    }

    /// Lists the recordings (a directory read and one stat per file) on the
    /// background executor; one scan at a time, the next poll starts another.
    fn scan_recordings(&mut self, cx: &mut Context<Self>) {
        if self.scan.is_some() {
            return;
        }
        let Some(meter) = self.meter.clone() else { return };
        self.scan = Some(cx.spawn(async move |this, cx| {
            let recordings = cx.background_executor().spawn(async move { read_recordings(&meter) }).await;
            let _ = this.update(cx, |model, cx| {
                model.scan = None;
                if model.recordings != recordings {
                    model.recordings = recordings;
                    cx.notify();
                }
            });
        }));
    }

    fn read_bests(&self) -> Vec<BestRow> {
        let Some(meter) = &self.meter else { return Vec::new() };
        meter
            .personal_bests()
            .into_iter()
            .map(|best| BestRow {
                boss: best
                    .target_name
                    .filter(|name| !name.is_empty())
                    .or_else(|| meter.mobs().mob_name(best.mob_code))
                    .unwrap_or_else(|| format!("Mob {}", best.mob_code)),
                character: best.character,
                dps: best.dps,
                achieved_at_ms: best.achieved_at,
                when: record_time(best.achieved_at),
            })
            .collect()
    }

    /// A running replay ends with an event that the meter model folds in; this
    /// reads its verdict once the core says the replay is over.
    fn read_replay(&self, cx: &Context<Self>) -> ReplayProgress {
        let Some(meter) = &self.meter else { return ReplayProgress::Idle };
        let ReplayProgress::Running { name } = &self.replay else {
            return self.replay.clone();
        };
        if meter.is_replaying() {
            return self.replay.clone();
        }
        match &self.meter_model.read(cx).replay {
            ReplayState::Finished { packets } => ReplayProgress::Finished { packets: *packets },
            ReplayState::Failed(error) => ReplayProgress::Failed(error.clone()),
            ReplayState::NotStarted => ReplayProgress::Running { name: name.clone() },
        }
    }
}

fn modified_ms(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64)
}

fn read_recordings(meter: &DpsMeter) -> Vec<RecordingItem> {
    meter
        .list_packet_recordings()
        .into_iter()
        .map(|file| {
            let modified_ms = modified_ms(Path::new(&file.path));
            RecordingItem {
                unfamiliar_server: file.name.starts_with(AUTOMATIC_PREFIX),
                when: modified_ms.map_or_else(|| "--".to_string(), record_time),
                path: PathBuf::from(file.path),
                name: file.name,
                bytes: file.bytes,
            }
        })
        .collect()
}
