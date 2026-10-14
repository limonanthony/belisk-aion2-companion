---
id: TASK-25.23
title: Finish splitting the Settings window into one file per page
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The Settings window's code is one very large file. Adding a control or a section means hunting through it, and a test that pins the whole tab order has to be edited for every new control.

## Why it happens

`crates/app/src/settings/view.rs` is about 2,866 lines. A `view/` folder already exists, but only the To-dos page (`view/todos_page.rs`) and the capture-check wording (`view/capture_check.rs`) were moved into it. The page methods still in the main file: `overlay_page` (around line 802), `behaviour_page` (1014), `capture_page` (1090, about 340 lines), `bests_page` (1433), `reminders_page` (1587), `general_page` (1687) and `about_page` (1697); keyboard handling, focus, opening the window and the tests take the rest. The test `tab_walks_the_sidebar_then_the_page_in_reading_order_and_wraps` (around line 2438) pins an exact sequence of 22 debug-selector ids, so any legitimate extra control fails it. The half-finished split is a second convention in the same folder.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Finish the split already started: move each page into `settings/view/<page>.rs` as an implementation on the same settings view, the same pattern as `todos_page.rs`. Keep the window chrome, focus handling, selects and tests in `view.rs` until a page file needs them. Check whether the tab-order test can name controls by accessible name rather than a hard-coded list of 22 ids.

## How to verify

Behaviour of every Settings page is unchanged (the existing GPUI keyboard and layout tests for Settings pass unchanged apart from moved code). `view.rs` is left with the window shell, focus and tests. No page is defined in two places.

## Related

A14 (TASK-25.24, shared controls) and A15 (TASK-25.25, Settings change fan-out) touch the same files; do this one first to avoid conflicts. TASK-13 (Settings window opens behind other windows, In Progress) edits the same window code, so coordinate.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each Settings page (overlay, behaviour, capture, bests, reminders, general, about) lives in its own file under settings/view/.
- [ ] #2 Every Settings page looks and behaves as before and the existing Settings tests pass.
- [ ] #3 No page method is left in the main settings view file.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present per the review: settings/view.rs is about 3,345 lines. This round only changed its store observer (see TASK-25.25); the split was not done.
<!-- SECTION:NOTES:END -->
