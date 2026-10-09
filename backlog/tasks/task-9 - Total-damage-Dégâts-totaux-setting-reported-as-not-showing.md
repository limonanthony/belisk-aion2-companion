---
id: TASK-9
title: Total damage (Dégâts totaux) setting reported as not showing
status: Done
assignee: []
created_date: '2026-10-06 22:02'
updated_date: '2026-10-09 12:36'
labels:
  - bug
dependencies: []
type: bug
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A player who turns on Total damage expects to see it; a friend running the shared build reports that it does not show.

Reported (verbatim, apparently French): "Dégats totaux settings ne s'affiche pas" (reported by the user's friend, 2026-10-07, build 01bc913)

## Scope

Covers the Total damage overlay setting (French "Dégâts totaux") in Settings and its effect on the meter, in French and English. Other overlay settings are not covered.

## Evidence notes

Observed: only the report above (≈ "the Total damage setting does not show / is not displayed"). No screenshot or log.

Derived: "Dégâts totaux" is the French label of the Settings → overlay key `settings-overlay-total-damage` (crates/app/i18n/fr/aion_2_dps_meter.ftl:261).

Unknown:
- Whether the setting itself is missing from the Settings window, or switching it on does not show total damage on the meter.
- Reproduction steps.
- Environment details beyond the build: shared zip of commit 01bc913 (v2.4.0).

Cause: Unknown.

Related: TASK-7 (English and French translations).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With French as the language, the Total damage (Dégâts totaux) setting is shown in Settings and, when on, total damage is shown on the meter.
- [ ] #2 The same holds in English.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: the setting worked in build 01bc913 too: it adds total damage to the Target board only, and the Session board always shows total damage, but the Settings row did not say so then. Since d110c7b (2026-10-07) the row reads 'On the Target board. The Session board always shows it.' / 'Sur le tableau Cible. Le tableau Session l'affiche toujours.' User: 'It's all good you can put this ticket as done.'
<!-- SECTION:NOTES:END -->
