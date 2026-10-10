---
id: TASK-25.11
title: Remove the upstream-merge rule from the docs and code comments
status: In Progress
assignee: []
created_date: '2026-10-08 12:38'
updated_date: '2026-10-09 17:21'
labels:
  - documentation
dependencies: []
parent_task_id: TASK-25
priority: medium
type: docs
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Remove the "keep core diffs small against upstream" rule, and everything justified by it, from the project's instructions, plans and code comments.

## Agreed (user decision, 2026-10-08)

- There is no upstream. This project is fully split from the projects it forked (Aether, and before it NOIA2).
- The fork was only a baseline to start from. Nothing will be merged from them.
- They get credit in README.md (TASK-27), plus whatever the licence requires.

## What still says otherwise (commit 7a48824)

- `AGENTS.md`: the Conventions bullet "Keep core diffs against upstream small and anchored; `git fetch upstream && git merge upstream/main` should stay cheap".
- `AGENTS.md`: the "Upstream merges" entry under "Things that will bite you", which covers resolving merges of Tauri-era paths and the renames from `src-tauri/src/dps_meter/**`.
- the removed docs/GPUI_MIGRATION.md (git history), around lines 336-339: the same rule.
- `crates/core/src/region.rs`, around lines 19-22: keeps the region profile as a process-wide global "on purpose", to keep the upstream diff small.
- Other comments in core that justify code by upstream (region.rs ~155 and ~245-247, engine/meter.rs, capture/processor.rs). TASK-26.2 lists them.

## Evidence

Git history: zero merge commits since the fork point `d9f1d98` (Aether 2.4.0, 2026-09-24). The core has moved to `crates/core`, a tree upstream does not have. The rule was costing comments and globals and buying nothing.

## Suggested direction (Proposed, not agreed)

- Delete the convention and the "Upstream merges" entry from AGENTS.md.
- Delete the same rule from the removed docs/GPUI_MIGRATION.md (git history).
- Drop the "upstream" justification from the region.rs comment. Whether the global itself goes is TASK-25.18.
- The local git remote named `upstream` still points at the Aether repository. Removing it is the user's own local choice; it is not a repository change.

## Coordination

- AGENTS.md had uncommitted edits by another agent on 2026-10-08. Make this change on top of the current file, not the review snapshot.
- TASK-26.5 trims other fork-era docs and TASK-26.2 removes fork-relative comments. Do this ticket first, or together with them, so the same lines are not edited twice.

## How to verify

Reading AGENTS.md, the removed docs/GPUI_MIGRATION.md (git history) and the core comments finds no request to keep diffs small against upstream and no merge instructions. The credits and the GPL-3.0 licence are unchanged.

## Related

- TASK-25.12, TASK-25.13, TASK-25.15: core restructuring that this rule used to block. They are now free to be clean cutovers.
- TASK-25.18: the region profile global.
- TASK-26.2, TASK-26.5: fork cleanup.
- TASK-27: the README credit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AGENTS.md and the removed docs/GPUI_MIGRATION.md (git history) no longer ask for small core diffs against upstream and no longer describe upstream merges
- [ ] #2 No code comment justifies a design by keeping the upstream diff small
- [ ] #3 Credits and the GPL-3.0 licence are unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: Built and reviewed; not yet checked by the user. The upstream-merge rule and the merge instructions are gone from AGENTS.md and the removed docs/GPUI_MIGRATION.md (git history), and no code comment justifies a design by the upstream diff any more. Whether the region profile stays a global is TASK-25.18.
<!-- SECTION:NOTES:END -->
