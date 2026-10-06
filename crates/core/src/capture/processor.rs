use lz4_flex::block::decompress;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::capture::parser::context::ParserContext;
use crate::capture::parser::nickname;
use crate::capture::parser::utils::read_varint;
use crate::config::SharedDpsMeterConfig;
use crate::capture::census;
use crate::storage::data_storage::DataStorage;
use crate::logger::AppLogger;

const KNOWN_PACKET_HEADERS: &[(u8, u8)] = &[
    (0x33, 0x36),
    (0x45, 0x36),
    (0x56, 0x36),
    (0x41, 0x36),
    (0x04, 0x38),
    (0x05, 0x38),
    (0x2A, 0x38),
    (0x2B, 0x38),
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
        usize::try_from(length_info.value.saturating_sub(3)).ok()
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
        }
    }

    pub fn set_combat_damage_enabled(&mut self, enabled: bool) {
        self.combat_damage_enabled = enabled;
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

            let Ok(total_packet_bytes) = usize::try_from(length_info.value.saturating_sub(3))
            else {
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
                let bundle_size = total_packet_bytes + 1;
                if offset + bundle_size > buffer.len() {
                    break;
                }
                self.unwrap_bundle(&buffer[offset + payload_start..offset + bundle_size]);
                offset += bundle_size;
            } else {
                self.parse_packet(current_packet, false);
                offset += total_packet_bytes;
            }
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

            let Ok(inner_total_bytes) = usize::try_from(length_info.value.saturating_sub(3)) else {
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
                (0x04, 0x38) if self.combat_damage_enabled => {
                    self.parse_damage_packet(payload, is_compressed_bundle)
                }
                (0x05, 0x38) if self.combat_damage_enabled => {
                    self.parse_dot_packet(payload, is_compressed_bundle)
                }
                (0x2A, 0x38) | (0x2B, 0x38) => {
                    self.parse_buff_packet(payload, is_compressed_bundle)
                }
                (0x04, 0x8D) => self.parse_summon_packet_048d(payload, is_compressed_bundle),
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
        ParserContext::new(&self.data_storage, &self.logger, &self.port)
    }
}

#[cfg(test)]
mod own_actor_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, RwLock};

    use super::StreamProcessor;
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::identity::{IdentityDecision, KnownCharacter};
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
        let processor = processor_over(&storage);
        Connection { processor, storage, events }
    }

    fn character(name: &str, class: &str) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id: 1001, class: class.to_string(), key: None }
    }

    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut frame = vec![u8::try_from(payload.len() + 1).expect("short frame")];
        frame.extend_from_slice(payload);
        frame
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

    fn remain_hp(target: u32, skips: [u8; 3], hp: u32) -> Vec<u8> {
        let mut payload = vec![0x00, 0x8D];
        payload.extend(varint(target));
        payload.extend_from_slice(&skips);
        payload.extend_from_slice(&hp.to_le_bytes());
        frame(&payload)
    }

    fn own_hp(target: u32) -> Vec<u8> {
        remain_hp(target, [1, 1, 3], 2_332)
    }

    fn mob_hp(target: u32, hp: u32) -> Vec<u8> {
        remain_hp(target, [4, 1, 1], hp)
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

        feed(&mut connection, &[mob_hp(MOB, 40_000), remain_hp(MOB + 1, [2, 1, 3], 2_332)]);

        assert_eq!(connection.storage.main_actor_id(), None);
    }

    #[test]
    fn a_signature_hp_above_the_boss_threshold_is_a_big_mob_and_binds_nobody() {
        let mut connection = connection();

        feed(&mut connection, &[remain_hp(MOB, [1, 1, 3], 9_800_000), own_hp(AFTER_ZONE_CHANGE)]);

        assert_eq!(connection.storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
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
        let log = std::env::temp_dir().join("aether-framing-tests").join("aether.log");
        let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
        StreamAssembler::new(storage, logger, "test".to_string(), config)
    }

    fn assembler() -> StreamAssembler {
        assembler_with(DpsMeterConfig::default())
    }

    fn unknown_packet(total_bytes: usize) -> Vec<u8> {
        let length = total_bytes + 3;
        let mut packet = if length < 0x80 {
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
                assembler.process_chunk(segment, false);
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
            assembler.process_chunk(segment, false);
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
            assembler.process_chunk(&[0x00], false);
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
            let parsed = assembler.process_chunk(segment, false);
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
        assembler.process_chunk(&bogus_heads(10), false);

        let resynced_at = feed_idle_bytes_until_drained(&mut assembler, Duration::from_secs(5))
            .expect("stream stayed blocked on the bogus heads");
        assert!(resynced_at < Duration::from_millis(1_800), "resynced after {resynced_at:?}, more than one cap");
    }

    #[test]
    fn a_large_packet_is_waited_for_after_a_hunt_drained_the_buffer() {
        let mut assembler = assembler();
        assembler.process_chunk(&[0xFF, 0x7F], false);
        std::thread::sleep(Duration::from_millis(1_100));
        assembler.process_chunk(&[], false);
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
}
