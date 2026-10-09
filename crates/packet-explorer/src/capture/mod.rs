//! Turns recording files (`.belisk` and the older `.aetherpc`) into server packets the way the meter's dispatcher
//! does: the core's recording reader and TCP reassembler, the TLS filter, and
//! the heartbeat that identifies the game server. Framing is `framer`.

pub mod framer;
pub mod labels;

use std::collections::HashMap;
use std::path::Path;

use meter_core::capture::recorder::replay_recording;
use meter_core::capture::tcp_reassembler::{TcpFlowKey, TcpReassembler};
use serde::Serialize;

use framer::{Frame, FrameStats, Framer};

pub const MAGIC: [u8; 3] = [0x0E, 0x00, 0x36];
const TLS_CONTENT_TYPES: [u8; 4] = [0x14, 0x15, 0x16, 0x17];
const TLS_VERSIONS: [u8; 5] = [0x00, 0x01, 0x02, 0x03, 0x04];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Direction {
    #[serde(rename = "s2c")]
    ServerToClient,
    #[serde(rename = "c2s")]
    ClientToServer,
}

impl Direction {
    pub fn label(self) -> &'static str {
        match self {
            Direction::ServerToClient => "S>C",
            Direction::ClientToServer => "C>S",
        }
    }
}

pub struct FileInfo {
    pub name: String,
    pub start: f64,
    pub end: f64,
    pub records: u64,
}

