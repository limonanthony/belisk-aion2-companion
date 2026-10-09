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

use crate::capture::capturer::{check_npcap_available, CapturedPacket, PcapCapturer};
use crate::capture::channel::Channel;
use crate::capture::dispatcher::{CaptureDispatcher, TcpReassemblyStatus};
use crate::capture::ping_tracker::PingTracker;
use crate::capture::recorder::{
    list_recordings, prune_recordings, replay_recording, PacketRecorder, RecordingFile,
    RecordingStatus, ReplayPacer,
};
use crate::capture::rolling_recorder::RollingRecorder;
use crate::capture::windivert_capturer::{check_windivert_status, WinDivertCapturer};
use crate::config::{CaptureBackendPriority, DpsMeterConfig, SharedDpsMeterConfig};
use crate::engine::calculator::DpsCalculator;
use crate::history::HistoryStore;
use crate::personal_best::{PersonalBest, PersonalBestStore};
use crate::models::combat::{CombatSnapshot, PvpCombatStatsRow, PvpWatchInfoResponse};
use crate::models::diagnostics::{DpsMeterState, MemorySnapshot};
use crate::models::session::SessionSummary;
use crate::mobs::catalog::{MobCatalog, MobStorage};
use crate::report::fight_report::{FightOutcome, FightReport};
use crate::storage::boss_wipe::BossWipe;
use crate::storage::data_storage::DataStorage;
use crate::events::{EventSink, MeterEvent, WipedAttempt};
use crate::logger::AppLogger;

const STALE_ASSEMBLER_IDLE_SECS: u64 = 30;
/// Replay reads from disk far faster than the dispatcher drains, so it waits
/// whenever the queue runs this deep. `try_send` drops the packet it is handed
/// when the channel is full, and a silently lossy replay is worse than a slow one.
const REPLAY_BACKPRESSURE_LIMIT: usize = 50_000;
/// A paced replay sleeps in slices this long so a cancel is noticed promptly.
const REPLAY_PACING_SLICE: Duration = Duration::from_millis(50);

/// Seconds since the epoch, for filenames that sort chronologically.
fn chrono_stamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
const PACKET_CHANNEL_CAPACITY: isize = 2_000_000;

/// Automatic recordings: how long each runs, and how many are kept.
const AUTO_RECORDING_PREFIX: &str = "auto";
const AUTO_RECORDING_DURATION: Duration = Duration::from_secs(120);
const AUTO_RECORDINGS_KEPT: usize = 5;

