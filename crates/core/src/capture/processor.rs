use lz4_flex::block::decompress;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::capture::parser::context::ParserContext;
use crate::capture::parser::nickname;
use crate::capture::parser::utils::{frame_total_bytes, read_varint};
use crate::config::SharedDpsMeterConfig;
use crate::capture::census;
use crate::storage::data_storage::DataStorage;
use crate::logger::AppLogger;

const KNOWN_PACKET_HEADERS: &[(u8, u8)] = &[
    (0x33, 0x36),
    (0x45, 0x36),
    (0x56, 0x36),
    (0x41, 0x36),
    (0x0B, 0x61),
    (0x0C, 0x61),
    (0x04, 0x38),
    (0x05, 0x38),
    (0x2A, 0x38),
    (0x2B, 0x38),
    (0x2C, 0x38),
    (0x42, 0x36),
    // Player death and killer. Parsed in the combat flow only, but known
    // everywhere: it keeps the stall resync and the census treating it as ours.
    (0x04, 0x8D),
    (0x00, 0x8D),
    // Field boss timers. No longer parsed since 2.2.0, but still a known
    // opcode: it keeps the stall resync and the census treating it as ours.
    (0x01, 0x91),
    (0xFF, 0xFF),
];

/// A head that keeps receiving bytes restarts its stall delay, since a large
/// packet can span many TCP segments; this bounds how long a garbage length
/// can hold back a live stream.
const MAX_INCOMPLETE_PACKET_WAIT: Duration = Duration::from_secs(1);

/// Packet prefix metadata.
///
/// An optional extension byte in the 0xF0..0xFE range may appear after the
/// length varint. It is not part of the business opcode.
#[derive(Debug, Clone, Copy)]
struct PacketPrefixInfo {
    /// Offset of the business payload and opcode.
    payload_offset: usize,
}

/// Resolve the transport prefix following the length field.
///
/// Supported layouts:
/// 1. [length varint][opcode...]
/// 2. [length varint][extraFlag][opcode...]
fn resolve_packet_prefix(packet: &[u8], length_offset: usize) -> Option<PacketPrefixInfo> {
    let first_byte = *packet.get(length_offset)?;
    let has_extra_flag = (0xF0..0xFF).contains(&first_byte);
    let payload_offset = length_offset + if has_extra_flag { 1 } else { 0 };

    if payload_offset >= packet.len() {
        return None;
    }

    Some(PacketPrefixInfo { payload_offset })
}

