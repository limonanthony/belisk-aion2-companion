//! Rolling recording of whole play sessions.
//!
//! A boss encounter or a game event worth investigating is only known to be
//! worth it afterwards, when starting a recording by hand is too late. While
//! this is enabled the game's packets are kept in files of
//! [`ROTATION_INTERVAL`] each, and the oldest are deleted once the files take
//! more than [`MAX_ROLLING_BYTES`].
//!
//! It is a second [`PacketRecorder`] next to the one the player and the
//! unfamiliar-server recording share, so all of them can run at once. Files use
//! the same format and replay like any other recording.
//!
//! Only game traffic is written. A packet belongs to the game when
//! - its connection is one the operating system lists for the game process
//!   (refreshed about once a second on a thread of its own, never per packet),
//! - or it goes to a port a game connection has used, which covers a new
//!   connection until the next refresh finds it,
//! - or its flow has carried the game's heartbeat, which covers a game the
//!   lookup cannot see (no lookup access, a replay-like source).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::capture::capturer::CapturedPacket;
use crate::capture::dispatcher::{is_web_flow, MAGIC};
use crate::capture::game_connections::{query_game_connections, GameConnection};
use crate::capture::recorder::{prune_recordings_to_size, PacketRecorder};
use crate::logger::AppLogger;

/// Rolling files are named `rolling-<unix ms>.aetherpc`.
pub const ROLLING_FILE_PREFIX: &str = "rolling-";
const ROLLING_NAME_PREFIX: &str = "rolling";

const ROTATION_INTERVAL: Duration = Duration::from_secs(30 * 60);
const MAX_ROLLING_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const CONNECTION_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(100);

type Endpoint = ([u8; 4], u16);

#[derive(Default)]
struct GameTrafficFilter {
    /// Local port and remote endpoint of each connection the game has open.
    connections: HashSet<(u16, Endpoint)>,
    server_ports: HashSet<u16>,
    /// Flows that carried the heartbeat, as an ordered pair of endpoints.
    heartbeat_flows: HashSet<(Endpoint, Endpoint)>,
}

impl GameTrafficFilter {
    fn set_connections(&mut self, connections: &[GameConnection]) {
        self.connections = connections
            .iter()
            .map(|connection| {
                (
                    connection.local_port,
                    (connection.remote_addr.octets(), connection.remote_port),
                )
            })
            .collect();
        self.server_ports
            .extend(connections.iter().map(|connection| connection.remote_port));
    }

    fn accepts(&mut self, packet: &CapturedPacket) -> bool {
        if is_web_flow(packet.src_port, packet.dst_port) {
            return false;
        }
        let source = (packet.src_ip, packet.src_port);
        let destination = (packet.dst_ip, packet.dst_port);

        if self.connections.contains(&(packet.src_port, destination))
            || self.connections.contains(&(packet.dst_port, source))
        {
            return true;
        }
        if self.server_ports.contains(&packet.src_port)
            || self.server_ports.contains(&packet.dst_port)
        {
            return true;
        }

        let flow = if source <= destination {
            (source, destination)
        } else {
            (destination, source)
        };
        if self.heartbeat_flows.contains(&flow) {
            return true;
        }
        if packet.data.windows(MAGIC.len()).any(|window| window == MAGIC) {
            self.heartbeat_flows.insert(flow);
            return true;
        }
        false
    }
}

struct RollingSession {
    dir: PathBuf,
    filter: GameTrafficFilter,
    file: PacketRecorder,
    file_opened_at: Option<Instant>,
    rotation_interval: Duration,
    max_bytes: u64,
    logger: Arc<AppLogger>,
}

impl RollingSession {
    fn new(dir: PathBuf, logger: Arc<AppLogger>) -> Self {
        Self {
            dir,
            filter: GameTrafficFilter::default(),
            file: PacketRecorder::new(),
            file_opened_at: None,
            rotation_interval: ROTATION_INTERVAL,
            max_bytes: MAX_ROLLING_BYTES,
            logger,
        }
    }