/// How long the overlay stays up after Reset before hiding again, so the
/// click visibly did something.
const RESET_PEEK_SECS: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CaptureBackend {
    WinDivert,
    Npcap,
}

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
    last_emitted_total_damage: Arc<Mutex<Option<u64>>>,
    last_snapshot: Arc<Mutex<Option<CombatSnapshot>>>,
    history: Arc<HistoryStore>,
    personal_bests: Arc<PersonalBestStore>,
    /// Until when the overlay stays up regardless of the idle rule: after
    /// Start, after Reset, and from the show hotkey.
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
    last_emitted_total_damage: Arc<Mutex<Option<u64>>>,
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
        *self.last_emitted_total_damage.lock().unwrap() = None;
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
                *self.last_emitted_total_damage.lock().unwrap() = Some(rest.total_damage);
                self.events.emit(MeterEvent::DpsSnapshot(rest));
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
        eprintln!(
            "[dps_meter] save: total_damage={} targets={}",
            snapshot.total_damage,
            snapshot.by_target_player_stats.len()
        );
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
        self.calculator.reset_snapshot_state();
        *self.last_emitted_total_damage.lock().unwrap() = None;
        // Keep last_snapshot as fallback for detail window when combat ends
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
            .unwrap_or_else(|| std::env::temp_dir().join("aether"))
            .join("recordings");
        let windivert_capturer =
            WinDivertCapturer::new(packet_channel.clone(), Arc::clone(&logger));
        let pcap_capturer = PcapCapturer::new(packet_channel.clone(), Arc::clone(&logger));
        let dispatcher = CaptureDispatcher::new(
            packet_channel.clone(),
            Arc::clone(&recorder),
            Arc::clone(&rolling_recorder),
            Arc::clone(&data_storage),
            Arc::clone(&logger),
            Arc::clone(&ping_tracker),
            Arc::clone(&config),
        );
        let history = Arc::new(HistoryStore::new());
        let personal_bests = Arc::new(PersonalBestStore::new());
        if let Some(dir) = app_data_dir {
            history.init_dir(dir.clone());
            history.load_from_disk();
            personal_bests.init(dir.join("history"), &history.get_records());
        }

        let running = Arc::new(AtomicBool::new(false));
        let last_emitted_total_damage = Arc::new(Mutex::new(None));
        let fight = Arc::new(FightCloser {
            events: Arc::clone(&events),
            logger: Arc::clone(&logger),
            config: Arc::clone(&config),
            data_storage: Arc::clone(&data_storage),
            calculator: Arc::clone(&calculator),
            history: Arc::clone(&history),
            personal_bests: Arc::clone(&personal_bests),
            running: Arc::clone(&running),
            last_emitted_total_damage: Arc::clone(&last_emitted_total_damage),
        });

        // Register main-actor callback: reset meter silently when player is identified.
        let fight_for_cb = Arc::clone(&fight);
        data_storage
            .main_actor_callback
            .lock()
            .unwrap()
            .replace(Box::new(move |actor_id, actor_name, sid| {
                let sid_str = sid.unwrap_or_default();
                eprintln!(
                    "[dps_meter] main actor detected: id={} name={} server={}",
                    actor_id, actor_name, sid_str
                );
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
            last_emitted_total_damage,
            last_snapshot: Arc::new(Mutex::new(None)),
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

    /// Whether an automatic recording is running right now.
    pub fn is_auto_recording(&self) -> bool {
        self.auto_record.lock().unwrap().active_since.is_some()
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

    pub fn start_dps_meter(&self) -> Result<(), String> {
        self.start(true)
    }

    /// Start the pipeline without opening a capture backend, for feeding it
    /// from a recording. No driver is loaded and no elevation is needed.
    pub fn start_for_replay(&self) -> Result<(), String> {
        self.start(false)
    }

    fn start(&self, with_capture: bool) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            self.emit_running_status();
            return Ok(());
        }

        self.clear_runtime_state();
        *self.auto_record.lock().unwrap() = AutoRecordState::default();
        self.dispatcher.start();
        if with_capture {
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
        }
        self.start_snapshot_loop();
        self.start_memory_snapshot_loop();
        self.emit_running_status();
        self.logger.info("dps meter started");
        Ok(())
    }

    fn start_capture_backend(
        &self,
        priority: CaptureBackendPriority,
    ) -> Result<CaptureBackend, String> {
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
                        .map_err(|npcap_error| {
                            format!(
                                "No capture backend available; WinDivert: {windivert_error}; Npcap: {npcap_error}"
                            )
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
                        .map_err(|windivert_error| {
                            format!(
                                "No capture backend available; Npcap: {npcap_error}; WinDivert: {windivert_error}"
                            )
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
        let _ = self.recorder.stop();
        self.sync_rolling_recording();
        // A fight still on the meter is kept, not thrown away: stopping the
        // meter or quitting right after a boss used to lose it.
        self.fight.save_fight_to_history();
        self.data_storage.mobs().save_now();
        self.clear_runtime_state();
        self.events.emit(MeterEvent::DpsIdleHidden(false));
        self.emit_running_status();
        self.logger.info("dps meter stopped");
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

    pub fn get_last_snapshot(&self) -> Option<CombatSnapshot> {
        self.last_snapshot.lock().unwrap().clone()
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

    pub fn delete_history_records(&self, ids: &[String]) -> usize {
        self.history.delete_records(ids)
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn check_state(&self) -> DpsMeterState {
        let capture_backend_active = self.active_capture_backend.lock().unwrap().is_some();
        let mut capture_available = capture_backend_active;
        let mut capture_error = None;
        // Npcap first: WinDivert is a driver Aether would load just to ask,
        // so it is only asked when Npcap cannot do the job.
        if !capture_backend_active {
            match check_npcap_available() {
                Ok(()) => capture_available = true,
                Err(npcap_error) => {
                    let windivert_status = check_windivert_status();
                    capture_available = windivert_status.available;
                    capture_error = (!capture_available).then(|| {
                        format!(
                            "{}; Npcap: {npcap_error}",
                            windivert_status
                                .error
                                .unwrap_or_else(|| "WinDivert unavailable".to_string())
                        )
                    });
                }
            }
        }
        let meter_running = self.is_running();
        let has_game_data = self.dispatcher.has_recent_ports();
        let player_identified = self.data_storage.main_actor_name().is_some();

        DpsMeterState {
            npcap_available: capture_available,
            npcap_error: capture_error,
            boss_only_filtered: self.data_storage.boss_only_filtered(),
            meter_running,
            has_game_data,
            player_identified,
        }
    }

    pub fn tcp_reassembly_status(&self) -> TcpReassemblyStatus {
        self.dispatcher.tcp_reassembly_status()
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
        let last_emitted_total_damage = Arc::clone(&self.last_emitted_total_damage);
        let last_snapshot = Arc::clone(&self.last_snapshot);
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
                    // Cache non-empty snapshots for detail window fallback
                    if snapshot.total_damage > 0 {
                        *last_snapshot.lock().unwrap() = Some(snapshot.clone());
                    }
                    let should_emit = {
                        let mut last_damage = last_emitted_total_damage.lock().unwrap();
                        let changed = last_damage
                            .map(|previous| previous != snapshot.total_damage)
                            .unwrap_or(snapshot.total_damage > 0);
                        if changed {
                            *last_damage = Some(snapshot.total_damage);
                        }
                        changed
                    };

                    if should_emit {
                        events.emit(MeterEvent::DpsSnapshot(snapshot));
                    }
                }

                let interval_ms = config.read().unwrap().dps_snapshot_interval_ms;
                // logger.debug(format!("snapshot loop sleep {}ms", interval_ms));
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
            // Only machine totals up front, and later only Aether's own
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

                let auto_record_enabled = config.read().unwrap().auto_record_unknown_server;
                auto_record_tick(
                    &auto_record,
                    &recorder,
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
                // logger.debug(format!("memory snapshot loop sleep {}ms", interval_ms));
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
        self.calculator.reset_snapshot_state();
        self.packet_channel.clear();
        self.dispatcher.clear();
        self.ping_tracker.reset();
        *self.last_emitted_total_damage.lock().unwrap() = None;
        *self.last_snapshot.lock().unwrap() = None;
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

    /// Write a diagnostics report next to the recordings.
    ///
    /// The clipboard is the convenient path, but this runs regardless: on launch
    /// day the report is worth having on disk even if the copy silently fails.
    pub fn save_diagnostics_report(&self, report: &str) -> Result<String, String> {
        std::fs::create_dir_all(&self.recordings_dir)
            .map_err(|error| format!("Cannot create {:?}: {error}", self.recordings_dir))?;
        let path = self
            .recordings_dir
            .join(format!("diagnostics-{}.txt", chrono_stamp()));
        std::fs::write(&path, report)
            .map_err(|error| format!("Cannot write {path:?}: {error}"))?;
        Ok(path.to_string_lossy().into_owned())
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
        meter.start_for_replay().expect("the pipeline starts without a capture backend");
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
        meter.data_storage.append_damage(big_hit(7, 900));
        assert!(meter.get_history().is_empty());

        meter.set_fight_tracking(false);

        assert!(!meter.fight_tracking());
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
}
