//! The game's pet messages: `00 90` is the collection snapshot sent once per
//! login, `0D 90` souls gained (a delta, one entry per pet), `09 90` a pet
//! obtained for the first time, `11 90` gains of the six counters the
//! snapshot's tail lists. The numbers are passed on untouched; levels and
//! thresholds are applied in `crate::pets`.

use serde::Serialize;

use super::wire_cursor::Cursor;

pub const SNAPSHOT_OPCODE: [u8; 2] = [0x00, 0x90];
pub const SOUL_GAINS_OPCODE: [u8; 2] = [0x0D, 0x90];
pub const COUNTER_GAINS_OPCODE: [u8; 2] = [0x11, 0x90];
pub const UNLOCK_OPCODE: [u8; 2] = [0x09, 0x90];

const SNAPSHOT_HEADER_BYTES: usize = 4;
const LEVEL_ENTRY_BYTES: usize = 12;
const PROGRESS_ENTRY_BYTES: usize = 8;
const COUNTER_GAIN_TRAILER: u8 = 0x02;
const UNLOCK_UNIDENTIFIED_BYTES: usize = 9;

/// A pet the player has obtained, at the level the game gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetLevel {
    pub pet_id: u32,
    pub level: u32,
}

/// Souls collected toward a pet's next level since its last level-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetProgress {
    pub pet_id: u32,
    pub progress: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetCounter {
    pub kind: u8,
    pub value: u32,
}

/// Pets never obtained are in neither list: absent means level 0 and no souls.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSnapshot {
    pub levels: Vec<PetLevel>,
    pub progress: Vec<PetProgress>,
    pub counters: Vec<PetCounter>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoulGain {
    pub pet_id: u32,
    pub amount: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterGain {
    pub kind: u8,
    pub amount: u32,
}

/// A pet obtained for the first time (level 0 to 1). The message also carries a
/// kind and two numbers whose meaning is unknown, which are not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetUnlock {
    pub pet_id: u32,
}

fn body_of<'a>(data: &'a [u8], opcode: [u8; 2]) -> Result<Cursor<'a>, String> {
    match data.strip_prefix(&opcode[..]) {
        Some(body) => Ok(Cursor::new(body)),
        None => Err(format!("not a {:02X} {:02X} frame", opcode[0], opcode[1])),
    }
}

fn truncated<T>(value: Option<T>, what: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("truncated at {what}"))
}

fn declared_list(cursor: &mut Cursor, entry_bytes: usize, what: &str) -> Result<u32, String> {
    truncated(cursor.count(entry_bytes), what)
}

/// `data` is the whole message, opcode included. The unidentified bytes after
/// the counters are skipped.
pub fn decode_snapshot(data: &[u8]) -> Result<PetSnapshot, String> {
    let mut cursor = body_of(data, SNAPSHOT_OPCODE)?;
    truncated(cursor.skip(SNAPSHOT_HEADER_BYTES), "the header")?;

    let level_count = declared_list(&mut cursor, LEVEL_ENTRY_BYTES, "the level count")?;
    let mut levels = Vec::with_capacity(level_count as usize);
    for _ in 0..level_count {
        let pet_id = truncated(cursor.u32(), "a level entry")?;
        let repeated_id = truncated(cursor.u32(), "a level entry")?;
        let level = truncated(cursor.u32(), "a level entry")?;
        if repeated_id != pet_id {
            return Err(format!("level entry for pet {pet_id} repeats id {repeated_id}"));
        }
        levels.push(PetLevel { pet_id, level });
    }

    let progress_count = declared_list(&mut cursor, PROGRESS_ENTRY_BYTES, "the progress count")?;
    let mut progress = Vec::with_capacity(progress_count as usize);
    for _ in 0..progress_count {
        let pet_id = truncated(cursor.u32(), "a progress entry")?;
        let souls = truncated(cursor.u32(), "a progress entry")?;
        progress.push(PetProgress { pet_id, progress: souls });
    }

    let counter_count = truncated(cursor.u8(), "the counter count")?;
    let mut counters = Vec::with_capacity(usize::from(counter_count));
    for _ in 0..counter_count {
        let kind = truncated(cursor.u8(), "a counter")?;
        let value = truncated(cursor.u32(), "a counter")?;
        counters.push(PetCounter { kind, value });
    }

    Ok(PetSnapshot { levels, progress, counters })
}

/// `data` is the whole message, opcode included: the own entity, the pet, a
/// count of 1, a kind and two numbers that are not identified, and a closing 0.
pub fn decode_unlock(data: &[u8]) -> Result<PetUnlock, String> {
    let mut cursor = body_of(data, UNLOCK_OPCODE)?;
    truncated(cursor.entity_id(), "the own entity")?;
    let pet_id = truncated(cursor.u32(), "the pet")?;
    let count = truncated(cursor.u32(), "the count")?;
    if count != 1 {
        return Err(format!("unlock count {count} is not 1"));
    }
    truncated(cursor.skip(UNLOCK_UNIDENTIFIED_BYTES), "the unidentified fields")?;
    let closing = truncated(cursor.u32(), "the closing number")?;
    if closing != 0 {
        return Err(format!("closing number {closing} is not 0"));
    }
    finished(&cursor)?;
    Ok(PetUnlock { pet_id })
}

