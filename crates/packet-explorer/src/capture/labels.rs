/// Which meter-core parser the processor hands an opcode to (see
/// `StreamProcessor::parse_packet`). Kept by hand: the dispatch is private.
pub fn parser_label(opcode: [u8; 2]) -> Option<&'static str> {
    match opcode {
        [0x33, 0x36] => Some("nickname::parse_main"),
        [0x45, 0x36] => Some("nickname::parse_other"),
        [0x56, 0x36] => Some("nickname::parse_main_combat_power"),
        [0x41, 0x36] => Some("summon::parse_summon_packet"),
        [0x04, 0x38] => Some("damage::parse_damage_packet (combat flow only)"),
        [0x05, 0x38] => Some("damage::parse_dot_packet (combat flow only)"),
        [0x2A, 0x38] | [0x2B, 0x38] => Some("buff::parse_buff_packet"),
        [0x04, 0x8D] => Some("summon::parse_summon_packet_048d"),
        [0x00, 0x8D] => Some("summon::parse_remain_hp_packet"),
        [0x01, 0x91] => Some("known opcode, no parser (field boss timers)"),
        _ => None,
    }
}

pub fn format_opcode(opcode: [u8; 2]) -> String {
    format!("{:02x}{:02x}", opcode[0], opcode[1])
}
