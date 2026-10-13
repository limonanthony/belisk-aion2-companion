//! What the game said about the local player's tickets, keys and Odyle Energy,
//! tied to the character it said it about. The core passes the game's numbers
//! through untouched: caps, owners and refill rules are the app's data.

use crate::capture::parser::ticket_record::{TicketReason, TicketRecord};
use crate::character_binding::{CharacterBinder, CharacterReport};
use crate::identity::KnownCharacter;

pub type TicketReport = CharacterReport<TicketContent>;

#[derive(Debug, Clone, PartialEq)]
pub enum TicketContent {
    /// Every ticket the game listed on entering the world.
    Snapshot(Vec<TicketRecord>),
    /// One ticket's new full value.
    Change { record: TicketRecord, reason: TicketReason, delta: Option<u32> },
}

/// Ties ticket messages to the local player's character (see
/// `CharacterBinder`) and drops the game's test tickets.
#[derive(Default)]
pub struct TicketBinder {
    binder: CharacterBinder<TicketContent>,
}

impl TicketBinder {
    pub fn reset(&mut self) {
        self.binder.reset();
    }

    pub fn character_named(&mut self, character: KnownCharacter, captured_at: f64) -> Vec<TicketReport> {
        self.binder.character_named(character, captured_at)
    }

    pub fn assume_character(&mut self, character: KnownCharacter) -> bool {
        self.binder.assume_character(character)
    }

    pub fn assumed(&self) -> Option<(&KnownCharacter, usize)> {
        self.binder.assumed()
    }

    pub fn snapshot(&mut self, records: Vec<TicketRecord>, captured_at: f64) -> Option<TicketReport> {
        let records = records.into_iter().filter(|record| !record.is_test_ticket()).collect();
        self.binder.snapshot(TicketContent::Snapshot(records), captured_at)
    }

    pub fn change(
        &mut self,
        record: TicketRecord,
        reason: TicketReason,
        delta: Option<u32>,
        captured_at: f64,
    ) -> Option<TicketReport> {
        if record.is_test_ticket() {
            return None;
        }
        self.binder.change(TicketContent::Change { record, reason, delta }, captured_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(name: &str) -> KnownCharacter {
        KnownCharacter { name: name.to_string(), server_id: 1302, class: "RANGER".to_string(), key: None }
    }

    fn record(id: u32, count: u32, charge: u32) -> TicketRecord {
        TicketRecord { flags: 0, id, remaining_ms: None, field_b: None, count, charge }
    }

    fn ids(report: &TicketReport) -> Vec<u32> {
        match &report.content {
            TicketContent::Snapshot(records) => records.iter().map(|record| record.id).collect(),
            TicketContent::Change { record, .. } => vec![record.id],
        }
    }

    #[test]
    fn a_snapshot_waits_for_the_name_that_follows_it() {
        let mut binder = TicketBinder::default();
        assert!(binder.snapshot(vec![record(8, 2, 0)], 100.0).is_none());

        let reports = binder.character_named(character("Alpha"), 104.0);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].character.name, "Alpha");
        assert_eq!(reports[0].captured_at, 100.0);
        assert_eq!(ids(&reports[0]), vec![8]);
    }

    #[test]
    fn a_name_more_than_fifteen_seconds_after_leaves_the_snapshot_unbound() {
        let mut binder = TicketBinder::default();
        binder.snapshot(vec![record(8, 2, 0)], 100.0);

        assert!(binder.character_named(character("Alpha"), 116.0).is_empty());
        assert!(binder.character_named(character("Alpha"), 117.0).is_empty());
    }

    #[test]
    fn a_name_up_to_two_seconds_before_a_snapshot_binds_it_at_once() {
        let mut binder = TicketBinder::default();
        binder.character_named(character("Alpha"), 100.0);

        let report = binder.snapshot(vec![record(8, 2, 0)], 102.0).expect("bound at once");

        assert_eq!(report.character.name, "Alpha");
    }

