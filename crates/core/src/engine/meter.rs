use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sysinfo::{
    get_current_pid, CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate,
    RefreshKind, System,
};

use crate::capture::capturer::{CapturedPacket, PcapCapturer};
use crate::capture::channel::Channel;
use crate::capture::dispatcher::CaptureDispatcher;
use crate::capture::drop_report::{DropReporter, PacketDropCounters, PacketDrops};
use crate::capture::ping_tracker::PingTracker;
use crate::capture::recorder::{
    list_recordings, prune_recordings, replay_recording, PacketRecorder, RecordingFile,
    RecordingStatus, ReplayPacer,
};
use crate::capture::recording_queue::{RecordingQueue, RECORDING_QUEUE_CAPACITY};
use crate::capture::rolling_recorder::RollingRecorder;
use crate::capture::windivert_capturer::WinDivertCapturer;
use crate::config::{CaptureBackendPriority, DpsMeterConfig, SharedDpsMeterConfig};
use crate::engine::calculator::DpsCalculator;
use crate::history::HistoryStore;
use crate::personal_best::{PersonalBest, PersonalBestStore};
use crate::models::combat::{CombatSnapshot, PvpCombatStatsRow, PvpWatchInfoResponse};
use crate::models::diagnostics::MemorySnapshot;
use crate::models::session::SessionSummary;
use crate::mobs::catalog::{MobCatalog, MobStorage};
use crate::report::fight_report::{FightOutcome, FightReport};
use crate::storage::boss_wipe::BossWipe;
use crate::storage::data_storage::DataStorage;
use crate::events::{EventSink, MeterEvent, WipedAttempt};
use crate::logger::AppLogger;

const STALE_ASSEMBLER_IDLE_SECS: u64 = 30;
/// Replay reads from disk far faster than the dispatcher drains, so it waits
/// whenever the queue runs this deep, half of `PACKET_CHANNEL_CAPACITY`.
/// `try_send` drops the packet it is handed when the channel is full, and a
/// silently lossy replay is worse than a slow one.
const REPLAY_BACKPRESSURE_LIMIT: usize = 4_096;
/// A paced replay sleeps in slices this long so a cancel is noticed promptly.
const REPLAY_PACING_SLICE: Duration = Duration::from_millis(50);

/// Packets between capture and the dispatcher. The worst burst in the
/// recordings is 3,900 packets/s of unfiltered TCP (game traffic peaks near
/// 150), so this is about 2 s of that burst or 55 s of game traffic; a full
/// queue is counted, not silent. About 6.5 MB at the 800-byte average payload
/// seen, 72 MB at the largest game payload (8.8 KB).
const PACKET_CHANNEL_CAPACITY: isize = 8_192;

/// Automatic recordings: how long each runs, and how many are kept.
const AUTO_RECORDING_PREFIX: &str = "auto";
const AUTO_RECORDING_DURATION: Duration = Duration::from_secs(120);
const AUTO_RECORDINGS_KEPT: usize = 5;

/// All the recording flushes of one stop or quit share this wait. The app ends
/// the process 3 s after the quit hooks start (`quit.rs`), and the history,
/// personal bests and mobs are written before the recordings are waited for.
const RECORDING_QUIT_FLUSH_BUDGET: Duration = Duration::from_millis(1_500);
/// A recording stopped on its own (the player's, an automatic one) can afford
/// to wait longer for its last packets.
const RECORDING_STOP_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the overlay stays up after Reset before hiding again, so the
/// click visibly did something.
const RESET_PEEK_SECS: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CaptureBackend {
    WinDivert,
    Npcap,
}

/// Why live capture could not start. The drivers' own errors are kept as they
/// reported them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureStartError {
    /// Neither driver could capture; `tried_first` is the one the backend
    /// priority opened first.
    NoBackend {
        tried_first: CaptureBackend,
        npcap: String,
        windivert: String,
    },
}

impl std::fmt::Display for CaptureStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoBackend { tried_first: CaptureBackend::Npcap, npcap, windivert } => write!(
                f,
                "No capture backend available; Npcap: {npcap}; WinDivert: {windivert}"
            ),
            Self::NoBackend { tried_first: CaptureBackend::WinDivert, npcap, windivert } => write!(
                f,
                "No capture backend available; WinDivert: {windivert}; Npcap: {npcap}"
            ),
        }
    }
}

impl std::error::Error for CaptureStartError {}

/// One automatic recording per meter session, started when game traffic
/// arrives from a server no fingerprint matches.
#[derive(Debug, Default)]
struct AutoRecordState {
    active_since: Option<Instant>,
    done: bool,
}

fn now_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub struct DpsMeter {
    events: Arc<dyn EventSink>,
    fight: Arc<FightCloser>,
    config: SharedDpsMeterConfig,
    logger: Arc<AppLogger>,
    data_storage: Arc<DataStorage>,
    calculator: Arc<DpsCalculator>,
    ping_tracker: Arc<PingTracker>,
    packet_channel: Channel<CapturedPacket>,
    recorder: Arc<PacketRecorder>,
    rolling_recorder: Arc<RollingRecorder>,
    session_recording: Arc<RecordingQueue>,
    rolling_recording: Arc<RecordingQueue>,
    drop_counters: PacketDropCounters,
    drop_reporter: Arc<Mutex<DropReporter>>,
    recordings_dir: PathBuf,
    replaying: Arc<AtomicBool>,
    windivert_capturer: WinDivertCapturer,
    pcap_capturer: PcapCapturer,
    active_capture_backend: Arc<Mutex<Option<CaptureBackend>>>,
    dispatcher: CaptureDispatcher,
    running: Arc<AtomicBool>,
    snapshot_running: Arc<AtomicBool>,
    memory_snapshot_running: Arc<AtomicBool>,
    snapshot_thread: Mutex<Option<JoinHandle<()>>>,
    memory_snapshot_thread: Mutex<Option<JoinHandle<()>>>,
    history: Arc<HistoryStore>,
    personal_bests: Arc<PersonalBestStore>,
    /// Until when the overlay stays up regardless of the idle rule: after
    /// Reset, and whenever the host asks with `peek_overlay`.
    peek_until: Arc<Mutex<Option<Instant>>>,
    auto_record: Arc<Mutex<AutoRecordState>>,
}

/// What a background thread needs to end a fight. The snapshot loop and the
/// main-actor callback run on threads that hold no `&DpsMeter`, so the pieces
/// a reset touches are shared out rather than reached through the meter.
struct FightCloser {
    events: Arc<dyn EventSink>,
    logger: Arc<AppLogger>,
    config: SharedDpsMeterConfig,
    data_storage: Arc<DataStorage>,
    calculator: Arc<DpsCalculator>,
    history: Arc<HistoryStore>,
    personal_bests: Arc<PersonalBestStore>,
    running: Arc<AtomicBool>,
    /// The frame the overlay was last sent, to tell whether the next one has
    /// anything new. `None` after a reset or a wipe, so the next frame with
    /// damage goes out.
    last_sent: Mutex<Option<CombatSnapshot>>,
}

impl FightCloser {
    fn snapshot(&self, target_damage_threshold: u64) -> Option<CombatSnapshot> {
        let cfg = self.config.read().unwrap();
        self.calculator.get_dps_snapshot(
            target_damage_threshold,
            cfg.hide_unknown_players,
            cfg.max_player_count,
        )
    }

    /// The fight is over: it goes to history if you were in it; a meter
    /// holding only other people's fights nearby is cleared without a record.
    fn end_fight(&self, was_in_fight: bool, reason: &str) {
        if was_in_fight {
            self.logger.info(format!("{reason}: fight saved and meter reset"));
            self.reset(true);
        } else {
            self.clear_runtime_state_nopacket();
            self.emit_empty_snap();
        }
    }

