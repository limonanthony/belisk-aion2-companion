use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::capture::assembler::StreamAssembler;
use crate::capture::capturer::CapturedPacket;
use crate::capture::channel::Channel;
use crate::capture::recorder::PacketRecorder;
use crate::capture::rolling_recorder::RollingRecorder;
use crate::capture::ping_tracker::PingTracker;
use crate::capture::tcp_reassembler::{TcpFlowKey, TcpReassembler};
use crate::config::SharedDpsMeterConfig;
use crate::storage::data_storage::DataStorage;
use crate::logger::AppLogger;

const TLS_CONTENT_TYPES: [u8; 4] = [0x14, 0x15, 0x16, 0x17];
const TLS_VERSIONS: [u8; 5] = [0x00, 0x01, 0x02, 0x03, 0x04];
const WEB_PORTS: [u16; 2] = [80, 443];
pub(crate) const MAGIC: [u8; 3] = [0x0E, 0x00, 0x36];
const MAGIC_LOCK_THRESHOLD: usize = 1;
const MAGIC_LOCK_WINDOW: Duration = Duration::from_secs(3);

#[derive(Default)]
struct RecentPortWindow {
    entries: VecDeque<(Instant, String)>,
    gap_time: Duration,
}

impl RecentPortWindow {
    fn new(gap_time: Duration) -> Self {
        Self {
            entries: VecDeque::new(),
            gap_time,
        }
    }

    fn add_and_get_locked(&mut self, key: String) -> Option<String> {
        let now = Instant::now();
        self.entries.push_back((now, key));
        while let Some((timestamp, _)) = self.entries.front() {
            if now.duration_since(*timestamp) > self.gap_time {
                self.entries.pop_front();
            } else {
                break;
            }
        }

        self.entries.iter().map(|(_, port)| port).min().cloned()
    }
}

struct DispatcherState {
    data_storage: Arc<DataStorage>,
    config: SharedDpsMeterConfig,
    // unified: StreamAssembler,
    // unified1: StreamAssembler,
    assemblers: HashMap<String, TrackedAssembler>,
    nickname_assemblers: HashMap<String, TrackedAssembler>,
    tcp_reassemblers: HashMap<TcpFlowKey, TrackedTcpReassembler>,
    magic_hits: HashMap<String, VecDeque<Instant>>,
    recent_ports: RecentPortWindow,
    logged_packets: usize,
    logged_magic_packets: usize,
}

struct TrackedAssembler {
    assembler: StreamAssembler,
    last_packet_at: Instant,
}

