---
id: TASK-25
title: Code review follow-ups (review of 7a48824)
status: To Do
assignee: []
created_date: '2026-10-08 12:33'
updated_date: '2026-10-08 15:38'
labels:
  - maintenance
dependencies: []
type: task
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A code review of commit 7a48824 (2026-10-08) found correctness problems, architecture and maintainability issues, test gaps, and leftovers from the projects this one was forked from. This parent work item groups the follow-up tickets so they can be worked through in order. Five reviewers read a frozen copy of that commit (architecture, code quality, simplicity, tests); the highest-severity findings were then re-checked by reading the code, and each child ticket says in its Evidence section whether its finding was verified that way or is reported by the review only.

Line numbers in the children refer to commit 7a48824. Core capture, parser, events, meter and data_storage changed afterwards, so each point must be re-located in the current code before acting.

## Children in this group

Correctness, created first:

- TASK-25.1: the overlay stops updating between hits (names, boss HP, buffs).
- TASK-25.2: "Hide when idle" has no effect.
- TASK-25.3: fight history can be wiped when the history file cannot be read.
- TASK-25.4: personal bests can be corrupted or resurrected.
- TASK-25.5: history and personal-best disk writes run on the capture, snapshot and UI threads.
- TASK-25.6: the UI thread does core work and disk I/O.
- TASK-25.7: capture can silently drop packets while the dispatcher writes recordings.
- TASK-25.8: damage-over-time ticks skip the player-skill filter.
- TASK-25.9: the stream buffer copies on every step, wipes silently at 50 MB, and every flow is parsed twice.
- TASK-25.10: question, should dungeon bosses be released after 120 s without a hit?

Architecture and maintainability, created second:

- TASK-25.11: decide whether to keep the "keep core diffs small against upstream" rule.
- TASK-25.12: make DpsMeter and MeterEvent the only interface between the app and the core (depends on TASK-25.11).
- TASK-25.13: packet parsers return records instead of writing storage from inside the stream processor (depends on TASK-25.9).
- TASK-25.14: packet-explorer has its own copy of the game's message framing.
- TASK-25.15: split the single lock around combat, identity and PvP state (depends on TASK-25.5, TASK-25.6, TASK-25.7).
- TASK-25.16: combat time follows the PC clock at processing time instead of the packet's time.
- TASK-25.17: FightReport copies the fight ledger field by field as a UI view-model.
- TASK-25.18: the region profile is a process-wide global and its automatic mode ignores detection in some parsers.
- TASK-25.19: capture hot path allocates string keys and repeats copies and scans for every packet.
- TASK-25.20: the meter rebuilds both boards and reformats every row on every snapshot.
- TASK-25.21: one JSON store protocol is copied three times, and history and bests use weaker ones.
- TASK-25.22: decide whether the meter may list running processes to find the game.
- TASK-25.23: finish splitting the Settings window into one file per page.
- TASK-25.24: to-dos and reminders rebuild controls the Settings kit already has.
- TASK-25.25: every Settings change wakes every consumer.
- TASK-25.26: English text reaches the UI, clipboard and accessibility labels.
- TASK-25.27: should the to-do row names be translated to French or stay English?
- TASK-25.28: small duplicated helpers that already diverge (unnamed player, fight duration, colours, weekday).
- TASK-25.29: single-instance code forgets its event handle before the listener thread can fail to start.
- TASK-25.30: the local time zone is read from Windows on every offset query.
- TASK-25.31: the dist build does not fail on incompatible licences and its tests are thin.

Created in a later batch: tests and comment clean-up (TASK-25.32: tests for contracts nobody tests; TASK-25.33: remove or rewrite tests that defend nothing or defend bugs; TASK-25.34: remove comments that narrate the code; TASK-25.35: docs that say things the code no longer does), and a separate work item for the fork leftovers (TASK-26, not a child of this parent). Those that belong to this review are added under this parent.

## Recommended order

Proposed, not agreed: TASK-25.2 and TASK-25.1 first, then TASK-25.3, TASK-25.4, TASK-25.5, TASK-25.6, TASK-25.7, TASK-25.8, TASK-25.9, and only then the architecture follow-ups. Reason: the first group is what players see or lose today; the threading tickets share a cause, and the architecture tickets should not start before them.

## Open decisions for the user, and where each one lives

- Always send the overlay frame every 200 ms, or only when something visible changed: TASK-25.1.
- Hide the overlay window between fights, or keep the waiting card and remove the setting: TASK-25.2.
- How history and personal-best writes leave the capture and UI threads (writer thread or snapshot under lock): TASK-25.5.
- Whether a dungeon boss is released after 120 s without a hit: TASK-25.10.

## Done when

All children are done. No other acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 15:38
---
Added 2026-10-08 after re-checking that every code-quality point from the review summary has a ticket: TASK-25.36 (eprintln! lost in the shipped app), TASK-25.37 (silently ignored results), TASK-25.38 (core layer folders and the utils file), TASK-25.39 (replay real recordings in cargo test; do it before TASK-25.9, 25.13, 25.14, 25.15). The misnamed DEFAULT_HIDE_KNOWN_PLAYERS went to TASK-26.3 as a comment.
---
<!-- COMMENTS:END -->