    /// Closes every boss attempt the game ended without a kill. Cheap when
    /// there is none.
    fn end_wiped_attempts(&self) {
        if !self.data_storage.has_boss_wipes() {
            return;
        }
        for wipe in self.data_storage.take_boss_wipes() {
            self.end_wiped_attempt(wipe);
        }
    }

    /// A wiped attempt is saved to history and sent to the host with the
    /// health the boss had left, then taken off the meter so the next pull on
    /// the boss counts from zero. It counts for no personal best. An attempt
    /// you did not hit the boss in is only taken off.
    fn end_wiped_attempt(&self, wipe: BossWipe) {
        let Some(mut snapshot) = self.snapshot(0) else {
            self.data_storage.forget_target(wipe.target_id);
            return;
        };
        if !self.you_hit(&snapshot, wipe.target_id) {
            self.data_storage.forget_target(wipe.target_id);
            return;
        }
        let hp_left = u32::try_from(wipe.hp_left).unwrap_or(u32::MAX);
        for target in snapshot
            .combat_infos
            .target_infos
            .values_mut()
            .chain(snapshot.last_target_info.as_mut())
            .filter(|target| target.id == wipe.target_id)
        {
            target.current_hp = Some(hp_left);
        }
        if let Some(target) = snapshot.combat_infos.target_infos.get(&wipe.target_id) {
            self.data_storage
                .mobs()
                .record_fight(&HashMap::from([(wipe.target_id, target.clone())]));
        }
        self.logger.info(format!(
            "boss wipe: target {} left at {}/{} ({:?}), attempt saved and taken off the meter",
            wipe.target_id, wipe.hp_left, wipe.max_hp, wipe.cause
        ));

        let dealers: Vec<u32> = snapshot
            .by_target_player_stats
            .get(&wipe.target_id)
            .map(|players| players.keys().copied().collect())
            .unwrap_or_default();
        let ledger = snapshot
            .combat_infos
            .target_infos
            .get(&wipe.target_id)
            .and_then(|target| self.calculator.fight_ledger(target, &dealers));
        let report = FightReport::from_snapshot(&snapshot, wipe.target_id, ledger.as_ref());
        let record = self.history.save_wiped(snapshot.clone(), wipe.target_id, |target, dealers| {
            self.calculator.fight_ledger(target, dealers)
        });
        if record.is_some() {
            self.events.emit(MeterEvent::HistoryUpdated);
        }
        self.data_storage.forget_target(wipe.target_id);
        self.forget_sent_frame();
        self.events.emit(MeterEvent::BossWiped(WipedAttempt {
            target_id: wipe.target_id,
            snapshot,
            report: report.map(|mut report| {
                report.outcome = FightOutcome::Wiped { hp_left, max_hp: u32::try_from(wipe.max_hp).unwrap_or(u32::MAX) };
                report
            }),
            record_id: record.map(|record| record.id),
        }));
        match self.snapshot(0).filter(|snapshot| snapshot.total_damage > 0) {
            Some(rest) => {
                self.send_frame_if_changed(rest);
            }
            None => self.emit_empty_snap(),
        }
    }

    /// Whether the local player hit `target_id`. Before the game has named the
    /// player nobody can be ruled out.
    fn you_hit(&self, snapshot: &CombatSnapshot, target_id: u32) -> bool {
        let Some(main_actor_id) = self.data_storage.main_actor_id() else {
            return true;
        };
        let Some(dealers) = snapshot.by_target_player_stats.get(&target_id) else {
            return false;
        };
        dealers.contains_key(&main_actor_id)
            || self
                .data_storage
                .own_actor_ids()
                .iter()
                .any(|actor_id| dealers.contains_key(actor_id))
    }

    fn reset(&self, emit_empty: bool) {
        // Capture before clearing
        self.save_fight_to_history();
        self.clear_runtime_state_nopacket();
        self.logger.info(format!(
            "dps meter runtime state reset (running={})",
            self.running.load(Ordering::SeqCst)
        ));
        if emit_empty {
            self.emit_empty_snap();
        }
    }

    /// Save what the meter holds to history, and learn personal bests from it.
    fn save_fight_to_history(&self) {
        let Some(snapshot) = self.snapshot(0) else {
            return;
        };
        self.data_storage
            .mobs()
            .record_fight(&snapshot.combat_infos.target_infos);
        if snapshot.total_damage == 0 {
            return;
        }
        let saved = self.history.save_and_clear(snapshot, |target, dealers| {
            self.calculator.fight_ledger(target, dealers)
        });
        self.events.emit(MeterEvent::HistoryUpdated);
        if self.personal_bests.record_and_save(&saved) {
            self.events.emit(MeterEvent::PersonalBestsUpdated);
        }
    }

    fn clear_runtime_state_nopacket(&self) {
        self.data_storage.clear();
        self.forget_sent_frame();
    }

    fn forget_sent_frame(&self) {
        *self.last_sent.lock().unwrap() = None;
    }

    /// Sends the host the frame when it shows something the last frame sent
    /// did not: any change in the picture, not only in the total damage (a
    /// name arriving, a boss's health, a death, a buff). While nothing
    /// changes nothing is sent, so an overlay left up over the game for hours
    /// is not redrawn. With no frame sent yet, only one with damage goes out.
    fn send_frame_if_changed(&self, frame: CombatSnapshot) {
        let mut last_sent = self.last_sent.lock().unwrap();
        let changed = match last_sent.as_ref() {
            Some(previous) => !previous.shows_the_same_as(&frame),
            None => frame.total_damage > 0,
        };
        if !changed {
            return;
        }
        *last_sent = Some(frame.clone());
        self.events.emit(MeterEvent::DpsSnapshot(frame));
    }

    fn emit_empty_snap(&self) {
        use crate::models::combat::CombatInfos;

        let empty = CombatSnapshot {
            total_damage: 0,
            by_target_player_skill_stats: HashMap::new(),
            by_target_player_stats: HashMap::new(),
            use_buffs_by_target: HashMap::new(),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: None,
                main_actor_name: None,
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 0.0,
            },
            last_target_info: None,
            last_target_all_players_overview_stats: Vec::new(),
            main_actor_received_player_overview_stats: Vec::new(),
            main_actor_dealt_player_overview_stats: Vec::new(),
            session: SessionSummary::default(),
        };
        self.events.emit(MeterEvent::DpsSnapshot(empty));
    }
}

