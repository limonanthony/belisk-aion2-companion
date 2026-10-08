---
id: TASK-25.24
title: To-dos and reminders rebuild controls the Settings kit already has
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The to-do and reminder windows rebuild controls that the Settings window already has (segmented choice, focus ring, key handling). The two copies already behave differently: arrow keys wrap in Settings but stop at the ends in the to-do form. A fix or restyle has to be made twice.

## Why it happens

`crates/app/src/settings/widgets.rs` (about 1,100 lines, lines 458-870 for button, switch, slider, segmented and select) is a real control kit: focus ring, accessibility roles, slider keys, a wrapping radio group. The overlay windows use `secondary::chrome` instead, so `crates/app/src/todos/view/form.rs` (around line 239) has its own segmented control, and `todos/view.rs` redefines the same cream focus ring (around line 61, `FOCUS_RING`) and segment track (around line 68, `SEGMENT_TRACK`). The Settings To-dos page does use the shared `widgets::switch` and `button`.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Share behaviour, not necessarily looks: the review cautions that the overlay windows and Settings may need different styling. Either (a) extract only the key handling (arrow, Home, End on a radio group) into one function both call, or (b) keep the controls Settings-only and delete the to-do copy by calling a chrome-level segmented control if three overlay windows need one. Do not promote the settings kit to a global UI kit for one extra caller.

## How to verify

Arrow, Home and End behave the same in the Settings segmented control and the to-do form's (whichever the project chooses). Existing to-do keyboard tests and Settings keyboard tests pass. The focus ring and segment track constants exist in one place per styling family.

## Related

A13 (TASK-25.23, Settings split; do it first), A15 (TASK-25.25).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Arrow, Home and End keys behave identically in every segmented choice in Settings and the to-do form.
- [ ] #2 The focus ring and segment track values are defined once per styling family, not copied between windows.
- [ ] #3 Existing to-do and Settings keyboard tests pass.
<!-- AC:END -->
