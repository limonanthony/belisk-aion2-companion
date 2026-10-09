use crate::capture::parser::buff_removal_record::read_buff_removal_record;
use crate::capture::parser::utils::{
    bytes_to_hex, read_u32_le_or_default as parse_u32_le, read_u64_le_or_default as parse_u64_le,
    read_varint,
};
use crate::capture::processor::StreamProcessor;

impl StreamProcessor {
    pub(crate) fn parse_buff_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        if packet.len() < 2 || !matches!(packet[0], 0x2A | 0x2B) || packet[1] != 0x38 {
            return false;
        }

        let opcode = packet[0];
        let mut offset = 2usize;

        let target_info = read_varint(packet, offset);
        if !target_info.is_valid() {
            return false;
        }
        // 2A carries a count byte before its record; 2B is the record alone.
        let mask_offset = offset + target_info.length + usize::from(opcode == 0x2A);
        offset = mask_offset + 1;
        if offset >= packet.len() {
            return false;
        }

        let instance_info = read_varint(packet, offset);
        if !instance_info.is_valid() {
            return false;
        }
        offset += instance_info.length;

        if offset + 4 > packet.len() {
            return false;
        }
        let skill_code = parse_u32_le(packet, offset);
        offset += 4;

        let is_valid_buff_skill = (110_000_000..=200_000_000).contains(&skill_code)
            || (20_000_000..30_000_000).contains(&skill_code);
        if !is_valid_buff_skill {
            return true;
        }

        if offset + 16 > packet.len() {
            return false;
        }
        let duration = parse_u32_le(packet, offset) as u64;
        offset += 8;
        let server_time = parse_u64_le(packet, offset);
        offset += 8;

        let actor_info = read_varint(packet, offset);
        if !actor_info.is_valid() {
            return false;
        }
        if actor_info.value <= 1 {
            return true;
        }

        if duration == u32::MAX as u64 {
            return true;
        }

        let server_start_ms = server_time.saturating_sub(duration);
        let buff = self.data_storage.save_buff(
            target_info.value as u32,
            instance_info.value as u32,
            actor_info.value as u32,
            skill_code,
            server_start_ms,
            duration,
        );
        let latency_ms = buff.last_start_ms as i64 - server_start_ms as i64;
        self.logger.debug(format!(
            "[{}] buff detected target={} actor={} skill={} duration_ms={} server_start_ms={} local_start_ms={} local_end_ms={} latency_ms={} coverage={:.3} active={} server_time={} opcode={:02X}38 packet_len={} packet_hex={}",
            self.port,
            buff.target_id,
            buff.actor_id,
            buff.skill_code,
            duration,
            server_start_ms,
            buff.last_start_ms,
            buff.last_end_ms,
            latency_ms,
            buff.coverage,
            buff.active,
            server_time,
            opcode,
            packet.len(),
            bytes_to_hex(packet)
        ));
        true
    }

    pub(crate) fn parse_buff_removal_packet(
        &mut self,
        packet: &[u8],
        _is_compressed_bundle: bool,
    ) -> bool {
        let Some(record) = read_buff_removal_record(packet) else {
            return false;
        };
        for instance_id in record.instance_ids {
            self.data_storage.end_buff(record.target_id, instance_id);
        }
        true
    }
}