struct TrackedTcpReassembler {
    reassembler: TcpReassembler,
    last_seen_at: Instant,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpReassemblyFlowStatus {
    pub connection: String,
    pub source: String,
    pub destination: String,
    pub next_sequence: Option<u32>,
    pub pending_segments: usize,
    pub held_bytes: usize,
    pub retransmits: u64,
    pub gap_skips: u64,
    pub idle_ms: u64,
    pub is_combat_flow: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpReassemblyStatus {
    pub combat_connection: Option<String>,
    pub total_flows: usize,
    pub total_pending_segments: usize,
    pub total_held_bytes: usize,
    pub total_retransmits: u64,
    pub total_gap_skips: u64,
    pub flows: Vec<TcpReassemblyFlowStatus>,
}

impl TrackedAssembler {
    fn new(assembler: StreamAssembler) -> Self {
        Self {
            assembler,
            last_packet_at: Instant::now(),
        }
    }

    fn mark_packet_received(&mut self) {
        self.last_packet_at = Instant::now();
    }
}

impl DispatcherState {
    fn new(
        data_storage: Arc<DataStorage>,
        _logger: Arc<AppLogger>,
        config: SharedDpsMeterConfig,
    ) -> Self {
        Self {
            data_storage: Arc::clone(&data_storage),
            config: Arc::clone(&config),
            assemblers: HashMap::new(),
            nickname_assemblers: HashMap::new(),
            tcp_reassemblers: HashMap::new(),
            magic_hits: HashMap::new(),
            recent_ports: RecentPortWindow::new(Duration::from_secs(2)),
            logged_packets: 0,
            logged_magic_packets: 0,
        }
    }

    fn clear(&mut self) {
        // self.unified.clear();
        // self.unified1.clear();
        for assembler in self.assemblers.values() {
            assembler.assembler.clear();
        }
        for assembler in self.nickname_assemblers.values() {
            assembler.assembler.clear();
        }
        self.assemblers.clear();
        self.nickname_assemblers.clear();
        self.tcp_reassemblers.clear();
        self.magic_hits.clear();
        self.recent_ports.entries.clear();
        self.logged_packets = 0;
        self.logged_magic_packets = 0;
    }

    fn tracked_assemblers_mut(
        &mut self,
        key: &str,
    ) -> impl Iterator<Item = &mut TrackedAssembler> + '_ {
        self.assemblers
            .get_mut(key)
            .into_iter()
            .chain(self.nickname_assemblers.get_mut(key))
    }

    fn ensure_candidate_assemblers(&mut self, key: &str, logger: &Arc<AppLogger>) {
        let data_storage = Arc::clone(&self.data_storage);
        let config = Arc::clone(&self.config);
        self.assemblers.entry(key.to_string()).or_insert_with({
            let key = key.to_string();
            let data_storage = Arc::clone(&data_storage);
            let logger = Arc::clone(logger);
            let config = Arc::clone(&config);
            move || TrackedAssembler::new(StreamAssembler::new(data_storage, logger, key, config))
        });

        self.nickname_assemblers
            .entry(key.to_string())
            .or_insert_with({
                let key = key.to_string();
                let logger = Arc::clone(logger);
                move || {
                    TrackedAssembler::new(StreamAssembler::new_nickname_only(
                        data_storage,
                        logger,
                        format!("{key}:nickname"),
                        config,
                    ))
                }
            });
    }
}

#[derive(Clone)]
pub struct CaptureDispatcher {
    channel: Channel<CapturedPacket>,
    recorder: Arc<PacketRecorder>,
    rolling_recorder: Arc<RollingRecorder>,
    logger: Arc<AppLogger>,
    ping_tracker: Arc<PingTracker>,
    running: Arc<AtomicBool>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    combat_port: Arc<RwLock<Option<String>>>,
    state: Arc<Mutex<DispatcherState>>,
}

impl CaptureDispatcher {
    pub fn new(
        channel: Channel<CapturedPacket>,
        recorder: Arc<PacketRecorder>,
        rolling_recorder: Arc<RollingRecorder>,
        data_storage: Arc<DataStorage>,
        logger: Arc<AppLogger>,
        ping_tracker: Arc<PingTracker>,
        config: SharedDpsMeterConfig,
    ) -> Self {
        let state = Arc::new(Mutex::new(DispatcherState::new(
            data_storage,
            Arc::clone(&logger),
            Arc::clone(&config),
        )));

        Self {
            channel,
            recorder,
            rolling_recorder,
            logger,
            ping_tracker,
            running: Arc::new(AtomicBool::new(false)),
            thread: Arc::new(Mutex::new(None)),
            combat_port: Arc::new(RwLock::new(None)),
            state,
        }
    }

    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let channel = self.channel.clone();
        let recorder = Arc::clone(&self.recorder);
        let rolling_recorder = Arc::clone(&self.rolling_recorder);
        let logger = Arc::clone(&self.logger);
        let ping_tracker = Arc::clone(&self.ping_tracker);
        let running = Arc::clone(&self.running);
        let combat_port = Arc::clone(&self.combat_port);
        let state = Arc::clone(&self.state);

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let mut packet = match channel.receive(Some(Duration::from_millis(100))) {
                    Some(packet) => packet,
                    None => continue,
                };

                // Tapped here, before reassembly, so a recording holds exactly
                // what capture produced -- replay then re-runs the real pipeline.
                recorder.record(&packet);
                rolling_recorder.record(&packet);

                if is_web_flow(packet.src_port, packet.dst_port) {
                    continue;
                }

                let mut state = state.lock().unwrap();
                let flow_key = TcpFlowKey {
                    src_ip: packet.src_ip,
                    src_port: packet.src_port,
                    dst_ip: packet.dst_ip,
                    dst_port: packet.dst_port,
                };
                let key = directional_connection_key(
                    packet.src_ip,
                    packet.src_port,
                    packet.dst_ip,
                    packet.dst_port,
                );
                let tracked_reassembler =
                    state.tcp_reassemblers.entry(flow_key).or_insert_with(|| {
                        TrackedTcpReassembler {
                            reassembler: TcpReassembler::default(),
                            last_seen_at: Instant::now(),
                        }
                    });
                tracked_reassembler.last_seen_at = Instant::now();
                let reassembler = &mut tracked_reassembler.reassembler;
                let previous_retransmits = reassembler.retransmits();
                let previous_gap_skips = reassembler.gap_skips();
                let reassembly = reassembler.feed(
                    packet.sequence,
                    std::mem::take(&mut packet.data),
                    packet.captured_at,
                );
                let retransmits = reassembler.retransmits();
                let gap_skips = reassembler.gap_skips();

