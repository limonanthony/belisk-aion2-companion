//! What `pets.json` stores: every server's pet collection, the pets pinned on
//! each server, the server being played, and the Pets window's Show and Order
//! choice. The collection and the level rule are the core's
//! (`meter_core::pets`), shared with packet-explorer.

use std::collections::BTreeMap;

use meter_core::pets::PetCollection;
use serde::{Deserialize, Serialize};

use super::list::ListChoice;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PetData {
    pub current_server: Option<u32>,
    pub servers: BTreeMap<u32, PetCollection>,
    /// The pets pinned on each server, in the order they were pinned. A
    /// separate map, not part of the collection, so a file written before pins
    /// existed loads with none and keeps its version.
    pub pinned: BTreeMap<u32, Vec<u32>>,
    /// The window's Show and Order choice, the same on every server.
    pub list: ListChoice,
}

impl PetData {
    pub fn server_mut(&mut self, server_id: u32) -> &mut PetCollection {
        self.servers.entry(server_id).or_default()
    }

    pub fn pinned(&self, server_id: u32) -> &[u32] {
        self.pinned.get(&server_id).map_or(&[], Vec::as_slice)
    }

    pub fn is_pinned(&self, server_id: u32, pet_id: u32) -> bool {
        self.pinned(server_id).contains(&pet_id)
    }

    /// Returns whether anything changed: a pet already pinned keeps its place.
    pub fn pin(&mut self, server_id: u32, pet_id: u32) -> bool {
        if self.is_pinned(server_id, pet_id) {
            return false;
        }
        self.pinned.entry(server_id).or_default().push(pet_id);
        true
    }

    /// Returns whether anything changed.
    pub fn unpin(&mut self, server_id: u32, pet_id: u32) -> bool {
        let Some(pins) = self.pinned.get_mut(&server_id) else {
            return false;
        };
        let before = pins.len();
        pins.retain(|&pinned| pinned != pet_id);
        let changed = pins.len() != before;
        if pins.is_empty() {
            self.pinned.remove(&server_id);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_keep_the_order_they_were_made_in_and_pinning_twice_changes_nothing() {
        let mut data = PetData::default();

        assert!(data.pin(1302, 1214));
        assert!(data.pin(1302, 1086));
        assert!(!data.pin(1302, 1214));

        assert_eq!(data.pinned(1302), [1214, 1086]);
        assert!(data.is_pinned(1302, 1086) && !data.is_pinned(1302, 1119));
    }

    #[test]
    fn pins_belong_to_their_server() {
        let mut data = PetData::default();
        data.pin(1302, 1214);

        assert!(data.pinned(1001).is_empty());
        assert!(!data.is_pinned(1001, 1214));
        assert!(data.pin(1001, 1214), "the same pet can be pinned on another server");
    }

    #[test]
    fn unpinning_removes_the_pet_and_reports_whether_it_was_pinned() {
        let mut data = PetData::default();
        data.pin(1302, 1214);
        data.pin(1302, 1086);
        data.pin(1302, 1119);

        assert!(data.unpin(1302, 1086));
        assert!(!data.unpin(1302, 1086));
        assert!(!data.unpin(1001, 1214), "nothing is pinned on that server");

        assert_eq!(data.pinned(1302), [1214, 1119]);
    }

    #[test]
    fn a_server_with_no_pins_left_is_not_kept_in_the_file() {
        let mut data = PetData::default();
        data.pin(1302, 1214);

        data.unpin(1302, 1214);

        assert_eq!(data, PetData::default());
    }
}
