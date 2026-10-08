---
id: TASK-25.12
title: Make DpsMeter and MeterEvent the only interface between the app and the core
status: To Do
assignee: []
created_date: '2026-10-08 12:38'
updated_date: '2026-10-08 14:48'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The app is meant to talk to the core only through the `MeterEvent` messages, but in practice it reaches into core internals everywhere. Any change inside the core (a history field, a report column, a storage payload) ripples through the overlay, history, detail, PvP, report windows and the test fixtures. The packet-explorer tool depends on core internals too.

## Why it happens

Every module of the core is public (`crates/core/src/lib.rs` re-exports the whole tree). The app holds the meter object itself (`Arc<DpsMeter>`) and imports the internals it needs. `EventSink` is only a one-way queue of notifications; if it were removed the app would still compile against the internals. The docs (AGENTS.md, the migration plan) claim the event interface is the seam, which is not what the code does.

Current app imports from the core (from the review, production code):

- `engine::meter::DpsMeter`
- `events::{EventSink, MeterEvent}`
- `history::HistoryRecord`
- `mobs::{catalog::MobCatalog, store::{BossCorrection, write_atomic}}`
- `report::fight_report::*` and `report::ledger::KillerKind`
- `storage::data_storage::MainActorDetectedPayload`
- `storage::loaders::load_npc_names_fr`
- `engine::session::session_skill_totals`
- `skill_names::resolve_skill_key`
- `models::combat::*` and `models::session::*`
- `config::*`, `identity::*`, `logger::AppLogger`, `preflight::*`, `region`
- `capture::recorder::list_recordings` and `capture::rolling_recorder::ROLLING_FILE_PREFIX`

Places in the app that use them: `meter/model.rs`, `history/model.rs`, `pvp/model.rs`, `detail/data.rs`, `report/mod.rs`, `secondary/mod.rs`, `game_data.rs`, `live_session.rs`. `ReportRequest::Saved` even carries a stored history record as a UI event payload.

Packet-explorer (a second binary in the workspace) uses public capture internals: `parser::utils` (the only public parser module), the census, `StreamAssembler`, `TcpReassembler`, and for its `parity` command a whole `DataStorage`.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

- Treat `DpsMeter` plus `MeterEvent` as the host interface. Add a small set of host methods on `DpsMeter` for what the app already calls: reset, history list and delete, PvP watch and stats, personal best, fight report, boss correction, config, recordings listing. Do not add a second event bus.
- Make the storage, report, engine and capture internals crate-private.
- Fold `MainActorDetectedPayload` into `MeterEvent`.
- Keep as public domain types only what the app really needs (for example `MeterEvent`, the combat snapshot or a thinner overlay type, `HistoryRecord` if history is stored as that type, `KnownCharacter`, the meter config, one of `FightReport` or `FightLedger`, `PersonalBest`, the preflight report).
- Give packet-explorer a narrow framing interface instead of public parser internals (see the shared-framing ticket, A3).
- If the owner drops the "keep core diffs small" rule (TASK-25.11) this can be a cutover rather than a two-shape period.

## How to verify

The workspace builds with the core internals crate-private; the app uses only the host methods and `MeterEvent`; the app test suite and a replay of a recording give the same boards as before. A grep of the app for imports of the internal modules listed above finds none.

## Related

Depends on TASK-25.11 (decision on the upstream-diff rule). A2 (TASK-25.13, parsers return records) and A3 (TASK-25.14, shared framing) touch the same modules; A6 (TASK-25.17, report view-model) shrinks what the app imports. The review's dead-code sweep of public functions with no callers (planned, not yet created) is easier after this.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The app builds and runs using only the meter's host methods and MeterEvent; it no longer imports the core's storage, report, mobs, session or capture internals.
- [ ] #2 Packet-explorer builds against a narrow framing interface, not public parser internals.
- [ ] #3 Behaviour of the overlay, history, report and PvP windows is unchanged (same boards from the same recording).
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 14:48
---
TASK-25.11 is decided (user, 2026-10-08): there is no upstream, the project is fully split from Aether/NOIA2. The core interface can change as a clean cutover; the dependency on TASK-25.11 is removed.
---
<!-- COMMENTS:END -->
