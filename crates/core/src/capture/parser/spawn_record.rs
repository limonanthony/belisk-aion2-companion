use crate::capture::parser::wire_cursor::Cursor;
use crate::region::RegionId;

const OPCODE: [u8; 2] = [0x41, 0x36];

const FLAG_BYTE_A: u32 = 0x01;
const FLAG_BYTE_B: u32 = 0x02;
const FLAG_BYTE_C: u32 = 0x04;
const FLAG_BOOL: u32 = 0x08;
const FLAG_OWNER: u32 = 0x10;
const KNOWN_FLAG_BITS: u32 = 0x1FFFF;

const DESCRIPTOR_HAS_NAME: u8 = 0x01;
const DESCRIPTOR_BYTE_A: u8 = 0x02;
const DESCRIPTOR_BYTE_B: u8 = 0x04;
const DESCRIPTOR_BYTE_AND_WORD: u8 = 0x08;
const DESCRIPTOR_BYTE_C: u8 = 0x10;
const KNOWN_DESCRIPTOR_BITS: u8 = 0x1F;

const STATE_VECTOR_A: u16 = 0x0001;
const STATE_WORD_A: u16 = 0x0002;
const STATE_SHORT: u16 = 0x0004;
const STATE_VECTOR_B: u16 = 0x0008;
const STATE_VECTOR_C: u16 = 0x0010;
const STATE_WORD_B: u16 = 0x0020;
const STATE_BYTE_A: u16 = 0x0040;
const STATE_WORD_C: u16 = 0x0080;
const STATE_BYTE_B: u16 = 0x0100;
const STATE_BYTE_C: u16 = 0x0200;
const STATE_VARINT: u16 = 0x0400;
const STATE_WORD_D: u16 = 0x0800;
const STATE_WORD_E: u16 = 0x1000;
const STATE_BYTE_D: u16 = 0x2000;
const STATE_WORD_F: u16 = 0x4000;
const STATE_WORD_G: u16 = 0x8000;

const STATE_FIXED_PREFIX_BYTES: usize = 18;
const STATE_FIXED_COUNTERS_BYTES: usize = 52;
const VECTOR_BYTES: usize = 12;

const ABNORMAL_HAS_CASTER: u8 = 0x01;
const ABNORMAL_HAS_SKILL: u8 = 0x02;
const ABNORMAL_HAS_VARINT: u8 = 0x04;
const ABNORMAL_HAS_WORD: u8 = 0x08;
const ABNORMAL_HAS_VECTOR: u8 = 0x10;
const ABNORMAL_UNKNOWN_BITS: u8 = 0xE0;
const ABNORMAL_FIXED_BYTES: usize = 4 + 8 + 8;

const MOB_CODES: std::ops::RangeInclusive<u32> = 1..=49_999_999;
const MAX_HP: u64 = 1 << 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlagsWidth {
    U16,
    U32,
}

