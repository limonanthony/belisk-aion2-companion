---
id: TASK-26.5
title: Trim fork-era docs
status: To Do
assignee: []
created_date: '2026-10-08 12:47'
updated_date: '2026-10-08 12:48'
labels:
  - documentation
dependencies: []
parent_task_id: TASK-26
priority: low
type: docs
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Some project documents still carry the forks' world: sections about the Tauri app, text about a Supabase cloud backend this project does not have, a previous maintainer's personal signing-key path, an AGENTS.md rule about merging a `src-tauri` upstream, and a README that gives a different key binding than the CHANGELOG. Readers, and agents following AGENTS.md, are told to deal with things that do not exist here, and a personal path of someone else sits in the repository.

## Why it happens

FORK.md was written to describe the fork's changes against Aether, and AGENTS.md's merge rule to keep upstream merges cheap. The Tauri app was then removed and no upstream is merged, but the documents were not cut down.

## Items (classification: edit these; keep the history ones)

| Item | Where | Action |
|---|---|---|
| FORK.md Tauri-era sections | `FORK.md` 9-13, 119, 181, 816 | Trim or move to a history document |
| FORK.md Supabase / cloud text | `FORK.md` 51-54, 98-104 | Remove (this project has no cloud) |
| A previous maintainer's personal signing-key path (`<user profile>/.aether-release/…`) | `FORK.md` 246-247 | Remove |
| AGENTS.md `src-tauri` upstream-merge rule | `AGENTS.md` 91-102 | Rewrite or drop, depending on the upstream-convention decision (TASK-25.11) |
| README key binding inconsistency: Alt+CapsLock in the README, Alt+E in the CHANGELOG | `README.md`; `CHANGELOG.md` | Check which is current in the code and fix the wrong one (check the code, do not guess) |
| CHANGELOG entries from the Aether and Chinese-era builds | `CHANGELOG.md` 231-233, 459-464, 585-598, 639-642 | Keep (history); do not rewrite |
| `docs/AION2_PACKET_PROTOCOL_ANALYSIS.zh-CN.md` | `docs/` | Keep (the upstream protocol reference); its links point to a NOIA developer's local paths, which may be noted but not necessarily fixed |

Credits and the licence stay (see TASK-27, the README work item). Statements that are false about today's code (not just old) are the separate docs-drift ticket (TASK-25.35); coordinate so the same lines in FORK.md and AGENTS.md are not edited twice.

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified. Line numbers must be re-located in the current files.

## Suggested direction (Proposed, not agreed)

Remove the Supabase text and the personal path outright. For the Tauri sections, keep a short "history" note pointing to the tag `tauri-final` instead of the full text. Do the AGENTS.md rule after the upstream-convention decision (TASK-25.11). Verify the key binding in the code before editing either document.

## How to verify

A search of FORK.md, AGENTS.md and README.md for "Supabase", a personal key path, "src-tauri" and "Tauri" finds only short history notes; README and CHANGELOG agree on the binding and both match the code; CHANGELOG history entries are unchanged.

## Related

TASK-25.35 (T4: docs that say things the code no longer does), TASK-25.11 (upstream-diff rule), TASK-27 (README), TASK-26 (parent).

Source: fork-leftovers inventory of commit 7a48824 (2026-10-08). Line numbers refer to that commit; re-locate each point in the current files before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 FORK.md no longer contains the Supabase text or the previous maintainer's personal signing-key path, and its Tauri sections are trimmed to a short history note.
- [ ] #2 AGENTS.md no longer has the src-tauri upstream-merge rule (rewritten or dropped after the upstream-convention decision).
- [ ] #3 README and CHANGELOG agree on the key binding, matching the code.
- [ ] #4 CHANGELOG history and the zh-CN protocol document are unchanged.
<!-- AC:END -->
