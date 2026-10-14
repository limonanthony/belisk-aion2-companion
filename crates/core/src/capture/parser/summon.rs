use crate::capture::parser::char_die_record::read_char_die_record;
use crate::capture::parser::damage::displayed_skill_code;
use crate::capture::parser::death_record::read_death_record;
use crate::capture::parser::hp_record::read_hp_record;
use crate::capture::parser::spawn_record::{flags_widths, read_spawn_record, SpawnOwner};
use crate::capture::processor::StreamProcessor;
use crate::mobs::catalog::exceeds_possible_boss_hp;

fn saturating_u32(hp: u64) -> u32 {
    u32::try_from(hp).unwrap_or(u32::MAX)
}

impl StreamProcessor {
    pub(crate) fn parse_summon_packet(
        &mut self,
        packet: &[u8],
        _is_compressed_bundle: bool,
    ) -> bool {
        let Some(spawn) = read_spawn_record(packet, flags_widths(crate::region::resolved())) else {
            return false;
        };

        self.data_storage.append_mob(spawn.entity_id, spawn.mob_code);
        let is_boss = self.data_storage.is_boss(spawn.mob_code);
        if is_boss {
            let boss_name = self
                .data_storage
                .mob_name(spawn.mob_code)
                .unwrap_or_else(|| "Unknown Boss".to_string());
            self.logger.info(format!(
                "[{}] 4136 spawn target={} mob_code={} name={} hp={}/{}",
                self.port, spawn.entity_id, spawn.mob_code, boss_name, spawn.current_hp, spawn.max_hp
            ));
        } else {
            self.logger.debug(|| {
                format!(
                    "[{}] 4136 spawn target={} mob_code={}",
                    self.port, spawn.entity_id, spawn.mob_code
                )
            });
        }

        if is_boss || !self.config.read().unwrap().boss_only {
            self.data_storage
                .set_mob_hp(spawn.entity_id, Some(spawn.current_hp), Some(spawn.max_hp));
        }

        match spawn.owner {
            SpawnOwner::Unknown => {}
            SpawnOwner::Entity(owner_id) => {
                self.data_storage.append_summon(owner_id, spawn.entity_id);
                self.logger.debug(|| {
                    format!(
                        "[{}] summon owner={} owner_name={} summon={}",
                        self.port,
                        owner_id,
                        self.data_storage.actor_name(owner_id).unwrap_or_else(|| "Unknown".to_string()),
                        spawn.entity_id
                    )
                });
            }
            SpawnOwner::None => self.data_storage.forget_summon_owner(spawn.entity_id),
        }
        true
    }

    pub(crate) fn parse_remain_hp_packet(
        &mut self,
        packet: &[u8],
        _is_compressed_bundle: bool,
    ) -> bool {
        let Some(record) = read_hp_record(packet) else {
            return false;
        };
        let target_id = record.entity_id;
        if target_id < 100 {
            return false;
        }

        // The local player's own record is the one that carries only small
        // stats. Its value is thousands; above the boss threshold the target
        // is a big mob.
        if self.own_actor.is_unsettled() {
            if let Some(value) = record.own_signature_value() {
                if !exceeds_possible_boss_hp(u64::from(value)) {
                    if self.data_storage.bind_own_actor(target_id) {
                        self.logger.info(format!(
                            "[{}] main actor bound to actor={} by its remain hp signature",
                            self.port, target_id
                        ));
                    }
                    if self.data_storage.main_actor_id() == Some(target_id) {
                        self.own_actor.settle();
                    }
                }
            }
        }

        if let Some(current_hp) = record.current_hp {
            let show_possible_boss = self.config.read().unwrap().show_possible_boss;
            if show_possible_boss && exceeds_possible_boss_hp(current_hp) {
                if let Some(mob_code) = self.data_storage.get_mob_code(target_id) {
                    self.data_storage.add_possible_boss(mob_code);
                }
            }
        }

        let is_target_player = self.data_storage.is_player(target_id);

        if is_target_player {
            if !self.config.read().unwrap().pvp_mode_on {
                return true;
            }

            match record.current_hp {
                Some(0) => {
                    let (newly_dead, killer) = self.data_storage.mark_player_dead(target_id);
                    if newly_dead {
                        self.logger.info(format!(
                            "[{}] player dead actor={} killer={}",
                            self.port,
                            target_id,
                            killer.as_deref().unwrap_or("unknown")
                        ));
                    }
                }
                Some(_) => self.data_storage.mark_player_alive(target_id),
                None => {}
            }

            if self.data_storage.main_actor_id() == Some(target_id) {
                return true;
            }

            let current_hp = record.current_hp.map(saturating_u32);
            let max_hp = record.max_hp.map(saturating_u32);
            if current_hp.is_none() && max_hp.is_none() {
                return true;
            }
            self.data_storage.set_player_hp(target_id, current_hp, max_hp);
            self.logger.debug(|| {
                format!(
                    "[{}] player remain hp actor={} name={} current_hp={:?} max_hp={:?}",
                    self.port,
                    target_id,
                    self.data_storage.actor_name(target_id).unwrap_or_else(|| "Unknown".to_string()),
                    current_hp,
                    max_hp
                )
            });
            return true;
        }

        let config = self.config.read().unwrap();
        if config.boss_only {
            let mob_code = self.data_storage.get_mob_code(target_id);
            let is_boss = mob_code.is_some_and(|code| self.data_storage.is_boss(code));
            if !is_boss {
                return true;
            }
        }
        drop(config);

        self.data_storage
            .set_mob_hp(target_id, record.current_hp, record.max_hp);
        true
    }

    pub(crate) fn parse_death_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Some(record) = read_death_record(packet) else {
            return false;
        };
        if record.died() {
            self.data_storage.mark_mob_dead(record.entity_id);
        } else if record.removed() {
            self.data_storage.mark_mob_removed(record.entity_id);
        }
        true
    }

    pub(crate) fn parse_char_die_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Some(record) = read_char_die_record(packet) else {
            return false;
        };
        self.data_storage.record_player_death_at(
            record.victim_id,
            displayed_skill_code(record.skill_code),
            record.killer_id,
            &record.killer_name,
            record.killer_mob_code,
            self.captured_at,
        );
        true
    }
}
