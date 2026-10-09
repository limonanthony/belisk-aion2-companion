use crate::capture::parser::wire_cursor::Cursor;

const OPCODE: [u8; 2] = [0x42, 0x36];

const FLAG_DIED: u32 = 3;
const FLAG_REMOVED: u32 = 7;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DeathRecord {
    pub entity_id: u32,
    flag: u32,
}

impl DeathRecord {
    /// Flag 3 is a death; 1 is an entity leaving the field and 7 a removal
    /// that is not a kill.
    pub(crate) fn died(&self) -> bool {
        self.flag == FLAG_DIED
    }

    /// The entity is gone and did not die. Flag 1 is not this: a boss that
    /// left range with flag 1 reported its health again minutes later.
    pub(crate) fn removed(&self) -> bool {
        self.flag == FLAG_REMOVED
    }
}

/// Reads one `42 36` frame, opcode included. Returns `None` unless the frame
/// parses and ends exactly where its last field ends.
pub(crate) fn read_death_record(frame: &[u8]) -> Option<DeathRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut cursor = Cursor::new(body);

    let entity_id = cursor.entity_id()?;
    cursor.varint32()?;
    let flag = cursor.varint32()?;

    cursor.is_at_end().then_some(DeathRecord { entity_id, flag })
}

#[cfg(test)]
pub(crate) fn death_frame(entity_id_varint: &[u8], flag: u8) -> Vec<u8> {
    let mut frame = OPCODE.to_vec();
    frame.extend_from_slice(entity_id_varint);
    frame.extend_from_slice(&[0, flag]);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTITY: [u8; 3] = [0xFA, 0xC2, 0x03];

    #[test]
    fn flag_three_is_a_death_of_the_named_entity() {
        let record = read_death_record(&death_frame(&ENTITY, 3)).unwrap();

        assert_eq!(record.entity_id, 57_722);
        assert!(record.died());
    }

    #[test]
    fn removal_flags_are_not_deaths() {
        for flag in [1, 7] {
            let record = read_death_record(&death_frame(&ENTITY, flag)).unwrap();

            assert!(!record.died(), "flag {flag}");
        }
    }

    #[test]
    fn only_flag_seven_is_a_removal() {
        for (flag, removed) in [(1, false), (3, false), (7, true)] {
            let record = read_death_record(&death_frame(&ENTITY, flag)).unwrap();

            assert_eq!(record.removed(), removed, "flag {flag}");
        }
    }

    #[test]
    fn a_frame_with_trailing_bytes_is_not_read() {
        let mut frame = death_frame(&ENTITY, 3);
        frame.push(0);

        assert!(read_death_record(&frame).is_none());
    }

    #[test]
    fn a_truncated_frame_is_not_read() {
        let frame = death_frame(&ENTITY, 3);

        assert!(read_death_record(&frame[..frame.len() - 1]).is_none());
    }
}
