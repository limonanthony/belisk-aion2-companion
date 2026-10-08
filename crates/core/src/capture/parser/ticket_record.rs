//! The game's ticket messages: `0B 61` is the full snapshot sent once per world
//! entry, `0C 61` is one ticket's new full value. Both are checked strictly
//! (every byte accounted for, impossible flag bits refused) so look-alike
//! frames under these opcodes are rejected rather than decoded.

use serde::Serialize;

use super::utils::read_varint;

pub const SNAPSHOT_OPCODE: [u8; 2] = [0x0B, 0x61];
pub const UPDATE_OPCODE: [u8; 2] = [0x0C, 0x61];
const RECORD_FLAG_BITS: u8 = 0x0F;
const MIN_RECORD_BYTES: usize = 5;
const TEST_TICKET_IDS: std::ops::RangeInclusive<u32> = 10..=12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketRecord {
    pub flags: u8,
    pub id: u32,
    pub remaining_ms: Option<i64>,
    pub field_b: Option<i64>,
    pub count: u32,
    pub charge: u32,
}

impl TicketRecord {
    /// The game's test tickets 10, 11 and 12 tick every ten minutes and mean
    /// nothing to a player.
    pub fn is_test_ticket(&self) -> bool {
        TEST_TICKET_IDS.contains(&self.id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TicketReason {
    Gain,
    TimedRefill,
    Spend,
    Unknown(u8),
}

impl TicketReason {
    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Gain,
            2 => Self::TimedRefill,
            3 => Self::Spend,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TicketUpdate {
    pub outer: u8,
    pub record: TicketRecord,
    pub reason: TicketReason,
    pub delta: Option<u32>,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn after_opcode(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 2 }
    }

    fn left(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| format!("truncated: {len} byte(s) wanted at offset {}, {} left", self.pos, self.left()))?;
        self.pos += len;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i64(&mut self) -> Result<i64, String> {
        let bytes = self.take(8)?;
        Ok(i64::from_le_bytes(bytes.try_into().expect("take(8) returns 8 bytes")))
    }

    fn varint(&mut self) -> Result<u32, String> {
        let varint = read_varint(self.data, self.pos);
        if !varint.is_valid() {
            return Err(format!("bad varint at offset {}", self.pos));
        }
        self.pos += varint.length;
        u32::try_from(varint.value).map_err(|_| format!("varint at offset {} does not fit u32", self.pos))
    }

    fn finish(&self) -> Result<(), String> {
        match self.left() {
            0 => Ok(()),
            extra => Err(format!("{extra} byte(s) left after the body")),
        }
    }
}

fn decode_record(reader: &mut Reader) -> Result<TicketRecord, String> {
    let flags = reader.u8()?;
    if flags & !RECORD_FLAG_BITS != 0 {
        return Err(format!("record flags {flags:#04x} set bits above bit 3"));
    }
    let id = reader.u32()?;
    let remaining_ms = if flags & 1 != 0 { Some(reader.i64()?) } else { None };
    let field_b = if flags & 2 != 0 { Some(reader.i64()?) } else { None };
    let count = if flags & 4 != 0 { reader.varint()? } else { 0 };
    let charge = if flags & 8 != 0 { reader.varint()? } else { 0 };
    Ok(TicketRecord { flags, id, remaining_ms, field_b, count, charge })
}

/// `data` is the whole message, opcode included.
pub fn decode_snapshot(data: &[u8]) -> Result<Vec<TicketRecord>, String> {
    if data.get(..2) != Some(&SNAPSHOT_OPCODE[..]) {
        return Err("not a 0B 61 frame".to_string());
    }
    let mut reader = Reader::after_opcode(data);
    let declared = reader.varint()? as usize;
    if declared.saturating_mul(MIN_RECORD_BYTES) > reader.left() {
        return Err(format!("{declared} records declared but only {} byte(s) follow", reader.left()));
    }
    let records = (0..declared).map(|_| decode_record(&mut reader)).collect::<Result<Vec<_>, _>>()?;
    reader.finish()?;
    Ok(records)
}

/// `data` is the whole message, opcode included.
pub fn decode_update(data: &[u8]) -> Result<TicketUpdate, String> {
    if data.get(..2) != Some(&UPDATE_OPCODE[..]) {
        return Err("not a 0C 61 frame".to_string());
    }
    let mut reader = Reader::after_opcode(data);
    let outer = reader.u8()?;
    if outer > 1 {
        return Err(format!("outer flags {outer:#04x} is not 0 or 1"));
    }
    let record = decode_record(&mut reader)?;
    let reason = TicketReason::from_code(reader.u8()?);
    let delta = if outer & 1 != 0 { Some(reader.u32()?) } else { None };
    reader.finish()?;
    Ok(TicketUpdate { outer, record, reason, delta })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn varint(mut value: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let low = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                bytes.push(low);
                return bytes;
            }
            bytes.push(low | 0x80);
        }
    }

    pub(crate) fn record(id: u32, remaining_ms: Option<i64>, count: Option<u32>, charge: Option<u32>) -> Vec<u8> {
        let flags = u8::from(remaining_ms.is_some()) | (u8::from(count.is_some()) << 2) | (u8::from(charge.is_some()) << 3);
        let mut bytes = vec![flags];
        bytes.extend(id.to_le_bytes());
        if let Some(ms) = remaining_ms {
            bytes.extend(ms.to_le_bytes());
        }
        if let Some(count) = count {
            bytes.extend(varint(count));
        }
        if let Some(charge) = charge {
            bytes.extend(varint(charge));
        }
        bytes
    }

    pub(crate) fn snapshot(records: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = SNAPSHOT_OPCODE.to_vec();
        bytes.extend(varint(records.len() as u32));
        records.iter().for_each(|record| bytes.extend(record));
        bytes
    }

    pub(crate) fn update(outer: u8, record: Vec<u8>, reason: u8, delta: Option<u32>) -> Vec<u8> {
        let mut bytes = UPDATE_OPCODE.to_vec();
        bytes.push(outer);
        bytes.extend(record);
        bytes.push(reason);
        if let Some(delta) = delta {
            bytes.extend(delta.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn absent_record_fields_read_as_zero() {
        let data = snapshot(&[
            record(4, None, None, Some(3)),
            record(60000001, None, Some(15), Some(125)),
            record(6, None, None, None),
            record(3, Some(25_200_000), None, None),
        ]);
        let records = decode_snapshot(&data).expect("valid snapshot");
        let counts: Vec<_> = records.iter().map(|record| (record.id, record.count, record.charge, record.remaining_ms)).collect();
        assert_eq!(
            counts,
            vec![(4, 0, 3, None), (60000001, 15, 125, None), (6, 0, 0, None), (3, 0, 0, Some(25_200_000))]
        );
        assert_eq!(records[2].flags, 0);
    }

    #[test]
    fn the_second_time_field_is_read_when_flag_bit_one_is_set() {
        let mut data = snapshot(&[]);
        data.pop();
        data.extend(varint(1));
        data.push(0b0111);
        data.extend(7u32.to_le_bytes());
        data.extend(1i64.to_le_bytes());
        data.extend(2i64.to_le_bytes());
        data.extend(varint(300));
        let records = decode_snapshot(&data).expect("valid snapshot");
        assert_eq!(records[0].remaining_ms, Some(1));
        assert_eq!(records[0].field_b, Some(2));
        assert_eq!(records[0].count, 300);
    }

    #[test]
    fn a_snapshot_with_stray_flags_trailing_bytes_or_an_impossible_count_is_rejected() {
        assert!(decode_snapshot(&snapshot(&[vec![0x10, 1, 0, 0, 0]])).is_err());
        let mut trailing = snapshot(&[record(4, None, Some(1), None)]);
        trailing.push(0);
        assert!(decode_snapshot(&trailing).is_err());
        let mut too_many = SNAPSHOT_OPCODE.to_vec();
        too_many.extend(varint(403_477));
        too_many.extend([0u8; 8]);
        assert!(decode_snapshot(&too_many).is_err());
    }

    #[test]
    fn a_snapshot_that_stops_inside_a_record_is_rejected() {
        let mut data = snapshot(&[record(4, None, Some(1), None), record(8, Some(5), Some(1), None)]);
        data.truncate(data.len() - 3);
        assert!(decode_snapshot(&data).is_err());
    }

    #[test]
    fn an_update_carries_the_reason_and_a_delta_only_when_outer_is_one() {
        let gain = decode_update(&update(1, record(60000001, None, Some(15), Some(225)), 1, Some(200))).expect("valid gain");
        assert_eq!((gain.reason, gain.delta, gain.record.charge), (TicketReason::Gain, Some(200), 225));
        let spend = decode_update(&update(0, record(60000001, None, None, Some(145)), 3, None)).expect("valid spend");
        assert_eq!((spend.reason, spend.delta, spend.record.count), (TicketReason::Spend, None, 0));
        let refill = decode_update(&update(0, record(60000001, None, Some(15), Some(60)), 2, None)).expect("valid refill");
        assert_eq!(refill.reason, TicketReason::TimedRefill);
        let other = decode_update(&update(0, record(4, None, Some(1), None), 9, None)).expect("valid other");
        assert_eq!(other.reason, TicketReason::Unknown(9));
    }

    #[test]
    fn an_update_with_a_bad_outer_byte_or_a_missing_delta_is_rejected() {
        assert!(decode_update(&update(2, record(4, None, Some(1), None), 3, None)).is_err());
        assert!(decode_update(&update(1, record(4, None, Some(1), None), 1, None)).is_err());
    }

    #[test]
    fn an_update_with_bytes_after_the_body_is_rejected() {
        let mut data = update(0, record(4, None, Some(1), None), 3, None);
        data.push(0);
        assert!(decode_update(&data).is_err());
    }

    #[test]
    fn only_ids_ten_to_twelve_are_test_tickets() {
        let is_test = |id| decode_snapshot(&snapshot(&[record(id, None, Some(1), None)])).expect("valid")[0].is_test_ticket();
        assert_eq!([9, 10, 11, 12, 13].map(is_test), [false, true, true, true, false]);
    }
}
