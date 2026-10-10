use crate::capture::parser::loot_record::{decode_loot_pickup, decode_loot_spawns};
use crate::capture::parser::pet_record::{decode_counter_gains, decode_snapshot, decode_soul_gains, decode_unlock};
use crate::capture::processor::StreamProcessor;
use crate::pets::PetContent;

impl StreamProcessor {
    pub(crate) fn parse_pet_snapshot_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(snapshot) = decode_snapshot(packet) else {
            return false;
        };
        self.data_storage.pets_snapshot(PetContent::Snapshot(snapshot), self.captured_at);
        true
    }

    pub(crate) fn parse_pet_soul_gains_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(gains) = decode_soul_gains(packet) else {
            return false;
        };
        let source = self.soul_sources.soul_gain_cause(self.captured_at).source;
        self.data_storage.pets_changed(PetContent::SoulGains { gains, source }, self.captured_at);
        true
    }

    pub(crate) fn parse_pet_unlock_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(unlock) = decode_unlock(packet) else {
            return false;
        };
        self.data_storage.pets_changed(PetContent::Unlock(unlock), self.captured_at);
        true
    }

    pub(crate) fn parse_pet_counter_gains_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(gains) = decode_counter_gains(packet) else {
            return false;
        };
        let looted_mobs = self.soul_sources.counter_gain_cause(self.captured_at).looted_mobs;
        self.data_storage.pets_changed(PetContent::CounterGains { gains, looted_mobs }, self.captured_at);
        true
    }

    pub(crate) fn parse_loot_spawn_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(spawns) = decode_loot_spawns(packet) else {
            return false;
        };
        for spawn in spawns {
            self.soul_sources.loot_spawned(self.captured_at, spawn);
        }
        true
    }

    pub(crate) fn parse_loot_pickup_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(pickup) = decode_loot_pickup(packet) else {
            return false;
        };
        let storage = &self.data_storage;
        self.soul_sources.picked_up(self.captured_at, pickup, |entity| storage.get_mob_code(entity));
        true
    }
}