fn inspect_stall_candidate(buffer: &[u8]) -> StallCandidate {
    let length_info = read_varint(buffer, 0);
    let declared_packet_len = if length_info.is_valid() && length_info.value > 0 {
        frame_total_bytes(&length_info)
    } else {
        None
    };
    let opcode = if length_info.is_valid() {
        resolve_packet_prefix(buffer, length_info.length).and_then(|prefix| {
            Some((
                *buffer.get(prefix.payload_offset)?,
                *buffer.get(prefix.payload_offset + 1)?,
            ))
        })
    } else {
        None
    };

    StallCandidate {
        declared_packet_len,
        opcode,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessorMode {
    Full,
    NicknameOnly,
}

#[derive(Debug, Clone, Copy)]
enum StallResyncMode {
    Immediate,
    Delayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StallCandidate {
    declared_packet_len: Option<usize>,
    opcode: Option<(u8, u8)>,
}

/// Whether a connection has settled which actor is the local character. The
/// game's name packet settles it, as does the character's own remain-HP packet
/// (see `parse_remain_hp_packet_at`); the first of them wins, and a connection
/// keeps its answer: a zone change opens a new connection with a new id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OwnActor {
    Unsettled,
    Settled,
}

impl OwnActor {
    pub(super) fn settle(&mut self) {
        *self = Self::Settled;
    }

    pub(super) fn is_unsettled(self) -> bool {
        self == Self::Unsettled
    }
}

pub struct StreamProcessor {
    pub(super) data_storage: Arc<DataStorage>,
    pub(super) logger: Arc<AppLogger>,
    pub(super) port: String,
    pub(super) config: SharedDpsMeterConfig,
    mode: ProcessorMode,
    stall_resync_mode: StallResyncMode,
    stalled_since: Option<Instant>,
    stalled_candidate: Option<StallCandidate>,
    stall_began: Option<Instant>,
    stalled_buffer_len: usize,
    hunting_packet_start: bool,
    combat_damage_enabled: bool,
    pub(super) own_actor: OwnActor,
    pub(super) captured_at: f64,
}

impl StreamProcessor {
    pub fn new(
        data_storage: Arc<DataStorage>,
        logger: Arc<AppLogger>,
        port: String,
        config: SharedDpsMeterConfig,
    ) -> Self {
        Self {
            data_storage,
            logger,
            port,
            config,
            mode: ProcessorMode::Full,
            stall_resync_mode: StallResyncMode::Immediate,
            stalled_since: None,
            stalled_candidate: None,
            stall_began: None,
            stalled_buffer_len: 0,
            hunting_packet_start: false,
            combat_damage_enabled: false,
            own_actor: OwnActor::Unsettled,
            captured_at: 0.0,
        }
    }

    pub fn new_nickname_only(
        data_storage: Arc<DataStorage>,
        logger: Arc<AppLogger>,
        port: String,
        config: SharedDpsMeterConfig,
    ) -> Self {
        Self {
            data_storage,
            logger,
            port,
            config,
            mode: ProcessorMode::NicknameOnly,
            stall_resync_mode: StallResyncMode::Delayed,
            stalled_since: None,
            stalled_candidate: None,
            stall_began: None,
            stalled_buffer_len: 0,
            hunting_packet_start: false,
            combat_damage_enabled: false,
            own_actor: OwnActor::Unsettled,
            captured_at: 0.0,
        }
    }

    pub fn set_combat_damage_enabled(&mut self, enabled: bool) {
        self.combat_damage_enabled = enabled;
    }

    pub fn set_captured_at(&mut self, captured_at: f64) {
        self.captured_at = captured_at;
    }

    pub fn reset_framing(&mut self) {
        self.stalled_since = None;
        self.stalled_candidate = None;
        self.stall_began = None;
        self.stalled_buffer_len = 0;
        self.hunting_packet_start = false;
    }

    pub fn consume_stream(&mut self, buffer: &[u8]) -> usize {
        let mut offset = 0usize;

        while offset < buffer.len() {
            if buffer[offset] == 0x00 {
                offset += 1;
                continue;
            }

            let length_info = read_varint(buffer, offset);
            if !length_info.is_valid() || length_info.value <= 0 {
                if offset + 5 > buffer.len() {
                    break;
                }
                offset += 1;
                continue;
            }

            let Some(total_packet_bytes) = frame_total_bytes(&length_info) else {
                offset += 1;
                continue;
            };

            if total_packet_bytes == 0 || total_packet_bytes > 65_535 {
                offset += 1;
                continue;
            }

            if offset + total_packet_bytes > buffer.len() {
                break;
            }

            let current_packet = &buffer[offset..offset + total_packet_bytes];
            let Some(prefix_info) = resolve_packet_prefix(current_packet, length_info.length)
            else {
                offset += 1;
                continue;
            };
            let payload_start = prefix_info.payload_offset;
            let is_bundle = payload_start + 1 < current_packet.len()
                && current_packet[payload_start] == 0xFF
                && current_packet[payload_start + 1] == 0xFF;

            if is_bundle {
                self.unwrap_bundle(&current_packet[payload_start..]);
            } else {
                self.parse_packet(current_packet, false);
            }
            offset += total_packet_bytes;
        }

        // if buffer.len() >= 4 {
        //     self.scan_for_embedded_048d(buffer);
        // }

        if offset == 0 && !buffer.is_empty() {
            let candidate: StallCandidate = inspect_stall_candidate(buffer);
            let is_known_or_undetermined = candidate
                .opcode
                .is_none_or(|opcode| KNOWN_PACKET_HEADERS.contains(&opcode));
            let delay_ms = if is_known_or_undetermined {
                let config = self.config.read().unwrap();
                match self.stall_resync_mode {
                    StallResyncMode::Immediate => config.full_processor_stall_resync_delay_ms,
                    StallResyncMode::Delayed => config.stall_resync_delay_ms,
                }
            } else {
                self.config
                    .read()
                    .unwrap()
                    .unknown_packet_stall_resync_delay_ms
            };

            if delay_ms == 0 {
                self.stalled_since = None;
                self.stalled_candidate = None;
                self.stall_began = None;
                
                self.logger.info(format!(
                    "[{}] stream stalled for {}ms with buffer_size={} declared_packet_len={:?} opcode={} known_or_undetermined={}, forcing resync by skipping 1 byte",
                    self.port,
                    delay_ms,
                    buffer.len(),
                    candidate.declared_packet_len,
                    candidate
                        .opcode
                        .map(|(first, second)| format!("{first:02X}{second:02X}"))
                        .unwrap_or_else(|| "undetermined".to_string()),
                    is_known_or_undetermined
                ));

                return 1;
            }

            if self.hunting_packet_start {
                self.stalled_since = None;
                self.stalled_candidate = None;
                self.stall_began = None;
                if buffer.len() == 1 {
                    self.hunting_packet_start = false;
                }
                return 1;
            }

            let now = Instant::now();
            if self.stalled_candidate != Some(candidate) {
                self.stalled_candidate = Some(candidate);
                self.stalled_since = Some(now);
                self.stall_began = Some(now);
                self.stalled_buffer_len = buffer.len();
                return 0;
            }

            if buffer.len() > self.stalled_buffer_len {
                self.stalled_buffer_len = buffer.len();
                self.stalled_since = Some(now);
            }
            let stall_began = *self.stall_began.get_or_insert(now);

            if let Some(stalled_since) = self.stalled_since {
                let delay = Duration::from_millis(delay_ms);
                if now.duration_since(stalled_since) >= delay
                    || now.duration_since(stall_began) >= delay.max(MAX_INCOMPLETE_PACKET_WAIT)
                {
                    self.logger.info(format!(
                        "[{}] stream stalled for {}ms with buffer_size={} declared_packet_len={:?} opcode={} known_or_undetermined={}, forcing resync by skipping 1 byte",
                        self.port,
                        delay_ms,
                        buffer.len(),
                        candidate.declared_packet_len,
                        candidate
                            .opcode
                            .map(|(first, second)| format!("{first:02X}{second:02X}"))
                            .unwrap_or_else(|| "undetermined".to_string()),
                        is_known_or_undetermined
                    ));
                    self.stalled_since = None;
                    self.stalled_candidate = None;
                    self.stall_began = None;
                    self.hunting_packet_start = true;
                    return 1;
                }
            } else {
                self.stalled_since = Some(now);
            }
        } else {
            self.stalled_since = None;
            self.stalled_candidate = None;
            self.stall_began = None;
            self.hunting_packet_start = false;
        }

        offset
    }

    fn unwrap_bundle(&mut self, payload: &[u8]) {
        if payload.len() < 7 {
            return;
        }

        let decompressed_size =
            u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]) as usize;
        if decompressed_size == 0 || decompressed_size > 5_000_000 {
            return;
        }

        let Ok(decompressed) = decompress(&payload[6..], decompressed_size) else {
            return;
        };

        let mut offset = 0usize;
        while offset < decompressed.len() {
            if decompressed[offset] == 0x00 {
                offset += 1;
                continue;
            }

            let length_info = read_varint(&decompressed, offset);
            if !length_info.is_valid() || length_info.value <= 0 {
                break;
            }

            let Some(inner_total_bytes) = frame_total_bytes(&length_info) else {
                break;
            };
            if inner_total_bytes == 0 {
                offset += 1;
                continue;
            }

            let inner_end = offset + inner_total_bytes;
            if inner_end > decompressed.len() {
                break;
            }

            let inner_packet = &decompressed[offset..inner_end];
            let Some(prefix_info) = resolve_packet_prefix(inner_packet, length_info.length) else {
                break;
            };
            let inner_payload_start = prefix_info.payload_offset;
            let is_nested_bundle = inner_packet.len() > inner_payload_start + 1
                && inner_packet[inner_payload_start] == 0xFF
                && inner_packet[inner_payload_start + 1] == 0xFF;

            if is_nested_bundle {
                self.unwrap_bundle(&inner_packet[inner_payload_start..]);
            } else {
                self.parse_packet(inner_packet, true);
            }

            offset += inner_total_bytes;
        }
    }

    fn parse_packet(&mut self, packet: &[u8], is_compressed_bundle: bool) -> bool {
        if packet.len() < 3 {
            return false;
        }

        let packet_length_info = read_varint(packet, 0);
        if !packet_length_info.is_valid() {
            return false;
        }

        let Some(prefix_info) = resolve_packet_prefix(packet, packet_length_info.length) else {
            return false;
        };

        let payload = &packet[prefix_info.payload_offset..];
        if payload.len() < 2 {
            return false;
        }

        // Tallied before dispatch, so unrecognised opcodes are counted too --
        // those are the ones worth knowing about on an unfamiliar service.
        let opcode = (payload[0], payload[1]);
        census::observe(opcode, payload.len(), KNOWN_PACKET_HEADERS.contains(&opcode));

        match self.mode {
            ProcessorMode::Full => match (payload[0], payload[1]) {
                (0x33, 0x36) => {
                    let parsed =
                        nickname::parse_main(&self.parser_context(), payload, is_compressed_bundle);
                    if parsed {
                        self.own_actor.settle();
                    }
                    parsed
                }
                (0x45, 0x36) => {
                    nickname::parse_other(&self.parser_context(), payload, is_compressed_bundle)
                }
                (0x56, 0x36) => {
                    nickname::parse_main_combat_power(&self.parser_context(), payload, is_compressed_bundle)
                }
                (0x41, 0x36) => self.parse_summon_packet(payload, is_compressed_bundle),
                (0x04, 0x38) if self.combat_damage_enabled && self.data_storage.wants_damage_packets() => {
                    self.parse_damage_packet(payload, is_compressed_bundle)
                }
                (0x05, 0x38) if self.combat_damage_enabled && self.data_storage.fight_tracking() => {
                    self.parse_dot_packet(payload, is_compressed_bundle)
                }
                (0x04, 0x8D) if self.combat_damage_enabled && self.data_storage.fight_tracking() => {
                    self.parse_char_die_packet(payload, is_compressed_bundle)
                }
                (0x2A, 0x38) | (0x2B, 0x38) if self.data_storage.fight_tracking() => {
                    self.parse_buff_packet(payload, is_compressed_bundle)
                }
                (0x2C, 0x38) if self.data_storage.fight_tracking() => {
                    self.parse_buff_removal_packet(payload, is_compressed_bundle)
                }
                (0x42, 0x36) => self.parse_death_packet(payload, is_compressed_bundle),
                (0x0B, 0x61) => self.parse_ticket_snapshot_packet(payload, is_compressed_bundle),
                (0x0C, 0x61) => self.parse_ticket_update_packet(payload, is_compressed_bundle),
                (0x00, 0x8D) => self.parse_remain_hp_packet(payload, is_compressed_bundle),
                _ => false,
            },
            ProcessorMode::NicknameOnly => match (payload[0], payload[1]) {
                (0x33, 0x36) => {
                    nickname::parse_main(&self.parser_context(), payload, is_compressed_bundle)
                }
                (0x45, 0x36) => {
                    nickname::parse_other(&self.parser_context(), payload, is_compressed_bundle)
                }
                (0x56, 0x36) => nickname::parse_main_combat_power(
                    &self.parser_context(),
                    payload,
                    is_compressed_bundle,
                ),
                (0x41, 0x36) => self.parse_summon_packet(payload, is_compressed_bundle),
                _ => false,
            },
        }
    }

    fn parser_context(&self) -> ParserContext<'_> {
        ParserContext::new(&self.data_storage, &self.logger, &self.port, self.captured_at)
    }
}

