//! What the Pets window lists for a tab, a search and the Show and Order
//! menu: the pinned pets first in the order they were pinned, then the rest
//! in the game's order or closest to their next level, and the tab's family
//! summary. The souls a level needs come from the core's rule.

use std::cmp::Ordering;

use meter_core::pets::{PetCollection, PetState, MAX_LEVEL, SOULS_PER_LEVEL};
use serde::{Deserialize, Serialize};

use crate::game_data::{self, PetFamily};

/// The Show part of the window's menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Show {
    #[default]
    All,
    NotOwned,
    NotMaxed,
}

/// The Order part of the window's menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    #[default]
    Game,
    ClosestToNextLevel,
}

/// The window's menu choice, kept in `pets.json` from one opening to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ListChoice {
    pub show: Show,
    pub order: Order,
}

impl ListChoice {
    pub fn filters(self) -> bool {
        self != Self::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    All,
    Family(PetFamily),
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::All,
        Tab::Family(PetFamily::Cogni),
        Tab::Family(PetFamily::Fera),
        Tab::Family(PetFamily::Natura),
        Tab::Family(PetFamily::Varian),
        Tab::Family(PetFamily::Special),
    ];

    fn holds(self, family: Option<PetFamily>) -> bool {
        match self {
            Tab::All => true,
            Tab::Family(tab) => family == Some(tab),
        }
    }
}

/// One pet as the window draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub pet_id: u32,
    pub state: PetState,
    /// None for a pet the bundled list does not know.
    pub family: Option<PetFamily>,
}

impl Row {
    pub fn is_maxed(&self) -> bool {
        self.state.level >= MAX_LEVEL
    }

    /// Level 0 and not a soul yet.
    pub fn never_obtained(&self) -> bool {
        self.state == PetState::default()
    }

    /// Souls toward the next level and the souls that level needs; a pet at
    /// the cap reads as a full last level.
    pub fn souls_of_needed(&self) -> (u32, u32) {
        souls_of_needed(self.state)
    }

    /// How much of the next level is done, 0 to 1; 1 at the cap.
    pub fn progress(&self) -> f32 {
        let (souls, needed) = self.souls_of_needed();
        souls as f32 / needed as f32
    }

    fn souls_left(&self) -> Option<u32> {
        let (souls, needed) = self.souls_of_needed();
        (!self.is_maxed()).then(|| needed.saturating_sub(souls))
    }
}

/// [`Row::souls_of_needed`] for any pet state.
pub fn souls_of_needed(state: PetState) -> (u32, u32) {
    let last = SOULS_PER_LEVEL[SOULS_PER_LEVEL.len() - 1];
    match SOULS_PER_LEVEL.get(state.level as usize) {
        Some(&needed) => (state.souls, needed),
        None => (last, last),
    }
}

/// The figures above a tab's list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub pets: usize,
    pub owned: usize,
    pub maxed: usize,
    pub levels: u32,
    pub levels_possible: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listing {
    pub pinned: Vec<Row>,
    pub rest: Vec<Row>,
    /// The whole tab, before the search and the menu.
    pub summary: Summary,
}

impl Listing {
    pub fn shown(&self) -> usize {
        self.pinned.len() + self.rest.len()
    }
}

/// Every pet the window knows: the bundled list in the game's order, with any
/// pet of the collection the list does not know at its place by id.
pub fn every_pet(collection: &PetCollection) -> Vec<Row> {
    let known = game_data::pet_ids();
    let mut ids: Vec<u32> = known.to_vec();
    ids.extend(collection.pets.keys().copied().filter(|pet_id| known.binary_search(pet_id).is_err()));
    ids.sort_unstable();
    ids.into_iter().map(|pet_id| Row { pet_id, state: collection.pet(pet_id), family: game_data::pet_family(pet_id) }).collect()
}

