use crate::capture::parser::damage_record::{
    AttackPosition, DamageRecord, KIND_CRITICAL, KIND_REFLECTED, SPECIAL_DOUBLE, SPECIAL_PARRY,
    SPECIAL_PERFECT, read_damage_record,
};
use crate::capture::parser::periodic_record::{
    read_periodic_record, PeriodicRecord, MODE_DAMAGE_TICK, MODE_HEAL_TICK,
};
use crate::capture::parser::utils::bytes_to_hex;
use crate::capture::processor::StreamProcessor;
use crate::models::packet::{ParsedDamagePacket, SpecialDamage};

/// Class skills have 8-digit codes; basic attacks, monster and system effects
/// have shorter ones.
const FIRST_CLASS_SKILL_CODE: u32 = 10_000_000;

/// Thamon's absorption buffs (the "roar" skills) recover the boss's absorbed
/// damage in periodic ticks the game credits to players; their effects are
/// 7-digit skills followed by two digits.
const THAMON_ABSORPTION_SKILLS: [u32; 2] = [1_604_660, 1_604_670];

impl StreamProcessor {
    pub(crate) fn parse_damage_packet(
        &mut self,
        packet: &[u8],
        _is_compressed_bundle: bool,
    ) -> bool {
        let Some(record) = read_damage_record(packet) else {
            return false;
        };
        if let Some(amount) = monster_hit_amount(&record) {
            self.data_storage.record_incoming_hit(
                record.target_id,
                record.actor_id,
                record.raw_skill_code,
                amount,
            );
        }
        let Some(parsed) = damage_event(&record) else {
            return true;
        };

        self.logger.debug(format!(
            "[{}] damage target={} actor={} skill={} ori_code={} damage={} multi_hit_count={} multi_hit_damage={} specials={:?} packet_len={} packet_hex={}",
            self.port,
            parsed.target_id,
            parsed.actor_id,
            parsed.skill_code,
            parsed.ori_skill_code,
            parsed.damage,
            parsed.multi_hit_count,
            parsed.multi_hit_damage,
            parsed.specials,
            packet.len(),
            bytes_to_hex(packet)
        ));
        self.data_storage.append_damage(parsed);
        true
    }

    pub(crate) fn parse_dot_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Some(tick) = read_periodic_record(packet) else {
            return false;
        };
        self.record_periodic_effect(&tick);

