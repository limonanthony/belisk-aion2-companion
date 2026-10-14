use std::ffi::{c_char, c_int, c_uint, CStr, CString};
use std::ptr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::net::Ipv4Addr;

use libloading::{Library, Symbol};
use serde::Serialize;

use crate::capture::channel::Channel;
use crate::capture::dispatcher::is_web_flow;
use crate::capture::game_connections::{query_game_connections, GameConnection};
use crate::capture::game_flow_gate::GameFlowGate;
use crate::logger::AppLogger;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedPacket {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub dst_ip: [u8; 4],
    pub dst_port: u16,
    pub sequence: u32,
    pub data: Vec<u8>,
    pub captured_at: f64,
}

#[derive(Debug, Clone)]
pub struct CaptureTarget {
    pub device_name: String,
    pub target_port: Option<String>,
}

type PcapT = *mut std::ffi::c_void;
type PcapIfT = *mut PcapIf;

#[repr(C)]
struct PcapIf {
    next: *mut PcapIf,
    name: *const c_char,
    description: *const c_char,
    addresses: *mut PcapAddr,
    flags: c_uint,
}

#[repr(C)]
struct PcapAddr {
    next: *mut PcapAddr,
    addr: *mut SockAddr,
    netmask: *mut SockAddr,
    broadaddr: *mut SockAddr,
    dstaddr: *mut SockAddr,
}

#[repr(C)]
struct SockAddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

#[repr(C)]
struct PcapPkthdr {
    ts_sec: i32,
    ts_usec: i32,
    caplen: c_uint,
    len: c_uint,
}

const PCAP_IF_LOOPBACK: c_uint = 0x0000_0001;
const AF_INET_FAMILY: u16 = 2;
const GAME_CONNECTION_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAGIC_PATTERN: [u8; 3] = [0x0E, 0x00, 0x36];

#[derive(Clone)]
struct DeviceInfo {
    name: String,
    description: String,
    has_addresses: bool,
    ipv4_addresses: Vec<Ipv4Addr>,
    is_loopback: bool,
}

impl DeviceInfo {
    fn label(&self) -> &str {
        if self.description.is_empty() {
            &self.name
        } else {
            &self.description
        }
    }

    fn is_virtual(&self) -> bool {
        let label = self.label().to_ascii_lowercase();
        let name = self.name.to_ascii_lowercase();

        self.is_loopback
            || name.contains("loopback")
            || label.contains("loopback")
            || label.contains("tap-windows")
            || label.contains("tap")
            || label.contains("wintun")
            || label.contains("wireguard")
    }
}

struct NpcapLib {
    _lib: Library,
    findalldevs: unsafe extern "C" fn(*mut PcapIfT, *mut c_char) -> c_int,
    freealldevs: unsafe extern "C" fn(PcapIfT),
    open_live: unsafe extern "C" fn(*const c_char, c_int, c_int, c_int, *mut c_char) -> PcapT,
    close: unsafe extern "C" fn(PcapT),
    next_ex: unsafe extern "C" fn(PcapT, *mut *mut PcapPkthdr, *mut *const u8) -> c_int,
}

impl NpcapLib {
    fn load() -> Result<Self, String> {
        let lib = unsafe {
            Library::new("wpcap.dll").map_err(|error| {
                format!(
                    "Failed to load wpcap.dll. Please install Npcap from https://npcap.com. Error: {error}"
                )
            })?
        };

        unsafe {
            let findalldevs: Symbol<unsafe extern "C" fn(*mut PcapIfT, *mut c_char) -> c_int> = lib
                .get(b"pcap_findalldevs")
                .map_err(|error| format!("pcap_findalldevs: {error}"))?;
            let freealldevs: Symbol<unsafe extern "C" fn(PcapIfT)> =
                lib.get(b"pcap_freealldevs")
                    .map_err(|error| format!("pcap_freealldevs: {error}"))?;
            let open_live: Symbol<
                unsafe extern "C" fn(*const c_char, c_int, c_int, c_int, *mut c_char) -> PcapT,
            > = lib
                .get(b"pcap_open_live")
                .map_err(|error| format!("pcap_open_live: {error}"))?;
            let close: Symbol<unsafe extern "C" fn(PcapT)> = lib
                .get(b"pcap_close")
                .map_err(|error| format!("pcap_close: {error}"))?;
            let next_ex: Symbol<
                unsafe extern "C" fn(PcapT, *mut *mut PcapPkthdr, *mut *const u8) -> c_int,
            > = lib
                .get(b"pcap_next_ex")
                .map_err(|error| format!("pcap_next_ex: {error}"))?;

            Ok(Self {
                findalldevs: *findalldevs,
                freealldevs: *freealldevs,
                open_live: *open_live,
                close: *close,
                next_ex: *next_ex,
                _lib: lib,
            })
        }
    }