/// The flags width of each service is a fact about its protocol; when the
/// region is not known, both widths are tried and the one that validates wins.
pub(crate) fn flags_widths(region: RegionId) -> &'static [FlagsWidth] {
    match region {
        RegionId::Global => &[FlagsWidth::U16],
        RegionId::Tw | RegionId::Kr => &[FlagsWidth::U32],
        RegionId::Auto => &[FlagsWidth::U16, FlagsWidth::U32],
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SpawnOwner {
    None,
    Entity(u32),
    /// The frame is intact up to the HP but its tail could not be read.
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SpawnRecord {
    pub entity_id: u32,
    pub mob_code: u32,
    pub current_hp: u64,
    pub max_hp: u64,
    pub owner: SpawnOwner,
}

/// Reads the first `41 36` frame field group up to the owner, opcode included.
/// Fields after the owner are optional blocks no consumer needs. A frame whose
/// abnormal list or owner cannot be read still yields its mob code and HP,
/// with the owner unknown; a width that reads the whole frame wins over one
/// that reads only its head.
pub(crate) fn read_spawn_record(frame: &[u8], widths: &[FlagsWidth]) -> Option<SpawnRecord> {
    let body = frame.strip_prefix(&OPCODE)?;
    let mut head_only = None;
    for width in widths {
        match read_with_width(body, *width) {
            Some(record) if record.owner == SpawnOwner::Unknown => {
                head_only.get_or_insert(record);
            }
            Some(record) => return Some(record),
            None => {}
        }
    }
    head_only
}

/// The entity and mob code a `41 36` frame spawns, for tools outside the crate.
pub fn spawned_mob(frame: &[u8]) -> Option<(u32, u32)> {
    let record = read_spawn_record(frame, flags_widths(crate::region::resolved()))?;
    Some((record.entity_id, record.mob_code))
}

fn read_with_width(body: &[u8], width: FlagsWidth) -> Option<SpawnRecord> {
    let mut cursor = Cursor::new(body);

    let entity_id = cursor.entity_id()?;
    let flags = match width {
        FlagsWidth::U16 => u32::from(cursor.u16()?),
        FlagsWidth::U32 => cursor.u32()?,
    };
    if flags & !KNOWN_FLAG_BITS != 0 {
        return None;
    }

    let mob_code = read_descriptor(&mut cursor)?;
    let (current_hp, max_hp) = read_state(&mut cursor)?;
    if !MOB_CODES.contains(&mob_code) || current_hp > max_hp || max_hp >= MAX_HP {
        return None;
    }

    let owner = read_owner(&mut cursor, flags, entity_id).unwrap_or(SpawnOwner::Unknown);
    Some(SpawnRecord { entity_id, mob_code, current_hp, max_hp, owner })
}

fn read_owner(cursor: &mut Cursor, flags: u32, entity_id: u32) -> Option<SpawnOwner> {
    skip_abnormals(cursor)?;

    for flag in [FLAG_BYTE_A, FLAG_BYTE_B, FLAG_BYTE_C] {
        if flags & flag != 0 {
            cursor.u8()?;
        }
    }
    if flags & FLAG_BOOL != 0 {
        cursor.bool_bit()?;
    }
    if flags & FLAG_OWNER == 0 {
        return Some(SpawnOwner::None);
    }
    match cursor.u32()? {
        0 => Some(SpawnOwner::None),
        owner if owner == entity_id => None,
        owner => Some(SpawnOwner::Entity(owner)),
    }
}

fn read_descriptor(cursor: &mut Cursor) -> Option<u32> {
    let descriptor = cursor.u8()?;
    if descriptor & !KNOWN_DESCRIPTOR_BITS != 0 {
        return None;
    }
    if descriptor & DESCRIPTOR_HAS_NAME != 0 {
        let name_bytes = cursor.count(1)?;
        cursor.skip(usize::try_from(name_bytes).ok()?)?;
    }
    let mob_code = cursor.u32()?;
    for (bit, bytes) in [
        (DESCRIPTOR_BYTE_A, 1),
        (DESCRIPTOR_BYTE_B, 1),
        (DESCRIPTOR_BYTE_AND_WORD, 5),
        (DESCRIPTOR_BYTE_C, 1),
    ] {
        if descriptor & bit != 0 {
            cursor.skip(bytes)?;
        }
    }
    Some(mob_code)
}

fn read_state(cursor: &mut Cursor) -> Option<(u64, u64)> {
    let mask = cursor.u16()?;
    cursor.skip(STATE_FIXED_PREFIX_BYTES)?;
    for (bit, bytes) in [
        (STATE_VECTOR_A, VECTOR_BYTES),
        (STATE_WORD_A, 4),
        (STATE_SHORT, 2),
        (STATE_VECTOR_B, VECTOR_BYTES),
        (STATE_VECTOR_C, VECTOR_BYTES),
        (STATE_WORD_B, 4),
    ] {
        if mask & bit != 0 {
            cursor.skip(bytes)?;
        }
    }
    cursor.u8()?;
    let current_hp = cursor.varint64()?;
    let max_hp = cursor.varint64()?;
    cursor.skip(STATE_FIXED_COUNTERS_BYTES)?;

    for (bit, bytes) in [
        (STATE_BYTE_A, 1),
        (STATE_WORD_C, 4),
        (STATE_BYTE_B, 1),
        (STATE_BYTE_C, 1),
    ] {
        if mask & bit != 0 {
            cursor.skip(bytes)?;
        }
    }
    if mask & STATE_VARINT != 0 {
        cursor.varint32()?;
    }
    for (bit, bytes) in [
        (STATE_WORD_D, 4),
        (STATE_WORD_E, 4),
        (STATE_BYTE_D, 1),
        (STATE_WORD_F, 4),
        (STATE_WORD_G, 4),
    ] {
        if mask & bit != 0 {
            cursor.skip(bytes)?;
        }
    }
    Some((current_hp, max_hp))
}

fn skip_abnormals(cursor: &mut Cursor) -> Option<()> {
    let count = cursor.count(1 + 1 + ABNORMAL_FIXED_BYTES)?;
    for _ in 0..count {
        let mask = cursor.u8()?;
        if mask & ABNORMAL_UNKNOWN_BITS != 0 {
            return None;
        }
        cursor.entity_id()?;
        cursor.skip(ABNORMAL_FIXED_BYTES)?;
        if mask & ABNORMAL_HAS_CASTER != 0 {
            cursor.entity_id()?;
            cursor.u8()?;
        }
        if mask & ABNORMAL_HAS_SKILL != 0 {
            cursor.u32()?;
        }
        cursor.bool_bit()?;
        if mask & ABNORMAL_HAS_VARINT != 0 {
            cursor.varint32()?;
        }
        if mask & ABNORMAL_HAS_WORD != 0 {
            cursor.u32()?;
        }
        if mask & ABNORMAL_HAS_VECTOR != 0 {
            cursor.skip(VECTOR_BYTES)?;
        }
    }
    Some(())
}

#[cfg(test)]
pub(crate) mod frames {
    use super::*;

    pub(crate) const MOB_CODE: u32 = 2_100_166;
    pub(crate) const OWNER: u32 = 0x0001_B0B4;

    pub(crate) fn varint(value: u64) -> Vec<u8> {
        let mut value = value;
        let mut bytes = Vec::new();
        loop {
            let group = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                bytes.push(group);
                return bytes;
            }
            bytes.push(group | 0x80);
        }
    }

    pub(crate) struct Spawn {
        pub(crate) entity_id: u32,
        pub(crate) flags: u32,
        pub(crate) flags_width: FlagsWidth,
        pub(crate) descriptor: Vec<u8>,
        pub(crate) state_mask: u16,
        pub(crate) state_optionals: Vec<u8>,
        pub(crate) state_trailing: Vec<u8>,
        pub(crate) hp: (u64, u64),
        pub(crate) abnormals: Vec<u8>,
        pub(crate) after_abnormals: Vec<u8>,
    }

    impl Spawn {
        pub(crate) fn global() -> Self {
            Self {
                entity_id: 53_848,
                flags: 0,
                flags_width: FlagsWidth::U16,
                descriptor: [&[0u8][..], &MOB_CODE.to_le_bytes()].concat(),
                state_mask: 0,
                state_optionals: Vec::new(),
                state_trailing: Vec::new(),
                hp: (119_700, 119_700),
                abnormals: vec![0],
                after_abnormals: Vec::new(),
            }
        }

        pub(crate) fn kr_tw() -> Self {
            Self { flags_width: FlagsWidth::U32, ..Self::global() }
        }

        pub(crate) fn frame(&self) -> Vec<u8> {
            let mut frame = OPCODE.to_vec();
            frame.extend(varint(u64::from(self.entity_id)));
            match self.flags_width {
                FlagsWidth::U16 => frame.extend((self.flags as u16).to_le_bytes()),
                FlagsWidth::U32 => frame.extend(self.flags.to_le_bytes()),
            }
            frame.extend(&self.descriptor);
            frame.extend(self.state_mask.to_le_bytes());
            frame.extend([0xAA; STATE_FIXED_PREFIX_BYTES]);
            frame.extend(&self.state_optionals);
            frame.push(1);
            frame.extend(varint(self.hp.0));
            frame.extend(varint(self.hp.1));
            frame.extend([0u8; STATE_FIXED_COUNTERS_BYTES]);
            frame.extend(&self.state_trailing);
            frame.extend(&self.abnormals);
            frame.extend(&self.after_abnormals);
            frame
        }
    }

    pub(crate) fn with_owner(mut spawn: Spawn) -> Spawn {
        spawn.flags |= FLAG_OWNER;
        spawn.after_abnormals.extend(OWNER.to_le_bytes());
        spawn
    }
}

#[cfg(test)]
mod tests {
    use super::frames::*;
    use super::*;

    const GLOBAL: &[FlagsWidth] = &[FlagsWidth::U16];
    const KR_TW: &[FlagsWidth] = &[FlagsWidth::U32];

    #[test]
    fn a_global_spawn_reads_its_mob_code_and_hp() {
        let record = read_spawn_record(&Spawn::global().frame(), GLOBAL).unwrap();

        assert_eq!(
            record,
            SpawnRecord {
                entity_id: 53_848,
                mob_code: MOB_CODE,
                current_hp: 119_700,
                max_hp: 119_700,
                owner: SpawnOwner::None,
            }
        );
    }

    #[test]
    fn a_global_spawn_with_the_owner_flag_reads_the_owner() {
        let spawn = with_owner(Spawn::global());

        let record = read_spawn_record(&spawn.frame(), GLOBAL).unwrap();

        assert_eq!(record.owner, SpawnOwner::Entity(OWNER));
    }

    #[test]
    fn a_kr_tw_spawn_has_four_byte_flags() {
        let spawn = with_owner(Spawn::kr_tw());

        let record = read_spawn_record(&spawn.frame(), KR_TW).unwrap();

        assert_eq!(record.mob_code, MOB_CODE);
        assert_eq!(record.owner, SpawnOwner::Entity(OWNER));
    }

    #[test]
    fn a_spawn_without_the_owner_flag_has_no_owner() {
        let mut spawn = Spawn::global();
        spawn.flags = FLAG_BYTE_C;
        spawn.after_abnormals = vec![1];

        let record = read_spawn_record(&spawn.frame(), GLOBAL).unwrap();

        assert_eq!(record.owner, SpawnOwner::None);
    }

    #[test]
    fn a_zero_owner_is_no_owner() {
        let mut spawn = Spawn::global();
        spawn.flags = FLAG_OWNER;
        spawn.after_abnormals = 0u32.to_le_bytes().to_vec();

        assert_eq!(read_spawn_record(&spawn.frame(), GLOBAL).unwrap().owner, SpawnOwner::None);
    }

    #[test]
    fn a_wrong_flags_width_does_not_validate() {
        assert!(read_spawn_record(&Spawn::global().frame(), KR_TW).is_none());
        assert!(read_spawn_record(&Spawn::kr_tw().frame(), GLOBAL).is_none());
    }

    #[test]
    fn an_unknown_region_takes_whichever_width_validates() {
        let both = flags_widths(RegionId::Auto);

        let global = read_spawn_record(&with_owner(Spawn::global()).frame(), both).unwrap();
        let kr_tw = read_spawn_record(&with_owner(Spawn::kr_tw()).frame(), both).unwrap();

        assert_eq!(global.owner, SpawnOwner::Entity(OWNER));
        assert_eq!(kr_tw.owner, SpawnOwner::Entity(OWNER));
    }

    #[test]
    fn a_named_descriptor_is_read_past_its_name() {
        let mut spawn = Spawn::global();
        spawn.descriptor = [&[DESCRIPTOR_HAS_NAME, 4][..], b"Mama", &MOB_CODE.to_le_bytes()].concat();

        assert_eq!(read_spawn_record(&spawn.frame(), GLOBAL).unwrap().mob_code, MOB_CODE);
    }

    #[test]
    fn state_optionals_and_abnormals_are_read_past_to_reach_the_owner() {
        let mut spawn = with_owner(Spawn::global());
        spawn.flags |= FLAG_BYTE_A;
        spawn.state_mask = STATE_VECTOR_A | STATE_SHORT | STATE_BYTE_C;
        spawn.state_optionals = vec![0xBB; VECTOR_BYTES + 2];
        spawn.state_trailing = vec![9];
        let mut abnormals = vec![1, ABNORMAL_HAS_CASTER | ABNORMAL_HAS_VECTOR];
        abnormals.extend(varint(7));
        abnormals.extend([0xCC; ABNORMAL_FIXED_BYTES]);
        abnormals.extend(varint(u64::from(OWNER)));
        abnormals.extend([1, 0b0000_0100]);
        abnormals.extend([0xDD; VECTOR_BYTES]);
        spawn.abnormals = abnormals;
        spawn.after_abnormals = [&[9][..], &OWNER.to_le_bytes()].concat();

        let record = read_spawn_record(&spawn.frame(), GLOBAL).unwrap();

        assert_eq!(record.owner, SpawnOwner::Entity(OWNER));
    }

    #[test]
    fn hp_above_u32_is_kept() {
        let mut spawn = Spawn::global();
        spawn.hp = (5_000_000_000, 6_000_000_000);

        let record = read_spawn_record(&spawn.frame(), GLOBAL).unwrap();

        assert_eq!((record.current_hp, record.max_hp), (5_000_000_000, 6_000_000_000));
    }

    #[test]
    fn a_dead_mob_spawns_with_zero_hp() {
        let mut spawn = Spawn::global();
        spawn.hp = (0, 119_700);

        assert_eq!(read_spawn_record(&spawn.frame(), GLOBAL).unwrap().current_hp, 0);
    }

    #[test]
    fn implausible_spawns_are_rejected() {
        let mut above_max = Spawn::global();
        above_max.hp = (10, 5);
        let mut unbounded = Spawn::global();
        unbounded.hp = (1, 1 << 40);
        let mut no_mob = Spawn::global();
        no_mob.descriptor = vec![0, 0, 0, 0, 0];
        let mut huge_code = Spawn::global();
        huge_code.descriptor = [&[0u8][..], &50_000_000u32.to_le_bytes()].concat();
        for spawn in [above_max, unbounded, no_mob, huge_code] {
            assert!(read_spawn_record(&spawn.frame(), GLOBAL).is_none());
        }
    }

    #[test]
    fn a_spawn_whose_tail_is_cut_keeps_its_mob_code_and_hp_with_an_unknown_owner() {
        let frame = with_owner(Spawn::global()).frame();

        let record = read_spawn_record(&frame[..frame.len() - 1], GLOBAL).unwrap();

        assert_eq!(
            (record.mob_code, record.current_hp, record.max_hp, record.owner),
            (MOB_CODE, 119_700, 119_700, SpawnOwner::Unknown)
        );
    }

    #[test]
    fn a_spawn_whose_abnormal_list_cannot_be_walked_keeps_its_mob_code_and_hp() {
        let mut spawn = with_owner(Spawn::global());
        spawn.abnormals = vec![3, ABNORMAL_UNKNOWN_BITS];

        let record = read_spawn_record(&spawn.frame(), GLOBAL).unwrap();

        assert_eq!((record.mob_code, record.owner), (MOB_CODE, SpawnOwner::Unknown));
    }

    #[test]
    fn an_owner_equal_to_the_spawned_entity_is_not_trusted() {
        let mut spawn = Spawn::global();
        spawn.flags = FLAG_OWNER;
        spawn.after_abnormals = spawn.entity_id.to_le_bytes().to_vec();

        let record = read_spawn_record(&spawn.frame(), GLOBAL).unwrap();

        assert_eq!((record.mob_code, record.owner), (MOB_CODE, SpawnOwner::Unknown));
    }

    #[test]
    fn a_width_that_reads_the_whole_frame_wins_over_one_that_reads_its_head() {
        let frame = with_owner(Spawn::global()).frame();

        let record = read_spawn_record(&frame, &[FlagsWidth::U32, FlagsWidth::U16]).unwrap();

        assert_eq!(record.owner, SpawnOwner::Entity(OWNER));
    }

    #[test]
    fn a_truncated_head_is_rejected() {
        let frame = Spawn::global().frame();

        assert!(read_spawn_record(&frame[..frame.len() - 55], GLOBAL).is_none());
    }

    #[test]
    fn flags_widths_follow_the_region() {
        assert_eq!(flags_widths(RegionId::Global), GLOBAL);
        assert_eq!(flags_widths(RegionId::Kr), KR_TW);
        assert_eq!(flags_widths(RegionId::Tw), KR_TW);
    }
}
