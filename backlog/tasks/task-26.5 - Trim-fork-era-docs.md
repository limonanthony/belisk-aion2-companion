---
id: TASK-26.5
title: Trim fork-era docs
status: In Progress
assignee: []
created_date: '2026-10-08 12:47'
updated_date: '2026-10-09 17:21'
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

the removed FORK.md (git history) was written to describe the fork's changes against Aether, and AGENTS.md's merge rule to keep upstream merges cheap. The Tauri app was then removed and no upstream is merged, but the documents were not cut down.

## Items (classification: edit these; keep the history ones)

| Item | Where | Action |
|---|---|---|
| the removed FORK.md (git history) Tauri-era sections | `the removed FORK.md (git history)` 9-13, 119, 181, 816 | Trim or move to a history document |
| the removed FORK.md (git history) Supabase / cloud text | `the removed FORK.md (git history)` 51-54, 98-104 | Remove (this project has no cloud) |
| A previous maintainer's personal signing-key path (a user-profile path under `.aether-release`) | `the removed FORK.md (git history)` 246-247 | Remove |
| AGENTS.md `src-tauri` upstream-merge rule | `AGENTS.md` 91-102 | Rewrite or drop, depending on the upstream-convention decision (TASK-25.11) |
| README key binding inconsistency: Alt+CapsLock in the README, Alt+E in the CHANGELOG | `README.md`; `CHANGELOG.md` | Check which is current in the code and fix the wrong one (check the code, do not guess) |
| CHANGELOG entries from the Aether and Chinese-era builds | `CHANGELOG.md` 231-233, 459-464, 585-598, 639-642 | Keep (history); do not rewrite |
| `docs/AION2_PACKET_PROTOCOL_ANALYSIS.zh-CN.md` | `docs/` | Keep (the upstream protocol reference); its links point to a NOIA developer's local paths, which may be noted but not necessarily fixed |

Credits and the licence stay (see TASK-27, the README work item). Statements that are false about today's code (not just old) are the separate docs-drift ticket (TASK-25.35); coordinate so the same lines in the removed FORK.md (git history) and AGENTS.md are not edited twice.

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified. Line numbers must be re-located in the current files.

## Suggested direction (Proposed, not agreed)

Remove the Supabase text and the personal path outright. For the Tauri sections, keep a short "history" note pointing to the tag `tauri-final` instead of the full text. Do the AGENTS.md rule after the upstream-convention decision (TASK-25.11). Verify the key binding in the code before editing either document.

## How to verify

A search of the removed FORK.md (git history), AGENTS.md and README.md for "Supabase", a personal key path, "src-tauri" and "Tauri" finds only short history notes; README and CHANGELOG agree on the binding and both match the code; CHANGELOG history entries are unchanged.

## Related

TASK-25.35 (T4: docs that say things the code no longer does), TASK-25.11 (upstream-diff rule), TASK-27 (README), TASK-26 (parent).

Source: fork-leftovers inventory of commit 7a48824 (2026-10-08). Line numbers refer to that commit; re-locate each point in the current files before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the removed FORK.md (git history) no longer contains the Supabase text or the previous maintainer's personal signing-key path, and its Tauri sections are trimmed to a short history note.
- [ ] #2 AGENTS.md no longer has the src-tauri upstream-merge rule (rewritten or dropped after the upstream-convention decision).
- [ ] #3 README and CHANGELOG agree on the key binding, matching the code.
- [ ] #4 CHANGELOG history and the zh-CN protocol document are unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: Built and reviewed; not yet checked by the user. the removed FORK.md (git history) trimmed: Supabase text, the old signing-key path and updater/versioning sections, the Tauri-era running and quality-gate notes, the upstream merge notes and the background-video section are gone; Always on top, live chat, light theme and the 2.2.0 removals are short history notes. CHANGELOG history and the Chinese protocol notes are kept as they are.
<!-- SECTION:NOTES:END -->
