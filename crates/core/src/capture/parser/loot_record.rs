//! The game's loot messages, as far as the pet tracker needs them: `1E 56`
//! announces loot objects with the entity they dropped from, `21 56` is the
//! player picking one up (by object id and serial), and `95 56` accompanies a
//! black-market purchase. Everything else in the messages is skipped.

use serde::Serialize;

use super::wire_cursor::Cursor;

pub const LOOT_SPAWN_OPCODE: [u8; 2] = [0x1E, 0x56];
pub const LOOT_PICKUP_OPCODE: [u8; 2] = [0x21, 0x56];
pub const PURCHASE_OPCODE: [u8; 2] = [0x95, 0x56];

const SPAWN_RECORD_BYTES: usize = 63;
const SPAWN_POSITION_BYTES: usize = 24;
const SPAWN_UNIDENTIFIED_BEFORE_ITEM_BYTES: usize = 9;
const PICKUP_UNIDENTIFIED_BEFORE_OBJECT_BYTES: usize = 2;

/// One loot object the game put on the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LootSpawn {
    /// The world entity the loot dropped from.
    pub dropper_entity: u32,
    pub object_id: u32,
    pub serial: u32,
    pub item_id: u32,
}

/// The local player picking a loot object up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LootPickup {
    pub object_id: u32,
    pub serial: u32,
}

fn truncated<T>(value: Option<T>, what: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("truncated at {what}"))
}

fn body_of(data: &[u8], opcode: [u8; 2]) -> Result<Cursor<'_>, String> {
    match data.strip_prefix(&opcode[..]) {
        Some(body) => Ok(Cursor::new(body)),
        None => Err(format!("not a {:02X} {:02X} frame", opcode[0], opcode[1])),
    }
}

/// `data` is the whole message, opcode included: a record count, then that
/// many fixed-size records, then a closing byte.
pub fn decode_loot_spawns(data: &[u8]) -> Result<Vec<LootSpawn>, String> {
    let mut cursor = body_of(data, LOOT_SPAWN_OPCODE)?;
    let count = truncated(cursor.u8(), "the record count")?;
    let mut spawns = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let mut record = Cursor::new(truncated(cursor.take(SPAWN_RECORD_BYTES), "a loot record")?);
        let dropper_entity = truncated(record.u32(), "the dropper")?;
        truncated(record.skip(SPAWN_POSITION_BYTES), "the positions")?;
        let object_id = truncated(record.u32(), "the object id")?;
        let serial = truncated(record.u32(), "the serial")?;
        truncated(record.skip(SPAWN_UNIDENTIFIED_BEFORE_ITEM_BYTES), "the bytes before the item")?;
        let item_id = truncated(record.u32(), "the item id")?;
        spawns.push(LootSpawn { dropper_entity, object_id, serial, item_id });
    }
    Ok(spawns)
}

