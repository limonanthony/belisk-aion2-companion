---
id: TASK-23
title: Rename the product to Kisk
status: To Do
assignee: []
created_date: '2026-10-08 11:32'
updated_date: '2026-10-08 14:50'
labels:
  - improvement
dependencies: []
type: enhancement
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Intent: the project gets a new player-facing name, **Kisk**, replacing "AION 2 DPS Meter". The user wants a memorable name; "DPS meter" was fine but generic. Nothing is renamed yet; this record only captures the decision for later implementation.

## Agreed (user decision, 2026-10-08)

- The player-facing name becomes **Kisk**.
- No rename has been done.

## Why Kisk (context)

- A Kisk is the AION totem a player plants to bind their respawn point: it sits beside the fight and is still there after a wipe.
- It is short, works in English and French, and is not tied to DPS, so it also fits the fight reports, reminders and to-dos.
- Rejected: "Nyerk" (Shugo catchphrase). Nyerk.xyz was the forum of AionScript, a botting tool for the original AION, and a capture tool must not share a name with a cheat community. "Daeva Tally" was too long. Also considered: Shugo Ledger, Blame Board, Parsley.
- A quick web search found no AION meter, tool or GitHub project named Kisk. This was not a trademark check.

## Proposed (not agreed)

- Keep "AION" out of the name itself (NCSoft trademark) and use it only in a descriptor or tagline, for example "Kisk — combat meter for AION 2".
- Do it in two steps. First the player-visible surfaces: window titles, About/Settings text, README, the dist zip name, and the English and French i18n strings. Internal identifiers (Cargo package `aion-2-dps-meter`, the settings/data folder holding settings.json, todos.json and recordings) later, because renaming them requires migrating existing players' files.

## Open questions

- Do Kisks exist in AION 2? They did in AION 1; if not, newer players may not catch the reference.
- Exact tagline wording.
- Whether and when the internal identifiers and data folder are renamed, and how existing user data migrates.
- Whether a formal trademark/name clash check is wanted before release.
<!-- SECTION:DESCRIPTION:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 14:50
---
Internal identifiers, first decision (user, 2026-10-08): recordings move to .kiskrec with a new magic and keep reading .aetherpc (TASK-26.6). The data folder, log name, settings keys and history format are still open in TASK-26.4.
---
<!-- COMMENTS:END -->