        let Some(parsed) = dot_damage_event(&tick) else {
            return true;
        };
        self.logger.debug(format!(
            "[{}] dot target={} actor={} skill={} ori_code={}, damage={}",
            self.port, parsed.target_id, parsed.actor_id, parsed.skill_code, parsed.ori_skill_code, parsed.damage
        ));
        self.data_storage.append_damage(parsed);
        true
    }

    /// Ticks of a class skill that heal are heals; damage ticks from a monster
    /// are damage the target took. A heal tick of a monster or system effect (a
    /// potion, natural recovery) is not a player's healing.
    fn record_periodic_effect(&self, tick: &PeriodicRecord) {
        let Some(amount) = tick.tick_amount else {
            return;
        };
        let skill_code = periodic_skill_code(tick);
        match tick.mode {
            MODE_HEAL_TICK if skill_code >= FIRST_CLASS_SKILL_CODE && !is_thamon_absorption_recovery(skill_code) => {
                self.data_storage.record_heal_tick(
                    tick.actor_id,
                    tick.target_id,
                    displayed_skill_code(skill_code),
                    amount,
                )
            }
            MODE_DAMAGE_TICK if damage_tick_source(tick) == Some(DamageTickSource::Monster) => self
                .data_storage
                .record_incoming_hit(tick.target_id, tick.actor_id, skill_code, amount),
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DamageTickSource {
    Player,
    Monster,
}

fn periodic_skill_code(tick: &PeriodicRecord) -> u32 {
    original_skill_code(tick.skill_code.unwrap_or(tick.effect_id))
}

/// Who a damage tick belongs to, from its skill; `None` for a Thamon
/// absorption recovery (named by its skill or by its effect) and for a tick
/// without a skill, which are neither damage dealt nor damage taken.
fn damage_tick_source(tick: &PeriodicRecord) -> Option<DamageTickSource> {
    let skill_code = periodic_skill_code(tick);
    if is_thamon_absorption_recovery(skill_code) || is_thamon_absorption_recovery(tick.effect_id) {
        return None;
    }
    if is_player_skill_code(skill_code) {
        Some(DamageTickSource::Player)
    } else if is_monster_skill_code(skill_code) {
        Some(DamageTickSource::Monster)
    } else {
        None
    }
}

/// Reflected hits, records without a damage amount (evades, effects, heals) and
/// hits from monster skills are not damage dealt by a player.
fn damage_event(record: &DamageRecord) -> Option<ParsedDamagePacket> {
    if record.kind == KIND_REFLECTED {
        return None;
    }
    let damage = record.amount?;

    let ori_skill_code = original_skill_code(record.raw_skill_code);
    if !is_player_skill_code(ori_skill_code) || is_thamon_absorption_recovery(ori_skill_code) {
        return None;
    }

    let is_crit = record.kind == KIND_CRITICAL;
    let mut specials = Vec::new();
    if record.has_special(SPECIAL_PARRY) {
        specials.push(SpecialDamage::Parry);
    }
    if record.has_special(SPECIAL_PERFECT) {
        specials.push(SpecialDamage::Perfect);
    }
    if record.has_special(SPECIAL_DOUBLE) {
        specials.push(SpecialDamage::Double);
    }
    match record.position {
        Some(AttackPosition::Back) => specials.push(SpecialDamage::Back),
        Some(AttackPosition::Front) => specials.push(SpecialDamage::Front),
        None => {}
    }
    if is_crit {
        specials.push(SpecialDamage::Critical);
    }

    Some(ParsedDamagePacket {
        target_id: record.target_id,
        actor_id: record.actor_id,
        skill_code: normalize_skill_id(ori_skill_code),
        ori_skill_code,
        damage,
        is_dot: false,
        is_crit,
        multi_hit_damage: record.multi_hit.damage,
        multi_hit_count: record.multi_hit.count,
        specials: specials
            .iter()
            .map(|special| special.as_str().to_string())
            .collect(),
        drain_amount: record.drain_amount.unwrap_or(0),
        regeneration_amount: record.regeneration_amount.unwrap_or(0),
    })
}

/// A damage tick of a player's skill, stored under its effect id. Like a hit,
/// it is no damage dealt when it belongs to a monster or is a Thamon
/// absorption recovery.
fn dot_damage_event(tick: &PeriodicRecord) -> Option<ParsedDamagePacket> {
    if tick.mode != MODE_DAMAGE_TICK
        || tick.actor_id < 1
        || damage_tick_source(tick) != Some(DamageTickSource::Player)
    {
        return None;
    }
    let damage = tick.tick_amount?;

    Some(ParsedDamagePacket {
        target_id: tick.target_id,
        actor_id: tick.actor_id,
        skill_code: normalize_skill_id(tick.effect_id),
        ori_skill_code: tick.effect_id,
        damage,
        is_dot: true,
        is_crit: false,
        multi_hit_damage: 0,
        multi_hit_count: 0,
        specials: Vec::new(),
        drain_amount: 0,
        regeneration_amount: 0,
    })
}

fn is_player_skill_code(code: u32) -> bool {
    (1..=299_999_999).contains(&code) && !(1_000_000..=9_999_999).contains(&code)
}

fn is_thamon_absorption_recovery(code: u32) -> bool {
    (100_000_000..=999_999_999).contains(&code) && THAMON_ABSORPTION_SKILLS.contains(&(code / 100))
}

/// The amount of a hit that a monster's skill landed, if it is one.
fn monster_hit_amount(record: &DamageRecord) -> Option<u64> {
    if record.kind == KIND_REFLECTED || !is_monster_skill_code(original_skill_code(record.raw_skill_code)) {
        return None;
    }
    record.amount
}

fn original_skill_code(raw: u32) -> u32 {
    if (3_000_000..=3_099_999).contains(&raw) {
        raw * 10 + 1
    } else {
        raw
    }
}

/// Not a player's skill, and not no skill at all.
fn is_monster_skill_code(code: u32) -> bool {
    code != 0 && !is_player_skill_code(code)
}

/// The code a skill's damage is kept under: an 8-digit skill without its
/// variant digits, any other code as it is.
pub(crate) fn displayed_skill_code(raw: u32) -> u32 {
    if (10_000_000..=99_999_999).contains(&raw) {
        normalize_skill_id(raw)
    } else {
        raw
    }
}

/// Codes that name one skill each, not a family of variants: the Elementalist
/// spirits' specials and 30_000_000..=30_999_999.
const EXACT_SKILL_CODES: [std::ops::RangeInclusive<u32>; 2] = [16_001_000..=16_001_999, 30_000_000..=30_999_999];

fn normalize_skill_id(raw: u32) -> u32 {
    if EXACT_SKILL_CODES.iter().any(|range| range.contains(&raw)) {
        raw
    } else {
        raw - (raw % 10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::parser::damage_record::{hit_frame, hit_frame_with_skill};

    const STATE_AND_EFFECT: [u8; 9] = [0x57, 0xFA, 0x3D, 0x6C, 0x01, 0x00, 0x00, 0x00, 0x05];

    fn event(frame: &[u8]) -> Option<ParsedDamagePacket> {
        damage_event(&read_damage_record(frame).expect("frame is readable"))
    }

    fn amount_tail(amount: &[u8]) -> Vec<u8> {
        let mut tail = STATE_AND_EFFECT.to_vec();
        tail.extend_from_slice(amount);
        tail.extend_from_slice(&[0x01, 0x00]);
        tail
    }

    #[test]
    fn crit_hit_with_extra_word_before_the_kind_is_a_crit_of_the_right_amount() {
        let mut after_action = vec![0xDE, 0xAD, 0xBE, 0xEF, 3];
        after_action.extend(amount_tail(&[0xE8, 0x07]));

        let parsed = event(&hit_frame(0x0005, &after_action)).unwrap();

        assert!(parsed.is_crit);
        assert_eq!(parsed.specials, ["CRITICAL"]);
        assert_eq!(parsed.damage, 1000);
        assert_eq!(parsed.actor_id, 18);
        assert_eq!(parsed.target_id, 16022);
    }

    #[test]
    fn reflected_damage_is_not_the_actors_damage() {
        let mut after_action = vec![KIND_REFLECTED];
        after_action.extend(amount_tail(&[0x64]));

        assert!(event(&hit_frame(0x0004, &after_action)).is_none());
    }

    #[test]
    fn record_without_amount_is_no_damage_event() {
        let mut after_action = vec![1];
        after_action.extend(STATE_AND_EFFECT);
        after_action.extend_from_slice(&[0x01, 0x00]);

        assert!(event(&hit_frame(0x0000, &after_action)).is_none());
    }

    #[test]
    fn flags_with_the_low_byte_high_bit_set_read_as_one_word() {
        let mut after_action = vec![2];
        after_action.extend(STATE_AND_EFFECT);
        after_action.extend_from_slice(&[0x2C, 0x07, 0x01, 0x00]);

        let parsed = event(&hit_frame(0x0084, &after_action)).unwrap();

        assert_eq!(parsed.damage, 44);
        assert!(!parsed.is_crit);
    }

    #[test]
    fn back_attack_after_a_multi_byte_heal_is_still_a_back_attack() {
        let mut after_action = vec![2, 0b0100_1110, 0x80, 0x80, 0x01, 1];
        after_action.extend(amount_tail(&[0x64]));

        let parsed = event(&hit_frame(0x0006, &after_action)).unwrap();

        assert_eq!(parsed.specials, ["PARRY", "PERFECT", "DOUBLE", "BACK"]);
    }

    #[test]
    fn multi_hit_list_reports_its_count_and_total() {
        let mut after_action = vec![2];
        after_action.extend_from_slice(&STATE_AND_EFFECT);
        after_action.extend_from_slice(&[0xD8, 0x04, 3, 0xC8, 0x01, 0xC8, 0x01, 0xC8, 0x01, 0x01, 0x00]);

        let parsed = event(&hit_frame(0x0024, &after_action)).unwrap();

        assert_eq!(parsed.damage, 600);
        assert_eq!(parsed.multi_hit_count, 3);
        assert_eq!(parsed.multi_hit_damage, 600);
    }

    #[test]
    fn skill_codes_are_normalised_to_the_base_skill() {
        let mut after_action = vec![2];
        after_action.extend(amount_tail(&[0x64]));

        let parsed = event(&hit_frame(0x0004, &after_action)).unwrap();

        assert_eq!(parsed.ori_skill_code, 11_020_151);
        assert_eq!(parsed.skill_code, 11_020_000);
    }

    #[test]
    fn spirit_specials_keep_their_own_skill_code() {
        for code in [16_001_104, 16_001_108, 16_001_116, 16_001_117] {
            let mut after_action = vec![2];
            after_action.extend(amount_tail(&[0x64]));

            let parsed = event(&hit_frame_with_skill(0x0004, code, &after_action)).unwrap();

            assert_eq!(parsed.skill_code, code);
            assert_eq!(displayed_skill_code(code), code);
        }
    }

    #[test]
    fn monster_skill_hit_is_not_counted() {
        let mut after_action = vec![2];
        after_action.extend(amount_tail(&[0x64]));

        assert!(event(&hit_frame_with_skill(0x0004, 1_223_380, &after_action)).is_none());
    }
}
