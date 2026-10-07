use crate::capture::parser::wire_cursor::Cursor;

const OPCODE: [u8; 2] = [0x00, 0x8D];

const HAS_SMALL_STATS: u8 = 0x01;
const HAS_BIG_STATS: u8 = 0x02;
const KNOWN_MASK_BITS: u8 = HAS_SMALL_STATS | HAS_BIG_STATS;

const SMALL_STAT_BYTES: usize = 5;
const BIG_STAT_BYTES: usize = 9;

const BIG_STAT_CURRENT_HP: u8 = 0;
const BIG_STAT_MAX_HP: u8 = 7;

/// A mask of exactly the small-stat bit marks the local character's own
/// record; the game sends no HP in it.
const OWN_SIGNATURE_MASK: u8 = HAS_SMALL_STATS;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HpRecord {
    pub entity_id: u32,
    mask: u8,
    first_small_stat: Option<u32>,
    pub current_hp: Option<u64>,
    pub max_hp: Option<u64>,
}

impl HpRecord {
    /// The first small stat of a record that carries nothing else.
    pub(crate) fn own_signature_value(&self) -> Option<u32> {
        if self.mask == OWN_SIGNATURE_MASK {
            self.first_small_stat
        } else {
            None
        }
    }
}

/// Reads one `00 8D` frame, opcode included. Returns `None` unless the frame
/// parses, carries no negative HP and ends exactly where its last field ends.
pub(crate) fn read_hp_record(frame: &[u8]) -> Option<HpRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut cursor = Cursor::new(body);

    let entity_id = cursor.entity_id()?;
    let mask = cursor.u8()?;
    if mask & !KNOWN_MASK_BITS != 0 {
        return None;
    }

    let mut first_small_stat = None;
    if mask & HAS_SMALL_STATS != 0 {
        let count = cursor.count(SMALL_STAT_BYTES)?;
        for index in 0..count {
            cursor.u8()?;
            let value = cursor.u32()?;
            if index == 0 {
                first_small_stat = Some(value);
            }
        }
    }

    let mut current_hp = None;
    let mut max_hp = None;
    if mask & HAS_BIG_STATS != 0 {
        let count = cursor.count(BIG_STAT_BYTES)?;
        for _ in 0..count {
            let kind = cursor.u8()?;
            let value = u64::try_from(cursor.i64()?).ok()?;
            match kind {
                BIG_STAT_CURRENT_HP => current_hp = Some(value),
                BIG_STAT_MAX_HP => max_hp = Some(value),
                _ => {}
            }
        }
    }

    cursor.is_at_end().then_some(HpRecord {
        entity_id,
        mask,
        first_small_stat,
        current_hp,
        max_hp,
    })
}

#[cfg(test)]
pub(crate) fn hp_frame(entity_id_varint: &[u8], mask: u8, stats: &[u8]) -> Vec<u8> {
    let mut frame = OPCODE.to_vec();
    frame.extend_from_slice(entity_id_varint);
    frame.push(mask);
    frame.extend_from_slice(stats);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTITY: [u8; 2] = [0xD0, 0x0B];

    fn big_stat(kind: u8, value: i64) -> Vec<u8> {
        let mut bytes = vec![kind];
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    fn big_stats(stats: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = vec![stats.len() as u8];
        bytes.extend(stats.concat());
        bytes
    }

    #[test]
    fn current_and_max_hp_are_read_as_64_bit_values() {
        let above_u32 = 5_000_000_000i64;
        let stats = big_stats(&[big_stat(0, above_u32), big_stat(7, 6_000_000_000)]);

        let record = read_hp_record(&hp_frame(&ENTITY, 2, &stats)).unwrap();

        assert_eq!(record.entity_id, 0x5D0);
        assert_eq!(record.current_hp, Some(5_000_000_000));
        assert_eq!(record.max_hp, Some(6_000_000_000));
    }

    #[test]
    fn hp_between_a_billion_and_u32_max_is_kept() {
        let stats = big_stats(&[big_stat(0, 3_000_000_000)]);

        let record = read_hp_record(&hp_frame(&ENTITY, 2, &stats)).unwrap();

        assert_eq!(record.current_hp, Some(3_000_000_000));
    }

    #[test]
    fn other_big_stat_kinds_are_not_hp() {
        let stats = big_stats(&[big_stat(5, 777)]);

        let record = read_hp_record(&hp_frame(&ENTITY, 2, &stats)).unwrap();

        assert_eq!((record.current_hp, record.max_hp), (None, None));
    }

    #[test]
    fn negative_hp_rejects_the_frame() {
        let stats = big_stats(&[big_stat(0, -1)]);

        assert!(read_hp_record(&hp_frame(&ENTITY, 2, &stats)).is_none());
    }

    #[test]
    fn small_stats_come_before_big_stats() {
        let mut stats = vec![2, 4, 0x2C, 0x01, 0, 0, 8, 0x13, 0x09, 0, 0];
        stats.extend(big_stats(&[big_stat(7, 12_930)]));

        let record = read_hp_record(&hp_frame(&ENTITY, 3, &stats)).unwrap();

        assert_eq!(record.max_hp, Some(12_930));
        assert_eq!(record.current_hp, None);
        assert_eq!(record.own_signature_value(), None);
    }

    #[test]
    fn a_small_stat_only_record_is_the_own_signature() {
        let record = read_hp_record(&hp_frame(&ENTITY, 1, &[1, 3, 0x1C, 0x09, 0, 0])).unwrap();

        assert_eq!(record.own_signature_value(), Some(2_332));
        assert_eq!(record.current_hp, None);
    }

    #[test]
    fn unknown_mask_bits_reject_the_frame() {
        assert!(read_hp_record(&hp_frame(&ENTITY, 4, &[1, 1, 0, 0, 0, 0])).is_none());
    }

    #[test]
    fn bytes_after_the_last_stat_reject_the_frame() {
        let mut stats = big_stats(&[big_stat(0, 100)]);
        stats.push(0);

        assert!(read_hp_record(&hp_frame(&ENTITY, 2, &stats)).is_none());
    }

    #[test]
    fn a_count_larger_than_the_frame_rejects_it() {
        assert!(read_hp_record(&hp_frame(&ENTITY, 2, &[200, 0, 1, 2])).is_none());
    }
}
