---
id: TASK-7
title: English and French translations
status: Done
assignee: []
created_date: '2026-10-06 15:36'
updated_date: '2026-10-09 09:48'
labels:
  - feature
dependencies: []
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants every piece of app and overlay text available in English and French.

## Agreed

User (2026-10-06): "I thought we said we should make it i18 friendly right away because I wanted french and english ..."

User (2026-10-06): "if the people who made the language made a i18 library they probably use in firefox i think it's fine, i'm ok with that. Start right away please"

- All app and overlay text is in English and French, through one Fluent setup (i18n-embed / i18n-embed-fl).
- This was planned since the GPUI migration (`docs/GPUI_MIGRATION.md`, "Translations: English and French") but never built.

## Proposed (stated to the user, not yet confirmed)

- Settings → General → Language picks the language; the default is the Windows language.
- Switching the language applies at once.
- Game names (bosses, dungeons, skills, mobs) stay English, since there is no French source for them.
- Numbers keep their current format.
- The user proofreads the French.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every label in every window and in Settings comes from the translation files, in English and French.
- [ ] #2 Settings → General → Language switches all open windows at once, and the choice is remembered.
- [ ] #3 A test fails when a key exists in one language and not the other.
- [ ] #4 Plurals read correctly in both languages (for example 0, 1 and 2 items in French).
- [ ] #5 The user has proofread the French.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: user tested and accepted: "Toolbar hub works great and I haven't seen issues with the resources. So you can move all of these to done if we find issues we'll fix them on the go. I right now see no issues left except the skill. You can start 1."
<!-- SECTION:NOTES:END -->
