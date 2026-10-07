use crate::capture::parser::wire_cursor::Cursor;

const OPCODE: [u8; 2] = [0x04, 0x38];

const HAS_EXTRA_BEFORE_KIND: u16 = 0x0001;
const HAS_SPECIAL_BLOCK: u16 = 0x0002;
const HAS_AMOUNT: u16 = 0x0004;
const HAS_EXTRA_AFTER_AMOUNT: u16 = 0x0008;
const HAS_FLAG_BIT: u16 = 0x0010;
const HAS_MULTI_HIT_LIST: u16 = 0x0020;
const HAS_WORD_LIST: u16 = 0x0040;
const HAS_EXTRA_BYTE: u16 = 0x0080;
const HAS_SECOND_WORD_LIST: u16 = 0x0100;
const HAS_DRAIN_AMOUNT: u16 = 0x0200;
const HAS_REGENERATION_AMOUNT: u16 = 0x0400;
const HAS_EXTRA_BEFORE_BOOL: u16 = 0x0800;
const HAS_TRAILING_BOOL: u16 = 0x1000;
const HAS_TRAILING_WORD: u16 = 0x2000;
const UNKNOWN_FLAGS: u16 = 0xC000;

pub(crate) const KIND_CRITICAL: u8 = 3;
pub(crate) const KIND_REFLECTED: u8 = 9;

pub(crate) const SPECIAL_PARRY: u8 = 0x02;
pub(crate) const SPECIAL_PERFECT: u8 = 0x04;
pub(crate) const SPECIAL_DOUBLE: u8 = 0x08;
const SPECIAL_HEAL_VALID: u8 = 0x20;

