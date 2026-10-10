---
id: TASK-34.3
title: Mockups for the pets window and the soul tracker
status: Done
assignee: []
created_date: '2026-10-10 12:46'
updated_date: '2026-10-10 14:18'
labels:
  - feature
dependencies: []
parent_task_id: TASK-34
priority: high
type: task
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The window and the tracker card are drawn before they are built (agreed: design through mockups first).

The window mockup shows the agreed parts: tabs by family plus All, search, pinned pets at the top, level and souls per pet. The Proposed items from the parent (family summary, filters, Special pets, toolbar count for pinned pets, typed-in counts, "as of" time for the last saved collection, black-market purchase, window label) appear as options for the user to pick or drop.

The tracker card mockup shows progress to the next level (for example "26/75 → Lv 3"), the level 3 case ("Lv 3 75/75"), and that it disappears 10 seconds after its last update.

Part of the parent work item.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Mockups in docs/gpui-mockups/ cover the window: family tabs plus All, search, pinned pets at the top, level and souls per pet, with each Proposed item from the parent drawn as an option.
- [x] #2 Mockups cover the tracker card: progress to the next level, the next level shown, the level 3 case (Lv 3 75/75) and the 10-second disappearance.
- [x] #3 The user approves one design for the window and one for the card.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-10: mockups in docs/gpui-mockups/pets/ (index.html, README.md): window layouts A/B/C, card designs A/B/C, label options; awaiting the user's choice.

2026-10-10: the user approved the mockups (round 1 and round 2). They are in docs/gpui-mockups/pets/ (README.md has the decisions); the approved versions are being copied to docs/gpui-mockups/approved/.
<!-- SECTION:NOTES:END -->