/// Server packets carry their payload from the opcode on; client packets are
/// encrypted, so each one is a whole reassembled chunk with no opcode.
pub struct Packet {
    pub id: usize,
    pub file: usize,
    pub t: f64,
    pub dir: Direction,
    pub in_bundle: bool,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn opcode(&self) -> Option<[u8; 2]> {
        match self.dir {
            Direction::ServerToClient => Some([*self.data.first()?, *self.data.get(1)?]),
            Direction::ClientToServer => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowRole {
    GameServer,
    GameClient,
    Other,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowInfo {
    pub file: usize,
    pub source: String,
    pub destination: String,
    pub role: FlowRole,
    pub chunks: u64,
    pub bytes: u64,
    pub bytes_before_first_heartbeat: u64,
    pub first: f64,
    pub last: f64,
    pub stats: FrameStats,
    pub leftover_bytes: usize,
}

pub struct Capture {
    pub files: Vec<FileInfo>,
    pub packets: Vec<Packet>,
    pub flows: Vec<FlowInfo>,
}

pub struct Chunk {
    pub t: f64,
    pub flow: usize,
    pub data: Vec<u8>,
    pub follows_gap: bool,
}

pub struct RawFile {
    pub info: FileInfo,
    pub flow_keys: Vec<TcpFlowKey>,
    pub chunks: Vec<Chunk>,
}

pub fn contains_heartbeat(data: &[u8]) -> bool {
    data.windows(MAGIC.len()).any(|window| window == MAGIC)
}

fn looks_like_tls(data: &[u8]) -> bool {
    data.len() >= 3
        && TLS_CONTENT_TYPES.contains(&data[0])
        && data[1] == 0x03
        && TLS_VERSIONS.contains(&data[2])
}

fn endpoint(ip: [u8; 4], port: u16) -> String {
    format!("{}.{}.{}.{}:{port}", ip[0], ip[1], ip[2], ip[3])
}

pub fn read_raw(path: &Path) -> Result<RawFile, String> {
    let mut flow_index: HashMap<TcpFlowKey, usize> = HashMap::new();
    let mut flow_keys = Vec::new();
    let mut reassemblers: Vec<TcpReassembler> = Vec::new();
    let mut gap_before_next_chunk: Vec<bool> = Vec::new();
    let mut chunks = Vec::new();
    let (mut start, mut end, mut records) = (f64::INFINITY, f64::NEG_INFINITY, 0u64);

    replay_recording(path, |packet| {
        records += 1;
        if packet.captured_at.is_finite() {
            start = start.min(packet.captured_at);
            end = end.max(packet.captured_at);
        }
        let key = TcpFlowKey {
            src_ip: packet.src_ip,
            src_port: packet.src_port,
            dst_ip: packet.dst_ip,
            dst_port: packet.dst_port,
        };
        let flow = *flow_index.entry(key).or_insert_with(|| {
            flow_keys.push(key);
            reassemblers.push(TcpReassembler::default());
            gap_before_next_chunk.push(false);
            flow_keys.len() - 1
        });
        let reassembly = reassemblers[flow].feed(packet.sequence, packet.data, packet.captured_at);
        gap_before_next_chunk[flow] |= reassembly.gap_skip.is_some();
        if reassembly.segments.is_empty() {
            return true;
        }
        let data = reassembly.segments.concat();
        if !looks_like_tls(&data) {
            let follows_gap = std::mem::take(&mut gap_before_next_chunk[flow]);
            chunks.push(Chunk { t: packet.captured_at, flow, data, follows_gap });
        }
        true
    })?;

    if records == 0 {
        return Err("the recording holds no packets".to_string());
    }
    let name = path
        .file_name()
        .map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned());
    Ok(RawFile {
        info: FileInfo { name, start, end, records },
        flow_keys,
        chunks,
    })
}

fn reverse(key: TcpFlowKey) -> TcpFlowKey {
    TcpFlowKey {
        src_ip: key.dst_ip,
        src_port: key.dst_port,
        dst_ip: key.src_ip,
        dst_port: key.src_port,
    }
}

/// Flows that carry the heartbeat are the game server's side; their reverse
/// flows are the client's encrypted side.
pub fn classify_flows(raw: &RawFile) -> Vec<FlowRole> {
    let mut has_heartbeat = vec![false; raw.flow_keys.len()];
    for chunk in &raw.chunks {
        if !has_heartbeat[chunk.flow] && contains_heartbeat(&chunk.data) {
            has_heartbeat[chunk.flow] = true;
        }
    }
    let index: HashMap<TcpFlowKey, usize> =
        raw.flow_keys.iter().enumerate().map(|(i, key)| (*key, i)).collect();
    raw.flow_keys
        .iter()
        .enumerate()
        .map(|(flow, key)| {
            if has_heartbeat[flow] {
                FlowRole::GameServer
            } else if index.get(&reverse(*key)).is_some_and(|&other| has_heartbeat[other]) {
                FlowRole::GameClient
            } else {
                FlowRole::Other
            }
        })
        .collect()
}

struct FramedFile {
    packets: Vec<Packet>,
    flows: Vec<FlowInfo>,
}

fn frame_file(raw: &RawFile, file: usize) -> FramedFile {
    let roles = classify_flows(raw);
    let mut framers: Vec<Framer> = raw.flow_keys.iter().map(|_| Framer::default()).collect();
    let mut flows: Vec<FlowInfo> = raw
        .flow_keys
        .iter()
        .zip(&roles)
        .map(|(key, role)| FlowInfo {
            file,
            source: endpoint(key.src_ip, key.src_port),
            destination: endpoint(key.dst_ip, key.dst_port),
            role: *role,
            chunks: 0,
            bytes: 0,
            bytes_before_first_heartbeat: 0,
            first: f64::INFINITY,
            last: f64::NEG_INFINITY,
            stats: FrameStats::default(),
            leftover_bytes: 0,
        })
        .collect();
    let mut seen_heartbeat = vec![false; flows.len()];
    let mut packets = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();

    for chunk in &raw.chunks {
        let flow = &mut flows[chunk.flow];
        flow.chunks += 1;
        flow.bytes += chunk.data.len() as u64;
        flow.first = flow.first.min(chunk.t);
        flow.last = flow.last.max(chunk.t);
        if !seen_heartbeat[chunk.flow] && contains_heartbeat(&chunk.data) {
            seen_heartbeat[chunk.flow] = true;
        }
        if !seen_heartbeat[chunk.flow] {
            flow.bytes_before_first_heartbeat += chunk.data.len() as u64;
        }

        match roles[chunk.flow] {
            FlowRole::GameServer => {
                if chunk.follows_gap {
                    framers[chunk.flow].discard_buffered();
                }
                framers[chunk.flow].feed(chunk.t, &chunk.data, &mut frames);
                for frame in frames.drain(..) {
                    packets.push(Packet {
                        id: 0,
                        file,
                        t: chunk.t,
                        dir: Direction::ServerToClient,
                        in_bundle: frame.in_bundle,
                        data: frame.payload,
                    });
                }
            }
            FlowRole::GameClient => packets.push(Packet {
                id: 0,
                file,
                t: chunk.t,
                dir: Direction::ClientToServer,
                in_bundle: false,
                data: chunk.data.clone(),
            }),
            FlowRole::Other => {}
        }
    }

    for (flow, framer) in flows.iter_mut().zip(framers) {
        flow.leftover_bytes = framer.leftover();
        flow.stats = framer.stats;
    }
    FramedFile { packets, flows }
}

pub fn load(paths: &[std::path::PathBuf]) -> Result<Capture, String> {
    let mut capture = Capture { files: Vec::new(), packets: Vec::new(), flows: Vec::new() };
    for path in paths {
        let raw = read_raw(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let framed = frame_file(&raw, capture.files.len());
        capture.files.push(raw.info);
        capture.packets.extend(framed.packets);
        capture.flows.extend(framed.flows);
    }
    for (index, packet) in capture.packets.iter_mut().enumerate() {
        packet.id = index + 1;
    }
    Ok(capture)
}
