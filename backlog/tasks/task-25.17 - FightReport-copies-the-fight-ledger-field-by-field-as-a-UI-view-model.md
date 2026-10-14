---
id: TASK-25.17
title: FightReport copies the fight ledger field by field as a UI view-model
status: To Do
assignee: []
created_date: '2026-10-08 12:40'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The fight report's data is a field-by-field copy of the stored fight ledger, with some display formatting mixed in, in the core. Every new report column has to be added in three places (the stored ledger, the report copy, the app's formatting), and the core carries user-interface concerns.

## Why it happens

`crates/core/src/report/fight_report.rs` (view-model, around lines 86-209 and 293-424) duplicates the types in `crates/core/src/report/ledger.rs` (around lines 50-144): `HealSkill`/`HealReport`, `TakenSkill`/`TakenReport`, `LedgerDeath`/`DeathReport` and `LedgerKiller`/`KillerReport` are copies with a few display extras (`is_you`, a `spec` array, buff `share`, `party_dps`). `FightReport::build` re-sorts, re-filters drain and regen and finds "you" by name. The app then does its own translation and place/player-count work in `crates/app/src/report/data.rs`. `party_dps` is the sum of per-player DPS (`fight_report.rs` around line 399), which is not a physical party rate. The tests in the file mostly assert the copy (see the tautological `the_party_dps_adds_the_players_up` in the test clean-up).

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Persist the ledger and the damage tables; build the report view in the app from the ledger. If a core type is still needed, compose existing types rather than cloning every field. Drop `party_dps` or define it as total damage divided by duration.

## How to verify

The fight report window shows the same rows and numbers for the same stored fights (including fights stored before the change, which must still open). A new report column needs a change in one place for data and one for formatting.

## Related

A1 (TASK-25.12, what the app may import). TASK-15 and its children (fight report, Done) own the product behaviour that must not change.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The report window shows the same numbers and rows as before for new and previously stored fights.
- [ ] #2 There is one set of types for healing, damage taken, deaths and killers, not a ledger copy and a report copy.
- [ ] #3 party_dps is either removed or defined as total damage divided by the fight duration.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present per the review (report/fight_report.rs against ledger.rs; app report/data.rs formats the copy). The tautological party-DPS test it had is replaced by one with an independent oracle (see TASK-25.33). Not otherwise touched.
<!-- SECTION:NOTES:END -->
