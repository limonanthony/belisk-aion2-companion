---
id: TASK-26
title: Clean up what is left of the forked projects
status: To Do
assignee: []
created_date: '2026-10-08 12:47'
updated_date: '2026-10-08 12:48'
labels:
  - maintenance
dependencies: []
type: task
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
This project is built from two earlier projects: NOIA2 by zdyoung (https://github.com/ZDYoung0519/NOIA2), then Aether by Helveticxa (https://github.com/Helveticxa/Aether-Aion2-DPS-meter-Global), which forked it. This repository continues from Aether; its Tauri app was removed and replaced by a Rust and GPUI app (the last Tauri state is tagged `tauri-final`). The capture pipeline, packet parsers and game-data catalogues come from that lineage and stay credited. What is left over from the forks that should not be there anymore is spread across code identifiers, package metadata, comments, dead code and documents. A scout inventoried it read-only against commit 7a48824 and this work item groups the clean-up. The same inventory is split into the children below; each child copies its own part.

"Leftover" here means one of: an identifier or name from the forks, package metadata that names the previous author, a comment that talks about upstream or the Tauri port, dead code inherited from the forks, or a document that still describes the forks' app. Credits and the licence are not leftovers and stay (see TASK-27, the README work item).

## Classification rules (apply them to every item)

- Load-bearing: the thing is written to disk or read by existing installs (data folders, file names, magic bytes, settings keys, file formats). Never rename it without a migration that reads the old form. Default is to keep it.
- Safe rename: the thing is not persisted and not read by anyone else (metadata, temp folder names, test names, probe strings, log prefixes). Rename freely; product-name strings wait for the Kisk rename (TASK-23) so they are not renamed twice.
- Dead: nothing reads it. Delete it.
- History: the document or section records what happened (CHANGELOG entries about Aether and the Chinese-era builds, the zh-CN protocol analysis). Keep it; do not rewrite history.
- Keep: credits, licence and notices (README Credits, the notices header, the About page credits) stay, with their provenance.

Rule for persisted identifiers: any change to a persisted identifier needs a migration (read both old and new) and is tied to the product rename, TASK-23 (Kisk). That decision is the user's, held in F4 below.

## Children

- F1: delete dead code inherited from the forks (TASK-26.1).
- F2: remove fork-relative and Tauri-port comments (TASK-26.2).
- F3: replace upstream identity in metadata and non-persisted names (TASK-26.3).
- F4: decide what happens to the persisted Aether identifiers (TASK-26.4).
- F5: trim fork-era docs (TASK-26.5).

Related: TASK-23 (Kisk rename) for F3 and F4; TASK-27 (README) for the credits statement; the code review follow-ups (TASK-25) for the tickets this overlaps (TASK-25.12 makes the unused-public sweep easier; TASK-25.11 decides the upstream-merge rule that F2 and F5 mention).

## Done when

All children are done. No other acceptance criteria.

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified, except that `last_total_damage` (F1) was verified unused by reading the code. The inventory is a snapshot: file paths and line numbers must be re-located in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->