                for tracked in state.tracked_assemblers_mut(&key) {
                    tracked.mark_packet_received();
                }
                if let Some(gap_skip) = reassembly.gap_skip {
                    logger.info(format!(
                        "tcp sequence gap skipped src={}:{} dst={}:{} missing_bytes={} waited={:.1}s held_bytes={}",
                        format_ipv4(packet.src_ip),
                        packet.src_port,
                        format_ipv4(packet.dst_ip),
                        packet.dst_port,
                        gap_skip.missing_bytes,
                        gap_skip.waited_secs,
                        gap_skip.held_bytes
                    ));
                    for tracked in state.tracked_assemblers_mut(&key) {
                        tracked.assembler.discard_buffered();
                    }
                }
                if reassembly.segments.is_empty() {
                    continue;
                }
                packet.data = reassembly.segments.concat();
                if retransmits != previous_retransmits || gap_skips != previous_gap_skips {
                    logger.debug(format!(
                        "tcp reassembly src={}:{} dst={}:{} retransmits={} gap_skips={}",
                        format_ipv4(packet.src_ip),
                        packet.src_port,
                        format_ipv4(packet.dst_ip),
                        packet.dst_port,
                        retransmits,
                        gap_skips
                    ));
                }
                ping_tracker.on_packet(&packet.data, packet.captured_at);

                if state.logged_packets < 20 {
                    logger.info(format!(
                        "dispatcher packet src={}:{} dst={}:{} sequence={} payload_len={} captured_at={:.3}",
                        format_ipv4(packet.src_ip),
                        packet.src_port,
                        format_ipv4(packet.dst_ip),
                        packet.dst_port,
                        packet.sequence,
                        packet.data.len(),
                        packet.captured_at
                    ));
                    state.logged_packets += 1;
                }

                if looks_like_tls_payload(&packet.data) {
                    // logger.debug(format!(
                    //     "dispatcher skip tls-like payload src={} dst={} len={}",
                    //     packet.src_port,
                    //     packet.dst_port,
                    //     packet.data.len()
                    // ));
                    continue;
                }

                let contains_magic = packet
                    .data
                    .windows(MAGIC.len())
                    .any(|window| window == MAGIC);

                if contains_magic {
                    // Carrying the heartbeat identifies this host as the game
                    // server, which is what region detection keys off.
                    crate::region::observe_server_ip(packet.src_ip);
                }

                if contains_magic && state.logged_magic_packets < 20 {
                    // logger.debug(format!(
                    //     "dispatcher magic packet key={} payload_len={} head={}",
                    //     key,
                    //     packet.data.len(),
                    //     format_packet_prefix(&packet.data, 24)
                    // ));
                    state.logged_magic_packets += 1;
                }

                let has_enough_magic_hits = if contains_magic {
                    state.ensure_candidate_assemblers(&key, &logger);
                    let now = Instant::now();
                    let hits = state.magic_hits.entry(key.clone()).or_default();
                    while hits.front().is_some_and(|hit_at| {
                        now.saturating_duration_since(*hit_at) > MAGIC_LOCK_WINDOW
                    }) {
                        hits.pop_front();
                    }
                    hits.push_back(now);
                    hits.len() > MAGIC_LOCK_THRESHOLD
                } else {
                    false
                };

                if contains_magic && has_enough_magic_hits {
                    if let Some(locked) = state.recent_ports.add_and_get_locked(key.clone()) {
                        *combat_port.write().unwrap() = Some(locked.clone());
                        state.ensure_candidate_assemblers(&locked, &logger);
                    }
                }

