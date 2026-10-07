---
id: TASK-15
title: End-of-fight report for boss fights
status: In Progress
assignee: []
created_date: '2026-10-07 12:14'
updated_date: '2026-10-07 16:45'
labels:
  - feature
dependencies: []
priority: medium
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After a boss fight, the user wants to open a full report of that fight from a button, and from it look at what each player did.

## Intent

Today the boss summary card shows who hit the boss, but not the whole picture of the fight. The user wants one place that shows how every player did and what happened to them, without having to read the live meter during the fight.

## Agreed scope

- After a boss fight, a button opens a full report of the fight.
- The report is for boss fights only. Fights that are not boss fights get no report (user decision, 2026-10-07: "non boss fights won't get the report only boss fights it's simpler that way and non boss fights are not that important").
- The report is one table with one row per player and the columns Damage, DPS, Healing, Damage taken and Deaths. Clicking a column header sorts the table by it; it starts sorted by Damage, and the chosen sort is kept while the report window is open. Deaths (who died and what killed them) are listed under the table.
- Where the report opens from (user decision, 2026-10-07):
  - A full-width "Open the fight report" button on the boss summary card. It is also shown when the card says the fight ended without the boss dying ("Fight ended, may resume").
  - A report button on each boss fight in History. Boss fights in History get a crown before the boss's name (user decision, 2026-10-07: "yeah use a crown").
- Clicking a player in the table, or their line under Deaths, opens that player's details inside the report window. A "‹ Report" control goes back to the table.
- Player details show:
  - Damage per skill.
  - Hit types (critical, back, front, parry, perfect, double hit) as a count and a rate, e.g. 665 critical hits, 41.9%.
  - Buff and skill uptime as minutes:seconds (e.g. 4:01) together with the share of the fight.
  - Damage taken for that player only; there is no party-wide breakdown of damage taken by source.
- Figure alignment (user decision, 2026-10-07): the key-figure tiles are left-aligned (label above value, at the left of each cell); the figures in the tables, with their column headers, are right-aligned.
- Order of work: mockups first for every screen that changes; implementation starts only after the user approves the mockups. The user approved them on 2026-10-07; implementation has started.

## Approved design

The approved mockups are in `docs/gpui-mockups/approved/`: `report.html` (the report and player details; with `report.css` and `report.js`), `meter.html` (the "Open the fight report" button on the summary card), `history.html` (the report button and the crown on boss records) and `tokens.md`, section "Fight report" (values). `docs/gpui-mockups/fight-report/` holds the history of the mockup round, not the design.

## Explicitly not included

- No report for fights that are not boss fights.
- No graph of the boss's health over time.
- No hit-by-hit event timeline. This is an end-of-fight report, not a timeline.
- No party-wide breakdown of damage taken by source.

## What the codebase already has (derived from reading the code)

- A per-skill detail window already exists and works for saved fights. It shows critical, back, front, parry, perfect and double-hit rates and buff coverage as a percentage.
- The boss summary card already exists.
- Saved fights keep per-player, per-skill and buff totals, but not deaths, damage taken or heals. Those would need to be recorded and saved, and fights saved before this change will not have them.

## Open questions (not decided)

1. Heals: which effects count as heals, as opposed to shields or life drain, must be checked on a recording with a healer in the party (Cleric or Chanter) before it is settled.

## Related records (not changed by this item)

- TASK-14.4: reads boss death and buff removal; the report relies on both being read correctly.
- TASK-14.2: damage hit and critical-hit reading; per-skill damage and crit counts in the report depend on it.
- TASK-3: dungeon boss not detected in one run; a boss fight must be detected for the report to be offered.
- TASK-4: skills shown as a raw id; per-skill rows in the report need real skill names.

## Subtasks

1. Mockups for the fight report and player details
2. Record deaths, damage taken and heals for each fight (after 1)
3. Fight report window (after 1 and 2)
4. Player details in the fight report (after 1 and 2)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The user has approved the mockups before any implementation starts.
- [ ] #2 After a boss fight, a button opens a report: one table with a row per player showing damage, DPS, healing, damage taken and deaths, sortable by clicking a column, with deaths (who died and what killed them) listed under it.
- [ ] #3 The report opens from the boss summary card (including when the fight ended without the boss dying) and from each boss fight in History.
- [ ] #4 Clicking a player shows, inside the report window, their per-skill damage, hit types as count and rate, and buff and skill uptime as minutes:seconds with the share of the fight; '‹ Report' returns to the table.
- [ ] #5 Fights saved before this change still open.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All parts (TASK-15.2, 15.3, 15.4) are built and reviewed; they wait for the user's test after a real boss fight.
<!-- SECTION:NOTES:END -->