#[cfg(test)]
mod own_actor_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, RwLock};

    use super::StreamProcessor;
    use crate::capture::parser::spawn_record::frames::{MOB_CODE, OWNER, Spawn, with_owner};
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::identity::{IdentityDecision, KnownCharacter};
    use crate::capture::parser::ticket_record::tests::{record, snapshot, update};
    use crate::capture::parser::ticket_record::TicketReason;
    use crate::tickets::{TicketContent, TicketReport};
    use crate::logger::AppLogger;
    use crate::mobs::catalog::MobCatalog;
    use crate::models::packet::ParsedDamagePacket;
    use crate::storage::data_storage::DataStorage;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<MeterEvent>>);
    impl EventSink for Recorder {
        fn emit(&self, event: MeterEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    const BEFORE_ZONE_CHANGE: u32 = 6812;
    const AFTER_ZONE_CHANGE: u32 = 7735;
    const MOB: u32 = 900;
    const NAME: &str = "Tester";
    const RANGER_SKILL: u32 = 14_010_000;
    const TEMPLAR_SKILL: u32 = 12_010_000;
    const ELEMENTALIST_SKILL: u32 = 16_010_000;

    struct Connection {
        processor: StreamProcessor,
        storage: Arc<DataStorage>,
        events: Arc<Recorder>,
    }

    fn processor_over(storage: &Arc<DataStorage>) -> StreamProcessor {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let log = std::env::temp_dir().join("aether-own-actor-tests").join("aether.log");
        let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
        StreamProcessor::new(Arc::clone(storage), logger, "test".to_string(), config)
    }

    impl Connection {
        fn reconnect(&mut self) {
            self.processor = processor_over(&self.storage);
        }
    }

    fn connection() -> Connection {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let events = Arc::new(Recorder::default());
        let storage = Arc::new(DataStorage::new(events.clone(), config, mobs));
        storage.set_fight_tracking(true);
        let processor = processor_over(&storage);
        Connection { processor, storage, events }
    }

    fn character(name: &str, class: &str) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id: 1001, class: class.to_string(), key: None }
    }

    fn frame(payload: &[u8]) -> Vec<u8> {
        let length = u32::try_from(payload.len() + 4).expect("small frame");
        let mut frame = varint(length);
        frame.extend_from_slice(payload);
        frame
    }

    fn filler(body_len: usize) -> Vec<u8> {
        let mut payload = vec![0x0B, 0xE3];
        payload.extend((0..body_len).map(|index| (index * 31 + 7) as u8));
        frame(&payload)
    }

    fn bundle(inner_frames: &[u8]) -> Vec<u8> {
        let mut payload = vec![0xFF, 0xFF];
        payload.extend_from_slice(&u32::try_from(inner_frames.len()).expect("small bundle").to_le_bytes());
        payload.extend_from_slice(&lz4_flex::block::compress(inner_frames));
        frame(&payload)
    }

    fn varint(mut value: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let low = (value & 0x7F) as u8;
            value >>= 7;
            bytes.push(if value == 0 { low } else { low | 0x80 });
            if value == 0 {
                return bytes;
            }
        }
    }

    fn small_stat_hp(target: u32, kind: u8, value: u32) -> Vec<u8> {
        let mut payload = vec![0x00, 0x8D];
        payload.extend(varint(target));
        payload.extend_from_slice(&[1, 1, kind]);
        payload.extend_from_slice(&value.to_le_bytes());
        frame(&payload)
    }

    fn big_stat_hp(target: u32, stats: &[(u8, u64)]) -> Vec<u8> {
        let mut payload = vec![0x00, 0x8D];
        payload.extend(varint(target));
        payload.push(2);
        payload.push(u8::try_from(stats.len()).expect("few stats"));
        for (kind, value) in stats {
            payload.push(*kind);
            payload.extend_from_slice(&i64::try_from(*value).expect("positive hp").to_le_bytes());
        }
        frame(&payload)
    }

    fn own_hp(target: u32) -> Vec<u8> {
        small_stat_hp(target, 3, 2_332)
    }

    fn mob_hp(target: u32, hp: u64) -> Vec<u8> {
        big_stat_hp(target, &[(0, hp)])
    }

    fn spawn(spawn: &Spawn) -> Vec<u8> {
        frame(&spawn.frame())
    }

    fn death_with_killer(victim: u32, killer: u32) -> Vec<u8> {
        let mut payload = vec![0x04, 0x8D];
        payload.extend(varint(victim));
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend(varint(killer));
        payload.extend_from_slice(&1302u16.to_le_bytes());
        payload.extend_from_slice(&[4]);
        payload.extend_from_slice(b"Slay");
        payload.push(0);
        payload.extend_from_slice(&[0; 9]);
        frame(&payload)
    }

    fn named_by_the_game(actor_id: u32, name: &str, key: Option<[u8; 6]>) -> Vec<u8> {
        let mut payload = vec![0x33, 0x36];
        payload.extend(varint(actor_id));
        payload.extend_from_slice(&[0; 5]);
        payload.push(u8::try_from(name.len()).expect("short name"));
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(&1302u16.to_le_bytes());
        payload.push(22);
        if let Some(key) = key {
            payload.extend_from_slice(&[9, 9, 9, 9]);
            payload.extend_from_slice(&key);
            payload.extend_from_slice(&1302u16.to_le_bytes());
            payload.extend_from_slice(&[9, 9]);
        }
        frame(&payload)
    }

    fn feed(connection: &mut Connection, packets: &[Vec<u8>]) {
        for packet in packets {
            assert!(connection.processor.parse_packet(packet, false), "packet {packet:02X?} parsed");
        }
    }

    fn hit(storage: &DataStorage, actor_id: u32, skill_code: u32) {
        storage.append_damage_at(
            ParsedDamagePacket {
                target_id: MOB,
                actor_id,
                skill_code,
                ori_skill_code: skill_code,
                damage: 100,
                is_dot: false,
                is_crit: false,
                multi_hit_damage: 0,
                multi_hit_count: 0,
                specials: Vec::new(),
                drain_amount: 0,
                regeneration_amount: 0,
            },
            1.0,
        );
    }

    fn decisions(connection: &Connection) -> Vec<IdentityDecision> {
        connection
            .events
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                MeterEvent::OwnIdentity(decision) => Some(decision.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_signature_binds_your_actor_on_a_fresh_connection_with_no_name_known() {
        let mut connection = connection();

        feed(&mut connection, &[mob_hp(MOB, 40_000), own_hp(AFTER_ZONE_CHANGE), mob_hp(MOB + 1, 40_000)]);

        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
        assert_eq!(connection.storage.main_actor_name(), None);
        assert_eq!(decisions(&connection), vec![IdentityDecision::SignatureBound { actor_id: AFTER_ZONE_CHANGE }]);
    }

    #[test]
    fn only_an_hp_packet_with_the_signature_counts() {
        let mut connection = connection();

        feed(&mut connection, &[mob_hp(MOB, 40_000), mob_hp(MOB + 1, 2_332)]);

        assert_eq!(connection.storage.main_actor_id(), None);
    }

    #[test]
    fn a_signature_hp_above_the_boss_threshold_is_a_big_mob_and_binds_nobody() {
        let mut connection = connection();

        feed(&mut connection, &[small_stat_hp(MOB, 3, 9_800_000), own_hp(AFTER_ZONE_CHANGE)]);

        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
    }

    #[test]
    fn a_mob_hp_above_u32_is_kept() {
        let mut connection = connection();

        feed(&mut connection, &[mob_hp(MOB, 5_000_000_000)]);

        assert_eq!(connection.storage.mob_id_hp_snapshot().get(&MOB), Some(&(5_000_000_000, 5_000_000_000)));
    }

    #[test]
    fn a_max_hp_stat_makes_the_max_exact_even_when_hp_first_seen_is_lower() {
        let mut connection = connection();

        feed(&mut connection, &[big_stat_hp(MOB, &[(0, 40_000), (7, 100_000)]), mob_hp(MOB, 30_000)]);

        assert_eq!(connection.storage.mob_id_hp_snapshot().get(&MOB), Some(&(30_000, 100_000)));
    }

    #[test]
    fn a_global_spawn_with_an_owner_registers_the_mob_hp_and_summon() {
        let mut connection = connection();
        let summon = with_owner(Spawn::global());

        feed(&mut connection, &[spawn(&summon)]);

        assert_eq!(connection.storage.get_mob_code(summon.entity_id), Some(MOB_CODE));
        assert_eq!(connection.storage.mob_id_hp_snapshot().get(&summon.entity_id), Some(&(119_700, 119_700)));
        assert_eq!(connection.storage.summon_owner_snapshot().get(&summon.entity_id), Some(&OWNER));
    }

    #[test]
    fn a_kr_tw_spawn_with_an_owner_registers_the_summon() {
        let mut connection = connection();
        let summon = with_owner(Spawn::kr_tw());

        feed(&mut connection, &[spawn(&summon)]);

        assert_eq!(connection.storage.get_mob_code(summon.entity_id), Some(MOB_CODE));
        assert_eq!(connection.storage.summon_owner_snapshot().get(&summon.entity_id), Some(&OWNER));
    }

    #[test]
    fn a_spawn_without_an_owner_registers_no_summon_and_frees_a_reused_entity_id() {
        let mut connection = connection();
        let summon = with_owner(Spawn::global());
        let reused = Spawn::global();

        feed(&mut connection, &[spawn(&summon)]);
        assert!(connection.storage.summon_owner_snapshot().contains_key(&summon.entity_id));
        feed(&mut connection, &[spawn(&reused)]);

        assert!(connection.storage.summon_owner_snapshot().is_empty());
    }

    #[test]
    fn a_death_record_is_not_summon_ownership() {
        let mut connection = connection();

        let parsed = connection.processor.parse_packet(&death_with_killer(54_321, 7_000), false);

        assert!(!parsed);
        assert!(connection.storage.summon_owner_snapshot().is_empty());
    }

    #[test]
    fn a_death_record_in_the_combat_flow_is_a_death_with_the_killer_the_game_names() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        feed(&mut connection, &[death_with_killer(54_321, 7_000)]);

        let deaths = connection.storage.combat_events(0.0, f64::MAX).deaths;
        assert_eq!(deaths.len(), 1);
        assert_eq!(deaths[0].victim_id, 54_321);
        assert_eq!(deaths[0].killer.name.as_deref(), Some("Slay"));
        assert!(connection.storage.summon_owner_snapshot().is_empty());
    }

    const PLAYER: u32 = 16_022;

    fn monster_hit_on_the_player(skill_code: u32) -> Vec<u8> {
        let effect_sequence_state = [0x57, 0xFA, 0x3D, 0x6C, 0x01, 0x00, 0x00, 0x00, 0x05];
        let mut payload = vec![0x04, 0x38, 0x96, 0x7D, 0x04, 0x00, 18];
        payload.extend_from_slice(&skill_code.to_le_bytes());
        payload.push(0x4A);
        payload.push(2);
        payload.extend_from_slice(&effect_sequence_state);
        payload.extend_from_slice(&[0x64, 0x01, 0x00]);
        frame(&payload)
    }

    #[test]
    fn a_monster_hit_on_a_player_is_damage_taken_and_moves_no_target() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);
        connection.storage.append_actor(PLAYER, "Victim", None);
        connection.storage.append_mob(18, 2_300_810);

        feed(&mut connection, &[monster_hit_on_the_player(1_607_630)]);

        let hits = connection.storage.combat_events(0.0, f64::MAX).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].victim_id, hits[0].source_mob_code, hits[0].skill_code, hits[0].amount), (PLAYER, Some(2_300_810), 1_607_630, 100));
        assert_eq!(connection.storage.last_target(), None);
        assert!(connection.storage.get_dps_stats_snapshot().is_empty());
    }

    #[test]
    fn a_hit_from_a_player_skill_is_damage_dealt_and_not_damage_taken() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);
        connection.storage.append_actor(PLAYER, "Victim", None);

        feed(&mut connection, &[monster_hit_on_the_player(RANGER_SKILL)]);

        assert!(connection.storage.combat_events(0.0, f64::MAX).hits.is_empty());
        assert_eq!(connection.storage.last_target(), Some(PLAYER));
    }

    fn heal_tick_on_self(skill_code: u32) -> Vec<u8> {
        let mut payload = vec![0x05, 0x38, 0xB5, 0x0E, 0x0B, 0xB5, 0x0E, 0x88, 0x07, 0xD9, 0xFA, 0x7F, 0x60, 0xF7, 0x1E, 0xB7, 0x03];
        payload.extend_from_slice(&skill_code.to_le_bytes());
        frame(&payload)
    }

    #[test]
    fn a_heal_tick_of_a_class_skill_is_a_heal_over_time_of_its_amount() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        feed(&mut connection, &[heal_tick_on_self(16_190_020)]);

        let heals = connection.storage.combat_events(0.0, f64::MAX).heals;
        assert_eq!(heals.len(), 1);
        assert_eq!((heals[0].healer_id, heals[0].target_id, heals[0].skill_code, heals[0].amount), (1_845, 1_845, 16_190_000, 439));
        assert_eq!(heals[0].kind, crate::report::events::HealKind::OverTime);
    }

    #[test]
    fn a_heal_tick_of_a_system_effect_is_not_a_players_healing() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        feed(&mut connection, &[heal_tick_on_self(2_011_101)]);

        assert!(connection.storage.combat_events(0.0, f64::MAX).heals.is_empty());
    }

    #[test]
    fn combat_packets_count_nothing_while_fights_are_not_tracked() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);
        connection.storage.set_fight_tracking(false);
        connection.storage.append_actor(PLAYER, "Victim", None);
        connection.storage.append_mob(18, 2_300_810);

        for packet in [
            monster_hit_on_the_player(1_607_630),
            heal_tick_on_self(16_190_020),
            death_with_killer(54_321, 7_000),
        ] {
            assert!(!connection.processor.parse_packet(&packet, false), "packet {packet:02X?} was skipped");
        }

        let events = connection.storage.combat_events(0.0, f64::MAX);
        assert!(events.hits.is_empty() && events.heals.is_empty() && events.deaths.is_empty());
        assert!(connection.storage.get_dps_stats_snapshot().is_empty());
    }

    #[test]
    fn a_mob_death_is_still_seen_while_fights_are_not_tracked() {
        let mut connection = connection();
        connection.storage.set_fight_tracking(false);
        let boss = Spawn::global();
        feed(&mut connection, &[spawn(&boss), mob_hp(boss.entity_id, 30_000), death(boss.entity_id, 3)]);

        assert_eq!(mob_hp_of(&connection, boss.entity_id).map(|hp| hp.0), Some(0));
    }

    fn death(entity: u32, flag: u8) -> Vec<u8> {
        let mut payload = vec![0x42, 0x36];
        payload.extend(varint(entity));
        payload.extend_from_slice(&[0, flag]);
        frame(&payload)
    }

    fn mob_hp_of(connection: &Connection, entity: u32) -> Option<(u64, u64)> {
        connection.storage.mob_id_hp_snapshot().get(&entity).copied()
    }

    #[test]
    fn a_death_flag_of_three_leaves_the_mob_with_zero_hp_and_its_max() {
        let mut connection = connection();
        let boss = Spawn::global();
        feed(&mut connection, &[spawn(&boss), mob_hp(boss.entity_id, 30_000)]);

        feed(&mut connection, &[death(boss.entity_id, 3)]);

        assert_eq!(mob_hp_of(&connection, boss.entity_id), Some((0, 119_700)));
    }

    #[test]
    fn removal_flags_leave_the_mob_alive() {
        for flag in [1, 7] {
            let mut connection = connection();
            let boss = Spawn::global();
            feed(&mut connection, &[spawn(&boss), death(boss.entity_id, flag)]);

            assert_eq!(mob_hp_of(&connection, boss.entity_id), Some((119_700, 119_700)), "flag {flag}");
        }
    }

    #[test]
    fn a_death_of_an_entity_with_no_recorded_hp_records_nothing() {
        let mut connection = connection();

        feed(&mut connection, &[death(MOB, 3)]);

        assert_eq!(mob_hp_of(&connection, MOB), None);
    }

    fn connection_where_the_spawned_mob_is_a_boss() -> Connection {
        let connection = connection();
        connection.storage.mobs().set_boss_correction(MOB_CODE, Some(crate::mobs::store::BossCorrection::Boss));
        connection
    }

    fn hit_on(storage: &DataStorage, target_id: u32, actor_id: u32) {
        storage.append_damage_at(
            ParsedDamagePacket {
                target_id,
                actor_id,
                skill_code: RANGER_SKILL,
                ori_skill_code: RANGER_SKILL,
                damage: 100,
                is_dot: false,
                is_crit: false,
                multi_hit_damage: 0,
                multi_hit_count: 0,
                specials: Vec::new(),
                drain_amount: 0,
                regeneration_amount: 0,
            },
            1.0,
        );
    }

    #[test]
    fn a_removal_of_a_boss_somebody_hit_is_a_wipe_and_leaving_range_is_not() {
        for (flag, wiped) in [(7, true), (1, false), (3, false)] {
            let mut connection = connection_where_the_spawned_mob_is_a_boss();
            let boss = Spawn::global();
            feed(&mut connection, &[spawn(&boss), mob_hp(boss.entity_id, 30_000)]);
            hit_on(&connection.storage, boss.entity_id, 18);

            feed(&mut connection, &[death(boss.entity_id, flag)]);

            let wipes = connection.storage.take_boss_wipes();
            assert_eq!(wipes.len(), usize::from(wiped), "flag {flag}");
            if wiped {
                assert_eq!((wipes[0].target_id, wipes[0].hp_left), (boss.entity_id, 30_000));
            }
        }
    }

    #[test]
    fn a_boss_hp_packet_back_at_full_after_a_loss_is_a_wipe() {
        let mut connection = connection_where_the_spawned_mob_is_a_boss();
        let boss = Spawn::global();
        feed(&mut connection, &[spawn(&boss), mob_hp(boss.entity_id, 30_000)]);
        hit_on(&connection.storage, boss.entity_id, 18);
        feed(&mut connection, &[mob_hp(boss.entity_id, 20_000)]);
        assert!(connection.storage.take_boss_wipes().is_empty());

        feed(&mut connection, &[mob_hp(boss.entity_id, 119_700)]);

        let wipes = connection.storage.take_boss_wipes();
        assert_eq!(wipes.len(), 1);
        assert_eq!(wipes[0].hp_left, 20_000);
    }

    fn dot_tick_of_skill(target: u32, mode: u8, actor: u32, skill_code: u32, damage: u8) -> Vec<u8> {
        let mut payload = vec![0x05, 0x38];
        payload.extend(varint(target));
        payload.push(mode);
        payload.extend(varint(actor));
        payload.push(1);
        payload.extend_from_slice(&skill_code.to_le_bytes());
        payload.push(damage);
        frame(&payload)
    }

    fn dot_tick(target: u32, mode: u8, actor: u32, damage: u8) -> Vec<u8> {
        dot_tick_of_skill(target, mode, actor, RANGER_SKILL, damage)
    }

    fn damage_dealt_by(connection: &Connection, actor: u32) -> u64 {
        connection
            .storage
            .get_dps_stats_snapshot()
            .values()
            .filter_map(|actors| actors.get(&actor))
            .flat_map(|skills| skills.values())
            .map(|stats| stats.total_damage)
            .sum()
    }

    #[test]
    fn only_the_damage_tick_mode_of_a_periodic_effect_counts() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        for mode in [0x00, 0x02, 0x08, 0x09, 0x0B, 0x30] {
            feed(&mut connection, &[dot_tick(MOB, mode, 18, 50)]);
        }
        assert_eq!(damage_dealt_by(&connection, 18), 0);

        feed(&mut connection, &[dot_tick(MOB, 0x0A, 18, 50)]);
        assert_eq!(damage_dealt_by(&connection, 18), 50);
    }

    const THAMON_ABSORPTION_RECOVERY: u32 = 160_467_011;
    const PLAYER_DAMAGE_OVER_TIME: u32 = 1_709_014_011;

    #[test]
    fn a_thamon_absorption_recovery_hit_is_no_damage_dealt_and_moves_no_target() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);
        connection.storage.append_actor(PLAYER, "Victim", None);

        feed(&mut connection, &[monster_hit_on_the_player(THAMON_ABSORPTION_RECOVERY)]);

        assert_eq!(damage_dealt_by(&connection, 18), 0);
        assert_eq!(connection.storage.last_target(), None);
        assert!(connection.storage.combat_events(0.0, f64::MAX).hits.is_empty());
    }

    #[test]
    fn a_thamon_absorption_recovery_tick_is_no_damage_dealt_and_moves_no_target() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        feed(&mut connection, &[dot_tick_of_skill(MOB, 0x0A, 18, THAMON_ABSORPTION_RECOVERY, 50)]);

        assert_eq!(damage_dealt_by(&connection, 18), 0);
        assert_eq!(connection.storage.last_target(), None);
    }

    #[test]
    fn a_player_damage_over_time_tick_still_counts() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        feed(&mut connection, &[dot_tick_of_skill(MOB, 0x0A, 18, PLAYER_DAMAGE_OVER_TIME, 50)]);

        assert_eq!(damage_dealt_by(&connection, 18), 50);
    }

    #[test]
    fn a_thamon_absorption_recovery_heal_tick_is_no_healing() {
        let mut connection = connection();
        connection.processor.set_combat_damage_enabled(true);

        feed(&mut connection, &[heal_tick_on_self(THAMON_ABSORPTION_RECOVERY)]);

        assert!(connection.storage.combat_events(0.0, f64::MAX).heals.is_empty());
    }

    const BUFF_SKILL: u32 = 110_100_001;
    const BUFF_DURATION_MS: u64 = 60_000;

    fn buff_applied(opcode: u8, target: u32, instance: u32, actor: u32) -> Vec<u8> {
        let mut payload = vec![opcode, 0x38];
        payload.extend(varint(target));
        if opcode == 0x2A {
            payload.push(1);
        }
        payload.push(1);
        payload.extend(varint(instance));
        payload.extend_from_slice(&BUFF_SKILL.to_le_bytes());
        payload.extend_from_slice(&BUFF_DURATION_MS.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend(varint(actor));
        frame(&payload)
    }

    fn buff_removed(target: u32, instance: u32) -> Vec<u8> {
        let mut payload = vec![0x2C, 0x38];
        payload.extend(varint(target));
        payload.extend_from_slice(&[1, 0]);
        payload.extend(varint(instance));
        payload.push(1);
        frame(&payload)
    }

    fn buff_length_ms(connection: &Connection, target: u32, actor: u32) -> u64 {
        let intervals = connection.storage.buff_intervals_snapshot();
        let interval = intervals[&target][&actor][&BUFF_SKILL].back().expect("one interval");
        interval.end_ms - interval.start_ms
    }

    #[test]
    fn a_removal_ends_the_buff_it_names_before_its_duration_runs_out() {
        for (opcode, instance) in [(0x2A, 7), (0x2B, 7), (0x2B, 300)] {
            let mut connection = connection();
            feed(&mut connection, &[buff_applied(opcode, MOB, instance, 18)]);
            assert_eq!(buff_length_ms(&connection, MOB, 18), BUFF_DURATION_MS);

            feed(&mut connection, &[buff_removed(MOB, instance)]);

            assert!(buff_length_ms(&connection, MOB, 18) < 5_000, "opcode {opcode:02X} instance {instance}");
        }
    }

    #[test]
    fn a_removal_of_another_instance_does_not_end_the_buff() {
        let mut connection = connection();
        feed(&mut connection, &[buff_applied(0x2A, MOB, 7, 18), buff_applied(0x2A, MOB, 8, 18)]);

        feed(&mut connection, &[buff_removed(MOB, 7)]);
        assert!(
            buff_length_ms(&connection, MOB, 18) >= BUFF_DURATION_MS,
            "the other instance is still running: the interval may run a millisecond over, never short"
        );

        feed(&mut connection, &[buff_removed(MOB, 8)]);
        assert!(buff_length_ms(&connection, MOB, 18) < 5_000);
    }

    #[test]
    fn a_buff_packet_from_a_summon_is_credited_to_its_owner() {
        let mut connection = connection();
        connection.storage.append_summon(18, 40_001);
        connection.storage.append_summon(18, 40_002);

        feed(&mut connection, &[buff_applied(0x2A, MOB, 7, 40_001), buff_applied(0x2B, MOB, 8, 40_002)]);

        let intervals = connection.storage.buff_intervals_snapshot();
        assert_eq!(intervals[&MOB].keys().copied().collect::<Vec<_>>(), [18]);
    }

    #[test]
    fn a_removal_of_a_buff_never_seen_changes_nothing() {
        let mut connection = connection();
        feed(&mut connection, &[buff_applied(0x2A, MOB, 7, 18)]);

        feed(&mut connection, &[buff_removed(MOB, 99), buff_removed(MOB + 1, 7)]);

        assert_eq!(buff_length_ms(&connection, MOB, 18), BUFF_DURATION_MS);
    }

    #[test]
    fn a_mob_the_game_already_described_is_not_you() {
        let mut connection = connection();
        connection.storage.append_mob(MOB, 2_300_810);

        feed(&mut connection, &[own_hp(MOB)]);

        assert_eq!(connection.storage.main_actor_id(), None);
    }

    #[test]
    fn a_connection_keeps_the_first_actor_the_signature_showed() {
        let mut connection = connection();

        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE), own_hp(MOB)]);

        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
    }

    #[test]
    fn a_new_connection_gives_the_named_character_its_new_actor_id() {
        let mut connection = connection();
        connection.storage.append_actor(BEFORE_ZONE_CHANGE, NAME, Some("1302"));
        connection.storage.set_main_actor(BEFORE_ZONE_CHANGE, NAME);

        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);

        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
        assert_eq!(connection.storage.main_actor_name().as_deref(), Some(NAME));
        assert_eq!(
            connection.storage.actor_id_name_snapshot().get(&AFTER_ZONE_CHANGE).map(String::as_str),
            Some(NAME)
        );
    }

    #[test]
    fn a_frame_with_a_two_byte_length_is_followed_by_the_next_frame() {
        let mut connection = connection();
        let mut stream = filler(200);
        stream.extend(own_hp(AFTER_ZONE_CHANGE));

        let consumed = connection.processor.consume_stream(&stream);

        assert_eq!(consumed, stream.len());
        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
    }

    #[test]
    fn a_frame_with_a_two_byte_length_inside_a_bundle_is_followed_by_the_next_inner_frame() {
        let mut connection = connection();
        let mut inner_frames = filler(200);
        inner_frames.extend(own_hp(AFTER_ZONE_CHANGE));
        let mut stream = bundle(&inner_frames);
        stream.extend(filler(20));

        let consumed = connection.processor.consume_stream(&stream);

        assert_eq!(consumed, stream.len());
        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
    }

    #[test]
    fn a_connection_that_named_the_character_keeps_what_the_game_said() {
        let mut connection = connection();

        feed(
            &mut connection,
            &[named_by_the_game(BEFORE_ZONE_CHANGE, NAME, None), own_hp(AFTER_ZONE_CHANGE)],
        );

        assert_eq!(connection.storage.main_actor_id(), Some(BEFORE_ZONE_CHANGE));
    }

    #[test]
    fn a_remembered_name_is_applied_when_the_class_the_skills_show_matches() {
        let mut connection = connection();
        connection
            .storage
            .set_known_characters(vec![character("Elsewhere", "TEMPLAR"), character("Remembered", "RANGER")]);

        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);
        assert_eq!(connection.storage.main_actor_name(), None, "no class is known yet: not guessed");

        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        assert_eq!(connection.storage.main_actor_name().as_deref(), Some("Remembered"));
        assert_eq!(
            connection.storage.actor_id_name_snapshot().get(&AFTER_ZONE_CHANGE).map(String::as_str),
            Some("Remembered")
        );
        assert!(decisions(&connection).contains(&IdentityDecision::RememberedApplied {
            actor_id: AFTER_ZONE_CHANGE,
            name: "Remembered".to_string(),
            class: "RANGER".to_string(),
        }));
    }

    #[test]
    fn a_class_seen_before_the_signature_still_names_you_when_the_signature_comes() {
        let mut connection = connection();
        connection.storage.set_known_characters(vec![character("Remembered", "RANGER")]);
        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);

        assert_eq!(connection.storage.main_actor_name().as_deref(), Some("Remembered"));
    }

    #[test]
    fn a_remembered_name_is_rejected_when_no_remembered_character_has_the_class() {
        let mut connection = connection();
        connection.storage.set_known_characters(vec![character("Remembered", "RANGER")]);

        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);
        hit(&connection.storage, AFTER_ZONE_CHANGE, TEMPLAR_SKILL);
        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        assert_eq!(connection.storage.main_actor_name(), None);
        assert!(connection.storage.actor_id_name_snapshot().get(&AFTER_ZONE_CHANGE).is_none());
        assert!(decisions(&connection).contains(&IdentityDecision::RememberedRejected {
            actor_id: AFTER_ZONE_CHANGE,
            class: "TEMPLAR".to_string(),
            remembered: 1,
        }));
    }

    #[test]
    fn a_list_that_arrives_after_the_class_still_names_you() {
        let mut connection = connection();
        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);
        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);
        assert_eq!(connection.storage.main_actor_name(), None);

        connection.storage.set_known_characters(vec![character("Remembered", "RANGER")]);

        assert_eq!(connection.storage.main_actor_name().as_deref(), Some("Remembered"));
    }

    #[test]
    fn the_game_naming_you_replaces_the_signature_and_the_remembered_name() {
        let mut connection = connection();
        connection.storage.set_known_characters(vec![character("Remembered", "RANGER")]);
        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);
        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        feed(&mut connection, &[named_by_the_game(MOB + 5, NAME, None)]);

        assert_eq!(connection.storage.main_actor_id(), Some(MOB + 5));
        assert_eq!(connection.storage.main_actor_name().as_deref(), Some(NAME));
        assert!(decisions(&connection).contains(&IdentityDecision::GameOverrode {
            actor_id: MOB + 5,
            previous_actor_id: Some(AFTER_ZONE_CHANGE),
            previous_name: Some("Remembered".to_string()),
        }));
    }

    #[test]
    fn the_game_naming_the_actor_the_signature_found_does_not_clear_the_meter() {
        let mut connection = connection();
        let resets = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&resets);
        connection
            .storage
            .main_actor_callback
            .lock()
            .unwrap()
            .replace(Box::new(move |_, _, _| {
                counted.fetch_add(1, Ordering::Relaxed);
            }));
        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);

        feed(&mut connection, &[named_by_the_game(AFTER_ZONE_CHANGE, NAME, None)]);

        assert_eq!(connection.storage.main_actor_name().as_deref(), Some(NAME));
        assert_eq!(resets.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn the_game_naming_you_hands_the_character_to_the_app_with_its_key() {
        let mut connection = connection();
        let key = [0x3A, 0xA5, 0x03, 0, 0, 0];

        feed(&mut connection, &[named_by_the_game(BEFORE_ZONE_CHANGE, NAME, Some(key))]);

        let seen: Vec<KnownCharacter> = connection
            .events
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                MeterEvent::CharacterSeen(character) => Some(character.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            seen,
            vec![KnownCharacter {
                name: NAME.to_string(),
                server_id: 1302,
                class: "ELEMENTALIST".to_string(),
                key: Some("3AA503000000".to_string()),
            }]
        );
    }

    fn ticket_reports(connection: &Connection) -> Vec<TicketReport> {
        connection
            .events
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                MeterEvent::Tickets(report) => Some(report.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn ticket_messages_reach_the_app_with_the_character_named_after_them_and_their_capture_time() {
        let mut connection = connection();
        connection.processor.set_captured_at(100.0);
        feed(
            &mut connection,
            &[frame(&snapshot(&[record(8, None, Some(2), None), record(10, None, Some(1), None)]))],
        );
        assert!(ticket_reports(&connection).is_empty(), "the snapshot waits for the name");

        connection.processor.set_captured_at(104.0);
        feed(&mut connection, &[named_by_the_game(BEFORE_ZONE_CHANGE, NAME, None)]);
        connection.processor.set_captured_at(500.0);
        feed(&mut connection, &[frame(&update(0, record(60000001, None, None, Some(245)), 3, None))]);

        let reports = ticket_reports(&connection);
        assert_eq!(reports.len(), 2);
        assert_eq!((reports[0].character.name.as_str(), reports[0].character.server_id), (NAME, 1302));
        assert_eq!(reports[0].captured_at, 100.0);
        let TicketContent::Snapshot(records) = &reports[0].content else { panic!("snapshot first") };
        assert_eq!(records.iter().map(|record| (record.id, record.count)).collect::<Vec<_>>(), vec![(8, 2)]);
        assert_eq!(reports[1].captured_at, 500.0);
        let TicketContent::Change { record, reason, delta } = &reports[1].content else { panic!("change second") };
        assert_eq!((record.id, record.charge, *reason, *delta), (60000001, 245, TicketReason::Spend, None));
    }

    #[test]
    fn a_look_alike_frame_under_a_ticket_opcode_emits_nothing() {
        let mut connection = connection();
        let snapshot_like = vec![0x0B, 0x61, 0x91, 0x80, 0x9A, 0x0C, 1, 2, 3, 4];
        let update_like = vec![0x0C, 0x61, 2, 1, 2, 3, 4, 5, 6, 7];
        connection.processor.set_captured_at(100.0);
        feed(&mut connection, &[named_by_the_game(BEFORE_ZONE_CHANGE, NAME, None)]);

        assert!(!connection.processor.parse_packet(&frame(&snapshot_like), false));
        assert!(!connection.processor.parse_packet(&frame(&update_like), false));

        assert!(ticket_reports(&connection).is_empty());
    }

    fn named_on_a_first_connection(connection: &mut Connection, known: Vec<KnownCharacter>) {
        connection.storage.set_known_characters(known);
        feed(connection, &[named_by_the_game(BEFORE_ZONE_CHANGE, NAME, None)]);
        connection.reconnect();
        feed(connection, &[own_hp(AFTER_ZONE_CHANGE)]);
    }

    #[test]
    fn a_carried_name_stays_when_the_new_actors_skills_show_the_class_it_had() {
        let mut connection = connection();
        named_on_a_first_connection(&mut connection, vec![character("Other", "RANGER")]);
        assert_eq!(connection.storage.main_actor_name().as_deref(), Some(NAME), "carried until a class shows");

        hit(&connection.storage, AFTER_ZONE_CHANGE, ELEMENTALIST_SKILL);

        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
        assert_eq!(connection.storage.main_actor_name().as_deref(), Some(NAME));
        assert!(decisions(&connection).iter().all(|decision| !matches!(decision, IdentityDecision::CarriedRejected { .. })));
    }

    #[test]
    fn a_carried_name_gives_way_to_the_remembered_character_of_the_class_the_skills_show() {
        let mut connection = connection();
        named_on_a_first_connection(&mut connection, vec![character("Other", "RANGER")]);

        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        assert_eq!(connection.storage.main_actor_name().as_deref(), Some("Other"));
        assert_eq!(
            connection.storage.actor_id_name_snapshot().get(&AFTER_ZONE_CHANGE).map(String::as_str),
            Some("Other")
        );
        let decisions = decisions(&connection);
        assert!(decisions.contains(&IdentityDecision::CarriedRejected {
            actor_id: AFTER_ZONE_CHANGE,
            name: NAME.to_string(),
            carried_class: Some("ELEMENTALIST".to_string()),
            class: "RANGER".to_string(),
        }));
        assert!(decisions.contains(&IdentityDecision::RememberedApplied {
            actor_id: AFTER_ZONE_CHANGE,
            name: "Other".to_string(),
            class: "RANGER".to_string(),
        }));
    }

    #[test]
    fn a_carried_name_gives_way_to_you_when_no_remembered_character_has_the_class() {
        let mut connection = connection();
        named_on_a_first_connection(&mut connection, Vec::new());

        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        assert_eq!(connection.storage.main_actor_name(), None);
        assert!(connection.storage.actor_id_name_snapshot().get(&AFTER_ZONE_CHANGE).is_none());
        assert!(decisions(&connection).contains(&IdentityDecision::RememberedRejected {
            actor_id: AFTER_ZONE_CHANGE,
            class: "RANGER".to_string(),
            remembered: 0,
        }));
        let last_detected = connection.events.0.lock().unwrap().iter().rev().find_map(|event| match event {
            MeterEvent::MainActorDetected(payload) => Some(payload.clone()),
            _ => None,
        });
        assert_eq!(last_detected.map(|payload| payload.actor_name), Some(String::new()));
    }

    #[test]
    fn a_carried_name_whose_class_was_never_known_is_not_kept_across_a_connection() {
        let mut connection = connection();
        connection.storage.append_actor(BEFORE_ZONE_CHANGE, NAME, Some("1302"));
        connection.storage.set_main_actor(BEFORE_ZONE_CHANGE, NAME);
        connection.reconnect();
        feed(&mut connection, &[own_hp(AFTER_ZONE_CHANGE)]);

        hit(&connection.storage, AFTER_ZONE_CHANGE, ELEMENTALIST_SKILL);

        assert_eq!(connection.storage.main_actor_name(), None);
    }

    #[test]
    fn the_game_naming_the_new_actor_wins_over_a_carried_name() {
        let mut connection = connection();
        named_on_a_first_connection(&mut connection, Vec::new());

        feed(&mut connection, &[named_by_the_game(AFTER_ZONE_CHANGE, "Newcomer", None)]);
        hit(&connection.storage, AFTER_ZONE_CHANGE, RANGER_SKILL);

        assert_eq!(connection.storage.main_actor_name().as_deref(), Some("Newcomer"));
    }
}

#[cfg(test)]
mod framing_tests {
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    use crate::capture::assembler::StreamAssembler;
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::logger::AppLogger;
    use crate::mobs::catalog::MobCatalog;
    use crate::storage::data_storage::DataStorage;

    const TCP_SEGMENT: usize = 1460;

    struct Discard;
    impl EventSink for Discard {
        fn emit(&self, _event: MeterEvent) {}
    }

    fn assembler_with(config: DpsMeterConfig) -> StreamAssembler {
        let config = Arc::new(RwLock::new(config));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = Arc::new(DataStorage::new(Arc::new(Discard), Arc::clone(&config), mobs));
        storage.set_fight_tracking(true);
        let log = std::env::temp_dir().join("aether-framing-tests").join("aether.log");
        let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
        StreamAssembler::new(storage, logger, "test".to_string(), config)
    }

    fn assembler() -> StreamAssembler {
        assembler_with(DpsMeterConfig::default())
    }

    fn unknown_packet(total_bytes: usize) -> Vec<u8> {
        let length_bytes = if total_bytes + 3 < 0x80 { 1 } else { 2 };
        let length = total_bytes + 4 - length_bytes;
        let mut packet = if length_bytes == 1 {
            vec![length as u8]
        } else {
            vec![(length & 0x7F) as u8 | 0x80, (length >> 7) as u8]
        };
        packet.extend_from_slice(&[0x0B, 0xE3]);
        packet.resize(total_bytes, 0xAA);
        packet
    }

    fn feed_segments(assembler: &mut StreamAssembler, stream: &[u8]) -> Vec<usize> {
        stream
            .chunks(TCP_SEGMENT)
            .map(|segment| {
                assembler.process_chunk(segment, false, 0.0);
                assembler.buffer_size()
            })
            .collect()
    }

    #[test]
    fn a_packet_over_8_kb_is_waited_for_while_it_arrives_in_segments() {
        let mut assembler = assembler();
        let mut stream = unknown_packet(13_000);
        stream.extend(unknown_packet(20));

        let buffered = feed_segments(&mut assembler, &stream);

        let still_incomplete = 13_000 / TCP_SEGMENT;
        for (index, size) in buffered.iter().enumerate().take(still_incomplete) {
            assert_eq!(*size, (index + 1) * TCP_SEGMENT, "nothing is dropped before the packet is whole");
        }
        assert_eq!(buffered.last(), Some(&0), "the packet and the one after it were framed");
    }

    #[test]
    fn a_packet_arriving_slower_than_the_stall_delay_is_still_waited_for() {
        let mut assembler = assembler_with(DpsMeterConfig {
            unknown_packet_stall_resync_delay_ms: 25,
            ..DpsMeterConfig::default()
        });
        let stream = unknown_packet(4_000);
        let mut fed = 0;

        for segment in stream.chunks(TCP_SEGMENT) {
            std::thread::sleep(Duration::from_millis(40));
            assembler.process_chunk(segment, false, 0.0);
            fed += segment.len();
            if fed < stream.len() {
                assert_eq!(assembler.buffer_size(), fed, "a head that keeps growing is not skipped");
            }
        }

        assert_eq!(assembler.buffer_size(), 0);
    }

    fn bogus_heads(count: usize) -> Vec<u8> {
        [0xFF, 0x7F].repeat(count)
    }

    fn feed_idle_bytes_until_drained(assembler: &mut StreamAssembler, limit: Duration) -> Option<Duration> {
        let started = std::time::Instant::now();
        while started.elapsed() < limit {
            std::thread::sleep(Duration::from_millis(20));
            assembler.process_chunk(&[0x00], false, 0.0);
            if assembler.buffer_size() == 0 {
                return Some(started.elapsed());
            }
        }
        None
    }

    #[test]
    fn garbage_after_a_large_packet_is_resynced_within_one_cap() {
        let mut assembler = assembler();
        let large_packet = unknown_packet(13_000);
        let garbage = bogus_heads(10);
        let mut stream = large_packet.clone();
        stream.extend_from_slice(&garbage);

        let mut fed = 0;
        let mut frame_parsed = false;
        for segment in stream.chunks(TCP_SEGMENT) {
            fed += segment.len();
            let parsed = assembler.process_chunk(segment, false, 0.0);
            frame_parsed |= parsed && fed >= large_packet.len();
        }

        assert!(frame_parsed, "the 13 KB frame was parsed once it was whole");
        assert_eq!(assembler.buffer_size(), garbage.len(), "only the garbage remains");

        let resynced_at = feed_idle_bytes_until_drained(&mut assembler, Duration::from_secs(5))
            .expect("stream stayed blocked on the garbage");
        assert!(resynced_at < Duration::from_millis(1_800), "resynced after {resynced_at:?}, more than one cap");
    }

    #[test]
    fn consecutive_bogus_heads_are_skipped_within_one_cap_while_live_data_keeps_arriving() {
        let mut assembler = assembler();
        assembler.process_chunk(&bogus_heads(10), false, 0.0);

        let resynced_at = feed_idle_bytes_until_drained(&mut assembler, Duration::from_secs(5))
            .expect("stream stayed blocked on the bogus heads");
        assert!(resynced_at < Duration::from_millis(1_800), "resynced after {resynced_at:?}, more than one cap");
    }

    #[test]
    fn a_large_packet_is_waited_for_after_a_hunt_drained_the_buffer() {
        let mut assembler = assembler();
        assembler.process_chunk(&[0xFF, 0x7F], false, 0.0);
        std::thread::sleep(Duration::from_millis(1_100));
        assembler.process_chunk(&[], false, 0.0);
        assert_eq!(assembler.buffer_size(), 0, "the hunt drained the buffer");

        let mut stream = unknown_packet(13_000);
        stream.extend(unknown_packet(20));
        let buffered = feed_segments(&mut assembler, &stream);

        let still_incomplete = 13_000 / TCP_SEGMENT;
        for (index, size) in buffered.iter().enumerate().take(still_incomplete) {
            assert_eq!(*size, (index + 1) * TCP_SEGMENT, "nothing is dropped before the packet is whole");
        }
        assert_eq!(buffered.last(), Some(&0), "the packet and the one after it were framed");
    }

    #[test]
    fn framing_restarts_at_the_next_frame_after_the_buffered_head_is_discarded() {
        let mut assembler = assembler();
        let head_of_a_large_packet = unknown_packet(13_000);
        assembler.process_chunk(&head_of_a_large_packet[..3 * TCP_SEGMENT], false, 0.0);
        assert_eq!(assembler.buffer_size(), 3 * TCP_SEGMENT);

        assembler.discard_buffered();
        assembler.process_chunk(&unknown_packet(20), false, 0.0);

        assert_eq!(assembler.buffer_size(), 0, "the frame after the gap was framed on its own");
    }
}