impl DpsMeter {
    pub fn new(
        events: Arc<dyn EventSink>,
        logger: Arc<AppLogger>,
        app_data_dir: Option<PathBuf>,
    ) -> Self {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(
            Arc::clone(&config),
            app_data_dir
                .clone()
                .map(|dir| MobStorage::new(dir, Arc::clone(&logger))),
        ));
        let data_storage = Arc::new(DataStorage::new(
            Arc::clone(&events),
            Arc::clone(&config),
            mobs,
        ));
        let calculator = Arc::new(DpsCalculator::new(Arc::clone(&data_storage)));
        let ping_tracker = Arc::new(PingTracker::new());
        let packet_channel = Channel::new(PACKET_CHANNEL_CAPACITY);
        let recorder = Arc::new(PacketRecorder::new());
        let replaying = Arc::new(AtomicBool::new(false));
        let rolling_recorder = Arc::new(RollingRecorder::new(
            Arc::clone(&logger),
            Arc::clone(&replaying),
        ));
        let recordings_dir = app_data_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("belisk"))
            .join("recordings");
        let windivert_capturer =
            WinDivertCapturer::new(packet_channel.clone(), Arc::clone(&logger));
        let pcap_capturer = PcapCapturer::new(packet_channel.clone(), Arc::clone(&logger));
        let session_recording = Arc::new(RecordingQueue::new(
            Arc::clone(&recorder) as _,
            RECORDING_QUEUE_CAPACITY,
        ));
        let rolling_recording = Arc::new(RecordingQueue::new(
            Arc::clone(&rolling_recorder) as _,
            RECORDING_QUEUE_CAPACITY,
        ));
        let drop_counters = PacketDropCounters::new(
            packet_channel.clone(),
            vec![Arc::clone(&session_recording), Arc::clone(&rolling_recording)],
        );
        let dispatcher = CaptureDispatcher::new(
            packet_channel.clone(),
            vec![Arc::clone(&session_recording), Arc::clone(&rolling_recording)],
            Arc::clone(&data_storage),
            Arc::clone(&logger),
            Arc::clone(&ping_tracker),
            Arc::clone(&config),
        );
        let history = Arc::new(HistoryStore::open(app_data_dir.clone(), Arc::clone(&logger)));
        let personal_bests = Arc::new(match app_data_dir {
            Some(dir) => PersonalBestStore::open(dir.join("history"), &history.get_records(), Arc::clone(&logger)),
            None => PersonalBestStore::new(),
        });

        let running = Arc::new(AtomicBool::new(false));
        let fight = Arc::new(FightCloser {
            events: Arc::clone(&events),
            logger: Arc::clone(&logger),
            config: Arc::clone(&config),
            data_storage: Arc::clone(&data_storage),
            calculator: Arc::clone(&calculator),
            history: Arc::clone(&history),
            personal_bests: Arc::clone(&personal_bests),
            running: Arc::clone(&running),
            last_sent: Mutex::new(None),
        });

        // Register main-actor callback: reset meter silently when player is identified.
        let fight_for_cb = Arc::clone(&fight);
        data_storage
            .main_actor_callback
            .lock()
            .unwrap()
            .replace(Box::new(move |actor_id, actor_name, sid| {
                let sid_str = sid.unwrap_or_default();
                fight_for_cb.logger.debug(format!(
                    "main actor detected: id={actor_id} name={actor_name} server={sid_str}"
                ));
                fight_for_cb.reset(false);
            }));

        Self {
            events,
            fight,
            config,
            logger,
            data_storage,
            calculator,
            ping_tracker,
            packet_channel,
            recorder,
            rolling_recorder,
            session_recording,
            rolling_recording,
            drop_counters,
            drop_reporter: Arc::new(Mutex::new(DropReporter::default())),
            recordings_dir,
            replaying,
            windivert_capturer,
            pcap_capturer,
            active_capture_backend: Arc::new(Mutex::new(None)),
            dispatcher,
            running,
            snapshot_running: Arc::new(AtomicBool::new(false)),
            memory_snapshot_running: Arc::new(AtomicBool::new(false)),
            snapshot_thread: Mutex::new(None),
            memory_snapshot_thread: Mutex::new(None),
            history,
            personal_bests,
            peek_until: Arc::new(Mutex::new(None)),
            auto_record: Arc::new(Mutex::new(AutoRecordState::default())),
        }
    }

    /// Keep the overlay up for `secs` even with no fight to show.
    pub fn peek_overlay(&self, secs: u64) {
        *self.peek_until.lock().unwrap() = Some(Instant::now() + Duration::from_secs(secs));
        self.events.emit(MeterEvent::DpsIdleHidden(false));
    }

    /// The capture backend in use, while the meter runs.
    pub fn capture_backend(&self) -> Option<CaptureBackend> {
        *self.active_capture_backend.lock().unwrap()
    }

    pub fn has_game_traffic(&self) -> bool {
        self.dispatcher.has_recent_ports()
    }

    pub fn main_actor_name(&self) -> Option<String> {
        self.data_storage.main_actor_name()
    }

    /// The characters earlier runs saw the player as, most recent first, for
    /// naming the player's actor when the game has not.
    pub fn set_known_characters(&self, characters: Vec<crate::identity::KnownCharacter>) {
        self.data_storage.set_known_characters(characters);
    }

    pub fn apply_config(&self, config: DpsMeterConfig) -> DpsMeterConfig {
        let config = config.normalized();
        *self.config.write().unwrap() = config.clone();
        crate::region::set_configured(config.region);
        self.sync_rolling_recording();
        self.logger.info(format!(
            "region profile applied: {}",
            config.region.label()
        ));
        self.logger.set_debug_enabled(config.output_debug_log);
        self.logger.info(format!(
            "config applied: dps_interval={}ms memory_interval={}ms stall_resync_delay={}ms full_processor_stall_resync_delay={}ms unknown_packet_stall_resync_delay={}ms capture_backend_priority={:?} boss_only={} pvp_mode_on={} pvp_overlay_position={:?} show_possible_boss={} my_muzhuang_only={} output_debug_log={}",
            config.dps_snapshot_interval_ms,
            config.memory_snapshot_interval_ms,
            config.stall_resync_delay_ms,
            config.full_processor_stall_resync_delay_ms,
            config.unknown_packet_stall_resync_delay_ms,
            config.capture_backend_priority,
            config.boss_only,
            config.pvp_mode_on,
            config.pvp_overlay_position,
            config.show_possible_boss,
            config.my_muzhuang_only,
            config.output_debug_log
        ));
        config
    }

    pub fn current_config(&self) -> DpsMeterConfig {
        self.config.read().unwrap().clone()
    }

    /// Turns PvP tracking on or off without touching any other setting.
    pub fn set_pvp_mode(&self, on: bool) {
        self.config.write().unwrap().pvp_mode_on = on;
    }

    /// Whether damage, heals, damage taken, deaths and buff uptime are counted.
    /// Off until the host turns it on, so a meter that starts with its window
    /// closed never counts a fight; capture, identity, the world and tickets
    /// run either way.
    pub fn fight_tracking(&self) -> bool {
        self.data_storage.fight_tracking()
    }

    /// Whether you are in a fight the meter counts: from your first hit until
    /// the fight ends (idle timeout, reset, or the meter closing).
    pub fn in_fight(&self) -> bool {
        self.data_storage.fight_tracking() && self.data_storage.activity_at().is_some()
    }

    /// Turns fight counting on or off, as the DPS meter window opens or
    /// closes. Turning it off ends the fight on the meter like an idle
    /// timeout does; turning it on counts from that moment.
    pub fn set_fight_tracking(&self, on: bool) {
        if self.data_storage.fight_tracking() == on {
            return;
        }
        self.data_storage.set_fight_tracking(on);
        if !on {
            let was_in_fight = self.data_storage.activity_at().is_some();
            self.fight.end_fight(was_in_fight, "meter closed");
        }
        self.logger.info(format!("fight tracking {}", if on { "on" } else { "off" }));
    }

    pub fn start_dps_meter(&self) -> Result<(), CaptureStartError> {
        if !self.begin_start() {
            return Ok(());
        }
        let capture_backend_priority =
            self.config.read().unwrap().capture_backend_priority.clone();
        let backend = match self.start_capture_backend(capture_backend_priority) {
            Ok(backend) => backend,
            Err(error) => {
                self.dispatcher.stop();
                self.running.store(false, Ordering::SeqCst);
                return Err(error);
            }
        };
        *self.active_capture_backend.lock().unwrap() = Some(backend);
        self.logger
            .info(format!("capture backend selected: {backend:?}"));
        self.sync_rolling_recording();
        self.finish_start();
        Ok(())
    }

    /// Start the pipeline without opening a capture backend, for feeding it
    /// from a recording. No driver is loaded and no elevation is needed.
    pub fn start_for_replay(&self) {
        if self.begin_start() {
            self.finish_start();
        }
    }

    /// False when the meter was already running.
    fn begin_start(&self) -> bool {
        if self.running.swap(true, Ordering::SeqCst) {
            self.emit_running_status();
            return false;
        }
        self.clear_runtime_state();
        *self.auto_record.lock().unwrap() = AutoRecordState::default();
        self.dispatcher.start();
        true
    }

    fn finish_start(&self) {
        self.start_snapshot_loop();
        self.start_memory_snapshot_loop();
        self.emit_running_status();
        self.logger.info("dps meter started");
    }

    fn start_capture_backend(
        &self,
        priority: CaptureBackendPriority,
    ) -> Result<CaptureBackend, CaptureStartError> {
        match priority {
            CaptureBackendPriority::WinDivertFirst => match self.windivert_capturer.start() {
                Ok(()) => Ok(CaptureBackend::WinDivert),
                Err(windivert_error) => {
                    self.logger.info(format!(
                        "WinDivert initialization failed, falling back to Npcap: {windivert_error}"
                    ));
                    self.pcap_capturer
                        .start()
                        .map(|_| CaptureBackend::Npcap)
                        .map_err(|npcap_error| CaptureStartError::NoBackend {
                            tried_first: CaptureBackend::WinDivert,
                            npcap: npcap_error,
                            windivert: windivert_error,
                        })
                }
            },
            CaptureBackendPriority::NpcapFirst => match self.pcap_capturer.start() {
                Ok(()) => Ok(CaptureBackend::Npcap),
                Err(npcap_error) => {
                    self.logger.info(format!(
                        "Npcap initialization failed, falling back to WinDivert: {npcap_error}"
                    ));
                    self.windivert_capturer
                        .start()
                        .map(|_| CaptureBackend::WinDivert)
                        .map_err(|windivert_error| CaptureStartError::NoBackend {
                            tried_first: CaptureBackend::Npcap,
                            npcap: npcap_error,
                            windivert: windivert_error,
                        })
                }
            },
        }
    }

    /// Rolling recording runs while the meter captures live and the setting is
    /// on; a replay feeds the same pipeline but is not the player's session.
    fn sync_rolling_recording(&self) {
        let capturing = self.active_capture_backend.lock().unwrap().is_some();
        if capturing && self.config.read().unwrap().record_every_session {
            self.rolling_recorder.enable(self.recordings_dir.clone());
        } else {
            flush_recording(
                &self.rolling_recording,
                "rolling",
                Instant::now() + RECORDING_QUIT_FLUSH_BUDGET,
                &self.logger,
            );
            self.rolling_recorder.disable();
        }
    }

    pub fn stop_dps_meter(&self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            self.emit_running_status();
            return;
        }

        self.stop_snapshot_loop();
        self.stop_memory_snapshot_loop();
        self.stop_active_capturer();
        self.dispatcher.stop();
        self.auto_record.lock().unwrap().active_since = None;
        // A fight still on the meter is kept, not thrown away: stopping the
        // meter or quitting right after a boss used to lose it.
        self.fight.save_fight_to_history();
        self.save_now();
        let _ = self.recorder.stop();
        self.sync_rolling_recording();
        log_packet_drops(&self.drop_counters, &self.drop_reporter, &self.logger, true);
        self.clear_runtime_state();
        self.events.emit(MeterEvent::DpsIdleHidden(false));
        self.emit_running_status();
        self.logger.info("dps meter stopped");
    }

    /// Writes whatever is still waiting for the disk (history, personal bests,
    /// mobs, then the recordings' queued packets) on the calling thread, for
    /// quitting. Does nothing when everything is saved. The recordings are
    /// waited for last and share one budget (`RECORDING_QUIT_FLUSH_BUDGET`); a
    /// writer that stays stuck makes later calls return at once.
    pub fn save_now(&self) {
        self.history.flush();
        self.personal_bests.flush();
        self.data_storage.mobs().save_now();
        let deadline = Instant::now() + RECORDING_QUIT_FLUSH_BUDGET;
        flush_recording(&self.session_recording, "session", deadline, &self.logger);
        flush_recording(&self.rolling_recording, "rolling", deadline, &self.logger);
    }

    /// Packets lost since the meter was created: before the dispatcher could
    /// read them, and out of the recordings. Also logged to belisk.log, at
    /// most once every 30 seconds while they happen.
    pub fn packet_drops(&self) -> PacketDrops {
        self.drop_counters.totals()
    }

    /// Reset asked for by the player, from the overlay or the hotkey. The
    /// overlay stays up a moment so the click visibly did something.
    pub fn reset_from_user(&self) {
        self.reset_dps_meter(true);
        self.peek_overlay(RESET_PEEK_SECS);
    }

    pub fn personal_best(&self, character: &str, mob_code: u32) -> Option<PersonalBest> {
        self.personal_bests.get(character, mob_code)
    }

    pub fn personal_bests(&self) -> Vec<PersonalBest> {
        self.personal_bests.all()
    }

    pub fn reset_personal_bests(&self) -> usize {
        let count = self.personal_bests.clear();
        self.events.emit(MeterEvent::PersonalBestsUpdated);
        count
    }

    pub fn reset_dps_meter(&self, emit_empty: bool) {
        self.fight.reset(emit_empty);
    }

    pub fn get_history(&self) -> Vec<crate::history::HistoryRecord> {
        self.history.get_records()
    }

    pub fn delete_all_history(&self) -> usize {
        self.history.clear_all()
    }

    pub fn delete_history_record(&self, id: &str) -> bool {
        self.history.delete_record(id)
    }

    pub fn mobs(&self) -> &Arc<MobCatalog> {
        self.data_storage.mobs()
    }

    pub fn logger(&self) -> &Arc<AppLogger> {
        &self.logger
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn get_dps_snapshot(&self, target_damage_threshold: u64) -> Option<CombatSnapshot> {
        self.fight.snapshot(target_damage_threshold)
    }

    /// The end-of-fight report of the boss fight against `target_id`: from the
    /// fight still on the meter, or else from the newest saved record of that
    /// target. `None` when the target is not a boss somebody has hit.
    pub fn fight_report(&self, target_id: u32) -> Option<FightReport> {
        if let Some(snapshot) = self.fight.snapshot(0) {
            if let Some(target) = snapshot.combat_infos.target_infos.get(&target_id) {
                let dealers: Vec<u32> = snapshot
                    .by_target_player_stats
                    .get(&target_id)
                    .map(|stats| stats.keys().copied().collect())
                    .unwrap_or_default();
                let ledger = self.calculator.fight_ledger(target, &dealers);
                return FightReport::from_snapshot(&snapshot, target_id, ledger.as_ref());
            }
        }
        self.history
            .get_records()
            .iter()
            .rev()
            .find(|record| record.target_id == target_id)
            .and_then(FightReport::from_record)
    }

    pub fn get_pvp_watch_info(&self, names: &[String]) -> PvpWatchInfoResponse {
        self.data_storage.get_pvp_watch_info(names)
    }

    pub fn get_pvp_combat_stats(&self) -> Vec<PvpCombatStatsRow> {
        self.data_storage.get_pvp_combat_stats()
    }

    pub fn clear_pvp_combat_stats(&self) {
        self.data_storage.clear_pvp_combat_stats();
    }

    fn start_snapshot_loop(&self) {
        if self.snapshot_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let events = Arc::clone(&self.events);
        let fight = Arc::clone(&self.fight);
        let calculator = Arc::clone(&self.calculator);
        let config = Arc::clone(&self.config);

        let snapshot_running = Arc::clone(&self.snapshot_running);
        let data_storage = Arc::clone(&self.data_storage);
        let peek_until = Arc::clone(&self.peek_until);

        let handle = thread::spawn(move || {
            let mut idle_hidden: Option<bool> = None;

            while snapshot_running.load(Ordering::SeqCst) {
                if !data_storage.fight_tracking() {
                    let interval_ms = config.read().unwrap().dps_snapshot_interval_ms;
                    thread::park_timeout(Duration::from_millis(interval_ms));
                    continue;
                }
                let cfg = config.read().unwrap();
                let hide_unknown = cfg.hide_unknown_players;
                let max_count = cfg.max_player_count;
                let hide_when_idle = cfg.hide_when_idle;
                let idle_reset_secs = cfg.idle_reset_secs;
                drop(cfg);

                fight.end_wiped_attempts();

                // A fight nobody has touched for the idle timeout is over.
                // Measured from your own last hit, so other people fighting
                // nearby cannot keep an old fight on screen forever.
                if idle_reset_secs > 0 {
                    if let Some(started) = data_storage.start_time() {
                        let activity = data_storage.activity_at();
                        let idle_for = now_seconds() - activity.unwrap_or(started);
                        if idle_for >= idle_reset_secs as f64 {
                            fight.end_fight(activity.is_some(), "idle timeout");
                        }
                    }
                }

                // Between fights the overlay steps aside, and it is back for
                // the first hit of the next one.
                let peeking = peek_until
                    .lock()
                    .unwrap()
                    .is_some_and(|until| Instant::now() < until);
                let hidden = hide_when_idle && !peeking && data_storage.activity_at().is_none();
                if idle_hidden != Some(hidden) {
                    idle_hidden = Some(hidden);
                    events.emit(MeterEvent::DpsIdleHidden(hidden));
                }

                if let Some(snapshot) = calculator.get_dps_snapshot(0, hide_unknown, max_count) {
                    fight.send_frame_if_changed(snapshot);
                }

                let interval_ms = config.read().unwrap().dps_snapshot_interval_ms;
                thread::park_timeout(Duration::from_millis(interval_ms));
            }
        });

        *self.snapshot_thread.lock().unwrap() = Some(handle);
    }

    fn stop_snapshot_loop(&self) {
        self.snapshot_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.snapshot_thread.lock().unwrap().take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }

    fn start_memory_snapshot_loop(&self) {
        if self.memory_snapshot_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let events = Arc::clone(&self.events);
        let config = Arc::clone(&self.config);
        let logger = Arc::clone(&self.logger);
        let data_storage = Arc::clone(&self.data_storage);
        let ping_tracker = Arc::clone(&self.ping_tracker);
        let packet_channel = self.packet_channel.clone();
        let windivert_capturer = self.windivert_capturer.clone();
        let pcap_capturer = self.pcap_capturer.clone();
        let active_capture_backend = Arc::clone(&self.active_capture_backend);
        let dispatcher = self.dispatcher.clone();
        let memory_snapshot_running = Arc::clone(&self.memory_snapshot_running);
        let recorder = Arc::clone(&self.recorder);
        let session_recording = Arc::clone(&self.session_recording);
        let drop_counters = self.drop_counters.clone();
        let drop_reporter = Arc::clone(&self.drop_reporter);
        let recordings_dir = self.recordings_dir.clone();
        let auto_record = Arc::clone(&self.auto_record);

        let handle = thread::spawn(move || {
            let pid = match get_current_pid() {
                Ok(pid) => pid,
                Err(error) => {
                    logger.error(format!("failed to resolve current pid: {error}"));
                    return;
                }
            };
            // Only machine totals up front, and later only Belisk's own
            // process. `System::new_all()` would load every process, which on
            // Windows opens each one -- a running game included -- with
            // PROCESS_VM_READ, holds the handles, and reads their memory for
            // command lines. Anti-cheat treats that as an attack on the game.
            let mut system = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_memory(MemoryRefreshKind::everything())
                    .with_cpu(CpuRefreshKind::everything()),
            );

            while memory_snapshot_running.load(Ordering::SeqCst) {
                let removed_ports = dispatcher
                    .cleanup_stale_assemblers(Duration::from_secs(STALE_ASSEMBLER_IDLE_SECS));
                if !removed_ports.is_empty() {
                    logger.info(format!(
                        "cleaned stale assembler ports: {}",
                        removed_ports.join(", ")
                    ));
                }

                log_packet_drops(&drop_counters, &drop_reporter, &logger, false);
                let auto_record_enabled = config.read().unwrap().auto_record_unknown_server;
                auto_record_tick(
                    &auto_record,
                    &recorder,
                    &session_recording,
                    &recordings_dir,
                    &dispatcher,
                    &logger,
                    auto_record_enabled,
                );

                let (cap_device, cap_port) = match *active_capture_backend.lock().unwrap() {
                    Some(CaptureBackend::WinDivert) => (
                        windivert_capturer.target_device(),
                        windivert_capturer.target_port(),
                    ),
                    Some(CaptureBackend::Npcap) => {
                        (pcap_capturer.target_device(), pcap_capturer.target_port())
                    }
                    None => (None, None),
                };

                if let Some(snapshot) = build_memory_snapshot(
                    &mut system,
                    pid,
                    &data_storage,
                    &ping_tracker,
                    &packet_channel,
                    &dispatcher,
                    cap_device,
                    cap_port,
                ) {
                    events.emit(MeterEvent::DpsMemory(snapshot));
                }

                let interval_ms = config.read().unwrap().memory_snapshot_interval_ms;
                thread::park_timeout(Duration::from_millis(interval_ms));
            }
        });

        *self.memory_snapshot_thread.lock().unwrap() = Some(handle);
    }

    fn stop_memory_snapshot_loop(&self) {
        self.memory_snapshot_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.memory_snapshot_thread.lock().unwrap().take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }

    fn clear_runtime_state(&self) {
        self.data_storage.clear();
        self.packet_channel.clear();
        self.dispatcher.clear();
        self.ping_tracker.reset();
        self.fight.forget_sent_frame();
    }

    // ── Packet recording and replay ──
    //
    // Recording captures a session so a parser can be iterated against it
    // offline. That matters most for a service that launches once: capture the
    // global servers on day one, then work against those bytes for as long as it
    // takes, without needing to be in game.

    pub fn start_packet_recording(&self) -> Result<String, String> {
        let path = self.recorder.start(&self.recordings_dir)?;
        self.logger
            .info(format!("packet recording started: {}", path.display()));
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn stop_packet_recording(&self) -> Result<Option<RecordingStatus>, String> {
        flush_recording(
            &self.session_recording,
            "session",
            Instant::now() + RECORDING_STOP_FLUSH_TIMEOUT,
            &self.logger,
        );
        let stopped = self.recorder.stop()?;
        if let Some(status) = &stopped {
            self.logger.info(format!(
                "packet recording stopped: {} packets, {} bytes",
                status.packets, status.bytes
            ));
        }
        Ok(stopped)
    }

    pub fn packet_recording_status(&self) -> RecordingStatus {
        self.recorder.status()
    }

    pub fn list_packet_recordings(&self) -> Vec<RecordingFile> {
        list_recordings(&self.recordings_dir)
    }

    pub fn is_replaying(&self) -> bool {
        self.replaying.load(Ordering::SeqCst)
    }

    pub fn cancel_packet_replay(&self) {
        self.replaying.store(false, Ordering::SeqCst);
    }

    /// Feed a recording back through the live pipeline.
    ///
    /// Packets go into the same channel the capturer writes to, so reassembly,
    /// dispatch, parsing and aggregation all run exactly as they would live.
    pub fn replay_packet_recording(&self, path: String) -> Result<(), String> {
        self.start_replay(path, None)
    }

    /// Like [`Self::replay_packet_recording`], but spaced out by the packets'
    /// capture times divided by `speed`, so the meter sees a fight at the pace
    /// it happened (`1.0`) or faster.
    pub fn replay_packet_recording_paced(&self, path: String, speed: f64) -> Result<(), String> {
        let pacer = ReplayPacer::new(speed)?;
        self.start_replay(path, Some(pacer))
    }

    fn start_replay(&self, path: String, mut pacer: Option<ReplayPacer>) -> Result<(), String> {
        if !self.is_running() {
            return Err(
                "Start the meter first -- replay feeds the same pipeline live capture uses."
                    .to_string(),
            );
        }
        if self.replaying.swap(true, Ordering::SeqCst) {
            return Err("A replay is already running.".to_string());
        }
        self.data_storage.reset_ticket_binding();

        let channel = self.packet_channel.clone();
        let replaying = Arc::clone(&self.replaying);
        let logger = Arc::clone(&self.logger);
        let events = Arc::clone(&self.events);

        thread::spawn(move || {
            let source = PathBuf::from(&path);
            let outcome = replay_recording(&source, |packet| {
                if let Some(pacer) = pacer.as_mut() {
                    let mut remaining = pacer.delay_before(packet.captured_at, Instant::now());
                    while !remaining.is_zero() {
                        if !replaying.load(Ordering::SeqCst) {
                            return false;
                        }
                        let slice = remaining.min(REPLAY_PACING_SLICE);
                        thread::sleep(slice);
                        remaining -= slice;
                    }
                }
                while channel.size() > REPLAY_BACKPRESSURE_LIMIT {
                    if !replaying.load(Ordering::SeqCst) {
                        return false;
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                if !channel.try_send(packet) {
                    return false;
                }
                replaying.load(Ordering::SeqCst)
            });

            match outcome {
                Ok(count) => {
                    logger.info(format!("replayed {count} packets from {path}"));
                    events.emit(MeterEvent::PacketReplayFinished(count));
                }
                Err(error) => {
                    logger.info(format!("replay failed for {path}: {error}"));
                    events.emit(MeterEvent::PacketReplayFailed(error));
                }
            }

            replaying.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    fn emit_running_status(&self) {
        self.events
            .emit(MeterEvent::DpsMeterStatus(self.is_running()));
    }

    fn stop_active_capturer(&self) {
        match self.active_capture_backend.lock().unwrap().take() {
            Some(CaptureBackend::WinDivert) => self.windivert_capturer.stop(),
            Some(CaptureBackend::Npcap) => self.pcap_capturer.stop(),
            None => {}
        }
    }
}

impl Drop for DpsMeter {
    fn drop(&mut self) {
        self.stop_snapshot_loop();
        self.stop_memory_snapshot_loop();
        self.windivert_capturer.stop();
        self.pcap_capturer.stop();
        self.dispatcher.stop();
        self.clear_runtime_state();
        self.save_now();
    }
}

/// Start or finish the session's automatic recording.
///
/// A server no fingerprint matches is exactly the case where a recording is
/// worth having -- global on launch day, or any service we have not seen --
/// and it is also the case where nobody thinks to press Record in time. So the
/// first two minutes of game traffic are kept, once per session, and only the
/// newest few are kept on disk. A recording started by hand is never touched.
fn auto_record_tick(
    state: &Mutex<AutoRecordState>,
    recorder: &PacketRecorder,
    recording: &RecordingQueue,
    dir: &std::path::Path,
    dispatcher: &CaptureDispatcher,
    logger: &AppLogger,
    enabled: bool,
) {
    let mut state = state.lock().unwrap();

    if let Some(since) = state.active_since {
        if since.elapsed() >= AUTO_RECORDING_DURATION || !enabled {
            state.active_since = None;
            state.done = true;
            flush_recording(
                recording,
                "auto",
                Instant::now() + RECORDING_STOP_FLUSH_TIMEOUT,
                logger,
            );
            match recorder.stop() {
                Ok(Some(status)) => logger.info(format!(
                    "auto recording finished: {} packets, {} bytes",
                    status.packets, status.bytes
                )),
                Ok(None) => {}
                Err(error) => logger.info(format!("auto recording stop failed: {error}")),
            }
            prune_recordings(dir, AUTO_RECORDING_PREFIX, AUTO_RECORDINGS_KEPT);
        }
        return;
    }

    if state.done
        || !enabled
        || recorder.status().recording
        || !dispatcher.has_recent_ports()
        || crate::region::status().detected.is_some()
    {
        return;
    }

    match recorder.start_named(dir, AUTO_RECORDING_PREFIX) {
        Ok(path) => {
            state.active_since = Some(Instant::now());
            logger.info(format!(
                "auto recording started (server not recognised): {}",
                path.display()
            ));
        }
        Err(error) => {
            state.done = true;
            logger.info(format!("auto recording could not start: {error}"));
        }
    }
}

/// Waits until `deadline` for the packets queued for a recording to reach its
/// file, so that closing the recording right after loses none of them.
fn flush_recording(recording: &RecordingQueue, name: &str, deadline: Instant, logger: &AppLogger) {
    if !recording.flush(deadline) {
        logger.info(format!(
            "{name} recording writer is still busy, closing the recording without waiting for the packets queued behind it"
        ));
    }
}

fn log_packet_drops(
    counters: &PacketDropCounters,
    reporter: &Mutex<DropReporter>,
    logger: &AppLogger,
    force: bool,
) {
    let line = reporter
        .lock()
        .unwrap()
        .line(counters.totals(), Instant::now(), force);
    if let Some(line) = line {
        logger.info(line);
    }
}

fn build_memory_snapshot(
    system: &mut System,
    pid: sysinfo::Pid,
    data_storage: &Arc<DataStorage>,
    ping_tracker: &Arc<PingTracker>,
    packet_channel: &Channel<CapturedPacket>,
    dispatcher: &CaptureDispatcher,
    cap_device: Option<String>,
    cap_port: Option<String>,
) -> Option<MemorySnapshot> {
    system.refresh_memory();
    system.refresh_cpu_usage();
    let _ = system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory().with_cpu(),
    );
    let process = system.process(pid)?;

    let total_memory = system.total_memory().max(1) as f64;
    let rss_bytes = process.memory() as f64;
    let vms_bytes = process.virtual_memory() as f64;
    let logical_cpu_count = system.cpus().len().max(1) as f32;
    let normalized_cpu_percent = (process.cpu_usage() / logical_cpu_count).clamp(0.0, 100.0);
    let mut packet_sizes: HashMap<String, usize> = dispatcher.assembler_buffer_sizes();
    let channel_size = packet_channel.size();
    if channel_size > 0 {
        packet_sizes.insert("channel".to_string(), channel_size);
    }

    let cpus = system.cpus();
    let system_cpu_percent = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };

    Some(MemorySnapshot {
        cpu_percent: normalized_cpu_percent,
        rss_mb: rss_bytes / (1024.0 * 1024.0),
        vms_mb: vms_bytes / (1024.0 * 1024.0),
        memory_percent: ((rss_bytes / total_memory) * 100.0) as f32,
        system_cpu_percent: system_cpu_percent.clamp(0.0, 100.0),
        system_memory_used_mb: system.used_memory() as f64 / (1024.0 * 1024.0),
        system_memory_total_mb: total_memory / (1024.0 * 1024.0),
        cap_device,
        cap_port: dispatcher.current_combat_port().or(cap_port),
        packet_sizes,
        ping_ms: ping_tracker.current_ping_ms(),
        main_actor_name: data_storage.main_actor_name(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Discard;

    impl EventSink for Discard {
        fn emit(&self, _event: MeterEvent) {}
    }

    #[test]
    fn stopping_a_running_meter_does_not_wait_out_the_snapshot_intervals() {
        let log = std::env::temp_dir().join(format!("meter-core-stop-{}.log", std::process::id()));
        let logger = Arc::new(AppLogger::new(log.clone(), Box::new(|_| {})));
        let meter = DpsMeter::new(Arc::new(Discard), logger, None);
        meter.start_for_replay();
        thread::sleep(Duration::from_millis(150));

        let started = Instant::now();
        meter.stop_dps_meter();
        let took = started.elapsed();

        let _ = std::fs::remove_file(log);
        assert!(!meter.is_running());
        assert!(took < Duration::from_millis(500), "stopping took {took:?}");
    }

    struct Recording(Mutex<Vec<&'static str>>);

    impl EventSink for Recording {
        fn emit(&self, event: MeterEvent) {
            let name = match event {
                MeterEvent::HistoryUpdated => "history-updated",
                MeterEvent::DpsSnapshot(_) => "dps-snapshot",
                _ => return,
            };
            self.0.lock().unwrap().push(name);
        }
    }

    fn big_hit(actor_id: u32, target_id: u32) -> crate::models::packet::ParsedDamagePacket {
        crate::models::packet::ParsedDamagePacket {
            target_id,
            actor_id,
            skill_code: 11_000_000,
            ori_skill_code: 11_000_000,
            damage: 5_000_000,
            is_dot: false,
            is_crit: false,
            multi_hit_damage: 0,
            multi_hit_count: 0,
            specials: Vec::new(),
            drain_amount: 0,
            regeneration_amount: 0,
        }
    }

    fn meter_with(events: Arc<impl EventSink + 'static>) -> DpsMeter {
        let log = std::env::temp_dir().join(format!("meter-core-tracking-{}.log", std::process::id()));
        let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
        DpsMeter::new(events, logger, None)
    }

    #[test]
    fn closing_the_meter_in_a_fight_ends_it_like_the_idle_timeout_and_saves_it() {
        let events = Arc::new(Recording(Mutex::new(Vec::new())));
        let meter = meter_with(Arc::clone(&events));
        meter.set_fight_tracking(true);
        assert!(!meter.in_fight());
        meter.data_storage.append_damage(big_hit(7, 900));
        assert!(meter.in_fight());
        assert!(meter.get_history().is_empty());

        meter.set_fight_tracking(false);

        assert!(!meter.fight_tracking());
        assert!(!meter.in_fight());
        assert_eq!(meter.get_history().len(), 1);
        assert!(meter.get_dps_snapshot(0).is_none_or(|snapshot| snapshot.total_damage == 0));
        assert!(events.0.lock().unwrap().contains(&"history-updated"));
    }

    #[test]
    fn a_meter_that_never_tracked_counts_no_fight_and_saves_no_history() {
        let events = Arc::new(Recording(Mutex::new(Vec::new())));
        let meter = meter_with(Arc::clone(&events));

        meter.data_storage.append_damage(big_hit(7, 900));
        meter.set_fight_tracking(false);

        assert!(!meter.fight_tracking());
        assert!(meter.get_history().is_empty());
        assert!(events.0.lock().unwrap().is_empty());
    }

    #[test]
    fn closing_a_meter_with_no_fight_on_it_saves_no_history() {
        let events = Arc::new(Recording(Mutex::new(Vec::new())));
        let meter = meter_with(Arc::clone(&events));
        meter.set_fight_tracking(true);

        meter.set_fight_tracking(false);

        assert!(meter.get_history().is_empty());
        assert!(!events.0.lock().unwrap().contains(&"history-updated"));
    }

    #[test]
    fn opening_the_meter_counts_from_that_moment() {
        let events = Arc::new(Recording(Mutex::new(Vec::new())));
        let meter = meter_with(events);
        meter.data_storage.append_damage(big_hit(7, 900));

        meter.set_fight_tracking(true);
        meter.data_storage.append_damage(big_hit(7, 900));

        let snapshot = meter.get_dps_snapshot(0).expect("a fight is on the meter");
        assert_eq!(snapshot.total_damage, 5_000_000);
    }

    const BOSS: u32 = 900;
    const BOSS_CODE: u32 = 2_340_601;
    const ME: u32 = 7;

    struct Wipes(Mutex<Vec<WipedAttempt>>, Mutex<Vec<String>>);

    impl EventSink for Wipes {
        fn emit(&self, event: MeterEvent) {
            match event {
                MeterEvent::BossWiped(wiped) => {
                    self.1.lock().unwrap().push("wiped".to_string());
                    self.0.lock().unwrap().push(wiped);
                }
                MeterEvent::DpsSnapshot(snapshot) => self.1.lock().unwrap().push(format!("snapshot {}", snapshot.total_damage)),
                _ => {}
            }
        }
    }

    fn meter_on_a_boss_you_hit() -> (DpsMeter, Arc<Wipes>) {
        let wipes = Arc::new(Wipes(Mutex::new(Vec::new()), Mutex::new(Vec::new())));
        let meter = meter_with(Arc::clone(&wipes));
        meter.set_fight_tracking(true);
        meter.data_storage.set_main_actor(ME, "Me");
        meter.data_storage.append_mob(BOSS, BOSS_CODE);
        meter.data_storage.set_mob_hp(BOSS, Some(1_000), Some(1_000));
        meter.data_storage.append_damage(big_hit(ME, BOSS));
        (meter, wipes)
    }

    fn total_damage_on_the_meter(meter: &DpsMeter) -> u64 {
        meter.get_dps_snapshot(0).map_or(0, |snapshot| snapshot.total_damage)
    }

    #[test]
    fn a_boss_back_at_full_health_is_saved_as_a_wipe_with_its_health_left_and_the_next_pull_starts_over() {
        let (meter, wipes) = meter_on_a_boss_you_hit();
        meter.data_storage.set_mob_hp(BOSS, Some(400), None);
        meter.data_storage.set_mob_hp(BOSS, Some(1_000), None);

        meter.fight.end_wiped_attempts();

        let history = meter.get_history();
        assert_eq!(history.len(), 1);
        assert!(history[0].wiped);
        let target = history[0].target_info.as_ref().unwrap();
        assert_eq!((target.current_hp, target.max_hp), (Some(400), Some(1_000)));
        assert!(history[0].ledger.is_some(), "the fight report is kept");
        assert!(meter.personal_bests().is_empty(), "a wipe is no personal best");

        let wiped = wipes.0.lock().unwrap();
        assert_eq!(wiped.len(), 1);
        assert_eq!(wiped[0].target_id, BOSS);
        assert_eq!(wiped[0].record_id.as_deref(), Some(history[0].id.as_str()));
        let shown = wiped[0].snapshot.combat_infos.target_infos.get(&BOSS).unwrap();
        assert_eq!(shown.current_hp, Some(400));
        let report = wiped[0].report.as_ref().expect("the wiped attempt's report travels with the event");
        assert_eq!(report.outcome, FightOutcome::Wiped { hp_left: 400, max_hp: 1_000 });
        assert_eq!(report.totals.damage, 5_000_000);

        assert_eq!(total_damage_on_the_meter(&meter), 0, "the failed attempt is off the meter");
        meter.data_storage.append_damage(big_hit(ME, BOSS));
        assert_eq!(total_damage_on_the_meter(&meter), 5_000_000);
    }

    #[test]
    fn a_wipe_is_followed_by_an_empty_snapshot_so_the_failed_attempt_leaves_the_board() {
        let (meter, wipes) = meter_on_a_boss_you_hit();
        meter.data_storage.set_mob_hp(BOSS, Some(400), None);
        meter.data_storage.set_mob_hp(BOSS, Some(1_000), None);

        meter.fight.end_wiped_attempts();

        assert_eq!(*wipes.1.lock().unwrap(), vec!["wiped".to_string(), "snapshot 0".to_string()]);
    }

    #[test]
    fn a_boss_that_leaves_the_field_alive_is_saved_as_a_wipe() {
        let (meter, wipes) = meter_on_a_boss_you_hit();
        meter.data_storage.set_mob_hp(BOSS, Some(300), None);
        meter.data_storage.mark_mob_removed(BOSS);

        meter.fight.end_wiped_attempts();

        let history = meter.get_history();
        assert_eq!(history.len(), 1);
        assert!(history[0].wiped);
        assert_eq!(history[0].target_info.as_ref().unwrap().current_hp, Some(300));
        assert_eq!(wipes.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_boss_healing_part_of_the_way_back_is_not_a_wipe() {
        let (meter, wipes) = meter_on_a_boss_you_hit();
        for current in [500, 700, 840] {
            meter.data_storage.set_mob_hp(BOSS, Some(current), None);
        }

        meter.fight.end_wiped_attempts();

        assert!(meter.get_history().is_empty());
        assert!(wipes.0.lock().unwrap().is_empty());
        assert_eq!(total_damage_on_the_meter(&meter), 5_000_000);
    }

    #[test]
    fn a_kill_is_not_a_wipe() {
        let (meter, wipes) = meter_on_a_boss_you_hit();
        meter.data_storage.set_mob_hp(BOSS, Some(0), None);
        meter.data_storage.mark_mob_dead(BOSS);
        meter.data_storage.mark_mob_removed(BOSS);

        meter.fight.end_wiped_attempts();

        assert!(meter.get_history().is_empty());
        assert!(wipes.0.lock().unwrap().is_empty());
        assert_eq!(total_damage_on_the_meter(&meter), 5_000_000);
    }

    #[test]
    fn a_wipe_of_a_boss_you_never_hit_is_taken_off_without_a_record() {
        let (meter, wipes) = meter_on_a_boss_you_hit();
        meter.data_storage.forget_target(BOSS);
        meter.data_storage.append_damage(big_hit(ME + 1, BOSS));
        meter.data_storage.set_mob_hp(BOSS, Some(400), None);
        meter.data_storage.set_mob_hp(BOSS, Some(1_000), None);

        meter.fight.end_wiped_attempts();

        assert!(meter.get_history().is_empty());
        assert!(wipes.0.lock().unwrap().is_empty());
        assert_eq!(total_damage_on_the_meter(&meter), 0);
    }

    #[derive(Default)]
    struct Frames(Mutex<Vec<CombatSnapshot>>);

    impl EventSink for Frames {
        fn emit(&self, event: MeterEvent) {
            if let MeterEvent::DpsSnapshot(frame) = event {
                self.0.lock().unwrap().push(frame);
            }
        }
    }

    const OTHER_PLAYER: u32 = ME + 1;

    fn meter_in_a_fight_with_an_unnamed_player() -> (DpsMeter, Arc<Frames>) {
        let frames = Arc::new(Frames::default());
        let meter = meter_with(Arc::clone(&frames));
        meter.set_fight_tracking(true);
        meter.data_storage.set_main_actor(ME, "Me");
        meter.data_storage.append_mob(BOSS, BOSS_CODE);
        meter.data_storage.set_mob_hp(BOSS, Some(1_000), Some(1_000));
        meter.data_storage.append_damage(big_hit(ME, BOSS));
        meter.data_storage.append_damage(big_hit(OTHER_PLAYER, BOSS));
        (meter, frames)
    }

    fn send_the_current_frame(meter: &DpsMeter) {
        let frame = meter.get_dps_snapshot(0).expect("a fight is on the meter");
        meter.fight.send_frame_if_changed(frame);
    }

    #[test]
    fn frames_with_nothing_new_are_not_sent_again() {
        let (meter, frames) = meter_in_a_fight_with_an_unnamed_player();

        for _ in 0..5 {
            send_the_current_frame(&meter);
        }

        assert_eq!(frames.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_name_arriving_while_nobody_hits_produces_a_frame() {
        let (meter, frames) = meter_in_a_fight_with_an_unnamed_player();
        send_the_current_frame(&meter);

        meter.data_storage.append_actor(OTHER_PLAYER, "Rin", None);
        send_the_current_frame(&meter);
        send_the_current_frame(&meter);

        let frames = frames.0.lock().unwrap();
        assert_eq!(frames.len(), 2);
        let name_in = |frame: &CombatSnapshot| frame.by_target_player_stats[&BOSS][&OTHER_PLAYER].actor_name.clone();
        assert_eq!(name_in(&frames[0]), "");
        assert_eq!(name_in(&frames[1]), "Rin");
        assert_eq!(frames[0].total_damage, frames[1].total_damage, "the damage did not change");
    }

    #[test]
    fn the_boss_health_changing_while_nobody_hits_produces_a_frame() {
        let (meter, frames) = meter_in_a_fight_with_an_unnamed_player();
        send_the_current_frame(&meter);

        meter.data_storage.set_mob_hp(BOSS, Some(900), None);
        send_the_current_frame(&meter);

        let frames = frames.0.lock().unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].combat_infos.target_infos[&BOSS].current_hp, Some(900));
    }

    #[test]
    fn a_frame_without_damage_is_not_sent_before_any_frame_was() {
        let frames = Arc::new(Frames::default());
        let meter = meter_with(Arc::clone(&frames));
        meter.set_fight_tracking(true);
        meter.data_storage.set_main_actor(ME, "Me");

        if let Some(frame) = meter.get_dps_snapshot(0) {
            meter.fight.send_frame_if_changed(frame);
        }

        assert!(frames.0.lock().unwrap().is_empty());
    }

    #[test]
    fn after_a_reset_the_next_frame_with_damage_goes_out_even_if_it_looks_like_the_last() {
        let (meter, frames) = meter_in_a_fight_with_an_unnamed_player();
        send_the_current_frame(&meter);

        meter.fight.forget_sent_frame();
        send_the_current_frame(&meter);

        assert_eq!(frames.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_saved_fight_and_a_delete_reach_the_files_when_the_meter_saves_on_quit() {
        let dir = crate::mobs::store::test_dir::TestDir::new();
        let log = std::env::temp_dir().join(format!("meter-core-persist-{}.log", std::process::id()));
        let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
        let on_disk = || HistoryStore::open(Some(dir.path().to_path_buf()), Arc::clone(&logger)).get_records();
        let meter = DpsMeter::new(Arc::new(Discard), Arc::clone(&logger), Some(dir.path().to_path_buf()));
        meter.set_fight_tracking(true);
        meter.data_storage.append_damage(big_hit(ME, BOSS));

        meter.set_fight_tracking(false);
        meter.save_now();
        assert_eq!(on_disk().len(), 1);

        let saved = meter.get_history().remove(0).id;
        assert!(meter.delete_history_record(&saved));
        meter.save_now();
        assert!(on_disk().is_empty());
    }

    #[derive(Default)]
    struct IdleSays(Mutex<Vec<bool>>);

    impl EventSink for IdleSays {
        fn emit(&self, event: MeterEvent) {
            if let MeterEvent::DpsIdleHidden(hidden) = event {
                self.0.lock().unwrap().push(hidden);
            }
        }
    }

    impl IdleSays {
        fn comes_to_say(&self, hidden: bool) -> bool {
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if self.0.lock().unwrap().last() == Some(&hidden) {
                    return true;
                }
                thread::sleep(Duration::from_millis(5));
            }
            false
        }
    }

    fn running_meter(hide_when_idle: bool) -> (DpsMeter, Arc<IdleSays>) {
        let said = Arc::new(IdleSays::default());
        let meter = meter_with(Arc::clone(&said));
        meter.apply_config(DpsMeterConfig { hide_when_idle, dps_snapshot_interval_ms: 10, ..meter.current_config() });
        meter.set_fight_tracking(true);
        meter.start_for_replay();
        (meter, said)
    }

    #[test]
    fn hiding_when_idle_hides_the_meter_until_your_first_hit() {
        let (meter, said) = running_meter(true);
        assert!(said.comes_to_say(true), "a meter with no fight steps aside");

        meter.data_storage.append_damage(big_hit(ME, BOSS));

        let back = said.comes_to_say(false);
        meter.stop_dps_meter();
        assert!(back, "your hit brings it back");
    }

    #[test]
    fn a_peek_shows_an_idle_meter_for_its_seconds_only() {
        let (meter, said) = running_meter(true);
        assert!(said.comes_to_say(true));

        meter.peek_overlay(1);
        let shown = said.0.lock().unwrap().last() == Some(&false);
        let hidden_again = said.comes_to_say(true);
        meter.stop_dps_meter();
        assert!(shown, "the peek shows it at once");
        assert!(hidden_again, "and it steps aside again when the peek is over");
    }

    #[test]
    fn with_hiding_when_idle_off_the_meter_never_hides() {
        let (meter, said) = running_meter(false);
        let first = said.comes_to_say(false);
        thread::sleep(Duration::from_millis(200));
        meter.stop_dps_meter();
        assert!(first, "the loop said where it stands");
        assert!(!said.0.lock().unwrap().contains(&true));
    }
}