                let is_candidate_connection = state.assemblers.contains_key(&key);
                if is_candidate_connection {
                    let combat_damage_enabled =
                        combat_port.read().unwrap().as_deref() == Some(key.as_str());
                    let assembler = state
                        .assemblers
                        .get_mut(&key)
                        .expect("candidate assembler must exist");
                    let _ = assembler.assembler.process_chunk(
                        &packet.data,
                        combat_damage_enabled,
                        packet.captured_at,
                    );

                    let nickname_assembler = state
                        .nickname_assemblers
                        .get_mut(&key)
                        .expect("candidate nickname assembler must exist");
                    let _ = nickname_assembler
                        .assembler
                        .process_chunk(&packet.data, false, packet.captured_at);
                }
            }
        });

        *self.thread.lock().unwrap() = Some(handle);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().unwrap().take() {
            let _ = handle.join();
        }
        *self.combat_port.write().unwrap() = None;
    }

    pub fn clear(&self) {
        *self.combat_port.write().unwrap() = None;
        self.state.lock().unwrap().clear();
    }

    pub fn current_combat_port(&self) -> Option<String> {
        self.combat_port.read().unwrap().clone()
    }

    /// Check whether any game data packets have been detected recently.
    /// Returns `false` if no matching packets have been seen (game may not be running).
    pub fn has_recent_ports(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.recent_ports.entries.is_empty()
    }

    pub fn cleanup_stale_assemblers(&self, max_idle: Duration) -> Vec<String> {
        let now = Instant::now();
        let mut state = self.state.lock().unwrap();
        let mut combat_port = self.combat_port.write().unwrap();
        let mut removed = Vec::new();
        state
            .tcp_reassemblers
            .retain(|_, tracked| now.duration_since(tracked.last_seen_at) <= max_idle);

        state.assemblers.retain(|key, tracked| {
            let is_stale = now.duration_since(tracked.last_packet_at) > max_idle;
            if !is_stale {
                return true;
            }

            tracked.assembler.clear();
            if combat_port.as_deref() == Some(key.as_str()) {
                *combat_port = None;
            }
            removed.push(key.clone());
            false
        });

        for key in &removed {
            if let Some(tracked) = state.nickname_assemblers.remove(key) {
                tracked.assembler.clear();
            }
        }

        removed
    }

    pub fn assembler_buffer_sizes(&self) -> HashMap<String, usize> {
        let state = self.state.lock().unwrap();
        let mut sizes = HashMap::new();

        // let unified_size = state.unified.buffer_size();
        // if unified_size > 0 {
        //     sizes.insert("unified".to_string(), unified_size);
        // }

        // let unified1_size = state.unified1.buffer_size();
        // if unified1_size > 0 {
        //     sizes.insert("unified1".to_string(), unified1_size);
        // }

        for (key, assembler) in &state.assemblers {
            let size = assembler.assembler.buffer_size();
            sizes.insert(key.clone(), size);
        }
        for (key, assembler) in &state.nickname_assemblers {
            let size = assembler.assembler.buffer_size();
            sizes.insert(format!("{key}:nickname"), size);
        }

        sizes
    }

    pub fn tcp_reassembly_status(&self) -> TcpReassemblyStatus {
        let now = Instant::now();
        let combat_connection = self.combat_port.read().unwrap().clone();
        let state = self.state.lock().unwrap();
        let mut flows: Vec<TcpReassemblyFlowStatus> = state
            .tcp_reassemblers
            .iter()
            .map(|(key, tracked)| {
                let connection =
                    directional_connection_key(key.src_ip, key.src_port, key.dst_ip, key.dst_port);
                TcpReassemblyFlowStatus {
                    source: format!("{}:{}", format_ipv4(key.src_ip), key.src_port),
                    destination: format!("{}:{}", format_ipv4(key.dst_ip), key.dst_port),
                    next_sequence: tracked.reassembler.next_sequence(),
                    pending_segments: tracked.reassembler.pending_segments(),
                    held_bytes: tracked.reassembler.held_bytes(),
                    retransmits: tracked.reassembler.retransmits(),
                    gap_skips: tracked.reassembler.gap_skips(),
                    idle_ms: now
                        .saturating_duration_since(tracked.last_seen_at)
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64,
                    is_combat_flow: combat_connection.as_deref() == Some(connection.as_str()),
                    connection,
                }
            })
            .collect();
        flows.sort_by(|left, right| {
            right
                .is_combat_flow
                .cmp(&left.is_combat_flow)
                .then_with(|| right.held_bytes.cmp(&left.held_bytes))
                .then_with(|| left.idle_ms.cmp(&right.idle_ms))
        });

        TcpReassemblyStatus {
            combat_connection,
            total_flows: flows.len(),
            total_pending_segments: flows.iter().map(|flow| flow.pending_segments).sum(),
            total_held_bytes: flows.iter().map(|flow| flow.held_bytes).sum(),
            total_retransmits: flows.iter().map(|flow| flow.retransmits).sum(),
            total_gap_skips: flows.iter().map(|flow| flow.gap_skips).sum(),
            flows,
        }
    }
}