    #[test]
    fn a_name_more_than_two_seconds_before_a_snapshot_does_not_bind_it() {
        let mut binder = TicketBinder::default();
        binder.character_named(character("Alpha"), 100.0);

        assert!(binder.snapshot(vec![record(8, 2, 0)], 102.5).is_none());

        let reports = binder.character_named(character("Beta"), 106.0);
        assert_eq!(reports[0].character.name, "Beta");
    }

    #[test]
    fn a_change_belongs_to_the_character_named_now() {
        let mut binder = TicketBinder::default();
        binder.character_named(character("Alpha"), 100.0);

        let report = binder.change(record(60000001, 0, 245), TicketReason::Spend, None, 500.0).expect("bound at once");

        assert_eq!(report.character.name, "Alpha");
        assert_eq!(report.captured_at, 500.0);
        assert_eq!(
            report.content,
            TicketContent::Change { record: record(60000001, 0, 245), reason: TicketReason::Spend, delta: None }
        );
    }

    #[test]
    fn a_change_before_any_name_waits_for_the_first_one() {
        let mut binder = TicketBinder::default();
        assert!(binder.change(record(4, 1, 0), TicketReason::Gain, Some(1), 100.0).is_none());

        let reports = binder.character_named(character("Alpha"), 110.0);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].captured_at, 100.0);
    }

    #[test]
    fn a_change_after_a_new_world_entry_waits_for_the_new_name_not_the_old_character() {
        let mut binder = TicketBinder::default();
        binder.character_named(character("Alpha"), 100.0);
        binder.snapshot(vec![record(8, 2, 0)], 500.0);
        assert!(binder.change(record(4, 1, 0), TicketReason::Gain, Some(1), 501.0).is_none());

        let reports = binder.character_named(character("Beta"), 504.0);

        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|report| report.character.name == "Beta"));
    }

    #[test]
    fn a_new_snapshot_discards_what_still_waited_from_the_previous_world() {
        let mut binder = TicketBinder::default();
        binder.snapshot(vec![record(8, 2, 0)], 100.0);
        binder.snapshot(vec![record(1, 6, 0)], 105.0);

        let reports = binder.character_named(character("Beta"), 109.0);

        assert_eq!(reports.len(), 1);
        assert_eq!(ids(&reports[0]), vec![1]);
    }

    #[test]
    fn waiting_changes_older_than_fifteen_seconds_are_dropped() {
        let mut binder = TicketBinder::default();
        binder.change(record(4, 1, 0), TicketReason::Gain, Some(1), 100.0);
        binder.change(record(6, 1, 0), TicketReason::Gain, Some(1), 120.0);

        let reports = binder.character_named(character("Alpha"), 121.0);

        assert_eq!(reports.iter().map(ids).collect::<Vec<_>>(), vec![vec![6]]);
    }

    #[test]
    fn test_tickets_are_dropped_from_snapshots_and_changes() {
        let mut binder = TicketBinder::default();
        binder.character_named(character("Alpha"), 100.0);

        let report = binder
            .snapshot(vec![record(8, 2, 0), record(10, 1, 0), record(11, 1, 0), record(12, 1, 0), record(13, 0, 0)], 101.0)
            .expect("bound at once");
        assert_eq!(ids(&report), vec![8, 13]);
        assert!(binder.change(record(11, 2, 0), TicketReason::TimedRefill, None, 700.0).is_none());
    }

    #[test]
    fn reset_forgets_the_name_and_everything_waiting() {
        let mut binder = TicketBinder::default();
        binder.character_named(character("Alpha"), 100.0);
        binder.change(record(4, 1, 0), TicketReason::Gain, Some(1), 101.0);
        binder.reset();

        assert!(binder.change(record(4, 2, 0), TicketReason::Gain, Some(1), 102.0).is_none());
        assert_eq!(binder.character_named(character("Beta"), 103.0).len(), 1);
    }
}
