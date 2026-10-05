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

/// Where a connection stands on learning which actor is the local character.
/// The game names the character on login (`Settled` once it did). A later
/// connection (a zone change) opens with the character's combat power and its
/// own HP instead: that HP packet is the first after the combat power
/// (`Expected`), and whatever it says settles the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OwnActor {
    Unsettled,
    Expected,
    Settled,
}

impl OwnActor {
    fn named_by_game(&mut self) {
        *self = Self::Settled;
    }

    fn combat_power_seen(&mut self) {
        if *self == Self::Unsettled {
            *self = Self::Expected;
        }
    }

    pub(super) fn claim_opening_hp_packet(&mut self) -> bool {
        let expected = *self == Self::Expected;
        if expected {
            *self = Self::Settled;
        }
        expected
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
            combat_damage_enabled: false,
            own_actor: OwnActor::Unsettled,
        }
    }

    pub fn set_combat_damage_enabled(&mut self, enabled: bool) {
        self.combat_damage_enabled = enabled;
    }

    pub fn consume_stream(&mut self, buffer: &[u8]) -> usize {
        let mut offset = 0usize;
        let max_packet_size_threshold = {
            let config = self.config.read().unwrap();
            usize::try_from(config.max_packet_size_threshold).unwrap_or(8 * 1024)
        };

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
                // let candidate = inspect_stall_candidate(&buffer[offset..]);
                // let is_explicitly_unknown = candidate
                //     .opcode
                //     .is_some_and(|opcode| !KNOWN_PACKET_HEADERS.contains(&opcode));
                // if is_explicitly_unknown && total_packet_bytes > max_packet_size_threshold {
                //     offset += 1;
                //     continue;
                // }
                if total_packet_bytes > max_packet_size_threshold {
                    offset += 1;
                    continue;
                }
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

            let now = Instant::now();
            if self.stalled_candidate != Some(candidate) {
                self.stalled_candidate = Some(candidate);
                self.stalled_since = Some(now);
                return 0;
            }

            if let Some(stalled_since) = self.stalled_since {
                if now.duration_since(stalled_since) >= Duration::from_millis(delay_ms) {
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
                    self.stalled_since = Some(now);
                    return 1;
                }
            } else {
                self.stalled_since = Some(now);
            }
        } else {
            self.stalled_since = None;
            self.stalled_candidate = None;
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
                        self.own_actor.named_by_game();
                    }
                    parsed
                }
                (0x45, 0x36) => {
                    nickname::parse_other(&self.parser_context(), payload, is_compressed_bundle)
                }
                (0x56, 0x36) => {
                    let parsed = nickname::parse_main_combat_power(
                        &self.parser_context(),
                        payload,
                        is_compressed_bundle,
                    );
                    if parsed {
                        self.own_actor.combat_power_seen();
                    }
                    parsed
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
    use std::sync::{Arc, RwLock};

    use super::StreamProcessor;
    use crate::config::DpsMeterConfig;
    use crate::events::{EventSink, MeterEvent};
    use crate::logger::AppLogger;
    use crate::mobs::catalog::MobCatalog;
    use crate::storage::data_storage::DataStorage;

    struct Silent;
    impl EventSink for Silent {
        fn emit(&self, _: MeterEvent) {}
    }

    const BEFORE_ZONE_CHANGE: u32 = 6812;
    const AFTER_ZONE_CHANGE: u32 = 7735;
    const MOB: u32 = 900;
    const NAME: &str = "Tester";

    fn connection(named: bool) -> (StreamProcessor, Arc<DataStorage>) {
        let config = Arc::new(RwLock::new(DpsMeterConfig::default()));
        let mobs = Arc::new(MobCatalog::open(Arc::clone(&config), None));
        let storage = Arc::new(DataStorage::new(Arc::new(Silent), Arc::clone(&config), mobs));
        if named {
            storage.set_main_actor(BEFORE_ZONE_CHANGE, NAME);
        }
        let log = std::env::temp_dir().join("aether-own-actor-tests").join("aether.log");
        let logger = Arc::new(AppLogger::new(log, Box::new(|_| {})));
        let processor = StreamProcessor::new(Arc::clone(&storage), logger, "test".to_string(), config);
        (processor, storage)
    }

    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut frame = vec![u8::try_from(payload.len() + 1).expect("short frame")];
        frame.extend_from_slice(payload);
        frame
    }

    fn combat_power() -> Vec<u8> {
        let mut payload = vec![0x56, 0x36];
        payload.extend_from_slice(&50_099u32.to_le_bytes());
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

    fn remain_hp(target: u32, hp: u32) -> Vec<u8> {
        let mut payload = vec![0x00, 0x8D];
        payload.extend(varint(target));
        payload.extend_from_slice(&[1, 1, 1]);
        payload.extend_from_slice(&hp.to_le_bytes());
        frame(&payload)
    }

    fn named_by_the_game(actor_id: u32, name: &str) -> Vec<u8> {
        let mut payload = vec![0x33, 0x36];
        payload.extend(varint(actor_id));
        payload.extend_from_slice(&[0; 5]);
        payload.push(u8::try_from(name.len()).expect("short name"));
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(&1302u16.to_le_bytes());
        payload.push(22);
        frame(&payload)
    }

    fn feed(processor: &mut StreamProcessor, packets: &[Vec<u8>]) {
        for packet in packets {
            assert!(processor.parse_packet(packet, false), "packet {packet:02X?} parsed");
        }
    }

    #[test]
    fn a_new_connection_gives_the_named_character_its_new_actor_id() {
        let (mut processor, storage) = connection(true);

        feed(&mut processor, &[combat_power(), remain_hp(AFTER_ZONE_CHANGE, 2_332)]);

        assert_eq!(storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
        assert_eq!(storage.main_actor_name().as_deref(), Some(NAME));
        assert_eq!(storage.actor_id_name_snapshot().get(&AFTER_ZONE_CHANGE).map(String::as_str), Some(NAME));
    }

    #[test]
    fn only_the_hp_packet_right_after_the_combat_power_counts() {
        let (mut processor, storage) = connection(true);

        feed(
            &mut processor,
            &[combat_power(), remain_hp(AFTER_ZONE_CHANGE, 2_332), remain_hp(MOB, 40_000)],
        );

        assert_eq!(storage.main_actor_id(), Some(AFTER_ZONE_CHANGE));
    }

    #[test]
    fn an_hp_packet_without_a_combat_power_before_it_changes_nothing() {
        let (mut processor, storage) = connection(true);

        feed(&mut processor, &[remain_hp(AFTER_ZONE_CHANGE, 2_332)]);

        assert_eq!(storage.main_actor_id(), Some(BEFORE_ZONE_CHANGE));
    }

    #[test]
    fn an_opening_hp_packet_above_the_boss_threshold_is_not_you_and_still_settles() {
        let (mut processor, storage) = connection(true);

        feed(
            &mut processor,
            &[combat_power(), remain_hp(MOB, 9_800_000), remain_hp(AFTER_ZONE_CHANGE, 2_332)],
        );

        assert_eq!(storage.main_actor_id(), Some(BEFORE_ZONE_CHANGE));
    }

    #[test]
    fn a_connection_that_named_the_character_keeps_what_the_game_said() {
        let (mut processor, storage) = connection(false);

        feed(
            &mut processor,
            &[named_by_the_game(BEFORE_ZONE_CHANGE, NAME), combat_power(), remain_hp(MOB, 40_000)],
        );

        assert_eq!(storage.main_actor_id(), Some(BEFORE_ZONE_CHANGE));
    }

    #[test]
    fn a_mob_the_game_already_described_is_not_the_character() {
        let (mut processor, storage) = connection(true);
        storage.append_mob(MOB, 2_300_810);

        feed(&mut processor, &[combat_power(), remain_hp(MOB, 5_153_701)]);

        assert_eq!(storage.main_actor_id(), Some(BEFORE_ZONE_CHANGE));
    }

    #[test]
    fn nobody_is_named_when_the_game_never_named_the_character() {
        let (mut processor, storage) = connection(false);

        feed(&mut processor, &[combat_power(), remain_hp(AFTER_ZONE_CHANGE, 2_332)]);

        assert_eq!(storage.main_actor_id(), None);
        assert_eq!(storage.main_actor_name(), None);
    }
}
