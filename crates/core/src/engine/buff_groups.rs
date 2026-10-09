//! Buff intervals of one caster on one target, joined where the player sees one
//! buff: skill ids whose English name is the same (the levels and effects of a
//! skill, or two keys of the name table) count as one buff, and its uptime is
//! the union of their intervals.

use std::collections::{HashMap, VecDeque};

use crate::models::combat::BuffInterval;
use crate::skill_names::english_name;

#[derive(PartialEq, Eq, Hash)]
enum Identity {
    Named(&'static str),
    Unnamed(u32),
}

/// The skill ids of one caster on one target that show as one buff.
pub(crate) struct BuffGroup<'a> {
    /// The member id with the most uptime in the range (the lowest on a tie),
    /// the one whose icon and name the buff shows.
    pub skill_code: u32,
    members: Vec<&'a VecDeque<BuffInterval>>,
}

impl BuffGroup<'_> {
    /// Time any member was up within the range, overlaps counted once.
    pub fn covered_ms(&self, range_start_ms: u64, range_end_ms: u64) -> u64 {
        covered_duration_ms(self.members.iter().flat_map(|intervals| intervals.iter()), range_start_ms, range_end_ms)
    }

    /// The member interval that ends last.
    pub fn last_interval(&self) -> Option<&BuffInterval> {
        self.members.iter().filter_map(|intervals| intervals.back()).max_by_key(|interval| interval.end_ms)
    }
}

/// `skills` joined by the name they show, in a stable order. `range` picks
/// which member represents a group.
pub(crate) fn group_by_name(
    skills: &HashMap<u32, VecDeque<BuffInterval>>,
    range_start_ms: u64,
    range_end_ms: u64,
) -> Vec<BuffGroup<'_>> {
    let mut members_of: HashMap<Identity, Vec<(u32, &VecDeque<BuffInterval>)>> = HashMap::new();
    for (&skill_code, intervals) in skills {
        let identity = english_name(skill_code).map_or(Identity::Unnamed(skill_code), Identity::Named);
        members_of.entry(identity).or_default().push((skill_code, intervals));
    }
    let mut groups: Vec<BuffGroup> = members_of
        .into_values()
        .map(|members| {
            let skill_code = members
                .iter()
                .map(|&(skill_code, intervals)| {
                    (std::cmp::Reverse(covered_duration_ms(intervals, range_start_ms, range_end_ms)), skill_code)
                })
                .min()
                .map_or(0, |(_, skill_code)| skill_code);
            BuffGroup { skill_code, members: members.into_iter().map(|(_, intervals)| intervals).collect() }
        })
        .collect();
    groups.sort_by_key(|group| group.skill_code);
    groups
}

pub(crate) fn covered_duration_ms<'a>(
    intervals: impl IntoIterator<Item = &'a BuffInterval>,
    range_start_ms: u64,
    range_end_ms: u64,
) -> u64 {
    if range_end_ms <= range_start_ms {
        return 0;
    }

    let mut ranges: Vec<(u64, u64)> = intervals
        .into_iter()
        .filter_map(|interval| {
            let start = interval.start_ms.max(range_start_ms);
            let end = interval.end_ms.min(range_end_ms);
            (end > start).then_some((start, end))
        })
        .collect();
    ranges.sort_unstable_by_key(|(start, _)| *start);

    let mut total = 0u64;
    let mut current: Option<(u64, u64)> = None;
    for (start, end) in ranges {
        match current {
            Some((current_start, current_end)) if start <= current_end => {
                current = Some((current_start, current_end.max(end)));
            }
            Some((current_start, current_end)) => {
                total = total.saturating_add(current_end.saturating_sub(current_start));
                current = Some((start, end));
            }
            None => current = Some((start, end)),
        }
    }
    if let Some((start, end)) = current {
        total = total.saturating_add(end.saturating_sub(start));
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPIRITS_DESCENT_SHORT: u32 = 167_300_011;
    const SPIRITS_DESCENT_LONG: u32 = 167_300_012;
    const ELEMENT_UNIFICATION: u32 = 167_800_011;
    const ELEMENT_UNIFICATION_PULSE: u32 = 167_800_021;
    const UNNAMED_A: u32 = 999_999_001;
    const UNNAMED_B: u32 = 999_999_002;

    fn secs(from: u64, to: u64) -> BuffInterval {
        BuffInterval { start_ms: from * 1_000, end_ms: to * 1_000 }
    }

    fn skills(entries: &[(u32, &[BuffInterval])]) -> HashMap<u32, VecDeque<BuffInterval>> {
        entries.iter().map(|&(code, intervals)| (code, intervals.iter().cloned().collect())).collect()
    }

    #[test]
    fn ids_with_one_name_are_one_group_with_the_union_of_their_intervals() {
        let skills = skills(&[
            (SPIRITS_DESCENT_SHORT, &[secs(0, 10), secs(20, 30)]),
            (SPIRITS_DESCENT_LONG, &[secs(5, 35)]),
        ]);

        let groups = group_by_name(&skills, 0, 60_000);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].skill_code, SPIRITS_DESCENT_LONG, "the id with the most uptime represents the buff");
        assert_eq!(groups[0].covered_ms(0, 60_000), 35_000, "0-35 s once, not the 20 s + 30 s sum");
        assert_eq!(groups[0].last_interval().map(|interval| interval.end_ms), Some(35_000));
    }

    #[test]
    fn two_keys_of_the_name_table_with_one_name_join() {
        let skills = skills(&[(ELEMENT_UNIFICATION, &[secs(0, 10)]), (ELEMENT_UNIFICATION_PULSE, &[secs(8, 9)])]);

        let groups = group_by_name(&skills, 0, 60_000);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].skill_code, ELEMENT_UNIFICATION);
        assert_eq!(groups[0].covered_ms(0, 60_000), 10_000);
    }

    #[test]
    fn different_names_and_unnamed_ids_stay_apart() {
        let skills = skills(&[
            (SPIRITS_DESCENT_SHORT, &[secs(0, 10)]),
            (ELEMENT_UNIFICATION, &[secs(0, 10)]),
            (UNNAMED_A, &[secs(0, 10)]),
            (UNNAMED_B, &[secs(0, 10)]),
        ]);

        let groups = group_by_name(&skills, 0, 60_000);

        assert_eq!(groups.iter().map(|group| group.skill_code).collect::<Vec<_>>(), [
            SPIRITS_DESCENT_SHORT,
            ELEMENT_UNIFICATION,
            UNNAMED_A,
            UNNAMED_B
        ]);
    }

    #[test]
    fn the_union_is_clipped_to_the_range() {
        let skills = skills(&[(SPIRITS_DESCENT_SHORT, &[secs(0, 100)]), (SPIRITS_DESCENT_LONG, &[secs(10, 90)])]);

        let groups = group_by_name(&skills, 20_000, 80_000);

        assert_eq!(groups[0].covered_ms(20_000, 80_000), 60_000);
    }
}
