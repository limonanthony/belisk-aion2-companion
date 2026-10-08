---
id: TASK-25.15
title: 'Split the single lock around combat, identity and PvP state'
status: To Do
assignee: []
created_date: '2026-10-08 12:38'
labels:
  - improvement
dependencies:
  - TASK-25.5
  - TASK-25.6
  - TASK-25.7
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

All live combat state, player identity and PvP tracking sit behind one lock. The packet parser, the snapshot thread and the UI all contend for it, and a snapshot cannot see a consistent cut of the data. This is recorded as a possible follow-up, not an urgent defect.

## Why it happens

`crates/core/src/storage/data_storage.rs` (about 2,000 lines) keeps in one `RwLock` the damage totals, actor metadata (name, server, class, combat power, spec), mob HP and codes, buffs, summons, fight clocks, identity (`OwnSource`, `OwnNamePending`, own actor ids), PvP attacker maps and the combat event log. Every ingest takes the write lock. Building a snapshot then takes about fifteen separate read locks and clones each map, so stats, times and names can be from slightly different moments. `clear` clones most maps under the write lock to preserve identity and PvP. The review also notes that the per-target time maps (`start_time_by_target`, `last_time_by_target`) and the PvP maps are unbounded (see A5 for the time maps).

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Do NOT start before the thread and disk fixes land. The review explicitly rejects splitting the lock while the history writes, UI-thread disk work and recording writes stay where they are, because the stalls would remain. When it is time: split along types that already exist (identity, PvP maps, the combat event log, combat totals), add one snapshot method that clones under a single read (the ledger path already does this), and make `clear` move the identity fields rather than clone every map. Do not add a repository layer or an actor runtime.

## How to verify

Snapshot, parser and UI contention is measured before and after on a recording replay; the totals from the same recording are unchanged; the project's identity tests still pass.

## Related

Depends on TASK-25.5 (history and personal-best writes off the capture, snapshot and UI threads), TASK-25.6 (UI thread core work) and TASK-25.7 (dropped packets). Subject to the decision in TASK-25.11. Related A5 (TASK-25.16, combat time and unbounded time maps).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Identity, PvP and combat totals no longer share one lock, or the review's measurements show the split is not worth it and the ticket is closed with that finding.
- [ ] #2 A snapshot is built from one consistent read of the data it uses.
- [ ] #3 Replaying the same recording gives identical totals and names.
<!-- AC:END -->
