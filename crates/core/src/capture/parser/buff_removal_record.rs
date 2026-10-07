use crate::capture::parser::wire_cursor::Cursor;

const OPCODE: [u8; 2] = [0x2C, 0x38];

const HAS_VARINT: u8 = 0x01;
const HAS_U32_FIRST: u8 = 0x02;
const HAS_U32_SECOND: u8 = 0x04;
const KNOWN_MASK_BITS: u8 = HAS_VARINT | HAS_U32_FIRST | HAS_U32_SECOND;

const MIN_ENTRY_BYTES: usize = 3;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BuffRemovalRecord {
    pub target_id: u32,
    pub instance_ids: Vec<u32>,
}

/// Reads one `2C 38` frame, opcode included. Returns `None` unless the frame
/// parses and ends exactly where its last entry ends.
pub(crate) fn read_buff_removal_record(frame: &[u8]) -> Option<BuffRemovalRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut cursor = Cursor::new(body);

    let target_id = cursor.entity_id()?;
    let count = cursor.count(MIN_ENTRY_BYTES)?;

    let mut instance_ids = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mask = cursor.u8()?;
        if mask & !KNOWN_MASK_BITS != 0 {
            return None;
        }
        instance_ids.push(cursor.varint32()?);
        cursor.u8()?;
        if mask & HAS_VARINT != 0 {
            cursor.varint32()?;
        }
        if mask & HAS_U32_FIRST != 0 {
            cursor.u32()?;
        }
        if mask & HAS_U32_SECOND != 0 {
            cursor.u32()?;
        }
    }

    cursor.is_at_end().then_some(BuffRemovalRecord { target_id, instance_ids })
}

#[cfg(test)]
pub(crate) fn removal_frame(target_varint: &[u8], entries: &[&[u8]]) -> Vec<u8> {
    let mut frame = OPCODE.to_vec();
    frame.extend_from_slice(target_varint);
    frame.push(u8::try_from(entries.len()).expect("few entries"));
    for entry in entries {
        frame.extend_from_slice(entry);
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: [u8; 2] = [0xB4, 0x39];

    #[test]
    fn a_removal_names_the_buff_instances_it_ends() {
        let frame = removal_frame(&TARGET, &[&[0, 0x2A, 1], &[0x01, 0x81, 0x01, 1, 0x07]]);

        let record = read_buff_removal_record(&frame).unwrap();

        assert_eq!(record.target_id, 7_348);
        assert_eq!(record.instance_ids, [0x2A, 0x81]);
    }

    #[test]
    fn optional_fields_are_skipped_by_their_mask_bits() {
        let frame = removal_frame(
            &TARGET,
            &[&[0x06, 5, 1, 9, 9, 9, 9, 8, 8, 8, 8], &[0, 6, 1]],
        );

        let record = read_buff_removal_record(&frame).unwrap();

        assert_eq!(record.instance_ids, [5, 6]);
    }

    #[test]
    fn an_unknown_mask_bit_is_not_read() {
        assert!(read_buff_removal_record(&removal_frame(&TARGET, &[&[0x08, 5, 1]])).is_none());
    }

    #[test]
    fn a_frame_with_trailing_bytes_is_not_read() {
        let mut frame = removal_frame(&TARGET, &[&[0, 5, 1]]);
        frame.push(0);

        assert!(read_buff_removal_record(&frame).is_none());
    }

    #[test]
    fn a_count_larger_than_the_frame_is_not_read() {
        let mut frame = removal_frame(&TARGET, &[&[0, 5, 1]]);
        frame[4] = 9;

        assert!(read_buff_removal_record(&frame).is_none());
    }
}
