use crate::capture::parser::wire_cursor::Cursor;

const OPCODE: [u8; 2] = [0x04, 0x8D];

const MAX_NAME_BYTES: usize = 72;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CharDieRecord {
    pub victim_id: u32,
    /// The skill that killed; 0 when the record names none.
    pub skill_code: u32,
    /// 0 when the record names no killer.
    pub killer_id: u32,
    /// The killer's name when it is a player; empty for a monster or nobody.
    pub killer_name: String,
    /// The killer's mob code when it is a monster; 0 for a player or nobody.
    pub killer_mob_code: u32,
}

/// Reads one `04 8D` frame, opcode included: a player's death with the killer
/// the game names. Returns `None` unless the frame parses and ends exactly where
/// its last field ends.
pub(crate) fn read_char_die_record(frame: &[u8]) -> Option<CharDieRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut cursor = Cursor::new(body);

    let victim_id = cursor.entity_id()?;
    let skill_code = cursor.u32()?;
    let killer_id = cursor.varint32()?;
    cursor.u16()?;
    let killer_name = cursor.text(MAX_NAME_BYTES)?;
    cursor.text(MAX_NAME_BYTES)?;
    cursor.u16()?;
    let killer_mob_code = cursor.varint32()?;
    cursor.skip(4)?;
    cursor.skip(2)?;

    cursor.is_at_end().then_some(CharDieRecord {
        victim_id,
        skill_code,
        killer_id,
        killer_name,
        killer_mob_code,
    })
}

#[cfg(test)]
pub(crate) fn char_die_frame(
    victim_id_varint: &[u8],
    skill_code: u32,
    killer_id_varint: &[u8],
    killer_name: &str,
) -> Vec<u8> {
    let mut frame = OPCODE.to_vec();
    frame.extend_from_slice(victim_id_varint);
    frame.extend_from_slice(&skill_code.to_le_bytes());
    frame.extend_from_slice(killer_id_varint);
    frame.extend_from_slice(&1302u16.to_le_bytes());
    frame.push(killer_name.len() as u8);
    frame.extend_from_slice(killer_name.as_bytes());
    frame.push(0);
    frame.extend_from_slice(&[0; 9]);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    const VICTIM: [u8; 3] = [0xEC, 0xC6, 0x02];
    const KILLER: [u8; 2] = [0xDE, 0x3E];

    #[test]
    fn a_player_killer_is_named() {
        let frame = char_die_frame(&VICTIM, 17_000_858, &KILLER, "Veyra");

        let record = read_char_die_record(&frame).unwrap();

        assert_eq!(record.victim_id, 41_836);
        assert_eq!(record.skill_code, 17_000_858);
        assert_eq!(record.killer_id, 8_030);
        assert_eq!(record.killer_name, "Veyra");
        assert_eq!(record.killer_mob_code, 0);
    }

    #[test]
    fn a_record_without_a_killer_is_read_with_the_fields_empty() {
        let frame = char_die_frame(&VICTIM, 0, &[0], "");

        let record = read_char_die_record(&frame).unwrap();

        assert_eq!(
            (
                record.killer_id,
                record.killer_name.as_str(),
                record.killer_mob_code
            ),
            (0, "", 0)
        );
    }

    #[test]
    fn a_monster_killer_carries_its_mob_code() {
        let mut frame = OPCODE.to_vec();
        frame.extend_from_slice(&VICTIM);
        frame.extend_from_slice(&1_607_630u32.to_le_bytes());
        frame.extend_from_slice(&KILLER);
        frame.extend_from_slice(&0u16.to_le_bytes());
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(&0u16.to_le_bytes());
        frame.extend_from_slice(&[0x8A, 0x8B, 0x8C, 0x01]);
        frame.extend_from_slice(&[0; 6]);

        let record = read_char_die_record(&frame).unwrap();

        assert_eq!(record.killer_name, "");
        assert_eq!(
            record.killer_mob_code,
            0x0A | (0x0B << 7) | (0x0C << 14) | (0x01 << 21)
        );
    }

    #[test]
    fn a_frame_with_bytes_after_the_last_field_is_not_read() {
        let mut frame = char_die_frame(&VICTIM, 1, &KILLER, "Veyra");
        frame.push(0);

        assert!(read_char_die_record(&frame).is_none());
    }

    #[test]
    fn a_truncated_frame_is_not_read() {
        let frame = char_die_frame(&VICTIM, 1, &KILLER, "Veyra");

        assert!(read_char_die_record(&frame[..frame.len() - 1]).is_none());
    }
}