    fn record(&mut self, packet: &CapturedPacket, now: Instant) {
        if !self.filter.accepts(packet) {
            return;
        }

        let rotation_due = self
            .file_opened_at
            .is_some_and(|opened_at| now.saturating_duration_since(opened_at) >= self.rotation_interval);
        if rotation_due || self.file.is_full_for(packet) {
            self.close_file();
        }
        if self.file_opened_at.is_none() {
            self.open_file(now);
        }
        self.file.record(packet);
    }

    /// A file that could not be created is not retried for a whole interval, so
    /// a full disk costs one failed attempt rather than one per packet.
    fn open_file(&mut self, now: Instant) {
        self.file_opened_at = Some(now);
        match self.file.start_named(&self.dir, ROLLING_NAME_PREFIX) {
            Ok(path) => {
                self.logger
                    .info(format!("rolling recording started: {}", path.display()));
                prune_recordings_to_size(&self.dir, ROLLING_FILE_PREFIX, self.max_bytes);
            }
            Err(error) => self
                .logger
                .info(format!("rolling recording could not start: {error}")),
        }
    }

    fn close_file(&mut self) {
        self.file_opened_at = None;
        match self.file.stop() {
            Ok(Some(status)) => self.logger.info(format!(
                "rolling recording closed: {} packets, {} bytes",
                status.packets, status.bytes
            )),
            Ok(None) => {}
            Err(error) => self
                .logger
                .info(format!("rolling recording could not be closed: {error}")),
        }
    }
}

pub struct RollingRecorder {
    session: Arc<Mutex<Option<RollingSession>>>,
    stop_refresh: Mutex<Option<Arc<AtomicBool>>>,
    /// Set while a recording is being replayed through the live pipeline, whose
    /// packets are not this PC's traffic.
    replaying: Arc<AtomicBool>,
    logger: Arc<AppLogger>,
}

impl RollingRecorder {
    pub fn new(logger: Arc<AppLogger>, replaying: Arc<AtomicBool>) -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            stop_refresh: Mutex::new(None),
            replaying,
            logger,
        }
    }

    /// Start recording into `dir`. Does nothing while already recording.
    pub fn enable(&self, dir: PathBuf) {
        if !self.install(RollingSession::new(dir.clone(), Arc::clone(&self.logger))) {
            return;
        }

        let stop = Arc::new(AtomicBool::new(false));
        *self.stop_refresh.lock().unwrap() = Some(Arc::clone(&stop));
        let session = Arc::clone(&self.session);
        let logger = Arc::clone(&self.logger);
        thread::spawn(move || {
            prune_recordings_to_size(&dir, ROLLING_FILE_PREFIX, MAX_ROLLING_BYTES);
            refresh_game_connections(&session, &stop, &logger);
        });
    }

    /// Close the current file and stop recording.
    pub fn disable(&self) {
        if let Some(stop) = self.stop_refresh.lock().unwrap().take() {
            stop.store(true, Ordering::SeqCst);
        }
        let Some(mut session) = self.session.lock().unwrap().take() else {
            return;
        };
        session.close_file();
    }

    pub fn record(&self, packet: &CapturedPacket) {
        self.record_at(packet, Instant::now());
    }

    fn record_at(&self, packet: &CapturedPacket, now: Instant) {
        if self.replaying.load(Ordering::Relaxed) {
            return;
        }
        let Ok(mut session) = self.session.lock() else {
            return;
        };
        if let Some(session) = session.as_mut() {
            session.record(packet, now);
        }
    }

    fn install(&self, session: RollingSession) -> bool {
        let mut current = self.session.lock().unwrap();
        if current.is_some() {
            return false;
        }
        *current = Some(session);
        true
    }
}

