---
id: doc-1
title: PM conventions
type: other
created_date: '2026-10-05 14:23'
updated_date: '2026-10-05 14:23'
---
How product work for AION 2 DPS Meter is tracked, prioritised, labelled, grouped, and accepted.

## Where records live

- Work items are tracked in Backlog.md in this repository (`backlog/`).
- Product documentation lives in Backlog.md documents (`backlog/docs/`).
- Technical documentation (architecture, implementation, tests) stays in `docs/` in the repository.
- Backlog.md records stay uncommitted until the user asks for a commit through the committer.

## Statuses and validation

- Statuses run To Do → In Progress → Done.
- The user tests in game before work is committed or completed, to avoid committing work that does not function.
- Work tested only by an agent stays In Progress.
- A task moves to Done only after the user says they tested it and accept it.

## Priorities

The configured priorities are High, Medium, and Low. The project has stated no overrides, so the shared defaults apply:

- High: agreed user-blocking breakage, and agreed near-term work. No urgent level is configured, so urgent breakage also takes High.
- Medium: accepted work without a date.
- Low: accepted work that is not planned soon.
- No priority: a bare captured idea.

## Labels

- Each task carries one category label, chosen by what that task changes: `bug`, `feature`, `improvement`, `documentation`, `maintenance`, or `research`.
- `research` means the deliverable is an answer, not a change.
- Backlog.md labels are free-form text without colours, so the shared palette's colours are not used here.

## Grouping and planning

- Related work is grouped as a parent work item with subtasks when needed. There are no epics.
- No milestones, due dates, or planning cadence have been agreed, so tasks carry none.