/// `data` is the whole message, opcode included.
pub fn decode_soul_gains(data: &[u8]) -> Result<Vec<SoulGain>, String> {
    let mut cursor = body_of(data, SOUL_GAINS_OPCODE)?;
    let count = truncated(cursor.u8(), "the gain count")?;
    if count == 0 {
        return Err("a soul gain message without gains".to_string());
    }
    let mut gains = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let pet_id = truncated(cursor.u32(), "a soul gain")?;
        let amount = truncated(cursor.u32(), "a soul gain")?;
        gains.push(SoulGain { pet_id, amount });
    }
    finished(&cursor)?;
    Ok(gains)
}

/// `data` is the whole message, opcode included.
pub fn decode_counter_gains(data: &[u8]) -> Result<Vec<CounterGain>, String> {
    let mut cursor = body_of(data, COUNTER_GAINS_OPCODE)?;
    let count = truncated(cursor.u8(), "the gain count")?;
    if count == 0 {
        return Err("a counter gain message without gains".to_string());
    }
    let mut gains = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let kind = truncated(cursor.u8(), "a counter gain")?;
        let amount = truncated(cursor.u32(), "a counter gain")?;
        gains.push(CounterGain { kind, amount });
    }
    let trailer = truncated(cursor.u8(), "the trailing byte")?;
    if trailer != COUNTER_GAIN_TRAILER {
        return Err(format!("trailing byte {trailer:#04x} is not {COUNTER_GAIN_TRAILER:#04x}"));
    }
    finished(&cursor)?;
    Ok(gains)
}