/// `data` is the whole message, opcode included.
pub fn decode_loot_pickup(data: &[u8]) -> Result<LootPickup, String> {
    let mut cursor = body_of(data, LOOT_PICKUP_OPCODE)?;
    truncated(cursor.skip(PICKUP_UNIDENTIFIED_BEFORE_OBJECT_BYTES), "the bytes before the object")?;
    let object_id = truncated(cursor.u32(), "the object id")?;
    let serial = truncated(cursor.u32(), "the serial")?;
    Ok(LootPickup { object_id, serial })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// 14:18:14.148 of session-1791634499923: loot dropped by entity 48710.
    pub(crate) const SPAWN_OF_PET_SOUL: [u8; 67] = [
        0x1e, 0x56, 0x01, 0x46, 0xbe, 0x00, 0x00, 0x48, 0x5c, 0x8d, 0xc7, 0x66, 0xe7, 0x91, 0xc7, 0x00,
        0x70, 0x0d, 0x46, 0x5b, 0x7f, 0x8d, 0xc7, 0x07, 0xc1, 0x91, 0xc7, 0x00, 0x64, 0x0d, 0x46, 0x0c,
        0x58, 0x00, 0x00, 0xac, 0xb0, 0x3f, 0x00, 0xf7, 0x43, 0xde, 0x1f, 0x01, 0x00, 0x00, 0x00, 0x02,
        0x3a, 0xa5, 0x03, 0x00, 0x00, 0x00, 0x16, 0x05, 0xe5, 0xc1, 0xc1, 0x25, 0xa1, 0x01, 0x00, 0x00,
        0x01, 0x03, 0x01,
    ];

    /// 14:18:14.507: the player picks that object up.
    pub(crate) const PICKUP_OF_PET_SOUL: [u8; 13] =
        [0x21, 0x56, 0x00, 0x00, 0x0c, 0x58, 0x00, 0x00, 0xac, 0xb0, 0x3f, 0x00, 0x01];

    /// 14:18:11.800: two loot objects dropped by entity 35544 in one message.
    const SPAWN_PAIR: [u8; 130] = [
        0x1e, 0x56, 0x02, 0xd8, 0x8a, 0x00, 0x00, 0xa8, 0x48, 0x95, 0xc7, 0x64, 0x91, 0x8b, 0xc7, 0x00,
        0xc0, 0x0d, 0x46, 0xbe, 0x1f, 0x95, 0xc7, 0x8a, 0xba, 0x8b, 0xc7, 0x00, 0xc8, 0x0d, 0x46, 0xff,
        0x58, 0x00, 0x00, 0x1e, 0x92, 0x3f, 0x00, 0xf7, 0x43, 0xde, 0x1f, 0x01, 0x00, 0x00, 0x00, 0x02,
        0x9d, 0xe5, 0x03, 0x00, 0x00, 0x00, 0x16, 0x05, 0x3d, 0xe9, 0xbf, 0x25, 0xa1, 0x01, 0x00, 0x00,
        0x01, 0x03, 0xd8, 0x8a, 0x00, 0x00, 0xa8, 0x48, 0x95, 0xc7, 0x64, 0x91, 0x8b, 0xc7, 0x00, 0xc0,
        0x0d, 0x46, 0x27, 0x0d, 0x95, 0xc7, 0x0d, 0x64, 0x8b, 0xc7, 0x00, 0x60, 0x0d, 0x46, 0xff, 0x58,
        0x00, 0x00, 0x21, 0x92, 0x3f, 0x00, 0xd1, 0xf2, 0x63, 0x24, 0x01, 0x00, 0x00, 0x00, 0x02, 0x9d,
        0xe5, 0x03, 0x00, 0x00, 0x00, 0x16, 0x05, 0x3d, 0xe9, 0xbf, 0x25, 0xa1, 0x01, 0x00, 0x00, 0x01,
        0x03, 0x02,
    ];

    #[test]
    fn a_loot_spawn_names_its_dropper_object_serial_and_item() {
        assert_eq!(
            decode_loot_spawns(&SPAWN_OF_PET_SOUL).unwrap(),
            vec![LootSpawn { dropper_entity: 48_710, object_id: 0x580c, serial: 0x3f_b0ac, item_id: 0x3_a53a }]
        );
    }

    #[test]
    fn a_loot_message_with_two_records_lists_both() {
        let spawns = decode_loot_spawns(&SPAWN_PAIR).unwrap();
        assert_eq!(spawns.len(), 2);
        assert!(spawns.iter().all(|spawn| spawn.dropper_entity == 35_544 && spawn.object_id == 0x58ff));
        assert_eq!((spawns[0].serial, spawns[1].serial), (0x3f_921e, 0x3f_9221));
    }

    #[test]
    fn a_truncated_loot_spawn_is_rejected() {
        assert!(decode_loot_spawns(&SPAWN_OF_PET_SOUL[..40]).is_err());
        assert!(decode_loot_spawns(&PICKUP_OF_PET_SOUL).is_err());
    }

    #[test]
    fn a_pickup_names_the_object_and_serial() {
        assert_eq!(
            decode_loot_pickup(&PICKUP_OF_PET_SOUL).unwrap(),
            LootPickup { object_id: 0x580c, serial: 0x3f_b0ac }
        );
        assert!(decode_loot_pickup(&PICKUP_OF_PET_SOUL[..9]).is_err());
    }
}
