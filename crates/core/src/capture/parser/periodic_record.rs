use crate::capture::parser::wire_cursor::Cursor;

const OPCODE: [u8; 2] = [0x05, 0x38];

const HAS_REMAINING_TOTAL: u8 = 0x01;
const HAS_TICK_AMOUNT: u8 = 0x02;
const HAS_WORD_LIST: u8 = 0x04;
const HAS_SKILL: u8 = 0x08;
const HAS_SOURCE_OVERRIDE: u8 = 0x10;
const HAS_FIRST_EXTRA_WORD: u8 = 0x20;
const HAS_SECOND_EXTRA_WORD: u8 = 0x40;
const UNKNOWN_MODES: u8 = 0x80;

/// Damage over time that carries a tick amount and the skill.
pub(crate) const MODE_DAMAGE_TICK: u8 = 0x0A;
/// Heal over time: remaining total, tick amount and skill.
pub(crate) const MODE_HEAL_TICK: u8 = 0x0B;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PeriodicRecord {
    pub target_id: u32,
    pub mode: u8,
    pub actor_id: u32,
    pub effect_id: u32,
    pub tick_amount: Option<u64>,
    pub skill_code: Option<u32>,
}

/// Reads one `05 38` frame, opcode included. Returns `None` unless the frame
/// parses and ends exactly where its last field ends.
pub(crate) fn read_periodic_record(frame: &[u8]) -> Option<PeriodicRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut cursor = Cursor::new(body);

    let target_id = cursor.entity_id()?;
    let mode = cursor.u8()?;
    if mode & UNKNOWN_MODES != 0 {
        return None;
    }
    let actor_id = cursor.entity_id()?;
    cursor.varint32()?;
    let effect_id = cursor.u32()?;

    if mode & HAS_REMAINING_TOTAL != 0 {
        cursor.varint64()?;
    }
    let tick_amount = if mode & HAS_TICK_AMOUNT != 0 {
        Some(cursor.varint64()?)
    } else {
        None
    };
    if mode & HAS_WORD_LIST != 0 {
        cursor.skip_list(4)?;
    }
    let skill_code = if mode & HAS_SKILL != 0 {
        Some(cursor.u32()?)
    } else {
        None
    };
    if mode & HAS_SOURCE_OVERRIDE != 0 {
        cursor.entity_id()?;
    }
    if mode & HAS_FIRST_EXTRA_WORD != 0 {
        cursor.skip(4)?;
    }
    if mode & HAS_SECOND_EXTRA_WORD != 0 {
        cursor.skip(4)?;
    }

    cursor.is_at_end().then_some(PeriodicRecord {
        target_id,
        mode,
        actor_id,
        effect_id,
        tick_amount,
        skill_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAL_TICK_ON_SELF: [u8; 21] = [
        0x05, 0x38, 0xB5, 0x0E, 0x0B, 0xB5, 0x0E, 0x88, 0x07, 0xD9, 0xFA, 0x7F, 0x60, 0xF7, 0x1E,
        0xB7, 0x03, 0x44, 0x0A, 0xF7, 0x00,
    ];
    const DAMAGE_TICK: [u8; 19] = [
        0x05, 0x38, 0xB3, 0xB9, 0x03, 0x0A, 0xB5, 0x0E, 0x06, 0x8B, 0xAF, 0x33, 0x60, 0xC9, 0x05,
        0x34, 0x48, 0xF6, 0x00,
    ];

    #[test]
    fn a_heal_tick_names_who_healed_whom_with_what_and_how_much() {
        let record = read_periodic_record(&HEAL_TICK_ON_SELF).unwrap();

        assert_eq!(record.mode, MODE_HEAL_TICK);
        assert_eq!((record.target_id, record.actor_id), (1845, 1845));
        assert_eq!(record.tick_amount, Some(439));
        assert_eq!(record.skill_code, Some(16_190_020));
    }

    #[test]
    fn a_damage_tick_carries_its_amount_and_skill() {
        let record = read_periodic_record(&DAMAGE_TICK).unwrap();

        assert_eq!(record.mode, MODE_DAMAGE_TICK);
        assert_eq!(record.target_id, 56_499);
        assert_eq!(record.tick_amount, Some(713));
        assert_eq!(record.skill_code, Some(16_140_340));
    }

    #[test]
    fn a_frame_with_bytes_after_the_last_field_is_not_read() {
        let mut frame = HEAL_TICK_ON_SELF.to_vec();
        frame.push(0);

        assert!(read_periodic_record(&frame).is_none());
    }

    #[test]
    fn a_truncated_frame_is_not_read() {
        assert!(read_periodic_record(&HEAL_TICK_ON_SELF[..20]).is_none());
    }

    #[test]
    fn a_mode_with_an_unknown_bit_is_not_read() {
        let mut frame = HEAL_TICK_ON_SELF.to_vec();
        frame[4] = 0x8B;

        assert!(read_periodic_record(&frame).is_none());
    }
}
