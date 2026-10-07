# End-of-fight report for boss fights: proposals

**Approved on 2026-10-07: direction C with the player details inside the
report, the crown on boss records** (user: "yeah use a crown, you can start
once you're done cleaning up the mockups", with every decision below). The
approved design now lives in `../approved/`: [the fight report and the
player details](../approved/report.html), the summary card's "Open the
fight report" button in [meter.html](../approved/meter.html?states=final,final-hover,provisional,summary-party,summary-fr,summary-locked),
the crown and report button on boss records in
[history.html](../approved/history.html), and the "Fight report" values in
[tokens.md](../approved/tokens.md). Build from there; this folder is the
history of how it was chosen and is not updated any more.

Open [index.html](index.html) in a browser (no server needed): the overview,
the chosen pages first, the boss-icon comparison, the decisions, then the
pages that were not chosen. Each page draws every state over three game
scenes, with the overlay's opacity and content scale and a "Mask names" and
"Server" switch in the controls; [entry.html](entry.html) adds a "Boss icon"
control for the crown and skull comparison, [align.html](align.html) an
"Alignment" control that switches the whole page between the decided
alignment and the three not chosen.
The pages use `../approved/shared.css`,
`../approved/overlay.css`, `../approved/kit.js`, the bundled Barlow Semi
Condensed, class icons and skill icons from `crates/app/assets/`; their own
code is [report.js](report.js) (invented fights and the shared pieces) and
[report.css](report.css) (new values, plus copies of the approved detail,
summary-card and history pieces so they look exactly as approved).

Game names are data: Basilus the False (Fallen Guardian's City, Hard),
Tyrant Tassin, Superbia, Sacred Beast of Life and the monsters from
`crates/core/data/npc_names_en.json`; class, heal and monster skills from
`skill_names_en.json` and `healing_skill_code.json`; servers from
`servers.json`. Which monster used which skill, the players and every figure
are invented.

## Scope (from TASK-15)

- Boss fights only; a button after the fight opens the report.
- For every player: damage and DPS, healing, damage taken, deaths (who, when
  in the fight as mm:ss, what killed them).
- A player's details: damage per skill, critical hits and the other hit
  types (back, front, parry, perfect, double, multi-hit) as counts, buff and
  skill uptime (as m:ss), heals per skill (no overheal), damage taken by
  monster skill, their deaths.
- Not included: a boss health graph, a hit-by-hit timeline.

## C, one table, every figure (chosen 2026-10-07)

[c.html](c.html). The user: "the table shows more infos without clicking".
A new 520 px overlay window (the detail window's width, card, header and
key-figure plate): "Fight report", the outcome chip and ×; the boss, where,
when, how many players and the fight time; then Damage, Party DPS, Healing,
Damage taken and Deaths on one plate.

- One row per player, the meter's bar and rank badge (cream for you):
  Damage, DPS, Healing, Taken, Deaths. A click on a header sorts by it; the
  bars and ranks follow the sorted column. Damage first; the sort is kept
  while the window is open. Nothing in a column: "—" and no bar; the name
  stays.
- Deaths listed under the table: time, player, "by" the monster and its
  skill, the last hit. Nobody died: one green line.
- Pointer on a row: the bar lifts by `soft`, a chevron shows (enter and leave
  only). A click on a row or a death opens that player inside the report.
- Fights saved before this change: Healing, Taken and Deaths show "--",
  cannot be sorted, and a note says they were not recorded.

States: fresh report, sorted by healing (a player with no heals), sorted by
damage taken, pointer on a row, keyboard focus on a row, nobody died,
several deaths (one player twice), 14 players on a field boss (scrolls), a
player with no damage taken, long names with server tags (ellipsis), fight
ended (may resume), a fight saved before this change, a player opened.

## Player details (chosen: inside the report)

[player.html](player.html). The approved detail window's layout, extended:
Healing, Damage taken and Deaths join the key figures; the hit types show
the count and the rate ("665 41.9%"); the skill table adds Crits beside
Crit %; a skill row opens to every hit type as count and rate. Four tabs:
Damage, Buffs (on the player, then on the boss: caster, a coverage bar, up
for "4:01" and the share of the fight), Healing (per skill: heals, crits,
amount) and Damage taken (per monster skill: hits, largest hit, amount, in
the health colour). A player who died has each death on a light `bad` plate
under the figures. Opens in the report window from a player's row or their
line under Deaths; "‹ Report" goes back to the report as it was left.

States: a player opened (Damage), a skill opened, pointer on a skill row,
pointer on "‹ Report", Buffs, Healing (the cleric), a player with no heals
(and two deaths), a player who died, Damage taken (the templar), no damage
taken, keyboard focus on a tab, long names (and a skill without an icon), a
fight saved before this change, and, not chosen, the same content in the
detail window.

## Opening the report (chosen: the wide button and History)

[entry.html](entry.html). The approved boss summary card gets "Open the
fight report", a full-width tonal button under the skills, on the final card
and on the provisional "Fight ended, may resume" card. Locked
(click-through): no button, as for Copy and × today. History: a 20 px report
button on every boss record before Delete, never on mob records.

States: the button, pointer on it, fight ended, party summary, French,
locked; History with the crown, History with the skull (side by side),
pointer on a report button (tooltip), keyboard focus on an old record's
button; then the placements not chosen: "Report" beside Copy and its French,
the header icon with its tooltip, "Fight report" only inside an opened
record.

## Decided 2026-10-07

The user settled every choice; reasons are on [index.html](index.html).

1. Report layout: C, one table ("the table shows more infos without
   clicking"). A, tabs ([a.html](a.html)) and B, one scrolling report
   ([b.html](b.html)) were not chosen; they stay in this folder as
   exploration history.
2. The button on the summary card: "Open the fight report", full width under
   the skills. "Report" beside Copy in the footer and an icon in the header
   were not chosen.
3. Also from History: a report button on each boss record, only on boss
   records. Boss records also get a small icon before the boss's name, so
   they stand out from mob records: the crown (chosen with the final
   approval, see below). "Only inside an opened record" was not chosen.
4. Player details open inside the report with "‹ Report", from a player's
   row or their line under Deaths. The detail window was not chosen.
5. Uptime as m:ss, "4:01".
6. Hit types as count and rate, "665 41.9%".
7. The report is also offered from the provisional "Fight ended, may resume"
   card.
8. Damage taken by source: per player only, in the player details.
9. C sorts by Damage first and keeps the sort while the window is open.
10. Figure alignment: plates left, tables right. The user: "it looked better
    when it was aligned start + table aligned end". See below.

## Figure alignment: plates left, tables right (decided 2026-10-07)

[align.html](align.html), also on [index.html](index.html). The key-figure
and hit-type plates keep label above value at the start (left) of each
cell, as the approved detail window does; every table puts its figures and
headers at the end (right) of each column, where decimal points and the K
and M suffixes line up down the column: the report's player table (Damage,
DPS, Healing, Taken, Deaths), the deaths list's time and last hit, the skill
table's Hits, Crits, Crit % and Damage, the Healing and Damage taken tabs'
columns and the Buffs tab's "Up for" and "of 5:12". Names, skill names,
icons, rank badges, bars and chevrons do not move.
[c.html](c.html) and [player.html](player.html) draw it.

- Not chosen, kept on align.html as history: All start (everything left),
  All center, and All end (everything right, recommended before the
  decision so each window would follow one rule; the plates would have lost
  their left edge and the approved detail window's plates would have had to
  follow). The "Alignment" control (default "Plates left, tables right
  (chosen)", or `?align=chosen|start|center|end`) switches every state:
  `pair` (the report fresh, then Mirelle's details on the Damage tab),
  `buffs`, `healing`, `taken` and `deaths`. On align.html the deaths list's
  last hit gets a 46 px column so its alignment shows; c.html lets it hug
  the right edge, which draws the same in the chosen mix.
- Every table header sits over its figures. A list that scrolls draws its
  figures inside the 8 px thumb gutter, so its header ends 8 px earlier; a
  list too short to scroll has no gutter and neither has its header. Fixed
  with the decision: the Buffs tab's "Up for" and "of 5:12" sat 8 px right
  of their figures, and the Healing and Damage taken headers (and a Damage
  tab with 9 skills or fewer) sat 8 px left of theirs.

## The boss icon in History: the crown (chosen 2026-10-07)

Drawn side by side on [entry.html](entry.html) as the states `his-crown` and
`his-skull` (also on [index.html](index.html)); the "Boss icon" control
switches the other History states. Both are Lucide icons, 12 px at stroke 2
in `hp` (`#e8735f`, the boss gauge's colour), in a 12 px slot at the
record's left edge, 5 px before the name. Mob records keep the empty slot so
every name starts at the same edge.

1. Lucide `crown` (chosen, "yeah use a crown"): it says "boss" and means
   nothing else in the meter.
2. Lucide `skull` (not chosen): the report already draws the skull for
   deaths (the Deaths column, the deaths list, a player's death plate), so
   on a record it could read as "someone died in this fight".

## Not chosen (history)

- [a.html](a.html), A, tabs: the same window; under the figures four tabs,
  Damage, Healing, Damage taken and Deaths (with its count), each a ranked
  list with a share column; the Damage taken tab also lists the party's
  damage by monster skill.
- [b.html](b.html), B, one scrolling report: Damage, Healing, Damage taken
  and Deaths as folding sections read top to bottom, scrolling inside
  372 px.
- On [entry.html](entry.html): "Report" beside Copy (in French Copy has to
  lose its label), a file icon in the header with the tooltip "Fight
  report", "Fight report" only inside an opened History record.
- On [player.html](player.html): the player details in the detail window in
  history mode, beside the report.

## Not settled by these drawings

- Which effects count as heals (shields, life drain) is checked on a
  recording with a healer in TASK-15.2; overheal is not shown.
- The last hit before a death assumes the meter can tie the death to it;
  TASK-15.2 confirms it.
- The overlay windows never take keyboard focus from the game; the focus
  states are for accessibility tools, as on the to-do window.
