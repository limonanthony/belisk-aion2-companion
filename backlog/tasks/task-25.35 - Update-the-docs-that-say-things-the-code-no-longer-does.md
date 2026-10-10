---
id: TASK-25.35
title: Update the docs that say things the code no longer does
status: To Do
assignee: []
created_date: '2026-10-08 12:45'
updated_date: '2026-10-08 12:48'
labels:
  - documentation
dependencies: []
parent_task_id: TASK-25
priority: low
type: docs
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Several project documents (AGENTS.md, the removed FORK.md (git history) and the removed migration plan docs/GPUI_MIGRATION.md (git history)) describe things the code no longer does, or describe a design the code never reached. A new contributor, or an agent following AGENTS.md, will act on rules that are false: for example, they would look for a Home screen and a tray icon that do not exist, keep core diffs small for an upstream merge that has not happened once, or believe the overlay hides between fights.

## Why it happens

The documents were written during and just after the move from the Tauri app (the forks) to the Rust and GPUI app, and describe the plan and the inherited behaviour. The code then moved on (French game names, no tray, no Home, hide-when-idle event dropped by the app) without the documents being updated.

## The drift (from the review)

| Claim | Where | What the code does |
|---|---|---|
| The core talks to the host through `MeterEvent` plus `EventSink` | AGENTS.md, the removed docs/GPUI_MIGRATION.md (git history) plan | The app holds `Arc<DpsMeter>` and imports history, report, storage, mobs and session internals directly (see TASK-25.12) |
| Keep core diffs against upstream small so Aether / NOIA2 parser fixes can be merged | the removed docs/GPUI_MIGRATION.md (git history) ~336-339, a comment in `crates/core/src/region.rs` | Zero merge commits since the fork point `d9f1d98`; the core lives in `crates/core`, a tree upstream does not have; about 19,000 lines differ (decision: TASK-25.11) |
| Never touch the game process: no `OpenProcess`, no `sysinfo` refresh of all processes | AGENTS.md, the removed docs/GPUI_MIGRATION.md (git history) ~320-321, the removed FORK.md (git history) section for 2.4.0 | The code lists processes with a Toolhelp snapshot and the TCP table to find the game's connections (`game_connections.rs`); the removed FORK.md (git history) ~542-551 accepts that; `sysinfo` is used for the own process only (`engine/meter.rs` ~772-781). The documents conflict with each other (decision: TASK-25.22) |
| Hide-when-idle hides the overlay window between fights with 8 / 4 / 15 s peeks | the removed FORK.md (git history) ~419-423, the removed docs/GPUI_MIGRATION.md (git history) "kept" list | The core emits the event; the app ignores it; the waiting card stays (bug and decision: TASK-25.2) |
| Game data (skills, mobs) stays English; there is no French source | the removed docs/GPUI_MIGRATION.md (git history) ~62-64 | French names exist (commit `461e644`); `game_data.rs` loads `load_npc_names_fr`; tests check that names follow the language |
| Home with start / stop, a tray icon and `aion2_focus` auto-hide are kept | the removed docs/GPUI_MIGRATION.md (git history) "kept" list | There is no Home, no tray and no `aion2_focus` in `crates/app` |
| The plan describes `crates/aether-core` and `src-tauri` depending on it | the removed docs/GPUI_MIGRATION.md (git history) ~341-348 | Only `crates/core` and `crates/app` exist (plus `crates/xtask` and `crates/packet-explorer`); the Tauri app was removed in commit `104c8f4` (tag `tauri-final`) |
| the removed FORK.md (git history) banner: capture, engine and anti-cheat sections "still describe the core" | the removed FORK.md (git history) top | Mostly true for the WinDivert sniff-only mode, Npcap first, and the Toolhelp hint; the Tauri plugin paths in the same file are history |
| No blocking in the meter's threads; the UI is fed through `bridge.rs` | AGENTS.md | `EventSink::emit` does not block, but history saves, recordings and UI-to-core locks do (TASK-25.5, TASK-25.6, TASK-25.7) |
| Test counts: core 323, app 456, and so on | AGENTS.md | Not re-counted by the review (builds were not allowed); re-count after the test clean-up (TASK-25.33) |

## Evidence

Reported by the review (static reading), not re-verified. Source: the "Docs drift" table of the architecture report. Line references are to commit 7a48824.

## Suggested direction (Proposed, not agreed)

Update each statement to what the code does now, once the related decision is made, rather than before. Rows tied to an open decision (the upstream-diff rule, the process listing, hide-when-idle) wait for that decision; the rest (French names, missing Home / tray / `aion2_focus`, `aether-core` / `src-tauri` names, test counts) can be fixed immediately. Changes are to technical documentation in the repository only. History that stays true as history (CHANGELOG, the removed FORK.md (git history) sections that describe what the forks did) is left alone, and trimming Tauri-era text is a separate ticket.

## How to verify

Read each documented claim in the table against the current code and confirm it is either true or removed. A search of AGENTS.md and the removed docs/GPUI_MIGRATION.md (git history) for `aether-core`, `src-tauri`, "tray" and "aion2_focus" finds only statements marked as history.

## Related

TASK-25.2 (hide when idle), TASK-25.12 (the app-core interface), TASK-25.11 (upstream-diff rule), TASK-25.22 (process listing), TASK-25.33 (test counts after the clean-up). TASK-26.5 (trim fork-era docs) overlaps for the removed FORK.md (git history) and the AGENTS.md merge rule; coordinate so the same lines are not edited twice.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each row of the drift table is either corrected to match the code or removed; rows tied to an open decision are updated once that decision is made.
- [ ] #2 AGENTS.md and the removed docs/GPUI_MIGRATION.md (git history) no longer mention aether-core, src-tauri, a tray, a Home screen or aion2_focus except as marked history.
- [ ] #3 Test counts in the docs match a fresh count.
<!-- AC:END -->
