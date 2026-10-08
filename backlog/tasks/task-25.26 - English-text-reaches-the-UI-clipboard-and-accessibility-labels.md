---
id: TASK-25.26
title: 'English text reaches the UI, clipboard and accessibility labels'
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: medium
type: bug
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Some text reaches the player in English only, including in the French interface: on the waiting card, in the clipboard text people paste into party chat, and in places screen readers read. The project rule is that every user-visible text, including the clipboard and accessibility labels, goes through the translation system with both an English and a French entry.

## Why it happens

The meter chrome mostly uses `fl!`, but a few places were never converted. Exact places from the review (commit 7a48824):

Overlay and meter (app-meter report):
- The header or capsule fallback "DPS Meter" in `crates/app/src/meter/view.rs` (around lines 555, 787 and 1145) and `meter/live.rs` (around line 430).
- The clipboard sign-off hard-coded in both the live and the summary copy paths (`meter/summary.rs` around line 311; and the live path).
- The tool buttons (reset, lock, copy, more, close) are icon-only with element ids but no accessibility label (`meter/view.rs` around lines 1453-1476).
- A core capture-failure reason is shown raw as the hint on the waiting card (`meter/view.rs` around line 784).

Settings and to-dos (settings report):
- `"/s"` on the personal-best rows (`settings/view.rs` around line 1552).
- The hour label `"{hour:02}:00"` (`settings/view.rs` around lines 2189-2190) and the `ClockTime` Display (`clock/wall.rs` around lines 24-26); a to-do service test expects `"21:00"`.
- The `"--"` for an unknown recording time (`settings/runtime.rs` around line 428).
- The product name in `settings/view.rs` around lines 1718, 1847 and 1896, and the driver names "Npcap" and "WinDivert" (`settings/view/capture_check.rs` around lines 55-56): the review says the product and driver names may stay English.
- The to-do class name is title-cased from the game's all-capitals name with no translation (`todos/list.rs` `class_name`, around lines 240-243).

French copy elsewhere uses `min` and `h` through Fluent, so these stand out.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Add translation keys in both the English and the French files for the fallback title, the clipboard sign-off, the unknown-time and per-second units and the hour format; give each tool button an accessibility label from `fl!`; map known capture failure reasons to keys, or keep the raw core text on the waiting card only if it is already translated at the core boundary. Leave the product name and driver names. When the product is renamed (TASK-23) the product-name literals change anyway.

## How to verify

With the interface in French, the waiting card, the copied clipboard text, the personal-best unit, the unknown recording time, the hour labels and the tool buttons' accessible names all appear in French, and in English with the English interface. The existing en/fr key-parity test passes and a test covers the clipboard sign-off per language.

## Related

TASK-7 (English and French translations, In Progress) set the rule and its five acceptance criteria; this ticket lists the places it missed, so check there first so work is not duplicated. TASK-23 (Kisk rename) for the product name literals.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In the French interface the meter's fallback title, the clipboard sign-off, the personal-best per-second unit, the unknown recording time and the hour labels are French.
- [ ] #2 Every tool button on the meter has an accessible name in the current language.
- [ ] #3 A key-parity test passes for the new keys in both languages.
<!-- AC:END -->