/// What the window shows: `query` narrows the tab to the pets whose English
/// or French name contains it (the "Pet <id>" fallback of an unknown pet is
/// `fallback_name`'s), `choice` filters and orders, pinned pets come first.
pub fn listing(
    collection: &PetCollection,
    pinned: &[u32],
    tab: Tab,
    query: &str,
    choice: ListChoice,
    fallback_name: impl Fn(u32) -> String,
) -> Listing {
    let in_tab: Vec<Row> = every_pet(collection).into_iter().filter(|row| tab.holds(row.family)).collect();
    let summary = summary(&in_tab);
    let query = query.trim().to_lowercase();
    let mut kept: Vec<Row> = in_tab
        .into_iter()
        .filter(|row| query.is_empty() || matches_name(row, &query, &fallback_name))
        .filter(|row| match choice.show {
            Show::All => true,
            Show::NotOwned => row.state.level == 0,
            Show::NotMaxed => !row.is_maxed(),
        })
        .collect();
    if choice.order == Order::ClosestToNextLevel {
        kept.sort_by(closest_first);
    }
    let pinned_rows = pinned.iter().filter_map(|pet_id| kept.iter().find(|row| row.pet_id == *pet_id).copied()).collect();
    kept.retain(|row| !pinned.contains(&row.pet_id));
    Listing { pinned: pinned_rows, rest: kept, summary }
}

fn matches_name(row: &Row, query: &str, fallback_name: &impl Fn(u32) -> String) -> bool {
    match row.family {
        Some(_) => game_data::pet_name_contains(row.pet_id, query),
        None => fallback_name(row.pet_id).to_lowercase().contains(query),
    }
}

