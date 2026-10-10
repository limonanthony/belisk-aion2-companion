---
id: TASK-28.2
title: 'Download updates automatically, throttled, then only ask to restart'
status: In Progress
assignee: []
created_date: '2026-10-08 15:23'
updated_date: '2026-10-10 02:28'
labels:
  - feature
dependencies:
  - TASK-28.1
parent_task_id: TASK-28
type: feature
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Step 2 of the update feature: an "auto update" setting. When it is on, the app downloads, unpacks and stages a new release by itself, and the player only sees a prompt to restart to update.

## Agreed (user, 2026-10-08)

- A setting enables auto update.
- With it on, the app downloads, unpacks and stages the update without a click.
- The download is throttled so it does not take too much of the network while the player is playing.
- The player then only sees "restart to update". A button or an icon with a hover tooltip are both fine; the user has no preference.
- Installing still waits for the player's click on Restart. The app never restarts by itself.

## How it works (Proposed, not agreed: the design from the discussion)

- **Same process as step 1.** The setting only lets Available move to Downloading without a click. Verification, staging, the restart order and the failure handling are step 1's.
- **Pause during fights, not only a speed cap.**
  - The core already knows when the player is fighting: `DataStorage::activity_at`, the same activity rule the idle reset uses.
  - Pausing the download while a fight is active protects ping better than a cap alone.
  - Keep a modest cap as well, around 1–2 MB/s, for open-world play where there is no clean "between fights".
  - The lean update archive is small (exe plus WinDivert files), so the download finishes quickly between fights.
  - Windows' BITS service (downloads on idle bandwidth) would also work, but it needs much more code than a read loop that sleeps.
- **Restart indicator.**
  - Once staged, the footer indicator changes to "Restart to update" (an icon whose tooltip says so, or a button).
  - It reacts only when the pointer enters or leaves, like every card action.
  - Settings → About shows the same status.
  - Text goes through `fl!` in English and French.
- **Setting.**
  - "Download updates automatically" in Settings, stored in `settings.json` like the other switches.
  - The default is an open question on the parent.
  - Turning it off during a download cancels the download and deletes the partial file.

## Out of scope

- Restarting by itself.
- Downloading or installing betas, unless the parent's beta-channel question is answered.

## How to verify

- With the setting on and a test release published, the update downloads without a click and the restart indicator appears.
- During an active fight (a replay works for this), the download pauses, and it resumes after the fight.
- The download never goes above the cap.
- With the setting off, the behaviour is step 1's.
- Unit tests for the pause/resume decision, driven by activity, and for the cap.

## Related

- TASK-28.1: step 1 (depends on it).
- TASK-25.1: the snapshot loop and activity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With the auto update setting on, a detected update downloads, unpacks and stages without a click, and the player then sees only a restart-to-update prompt
- [ ] #2 The download is throttled so it does not use the whole connection while the player is playing
- [ ] #3 The app never restarts or installs an update without the player's click on Restart
- [ ] #4 With the setting off, updating works as in step 1
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: built and reviewed; see TASK-28's note.

2026-10-10: see TASK-28: v1.0.1 fixes the v1.0.0 update-check crash; the real update test (1.0.1 → 1.0.2) remains.
<!-- SECTION:NOTES:END -->