fn finished(cursor: &Cursor) -> Result<(), String> {
    if cursor.is_at_end() {
        Ok(())
    } else {
        Err("bytes left after the body".to_string())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::capture::parser::ticket_record::tests::varint;

    pub(crate) fn snapshot(levels: &[(u32, u32)], progress: &[(u32, u32)], counters: &[(u8, u32)]) -> Vec<u8> {
        let mut bytes = SNAPSHOT_OPCODE.to_vec();
        bytes.extend(1170u32.to_le_bytes());
        bytes.extend(varint(levels.len() as u32));
        for (pet_id, level) in levels {
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(level.to_le_bytes());
        }
        bytes.extend(varint(progress.len() as u32));
        for (pet_id, souls) in progress {
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(souls.to_le_bytes());
        }
        bytes.push(counters.len() as u8);
        for (kind, value) in counters {
            bytes.push(*kind);
            bytes.extend(value.to_le_bytes());
        }
        bytes
    }

    pub(crate) fn soul_gains(gains: &[(u32, u32)]) -> Vec<u8> {
        let mut bytes = SOUL_GAINS_OPCODE.to_vec();
        bytes.push(gains.len() as u8);
        for (pet_id, amount) in gains {
            bytes.extend(pet_id.to_le_bytes());
            bytes.extend(amount.to_le_bytes());
        }
        bytes
    }

    pub(crate) fn counter_gains(gains: &[(u8, u32)]) -> Vec<u8> {
        let mut bytes = COUNTER_GAINS_OPCODE.to_vec();
        bytes.push(gains.len() as u8);
        for (kind, amount) in gains {
            bytes.push(*kind);
            bytes.extend(amount.to_le_bytes());
        }
        bytes.push(COUNTER_GAIN_TRAILER);
        bytes
    }

    pub(crate) fn unlock(pet_id: u32, kind: u8, number: u32, value: u32) -> Vec<u8> {
        let mut bytes = UNLOCK_OPCODE.to_vec();
        bytes.extend(varint(6871));
        bytes.extend(pet_id.to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.push(kind);
        bytes.extend(number.to_le_bytes());
        bytes.extend(value.to_le_bytes());
        bytes.extend(0u32.to_le_bytes());
        bytes
    }

    const SIX_COUNTERS: [(u8, u32); 6] = [(1, 222), (2, 0), (3, 0), (4, 961), (5, 0), (6, 8905)];

    #[test]
    fn a_snapshot_lists_the_levels_the_progress_and_the_counters_and_skips_the_tail() {
        let mut data = snapshot(&[(1086, 2), (1214, 3)], &[(1086, 27), (1119, 4)], &SIX_COUNTERS);
        data.extend([0xAA, 0x05, 0x02, 0x02, 0x06, 0x00]);

        let decoded = decode_snapshot(&data).expect("valid snapshot");

        assert_eq!(
            decoded.levels,
            vec![PetLevel { pet_id: 1086, level: 2 }, PetLevel { pet_id: 1214, level: 3 }]
        );
        assert_eq!(
            decoded.progress,
            vec![PetProgress { pet_id: 1086, progress: 27 }, PetProgress { pet_id: 1119, progress: 4 }]
        );
        assert_eq!(decoded.counters.len(), 6);
        assert_eq!(decoded.counters[3], PetCounter { kind: 4, value: 961 });
    }

    #[test]
    fn a_snapshot_of_an_empty_collection_decodes() {
        let decoded = decode_snapshot(&snapshot(&[], &[], &[])).expect("valid snapshot");

        assert_eq!((decoded.levels.len(), decoded.progress.len(), decoded.counters.len()), (0, 0, 0));
    }

    #[test]
    fn a_snapshot_cut_anywhere_before_its_counters_end_is_rejected() {
        let data = snapshot(&[(1086, 2)], &[(1086, 27)], &SIX_COUNTERS);

        for length in 0..data.len() {
            assert!(decode_snapshot(&data[..length]).is_err(), "{length} bytes decoded");
        }
    }

    #[test]
    fn a_snapshot_with_an_impossible_count_or_a_level_entry_that_repeats_another_id_is_rejected() {
        let mut too_many = SNAPSHOT_OPCODE.to_vec();
        too_many.extend(1170u32.to_le_bytes());
        too_many.extend(varint(403_477));
        too_many.extend([0u8; 12]);
        assert!(decode_snapshot(&too_many).is_err());

        let mut mismatched = snapshot(&[(1086, 2)], &[], &[]);
        mismatched[10] ^= 1;
        assert!(decode_snapshot(&mismatched).is_err());
    }

    #[test]
    fn a_frame_under_another_opcode_is_rejected() {
        assert!(decode_snapshot(&soul_gains(&[(1086, 1)])).is_err());
        assert!(decode_soul_gains(&snapshot(&[], &[], &[])).is_err());
        assert!(decode_counter_gains(&soul_gains(&[(1086, 1)])).is_err());
    }

    #[test]
    fn a_single_soul_gain_decodes_from_a_recorded_frame() {
        let recorded = [0x0D, 0x90, 0x01, 0x3E, 0x04, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];

        assert_eq!(decode_soul_gains(&recorded), Ok(vec![SoulGain { pet_id: 1086, amount: 1 }]));
    }

    #[test]
    fn a_batch_of_soul_gains_keeps_every_entry_in_order() {
        let batch = [(1012, 5), (1105, 10), (1110, 5), (1137, 5), (1153, 5), (1197, 5), (1202, 5)];

        let gains = decode_soul_gains(&soul_gains(&batch)).expect("valid batch");

        assert_eq!(gains.iter().map(|gain| (gain.pet_id, gain.amount)).collect::<Vec<_>>(), batch);
    }

    #[test]
    fn a_soul_gain_message_with_no_gains_missing_bytes_or_extra_bytes_is_rejected() {
        assert!(decode_soul_gains(&soul_gains(&[])).is_err());
        let data = soul_gains(&[(1086, 1), (1119, 1)]);
        assert!(decode_soul_gains(&data[..data.len() - 1]).is_err());
        let mut extra = data;
        extra.push(0);
        assert!(decode_soul_gains(&extra).is_err());
    }

    #[test]
    fn counter_gains_decode_with_their_kinds_and_amounts() {
        let gains = decode_counter_gains(&counter_gains(&[(1, 1), (6, 10)])).expect("valid gains");

        assert_eq!(gains, vec![CounterGain { kind: 1, amount: 1 }, CounterGain { kind: 6, amount: 10 }]);
    }

    #[test]
    fn counter_gains_without_their_trailing_byte_or_with_another_one_are_rejected() {
        let data = counter_gains(&[(4, 1)]);
        assert!(decode_counter_gains(&data[..data.len() - 1]).is_err());
        let mut other_trailer = data.clone();
        *other_trailer.last_mut().unwrap() = 0x03;
        assert!(decode_counter_gains(&other_trailer).is_err());
        let mut extra = data;
        extra.push(0);
        assert!(decode_counter_gains(&extra).is_err());
        assert!(decode_counter_gains(&counter_gains(&[])).is_err());
    }

    #[test]
    fn an_unlock_decodes_from_a_recorded_frame() {
        let recorded = [
            0x09, 0x90, 0xD7, 0x76, 0xB6, 0x04, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05, 0x05, 0x00, 0x00, 0x00, 0xD0,
            0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(decode_unlock(&recorded), Ok(PetUnlock { pet_id: 1206 }));
    }

    #[test]
    fn an_unlock_that_is_cut_short_has_bytes_after_it_or_other_fixed_fields_is_rejected() {
        let recorded = unlock(1134, 3, 6, 10850);
        for length in 0..recorded.len() {
            assert!(decode_unlock(&recorded[..length]).is_err(), "{length} bytes decoded");
        }
        let mut extra = recorded.clone();
        extra.push(0);
        assert!(decode_unlock(&extra).is_err());
        let mut two_pets = recorded.clone();
        two_pets[8] = 2;
        assert!(decode_unlock(&two_pets).is_err());
        let mut not_closed = recorded;
        *not_closed.last_mut().unwrap() = 1;
        assert!(decode_unlock(&not_closed).is_err());
        assert!(decode_unlock(&soul_gains(&[(1086, 1)])).is_err());
    }
}