/// The fewest souls left first, pets at the cap last; then the larger share
/// of the level done; then the game's order.
fn closest_first(a: &Row, b: &Row) -> Ordering {
    match (a.souls_left(), b.souls_left()) {
        (Some(left_a), Some(left_b)) => left_a
            .cmp(&left_b)
            .then_with(|| b.progress().total_cmp(&a.progress()))
            .then(a.pet_id.cmp(&b.pet_id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.pet_id.cmp(&b.pet_id),
    }
}

fn summary(rows: &[Row]) -> Summary {
    Summary {
        pets: rows.len(),
        owned: rows.iter().filter(|row| row.state.level >= 1).count(),
        maxed: rows.iter().filter(|row| row.is_maxed()).count(),
        levels: rows.iter().map(|row| row.state.level).sum(),
        levels_possible: rows.len() as u32 * MAX_LEVEL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOSSA: u32 = 1004;
    const SPARKIE: u32 = 1006;
    const KALGOLEM: u32 = 1084;
    const FUNGEN: u32 = 1086;
    const BLACK_SPIRIT: u32 = 1214;
    const UNKNOWN: u32 = 1300;

    fn collection(pets: &[(u32, u32, u32)]) -> PetCollection {
        let mut collection = PetCollection { snapshot_at: Some(100.0), ..PetCollection::default() };
        for &(pet_id, level, souls) in pets {
            collection.pets.insert(pet_id, PetState { level, souls });
        }
        collection
    }

    fn fallback(pet_id: u32) -> String {
        format!("Pet {pet_id}")
    }

    fn ids(rows: &[Row]) -> Vec<u32> {
        rows.iter().map(|row| row.pet_id).collect()
    }

    fn list(collection: &PetCollection, pinned: &[u32], tab: Tab, query: &str, choice: ListChoice) -> Listing {
        listing(collection, pinned, tab, query, choice, fallback)
    }

    #[test]
    fn every_pet_of_the_list_shows_in_game_order_with_an_unknown_one_at_its_place() {
        let pets = collection(&[(UNKNOWN, 1, 2), (FOSSA, 2, 27)]);

        let all = list(&pets, &[], Tab::All, "", ListChoice::default());

        assert_eq!(all.rest.len(), game_data::pet_ids().len() + 1);
        assert!(all.rest.windows(2).all(|pair| pair[0].pet_id < pair[1].pet_id));
        assert_eq!(all.rest.last().map(|row| (row.pet_id, row.family)), Some((UNKNOWN, None)));
        let fossa = all.rest.iter().find(|row| row.pet_id == FOSSA).unwrap();
        assert_eq!(fossa.state, PetState { level: 2, souls: 27 });
        assert!(all.pinned.is_empty());
    }

    #[test]
    fn a_family_tab_keeps_its_family_and_its_summary_counts_the_whole_tab() {
        let pets = collection(&[(FOSSA, 3, 0), (SPARKIE, 1, 4), (KALGOLEM, 3, 0)]);

        let fera = list(&pets, &[], Tab::Family(PetFamily::Fera), "spark", ListChoice::default());

        assert!(ids(&fera.rest).contains(&SPARKIE) && fera.rest.iter().all(|row| row.family == Some(PetFamily::Fera)));
        let fera_count = game_data::pet_ids().iter().filter(|id| game_data::pet_family(**id) == Some(PetFamily::Fera)).count();
        assert_eq!(
            fera.summary,
            Summary { pets: fera_count, owned: 2, maxed: 1, levels: 4, levels_possible: fera_count as u32 * 3 },
            "the summary is the tab's, before the search"
        );
    }

    #[test]
    fn the_search_reads_english_and_french_names_and_the_fallback_of_an_unknown_pet() {
        let pets = collection(&[(UNKNOWN, 1, 0)]);

        assert_eq!(ids(&list(&pets, &[], Tab::All, "  Black SPIRIT ", ListChoice::default()).rest), vec![BLACK_SPIRIT]);
        assert_eq!(ids(&list(&pets, &[], Tab::All, "ténèbres", ListChoice::default()).rest), vec![BLACK_SPIRIT]);
        assert_eq!(ids(&list(&pets, &[], Tab::All, "pet 1300", ListChoice::default()).rest), vec![UNKNOWN]);
        assert_eq!(list(&pets, &[], Tab::Family(PetFamily::Fera), "ténèbres", ListChoice::default()).shown(), 0);
    }

    #[test]
    fn pinned_pets_come_first_in_pin_order_when_the_tab_search_and_menu_keep_them() {
        let pets = collection(&[(FOSSA, 2, 27), (KALGOLEM, 3, 0), (FUNGEN, 1, 3)]);
        let pins = [KALGOLEM, FOSSA, FUNGEN];

        let all = list(&pets, &pins, Tab::All, "", ListChoice::default());
        assert_eq!(ids(&all.pinned), vec![KALGOLEM, FOSSA, FUNGEN]);
        assert!(!all.rest.iter().any(|row| pins.contains(&row.pet_id)));

        let fera = list(&pets, &pins, Tab::Family(PetFamily::Fera), "", ListChoice::default());
        assert_eq!(ids(&fera.pinned), vec![FOSSA], "pinned on top of their own family's tab only");

        let not_maxed = list(&pets, &pins, Tab::All, "", ListChoice { show: Show::NotMaxed, ..ListChoice::default() });
        assert_eq!(ids(&not_maxed.pinned), vec![FOSSA, FUNGEN]);
    }

    #[test]
    fn show_keeps_the_pets_not_owned_yet_or_not_at_the_cap() {
        let pets = collection(&[(FOSSA, 2, 27), (KALGOLEM, 3, 0), (SPARKIE, 0, 3)]);
        let shown = |show| list(&pets, &[], Tab::All, "", ListChoice { show, ..ListChoice::default() }).rest;

        let not_owned = shown(Show::NotOwned);
        assert!(not_owned.iter().all(|row| row.state.level == 0));
        assert!(not_owned.iter().any(|row| row.pet_id == SPARKIE), "souls but no level yet is not owned");
        let not_maxed = shown(Show::NotMaxed);
        assert!(not_maxed.iter().any(|row| row.pet_id == FOSSA) && !not_maxed.iter().any(|row| row.pet_id == KALGOLEM));
    }

    #[test]
    fn closest_to_the_next_level_puts_the_fewest_souls_left_first_and_the_cap_last() {
        let pets = collection(&[(FOSSA, 2, 70), (SPARKIE, 1, 24), (KALGOLEM, 3, 0), (FUNGEN, 0, 4), (BLACK_SPIRIT, 1, 20)]);
        let choice = ListChoice { order: Order::ClosestToNextLevel, ..ListChoice::default() };

        let rows = list(&pets, &[], Tab::All, "", choice).rest;

        assert_eq!(ids(&rows[..4]), vec![SPARKIE, FUNGEN, FOSSA, BLACK_SPIRIT], "1 left at 96%, 1 left at 80%, then 5 and 5");
        assert_eq!(rows.last().map(|row| row.pet_id), Some(KALGOLEM));
    }

    #[test]
    fn souls_read_against_the_next_level_and_a_pet_at_the_cap_is_full() {
        let row = |level, souls| Row { pet_id: FOSSA, state: PetState { level, souls }, family: None };
        assert_eq!(row(0, 0).souls_of_needed(), (0, 5));
        assert_eq!(row(1, 24).souls_of_needed(), (24, 25));
        assert_eq!(row(2, 27).souls_of_needed(), (27, 75));
        assert_eq!(row(3, 0).souls_of_needed(), (75, 75));
        assert_eq!(row(3, 0).progress(), 1.0);
        assert!(row(0, 0).never_obtained() && !row(0, 1).never_obtained());
        assert!(ListChoice { order: Order::ClosestToNextLevel, ..ListChoice::default() }.filters());
        assert!(!ListChoice::default().filters());
    }
}