    fn find_all_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let mut all_devices: PcapIfT = ptr::null_mut();
        let mut errbuf = [0u8; 256];

        let ret =
            unsafe { (self.findalldevs)(&mut all_devices, errbuf.as_mut_ptr() as *mut c_char) };
        if ret != 0 || all_devices.is_null() {
            let error = unsafe { CStr::from_ptr(errbuf.as_ptr() as *const c_char) }
                .to_string_lossy()
                .to_string();
            return Err(format!("pcap_findalldevs failed: {error}"));
        }

        let mut devices = Vec::new();
        let mut current = all_devices;
        while !current.is_null() {
            let device = unsafe { &*current };
            let name = if device.name.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(device.name) }
                    .to_string_lossy()
                    .to_string()
            };
            let description = if device.description.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(device.description) }
                    .to_string_lossy()
                    .to_string()
            };

            devices.push(DeviceInfo {
                name,
                description,
                has_addresses: !device.addresses.is_null(),
                ipv4_addresses: collect_ipv4_addresses(device.addresses),
                is_loopback: (device.flags & PCAP_IF_LOOPBACK) != 0,
            });

            current = device.next;
        }

        unsafe { (self.freealldevs)(all_devices) };
        Ok(devices)
    }

    fn open_live_handle(&self, device_name: &str, timeout_ms: i32) -> Result<PcapT, String> {
        let device_name = CString::new(device_name)
            .map_err(|error| format!("Invalid device name for pcap_open_live: {error}"))?;
        let mut errbuf = [0u8; 256];

        let handle = unsafe {
            (self.open_live)(
                device_name.as_ptr(),
                65_535,
                1,
                timeout_ms,
                errbuf.as_mut_ptr() as *mut c_char,
            )
        };

        if handle.is_null() {
            let error = unsafe { CStr::from_ptr(errbuf.as_ptr() as *const c_char) }
                .to_string_lossy()
                .to_string();
            return Err(format!("pcap_open_live failed: {error}"));
        }

        Ok(handle)
    }
}

unsafe impl Send for NpcapLib {}
unsafe impl Sync for NpcapLib {}

#[derive(Clone)]
pub struct PcapCapturer {
    channel: Channel<CapturedPacket>,
    logger: Arc<AppLogger>,
    running: Arc<AtomicBool>,
    detector_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    capture_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
    target_device: Arc<RwLock<Option<String>>>,
    target_port: Arc<RwLock<Option<String>>>,
    detection_interval: Duration,
    detection_timeout: Duration,
}

impl PcapCapturer {
    pub fn new(channel: Channel<CapturedPacket>, logger: Arc<AppLogger>) -> Self {
        Self {
            channel,
            logger,
            running: Arc::new(AtomicBool::new(false)),
            detector_thread: Arc::new(Mutex::new(None)),
            capture_threads: Arc::new(Mutex::new(Vec::new())),
            target_device: Arc::new(RwLock::new(None)),
            target_port: Arc::new(RwLock::new(None)),
            detection_interval: Duration::from_secs(20),
            detection_timeout: Duration::from_secs(1),
        }
    }