impl Drop for CaptureDispatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The game never talks over HTTP or HTTPS; CDN downloads on these ports
/// carry the heartbeat bytes by chance.
pub(crate) fn is_web_flow(src_port: u16, dst_port: u16) -> bool {
    WEB_PORTS.contains(&src_port) || WEB_PORTS.contains(&dst_port)
}

fn looks_like_tls_payload(data: &[u8]) -> bool {
    data.len() >= 3
        && TLS_CONTENT_TYPES.contains(&data[0])
        && data[1] == 0x03
        && TLS_VERSIONS.contains(&data[2])
}

fn directional_connection_key(
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
) -> String {
    format!(
        "{}:{src_port}->{}:{dst_port}",
        format_ipv4(src_ip),
        format_ipv4(dst_ip)
    )
}

fn format_ipv4(ip: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    use super::*;
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::mobs::catalog::MobCatalog;

    const CLIENT: [u8; 4] = [192, 168, 1, 20];
    const SERVER: [u8; 4] = [52, 1, 2, 3];
    const CDN: [u8; 4] = [99, 84, 1, 2];
    const HEARTBEAT: [u8; 5] = [0x01, 0x0E, 0x00, 0x36, 0x02];

    struct Silent;
    impl EventSink for Silent {
        fn emit(&self, _: MeterEvent) {}
    }

    fn dispatcher() -> CaptureDispatcher {
        let dir = std::env::temp_dir().join("aether-dispatcher-test");
        let logger = Arc::new(AppLogger::new(dir.join("test.log"), Box::new(|_| {})));
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = Arc::new(DataStorage::new(Arc::new(Silent), Arc::clone(&config), mobs));
        storage.set_fight_tracking(true);
        CaptureDispatcher::new(
            Channel::new(1_000),
            Arc::new(PacketRecorder::new()),
            Arc::new(RollingRecorder::new(Arc::clone(&logger), Arc::new(AtomicBool::new(false)))),
            storage,
            logger,
            Arc::new(PingTracker::new()),
            config,
        )
    }

    fn heartbeat_from(server: [u8; 4], server_port: u16, sequence: u32) -> CapturedPacket {
        CapturedPacket {
            src_ip: server,
            src_port: server_port,
            dst_ip: CLIENT,
            dst_port: 50_000,
            sequence,
            data: HEARTBEAT.to_vec(),
            captured_at: 1_700_000_000.0,
        }
    }

    /// Feeds `web_packets`, then a game flow as the last packets. Packets are
    /// handled in order, so once the game flow is locked the web packets
    /// have been handled too. Returns the locked flow and the flows that got
    /// assemblers.
    fn feed_then_game_flow(dispatcher: &CaptureDispatcher, web_packets: Vec<CapturedPacket>) -> (String, Vec<String>) {
        for packet in web_packets
            .into_iter()
            .chain([heartbeat_from(SERVER, 13_328, 3_000), heartbeat_from(SERVER, 13_328, 3_005)])
        {
            assert!(dispatcher.channel.try_send(packet));
        }
        dispatcher.start();
        let deadline = Instant::now() + Duration::from_secs(5);
        let locked = loop {
            if let Some(locked) = dispatcher.current_combat_port() {
                break locked;
            }
            assert!(Instant::now() < deadline, "the game flow was not locked");
            thread::sleep(Duration::from_millis(10));
        };
        let mut flows: Vec<String> = dispatcher.assembler_buffer_sizes().into_keys().collect();
        flows.sort();
        dispatcher.stop();
        (locked, flows)
    }

    #[test]
    fn a_flow_on_a_web_port_carrying_the_heartbeat_bytes_is_never_a_candidate() {
        let dispatcher = dispatcher();
        let web_packets = vec![
            heartbeat_from(CDN, 443, 1_000),
            heartbeat_from(CDN, 443, 1_005),
            heartbeat_from(CDN, 80, 2_000),
            heartbeat_from(CDN, 80, 2_005),
        ];

        let (_, flows) = feed_then_game_flow(&dispatcher, web_packets);

        assert!(flows.iter().all(|flow| flow.starts_with("52.1.2.3:13328")), "{flows:?}");
    }

    #[test]
    fn a_flow_on_the_game_port_carrying_the_heartbeat_is_still_locked() {
        let dispatcher = dispatcher();

        let (locked, flows) = feed_then_game_flow(&dispatcher, Vec::new());

        assert_eq!(locked, "52.1.2.3:13328->192.168.1.20:50000");
        assert_eq!(flows.len(), 2, "the flow and its nickname assembler: {flows:?}");
    }
}