fn refresh_game_connections(
    session: &Mutex<Option<RollingSession>>,
    stop: &AtomicBool,
    logger: &AppLogger,
) {
    let mut last_error: Option<String> = None;
    while !stop.load(Ordering::SeqCst) {
        match query_game_connections() {
            Ok(connections) => {
                last_error = None;
                if let Some(session) = session.lock().unwrap().as_mut() {
                    session.filter.set_connections(&connections);
                }
            }
            Err(error) => {
                if last_error.as_deref() != Some(error.as_str()) {
                    logger.info(format!(
                        "rolling recording: game connection lookup unavailable, keeping flows that carry the game heartbeat: {error}"
                    ));
                    last_error = Some(error);
                }
            }
        }

        let deadline = Instant::now() + CONNECTION_REFRESH_INTERVAL;
        while !stop.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::sleep(STOP_POLL_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::recorder::{list_recordings, replay_recording};
    use std::fs;
    use std::net::Ipv4Addr;
    use std::path::Path;

    const GAME_SERVER: [u8; 4] = [52, 1, 2, 3];
    const WEB_SERVER: [u8; 4] = [93, 184, 216, 34];
    const CLIENT: [u8; 4] = [192, 168, 1, 20];

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aether-rolling-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn logger(dir: &Path) -> Arc<AppLogger> {
        Arc::new(AppLogger::new(dir.join("test.log"), Box::new(|_| {})))
    }

    fn recorder(dir: &Path) -> RollingRecorder {
        let recorder = RollingRecorder::new(logger(dir), Arc::new(AtomicBool::new(false)));
        assert!(recorder.install(RollingSession::new(dir.to_path_buf(), logger(dir))));
        recorder
    }

    fn connect(recorder: &RollingRecorder, connections: &[GameConnection]) {
        recorder
            .session
            .lock()
            .unwrap()
            .as_mut()
            .expect("enabled")
            .filter
            .set_connections(connections);
    }

    fn game_connection(local_port: u16, server: [u8; 4], server_port: u16) -> GameConnection {
        GameConnection {
            local_addr: Ipv4Addr::from(CLIENT),
            local_port,
            remote_addr: Ipv4Addr::from(server),
            remote_port: server_port,
        }
    }

    fn from_server(server: [u8; 4], server_port: u16, local_port: u16, data: &[u8]) -> CapturedPacket {
        CapturedPacket {
            src_ip: server,
            src_port: server_port,
            dst_ip: CLIENT,
            dst_port: local_port,
            sequence: 1,
            data: data.to_vec(),
            captured_at: 1_700_000_000.0,
        }
    }

    fn to_server(server: [u8; 4], server_port: u16, local_port: u16, data: &[u8]) -> CapturedPacket {
        CapturedPacket {
            src_ip: CLIENT,
            src_port: local_port,
            dst_ip: server,
            dst_port: server_port,
            sequence: 1,
            data: data.to_vec(),
            captured_at: 1_700_000_000.0,
        }
    }

    fn recorded_payloads(dir: &Path) -> Vec<Vec<Vec<u8>>> {
        let mut files = list_recordings(dir);
        files.sort_by(|a, b| a.name.cmp(&b.name));
        files
            .iter()
            .filter(|file| file.name.starts_with(ROLLING_FILE_PREFIX))
            .map(|file| {
                let mut payloads = Vec::new();
                replay_recording(Path::new(&file.path), |packet| {
                    payloads.push(packet.data);
                    true
                })
                .expect("replay");
                payloads
            })
            .collect()
    }

    #[test]
    fn keeps_packets_of_game_connections_in_both_directions_and_nothing_else() {
        let dir = temp_dir("filter");
        let rolling = recorder(&dir);
        connect(&rolling, &[game_connection(50_000, GAME_SERVER, 13_328)]);
        let now = Instant::now();

        rolling.record_at(&to_server(GAME_SERVER, 13_328, 50_000, b"up"), now);
        rolling.record_at(&from_server(GAME_SERVER, 13_328, 50_000, b"down"), now);
        rolling.record_at(&to_server(WEB_SERVER, 443, 50_001, b"https"), now);
        rolling.record_at(&from_server(WEB_SERVER, 443, 50_001, b"https back"), now);
        rolling.record_at(&to_server(GAME_SERVER, 9_999, 50_002, b"other port"), now);
        rolling.disable();

        assert_eq!(recorded_payloads(&dir), vec![vec![b"up".to_vec(), b"down".to_vec()]]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_a_new_connection_to_a_known_game_port_before_the_next_refresh() {
        let dir = temp_dir("new-connection");
        let rolling = recorder(&dir);
        connect(&rolling, &[game_connection(50_000, GAME_SERVER, 13_328)]);
        let now = Instant::now();

        rolling.record_at(&to_server([52, 9, 9, 9], 13_328, 50_010, b"new channel"), now);
        rolling.record_at(&to_server([52, 9, 9, 9], 8_080, 50_011, b"unrelated"), now);
        rolling.disable();

        assert_eq!(recorded_payloads(&dir), vec![vec![b"new channel".to_vec()]]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_a_lookup_result_keeps_the_flow_that_carries_the_heartbeat() {
        let dir = temp_dir("fallback");
        let rolling = recorder(&dir);
        let now = Instant::now();
        let heartbeat = [0x01, 0x0E, 0x00, 0x36, 0x02];

        rolling.record_at(&to_server(WEB_SERVER, 443, 50_001, b"https"), now);
        rolling.record_at(&from_server(GAME_SERVER, 13_328, 50_000, &heartbeat), now);
        rolling.record_at(&to_server(GAME_SERVER, 13_328, 50_000, b"reply"), now);
        rolling.record_at(&from_server(GAME_SERVER, 13_328, 50_000, b"data"), now);
        rolling.record_at(&to_server(WEB_SERVER, 443, 50_001, b"https again"), now);
        rolling.disable();

        assert_eq!(
            recorded_payloads(&dir),
            vec![vec![heartbeat.to_vec(), b"reply".to_vec(), b"data".to_vec()]]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn starts_a_new_file_after_the_rotation_interval() {
        let dir = temp_dir("rotation");
        let rolling = recorder(&dir);
        connect(&rolling, &[game_connection(50_000, GAME_SERVER, 13_328)]);
        let start = Instant::now();
        let packet = |data: &[u8]| to_server(GAME_SERVER, 13_328, 50_000, data);

        rolling.record_at(&packet(b"first"), start);
        rolling.record_at(&packet(b"second"), start + Duration::from_secs(29 * 60));
        rolling.record_at(&packet(b"third"), start + Duration::from_secs(30 * 60));
        rolling.record_at(&packet(b"fourth"), start + Duration::from_secs(31 * 60));
        rolling.disable();

        assert_eq!(
            recorded_payloads(&dir),
            vec![
                vec![b"first".to_vec(), b"second".to_vec()],
                vec![b"third".to_vec(), b"fourth".to_vec()]
            ]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reaching_the_size_cap_starts_a_new_file_and_keeps_recording() {
        let dir = temp_dir("size-cap");
        let rolling = recorder(&dir);
        connect(&rolling, &[game_connection(50_000, GAME_SERVER, 13_328)]);
        let two_five_byte_packets = 12 + 2 * (28 + 5);
        rolling.session.lock().unwrap().as_mut().unwrap().file =
            PacketRecorder::with_max_bytes(two_five_byte_packets);
        let now = Instant::now();
        let packet = |data: &[u8]| to_server(GAME_SERVER, 13_328, 50_000, data);

        for data in [b"one11", b"two22", b"thr33", b"fou44", b"fiv55"] {
            rolling.record_at(&packet(data), now);
        }
        rolling.disable();

        assert_eq!(
            recorded_payloads(&dir),
            vec![
                vec![b"one11".to_vec(), b"two22".to_vec()],
                vec![b"thr33".to_vec(), b"fou44".to_vec()],
                vec![b"fiv55".to_vec()]
            ]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_web_port_flow_carrying_the_heartbeat_bytes_is_not_a_game_flow() {
        let dir = temp_dir("web-heartbeat");
        let rolling = recorder(&dir);
        let now = Instant::now();
        let heartbeat = [0x01, 0x0E, 0x00, 0x36, 0x02];

        rolling.record_at(&from_server(WEB_SERVER, 443, 49_901, &heartbeat), now);
        rolling.record_at(&to_server(WEB_SERVER, 443, 49_901, b"next"), now);
        rolling.record_at(&from_server(WEB_SERVER, 80, 49_902, &heartbeat), now);
        rolling.disable();

        assert!(recorded_payloads(&dir).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rotation_deletes_the_oldest_rolling_files_and_only_those() {
        let dir = temp_dir("rotation-prune");
        fs::create_dir_all(&dir).expect("dir");
        for (name, age_secs) in [
            ("rolling-1.aetherpc", 300),
            ("rolling-2.aetherpc", 200),
            ("session-1.aetherpc", 400),
            ("auto-1.aetherpc", 400),
        ] {
            let path = dir.join(name);
            fs::write(&path, vec![0u8; 1_000]).expect("write");
            fs::File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.set_modified(std::time::SystemTime::now() - Duration::from_secs(age_secs)))
                .expect("set modified");
        }
        let rolling = recorder(&dir);
        rolling.session.lock().unwrap().as_mut().unwrap().max_bytes = 500;
        connect(&rolling, &[game_connection(50_000, GAME_SERVER, 13_328)]);

        rolling.record_at(&to_server(GAME_SERVER, 13_328, 50_000, b"now"), Instant::now());
        rolling.disable();

        let mut names: Vec<String> = list_recordings(&dir).into_iter().map(|file| file.name).collect();
        names.retain(|name| !name.starts_with(ROLLING_FILE_PREFIX));
        names.sort();
        assert_eq!(names, vec!["auto-1.aetherpc", "session-1.aetherpc"]);
        let rolling_left: Vec<String> = list_recordings(&dir)
            .into_iter()
            .map(|file| file.name)
            .filter(|name| name.starts_with(ROLLING_FILE_PREFIX))
            .collect();
        assert_eq!(rolling_left.len(), 1, "the file in progress is kept, the old ones go: {rolling_left:?}");
        assert!(!rolling_left.contains(&"rolling-1.aetherpc".to_string()));
        assert!(!rolling_left.contains(&"rolling-2.aetherpc".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_alongside_a_manual_recording() {
        let dir = temp_dir("coexist");
        let rolling = recorder(&dir);
        connect(&rolling, &[game_connection(50_000, GAME_SERVER, 13_328)]);
        let manual = PacketRecorder::new();
        let packet = to_server(GAME_SERVER, 13_328, 50_000, b"both");
        let now = Instant::now();

        rolling.record_at(&packet, now);
        let manual_path = manual.start(&dir).expect("a manual recording starts while rolling runs");
        rolling.record_at(&packet, now);
        manual.record(&packet);
        manual.stop().expect("stop manual");
        rolling.record_at(&packet, now);
        rolling.disable();

        assert_eq!(recorded_payloads(&dir), vec![vec![b"both".to_vec(); 3]]);
        let mut manual_packets = 0;
        replay_recording(&manual_path, |_| {
            manual_packets += 1;
            true
        })
        .expect("replay manual");
        assert_eq!(manual_packets, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_nothing_while_disabled_or_while_a_replay_is_running() {
        let dir = temp_dir("suspended");
        let replaying = Arc::new(AtomicBool::new(false));
        let rolling = RollingRecorder::new(logger(&dir), Arc::clone(&replaying));
        let packet = from_server(GAME_SERVER, 13_328, 50_000, &[0x0E, 0x00, 0x36]);
        let now = Instant::now();

        rolling.record_at(&packet, now);
        assert!(list_recordings(&dir).is_empty(), "disabled");

        assert!(rolling.install(RollingSession::new(dir.clone(), logger(&dir))));
        replaying.store(true, Ordering::SeqCst);
        rolling.record_at(&packet, now);
        assert!(list_recordings(&dir).is_empty(), "replaying");

        replaying.store(false, Ordering::SeqCst);
        rolling.record_at(&packet, now);
        rolling.disable();
        assert_eq!(recorded_payloads(&dir).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