    pub fn run(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let channel = self.channel.clone();
        let logger = Arc::clone(&self.logger);
        let running = Arc::clone(&self.running);
        let capture_threads = Arc::clone(&self.capture_threads);
        let target_device = Arc::clone(&self.target_device);
        let target_port = Arc::clone(&self.target_port);
        let detection_interval = self.detection_interval;
        let detection_timeout = self.detection_timeout;

        let detector_handle = thread::spawn(move || {
            let npcap = match NpcapLib::load() {
                Ok(api) => Arc::new(api),
                Err(error) => {
                    logger.error(format!("Npcap initialization failed: {error}"));
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            let mut last_device_inventory = String::new();
            let mut last_detected_target = String::new();
            let mut last_hinted_devices = String::new();
            let mut game_connections = GameConnectionWatch::new(Arc::clone(&logger));

            while running.load(Ordering::SeqCst) {
                game_connections.refresh();
                // 1. find magic devices
                let devices = match npcap.find_all_devices() {
                    Ok(devices) => devices,
                    Err(error) => {
                        logger.error(format!("Failed to enumerate capture devices: {error}"));
                        wait_for_next_detection(&running, detection_interval, &mut game_connections);
                        continue;
                    }
                };

                let devices = prioritize_devices(devices);
                let inventory = format_device_inventory(&devices);
                if inventory != last_device_inventory {
                    if devices.is_empty() {
                        logger.info("capture device: none");
                    } else {
                        for device_line in &devices {
                            logger.info(format!(
                                "capture device: {} | desc={} | has_addresses={} | loopback={} | virtual={}",
                                device_line.name,
                                if device_line.description.is_empty() {
                                    "--"
                                } else {
                                    device_line.description.as_str()
                                },
                                device_line.has_addresses,
                                device_line.is_loopback,
                                device_line.is_virtual()
                            ));
                        }
                    }
                    last_device_inventory = inventory;
                }

                let devices = prefer_devices_with_game_connection(devices, game_connections.current());
                let hinted_devices = devices
                    .iter()
                    .filter(|device| carries_game_connection(device, game_connections.current()))
                    .map(|device| device.label())
                    .collect::<Vec<_>>()
                    .join(", ");
                if hinted_devices != last_hinted_devices {
                    if !hinted_devices.is_empty() {
                        logger.info(format!(
                            "game connection hint: probing first: {hinted_devices}"
                        ));
                    }
                    last_hinted_devices = hinted_devices;
                }
                let mut detected_target: Option<CaptureTarget> = None;

                for device in devices {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Some(target_port_value) = inspect_device_for_magic(
                        npcap.as_ref(),
                        &device,
                        detection_timeout,
                        &running,
                    ) {
                        detected_target = Some(CaptureTarget {
                            device_name: device.name,
                            target_port: Some(target_port_value),
                        });
                        break;
                    }
                }

                if let Some(target) = detected_target {
                    let detected_signature = format!(
                        "{}@{}",
                        target.device_name,
                        target.target_port.as_deref().unwrap_or("--")
                    );
                    if detected_signature != last_detected_target {
                        logger.info(format!(
                            "capture target detected: device={} port={}",
                            target.device_name,
                            target.target_port.as_deref().unwrap_or("--")
                        ));
                        last_detected_target = detected_signature;
                    }

                    let previous_device = target_device.read().unwrap().clone();
                    let should_restart = previous_device.as_deref()
                        != Some(target.device_name.as_str())
                        || capture_threads.lock().unwrap().is_empty();

                    *target_device.write().unwrap() = Some(target.device_name.clone());
                    *target_port.write().unwrap() = target.target_port.clone();

                    if should_restart {
                        stop_capture_threads(&capture_threads);
                        start_capture_thread(
                            Arc::clone(&npcap),
                            target.device_name,
                            channel.clone(),
                            Arc::clone(&running),
                            Arc::clone(&capture_threads),
                            Arc::clone(&logger),
                        );
                    }
                }

                wait_for_next_detection(&running, detection_interval, &mut game_connections);
            }

            stop_capture_threads(&capture_threads);
        });

        *self.detector_thread.lock().unwrap() = Some(detector_handle);
    }

    pub fn start(&self) -> Result<(), String> {
        self.stop();
        NpcapLib::load()?;
        self.run();
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.detector_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
        stop_capture_threads(&self.capture_threads);
        *self.target_device.write().unwrap() = None;
        *self.target_port.write().unwrap() = None;
    }

    pub fn target_device(&self) -> Option<String> {
        self.target_device.read().unwrap().clone()
    }

    pub fn target_port(&self) -> Option<String> {
        self.target_port.read().unwrap().clone()
    }

    pub fn channel(&self) -> Channel<CapturedPacket> {
        self.channel.clone()
    }
}

impl Drop for PcapCapturer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A stricter check than loading `wpcap.dll`, for the startup gate.
///
/// Loading `wpcap.dll` only proves a file is on the search path. It keeps
/// succeeding after the Npcap service is stopped or its driver is uninstalled
/// from under it, so a gate built on that would let a user into an app that can
/// never see a packet. Enumerating adapters actually reaches the driver.
///
/// Returns how many non-loopback adapters are visible.
pub fn probe_npcap() -> Result<usize, String> {
    let npcap = NpcapLib::load().map_err(|_| {
        "Npcap is not installed, or was installed without WinPcap API-compatible Mode."
            .to_string()
    })?;

    let devices = npcap.find_all_devices().map_err(|error| {
        format!("Npcap is installed but would not list adapters ({error}).")
    })?;

    let usable = devices.iter().filter(|device| !device.is_loopback).count();
    if usable == 0 {
        return Err(
            "Npcap answered but reported no network adapters. Its driver service may be stopped."
                .to_string(),
        );
    }

    Ok(usable)
}

fn format_device_inventory(devices: &[DeviceInfo]) -> String {
    if devices.is_empty() {
        return "none".to_string();
    }

    devices
        .iter()
        .map(|device| {
            format!(
                "{}|{}|{}|{}|{}",
                device.name,
                device.description,
                device.has_addresses,
                device.is_loopback,
                device.is_virtual()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_ipv4_addresses(mut address: *mut PcapAddr) -> Vec<Ipv4Addr> {
    let mut addresses = Vec::new();
    while !address.is_null() {
        let entry = unsafe { &*address };
        if !entry.addr.is_null() {
            let sockaddr = unsafe { &*entry.addr };
            if sockaddr.sa_family == AF_INET_FAMILY {
                let [_, _, a, b, c, d, ..] = sockaddr.sa_data;
                addresses.push(Ipv4Addr::new(a, b, c, d));
            }
        }
        address = entry.next;
    }
    addresses
}

fn prioritize_devices(mut devices: Vec<DeviceInfo>) -> Vec<DeviceInfo> {
    devices.sort_by_key(|device| {
        let loopback_priority = if device.is_loopback || device.is_virtual() {
            0
        } else {
            1
        };
        let address_priority =
            if !device.is_loopback && !device.is_virtual() && !device.has_addresses {
                1
            } else {
                0
            };
        let name = device.label().to_ascii_lowercase();
        (loopback_priority, address_priority, name)
    });
    devices
}

fn carries_game_connection(device: &DeviceInfo, connections: &[GameConnection]) -> bool {
    connections.iter().any(|connection| {
        if connection.goes_through_loopback() {
            device.is_loopback
        } else {
            device.ipv4_addresses.contains(&connection.local_addr)
        }
    })
}

fn prefer_devices_with_game_connection(
    mut devices: Vec<DeviceInfo>,
    connections: &[GameConnection],
) -> Vec<DeviceInfo> {
    devices.sort_by_key(|device| !carries_game_connection(device, connections));
    devices
}

fn inspect_device_for_magic(
    npcap: &NpcapLib,
    device: &DeviceInfo,
    timeout: Duration,
    running: &Arc<AtomicBool>,
) -> Option<String> {
    let handle = npcap.open_live_handle(&device.name, 100).ok()?;
    let started_at = Instant::now();

    while running.load(Ordering::SeqCst) && started_at.elapsed() < timeout {
        match next_captured_packet(npcap, handle) {
            CaptureRead::Packet(packet) => {
                if !is_web_flow(packet.src_port, packet.dst_port)
                    && packet
                        .data
                        .windows(MAGIC_PATTERN.len())
                        .any(|window| window == MAGIC_PATTERN)
                {
                    let (a, b) = if packet.src_port <= packet.dst_port {
                        (packet.src_port, packet.dst_port)
                    } else {
                        (packet.dst_port, packet.src_port)
                    };
                    unsafe { (npcap.close)(handle) };
                    return Some(format!("{a}-{b}"));
                }
            }
            CaptureRead::Timeout => continue,
            CaptureRead::End => break,
            CaptureRead::Error => break,
        }
    }

    unsafe { (npcap.close)(handle) };
    None
}

fn start_capture_thread(
    npcap: Arc<NpcapLib>,
    device_name: String,
    channel: Channel<CapturedPacket>,
    running: Arc<AtomicBool>,
    capture_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
    logger: Arc<AppLogger>,
) {
    let handle = thread::spawn(move || {
        let capture_handle = match npcap.open_live_handle(&device_name, 100) {
            Ok(handle) => handle,
            Err(error) => {
                logger.error(format!("Failed to open capture on {device_name}: {error}"));
                return;
            }
        };

        let mut gate = GameFlowGate::new(Instant::now());
        while running.load(Ordering::SeqCst) {
            match next_captured_packet(&npcap, capture_handle) {
                CaptureRead::Packet(packet) => {
                    let admitted = gate.admits(&packet, Instant::now(), |connection| {
                        logger.info(format!(
                            "Npcap capture candidate detected on {device_name}: {}-{}",
                            connection.0, connection.1
                        ));
                    });
                    if admitted {
                        let _ = channel.try_send(packet);
                    }
                }
                CaptureRead::Timeout => continue,
                CaptureRead::End => break,
                CaptureRead::Error => break,
            }
        }

        unsafe { (npcap.close)(capture_handle) };
    });

    capture_threads.lock().unwrap().push(handle);
}

fn stop_capture_threads(capture_threads: &Arc<Mutex<Vec<JoinHandle<()>>>>) {
    let handles = {
        let mut guard = capture_threads.lock().unwrap();
        std::mem::take(&mut *guard)
    };

    for handle in handles {
        let _ = handle.join();
    }
}

enum CaptureRead {
    Packet(CapturedPacket),
    Timeout,
    End,
    Error,
}

fn next_captured_packet(npcap: &NpcapLib, handle: PcapT) -> CaptureRead {
    let mut header: *mut PcapPkthdr = ptr::null_mut();
    let mut data: *const u8 = ptr::null();

    let ret = unsafe { (npcap.next_ex)(handle, &mut header, &mut data) };
    match ret {
        1 => {
            let header = unsafe { &*header };
            let frame = unsafe { std::slice::from_raw_parts(data, header.caplen as usize) };
            parse_captured_packet(frame, header)
                .map(CaptureRead::Packet)
                .unwrap_or(CaptureRead::Timeout)
        }
        0 => CaptureRead::Timeout,
        -2 => CaptureRead::End,
        _ => CaptureRead::Error,
    }
}

fn parse_captured_packet(frame: &[u8], header: &PcapPkthdr) -> Option<CapturedPacket> {
    let ip_offset = detect_ip_offset(frame)?;
    if frame.len() < ip_offset + 20 {
        return None;
    }

    let ip_header = &frame[ip_offset..];
    if (ip_header[0] >> 4) != 4 {
        return None;
    }

    let ip_header_len = ((ip_header[0] & 0x0F) as usize) * 4;
    if ip_header[9] != 6 {
        return None;
    }

    let tcp_offset = ip_offset + ip_header_len;
    if frame.len() < tcp_offset + 20 {
        return None;
    }

    let tcp_header = &frame[tcp_offset..];
    let src_port = u16::from_be_bytes([tcp_header[0], tcp_header[1]]);
    let dst_port = u16::from_be_bytes([tcp_header[2], tcp_header[3]]);
    let sequence = u32::from_be_bytes([tcp_header[4], tcp_header[5], tcp_header[6], tcp_header[7]]);
    let tcp_header_len = ((tcp_header[12] >> 4) as usize) * 4;
    let payload_offset = tcp_offset + tcp_header_len;
    let ip_total_len =
        usize::from(u16::from_be_bytes([ip_header[2], ip_header[3]])).min(frame.len() - ip_offset);
    let payload_end = ip_offset + ip_total_len;
    if payload_offset >= payload_end {
        return None;
    }

    let payload = &frame[payload_offset..payload_end];
    if payload.is_empty() {
        return None;
    }

    Some(CapturedPacket {
        src_ip: [ip_header[12], ip_header[13], ip_header[14], ip_header[15]],
        src_port,
        dst_ip: [ip_header[16], ip_header[17], ip_header[18], ip_header[19]],
        dst_port,
        sequence,
        data: payload.to_vec(),
        captured_at: pcap_header_timestamp_seconds(header),
    })
}

fn detect_ip_offset(frame: &[u8]) -> Option<usize> {
    if frame.len() >= 14 {
        let ether_type = u16::from_be_bytes([frame[12], frame[13]]);
        if ether_type == 0x0800 {
            return Some(14);
        }
        if frame[0] == 2 && frame[1] == 0 && frame[2] == 0 && frame[3] == 0 {
            return Some(4);
        }
    }

    if !frame.is_empty() && (frame[0] >> 4) == 4 {
        return Some(0);
    }

    None
}

fn pcap_header_timestamp_seconds(header: &PcapPkthdr) -> f64 {
    let from_pcap = (header.ts_sec as f64) + (header.ts_usec as f64 / 1_000_000.0);
    if from_pcap > 1_000_000_000.0 {
        from_pcap
    } else {
        current_timestamp_seconds()
    }
}

fn current_timestamp_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

fn sleep_while_running(running: &Arc<AtomicBool>, duration: Duration) {
    let deadline = Instant::now() + duration;
    while running.load(Ordering::SeqCst) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
    }
}

struct GameConnectionWatch {
    logger: Arc<AppLogger>,
    connections: Vec<GameConnection>,
    last_error: Option<String>,
}

impl GameConnectionWatch {
    fn new(logger: Arc<AppLogger>) -> Self {
        Self {
            logger,
            connections: Vec::new(),
            last_error: None,
        }
    }

    fn current(&self) -> &[GameConnection] {
        &self.connections
    }

    /// Returns true when the game has connections and the set differs from the previous read.
    fn refresh(&mut self) -> bool {
        let latest = match query_game_connections() {
            Ok(connections) => {
                self.last_error = None;
                connections
            }
            Err(error) => {
                if self.last_error.as_deref() != Some(error.as_str()) {
                    self.logger
                        .info(format!("game connection hint unavailable: {error}"));
                    self.last_error = Some(error);
                }
                Vec::new()
            }
        };

        if latest == self.connections {
            return false;
        }

        if latest.is_empty() {
            self.logger.info("game connections: none");
        } else {
            self.logger.info(format!(
                "game connections: {}",
                latest
                    .iter()
                    .map(GameConnection::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        let has_connections = !latest.is_empty();
        self.connections = latest;
        has_connections
    }
}

fn wait_for_next_detection(
    running: &Arc<AtomicBool>,
    detection_interval: Duration,
    game_connections: &mut GameConnectionWatch,
) {
    let deadline = Instant::now() + detection_interval;
    while running.load(Ordering::SeqCst) && Instant::now() < deadline {
        sleep_while_running(running, GAME_CONNECTION_POLL_INTERVAL.min(detection_interval));
        if game_connections.refresh() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, is_loopback: bool, addresses: &[Ipv4Addr]) -> DeviceInfo {
        DeviceInfo {
            name: name.to_string(),
            description: name.to_string(),
            has_addresses: !addresses.is_empty(),
            ipv4_addresses: addresses.to_vec(),
            is_loopback,
        }
    }

    fn connection(local: Ipv4Addr, remote: Ipv4Addr) -> GameConnection {
        GameConnection {
            local_addr: local,
            local_port: 50_000,
            remote_addr: remote,
            remote_port: 13_328,
        }
    }

    fn names(devices: &[DeviceInfo]) -> Vec<&str> {
        devices.iter().map(|device| device.name.as_str()).collect()
    }

    fn sample_devices() -> Vec<DeviceInfo> {
        prioritize_devices(vec![
            device("ethernet", false, &[Ipv4Addr::new(192, 168, 1, 20)]),
            device("wifi", false, &[Ipv4Addr::new(10, 0, 0, 5)]),
            device("loopback", true, &[]),
        ])
    }

    #[test]
    fn adapter_owning_the_local_address_is_probed_first() {
        let connections = [connection(
            Ipv4Addr::new(10, 0, 0, 5),
            Ipv4Addr::new(203, 0, 113, 7),
        )];

        let ordered = prefer_devices_with_game_connection(sample_devices(), &connections);

        assert_eq!(names(&ordered), ["wifi", "loopback", "ethernet"]);
    }

    #[test]
    fn loopback_adapter_is_probed_first_when_the_remote_is_localhost() {
        let connections = [connection(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST)];

        let ordered = prefer_devices_with_game_connection(sample_devices(), &connections);

        assert_eq!(names(&ordered), ["loopback", "ethernet", "wifi"]);
    }

    #[test]
    fn device_order_is_unchanged_without_a_hint() {
        let ordered = prefer_devices_with_game_connection(sample_devices(), &[]);

        assert_eq!(names(&ordered), names(&sample_devices()));
    }

    #[test]
    fn device_order_is_unchanged_when_no_adapter_owns_the_local_address() {
        let connections = [connection(
            Ipv4Addr::new(172, 16, 0, 9),
            Ipv4Addr::new(203, 0, 113, 7),
        )];

        let ordered = prefer_devices_with_game_connection(sample_devices(), &connections);

        assert_eq!(names(&ordered), names(&sample_devices()));
    }

    fn sockaddr(family: u16, sa_data: [u8; 14]) -> SockAddr {
        SockAddr {
            sa_family: family,
            sa_data,
        }
    }

    #[test]
    fn collects_only_ipv4_addresses_from_the_pcap_address_chain() {
        const AF_INET6_FAMILY: u16 = 23;
        let mut ipv4 = sockaddr(AF_INET_FAMILY, [0x34, 0x12, 192, 168, 1, 20, 0, 0, 0, 0, 0, 0, 0, 0]);
        let mut ipv6 = sockaddr(AF_INET6_FAMILY, [0, 0, 0, 0, 0, 0, 0xFE, 0x80, 0, 0, 0, 0, 0, 0]);
        let mut second_ipv4 = sockaddr(AF_INET_FAMILY, [0, 0, 10, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0]);

        let mut last = PcapAddr {
            next: ptr::null_mut(),
            addr: &mut second_ipv4,
            netmask: ptr::null_mut(),
            broadaddr: ptr::null_mut(),
            dstaddr: ptr::null_mut(),
        };
        let mut without_address = PcapAddr {
            next: &mut last,
            addr: ptr::null_mut(),
            netmask: ptr::null_mut(),
            broadaddr: ptr::null_mut(),
            dstaddr: ptr::null_mut(),
        };
        let mut v6_entry = PcapAddr {
            next: &mut without_address,
            addr: &mut ipv6,
            netmask: ptr::null_mut(),
            broadaddr: ptr::null_mut(),
            dstaddr: ptr::null_mut(),
        };
        let mut first = PcapAddr {
            next: &mut v6_entry,
            addr: &mut ipv4,
            netmask: ptr::null_mut(),
            broadaddr: ptr::null_mut(),
            dstaddr: ptr::null_mut(),
        };

        assert_eq!(
            collect_ipv4_addresses(&mut first),
            [Ipv4Addr::new(192, 168, 1, 20), Ipv4Addr::new(10, 0, 0, 5)]
        );
        assert!(collect_ipv4_addresses(ptr::null_mut()).is_empty());
    }
}