const POSITION_BACK: u8 = 1;
const POSITION_FRONT: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttackPosition {
    Back,
    Front,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MultiHit {
    pub count: u32,
    pub damage: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DamageRecord {
    pub target_id: u32,
    pub actor_id: u32,
    pub raw_skill_code: u32,
    pub kind: u8,
    pub special_bits: u8,
    pub position: Option<AttackPosition>,
    pub amount: Option<u64>,
    pub heal_amount: Option<u64>,
    pub drain_amount: Option<u64>,
    pub regeneration_amount: Option<u64>,
    pub multi_hit: MultiHit,
}

impl DamageRecord {
    pub(crate) fn has_special(&self, bit: u8) -> bool {
        self.special_bits & bit != 0
    }
}

/// Reads one `04 38` frame, opcode included. Returns `None` unless the frame
/// parses and ends exactly where its last field ends.
pub(crate) fn read_damage_record(frame: &[u8]) -> Option<DamageRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut cursor = Cursor::new(body);

    let target_id = cursor.entity_id()?;
    let flags = cursor.u16()?;
    if flags & UNKNOWN_FLAGS != 0 {
        return None;
    }
    let actor_id = cursor.entity_id()?;
    let raw_skill_code = cursor.u32()?;
    cursor.skip(1)?;
    if flags & HAS_EXTRA_BEFORE_KIND != 0 {
        cursor.skip(4)?;
    }
    let kind = cursor.u8()?;

    let mut special_bits = 0u8;
    let mut position = None;
    let mut heal_amount = None;
    if flags & HAS_SPECIAL_BLOCK != 0 {
        for bit in 0..6 {
            special_bits |= u8::from(cursor.bool_bit()?) << bit;
        }
        let carried_amount = cursor.varint64()?;
        if special_bits & SPECIAL_HEAL_VALID != 0 {
            heal_amount = Some(carried_amount);
        }
        special_bits |= u8::from(cursor.bool_bit()?) << 6;
        position = match cursor.u8()? {
            POSITION_BACK => Some(AttackPosition::Back),
            POSITION_FRONT => Some(AttackPosition::Front),
            _ => None,
        };
    }

    cursor.skip(4)?;
    cursor.skip(4)?;
    cursor.varint32()?;

    let amount = if flags & HAS_AMOUNT != 0 {
        Some(cursor.varint64()?)
    } else {
        None
    };
    if flags & HAS_EXTRA_AFTER_AMOUNT != 0 {
        cursor.skip(4)?;
    }
    if flags & HAS_FLAG_BIT != 0 {
        special_bits |= u8::from(cursor.bool_bit()?) << 7;
    }

    let mut multi_hit = MultiHit::default();
    if flags & HAS_MULTI_HIT_LIST != 0 {
        multi_hit.count = cursor.count(1)?;
        for _ in 0..multi_hit.count {
            multi_hit.damage = multi_hit.damage.saturating_add(cursor.varint64()?);
        }
    }
    if flags & HAS_WORD_LIST != 0 {
        cursor.skip_list(4)?;
    }
    if flags & HAS_EXTRA_BYTE != 0 {
        cursor.skip(1)?;
    }
    if flags & HAS_SECOND_WORD_LIST != 0 {
        cursor.skip_list(4)?;
    }
    cursor.skip(2)?;
    let drain_amount = if flags & HAS_DRAIN_AMOUNT != 0 {
        Some(cursor.varint64()?)
    } else {
        None
    };
    let regeneration_amount = if flags & HAS_REGENERATION_AMOUNT != 0 {
        Some(cursor.varint64()?)
    } else {
        None
    };
    if flags & HAS_EXTRA_BEFORE_BOOL != 0 {
        cursor.skip(4)?;
    }
    if flags & HAS_TRAILING_BOOL != 0 {
        cursor.bool_bit()?;
    }
    if flags & HAS_TRAILING_WORD != 0 {
        cursor.skip(4)?;
    }

    cursor.is_at_end().then_some(DamageRecord {
        target_id,
        actor_id,
        raw_skill_code,
        kind,
        special_bits,
        position,
        amount,
        heal_amount,
        drain_amount,
        regeneration_amount,
        multi_hit,
    })
}

#[cfg(test)]
pub(crate) fn hit_frame(flags: u16, after_action: &[u8]) -> Vec<u8> {
    hit_frame_with_skill(flags, 11_020_151, after_action)
}

#[cfg(test)]
pub(crate) fn hit_frame_with_skill(flags: u16, skill_code: u32, after_action: &[u8]) -> Vec<u8> {
    let mut frame = OPCODE.to_vec();
    frame.extend_from_slice(&[0x96, 0x7D]);
    frame.extend_from_slice(&flags.to_le_bytes());
    frame.push(18);
    frame.extend_from_slice(&skill_code.to_le_bytes());
    frame.push(0x4A);
    frame.extend_from_slice(after_action);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    const EFFECT_SEQUENCE_STATE: [u8; 9] = [0x57, 0xFA, 0x3D, 0x6C, 0x01, 0x00, 0x00, 0x00, 0x05];

    fn after_action(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    #[test]
    fn flag_bit_after_the_special_block_continues_the_same_bool_byte() {
        let body = after_action(&[
            &[2, 0b1100_0000, 0x00, 0],
            &EFFECT_SEQUENCE_STATE,
            &[0x64, 0x01, 0x00],
        ]);

        let record = read_damage_record(&hit_frame(0x0016, &body)).unwrap();

        assert_eq!(record.special_bits, 0xC0);
        assert_eq!(record.position, None);
    }

    #[test]
    fn position_two_is_a_front_attack() {
        let body = after_action(&[
            &[2, 0x00, 0x05, 2],
            &EFFECT_SEQUENCE_STATE,
            &[0x64, 0x01, 0x00],
        ]);

        let record = read_damage_record(&hit_frame(0x0006, &body)).unwrap();

        assert_eq!(record.position, Some(AttackPosition::Front));
    }

    #[test]
    fn every_optional_field_is_read_past() {
        let body = after_action(&[
            &[2],
            &EFFECT_SEQUENCE_STATE,
            &[0x64],
            &[1, 2, 3, 4],
            &[2, 0, 0, 0, 0, 0, 0, 0, 0],
            &[0x09],
            &[0],
            &[0x01, 0x00],
            &[0x81, 0x01],
            &[0x07],
            &[5, 6, 7, 8],
            &[0x01],
            &[9, 10, 11, 12],
        ]);

        let record = read_damage_record(&hit_frame(0x3FCC, &body)).unwrap();

        assert_eq!(record.amount, Some(100));
    }

    #[test]
    fn frame_with_bytes_after_the_last_field_is_rejected() {
        let mut body = after_action(&[&[2], &EFFECT_SEQUENCE_STATE, &[0x64, 0x01, 0x00]]);
        assert!(read_damage_record(&hit_frame(0x0004, &body)).is_some());

        body.push(0);

        assert!(read_damage_record(&hit_frame(0x0004, &body)).is_none());
    }

    #[test]
    fn truncated_frame_is_rejected() {
        let body = after_action(&[&[2], &EFFECT_SEQUENCE_STATE, &[0x64, 0x01]]);

        assert!(read_damage_record(&hit_frame(0x0004, &body)).is_none());
    }

    #[test]
    fn flags_with_unknown_high_bits_are_rejected() {
        let body = after_action(&[&[2], &EFFECT_SEQUENCE_STATE, &[0x64, 0x01, 0x00]]);

        assert!(read_damage_record(&hit_frame(0x4004, &body)).is_none());
    }
}
