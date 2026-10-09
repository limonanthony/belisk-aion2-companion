# Updates from GitHub Releases, and the changelog: mockups

**Approved on 2026-10-09.** The user answered "Approve all": every one of
the 21 Proposed items below is agreed as drawn; and "All OK" to the answers
given for the open questions (below, "Agreed (2026-10-09, were open)"). The
round covered TASK-28 (updates), TASK-28.1 (check, update on click,
restart), TASK-28.2 (download by itself, throttled) and TASK-29 (the
changelog in the app). The
approved design now lives in `../approved/`: the toolbar's update icon in
[toolbar.html](../approved/toolbar.html?states=update,updateTip,updateDownloading,updateDownloadingTip,updatePaused,updateVerifying,updateReady,updateReadyTip,updateReadyFight,updateFailed,updateWarn,updateFolded,updateFoldedWarn,updateFocus,updateFr,updateFrDownloading,updateFrReady,updateRestarted,tryUpdate)
(drawn by `hub.js`, styled in `hub.css`),
[What's new](../approved/whatsnew.html), Settings › About › Updates and
About › Changelog in
[settings.html](../approved/settings.html?states=about,about:checking,about:justchecked,about:available,about:downloading,about:paused,about:verifying,about:ready,about:failed,about:offline,about:limited,about:reset,changelog,changelog:other,changelog:older)
(drawn by `updates.js`, styled in `updates.css`), the product name in
`app-name.js`, and the "Updates and the changelog" values in
[tokens.md](../approved/tokens.md). Build from there; this folder is the
history of how it was chosen and is not updated any more.

Open in a browser, no server needed: start at [index.html](index.html), which
links every page and every state (`?states=<id>` opens one state alone;
Settings takes `&lang=fr`).

| Page | What it shows |
|---|---|
| [index.html](index.html) | Every page and state, the behaviour that is not drawn, what to answer |
| [toolbar.html](toolbar.html) | The toolbar's update icon: 20 states (available, downloading, paused, checking, ready, ready during a fight, failed, folded, French, after the restart) and a working one ("Try it") |
| [settings.html](settings.html) | Settings › About › Updates in each state (11), and Settings › About › Changelog (3); English and French |
| [whatsnew.html](whatsnew.html) | The What's new window: one version, skipped versions, scrolled, toolbar low, during a fight, fresh install, French, and the Markdown subset beside its source |

Built on the approved design, which is not edited: the pages load
`../approved/shared.css`, `overlay.css`, `todos.css` (the bundled Barlow Semi
Condensed), `meter.css`, `hub.css`, `app.css`, `kit.js`, `hub.js` and
`app.js` as they are. The toolbar is the approved `Hub.toolbar()` drawing;
this round only inserts its slot. The main window is the approved
`App` shell; About's existing rows use the build's Fluent strings
(`settings-about-*`, en and fr).

| File | What it is |
|---|---|
| `updates.js` | The product name constant, the two invented changelog files, a parser for the proposed Markdown subset and its parity check, the strings (en, fr), the update slot, the What's new window, About's Updates group and the Changelog page; Lucide icons not in `kit.js` / `app.js` (paths from gpui-kit-assets 0.6.6) |
| `updates.css` | Only what this round adds (values from tokens.md; new ones marked "new"), plus page-only chrome (the Markdown source block, decision tags and the index, copied from `../todos-autofill/autofill.css` and `../hub/hub.css`) |

Versions, sizes, dates, times and the changelog's text are invented. The
build runs 1.5.1 and GitHub's latest release is 1.6.0 (4.2 MB). When this
round was drawn the reset number (0.1.0 or 1.0.0) was not decided; it is
1.0.0 now, so the invented history starts there.

**The product name** comes from one constant, `APP` in `updates.js`,
standing for the Fluent term `-app-name` ("AION 2 DPS Meter" for now; the
name is being reconsidered, TASK-23). The approved toolbar drawing said
"Kisk" in its quit tooltip; these pages replaced it with `APP` so every name
on them came from that one place, as the build's `toolbar-quit = Quit
{ -app-name }` already does. On promotion the constant moved to
`../approved/app-name.js`, which the approved toolbar now reads too
(toolbar.html and whatsnew.html here load it for that reason). Since
2026-10-09 the name is Belisk, in that file, so this round's toolbar says
Belisk too; its About, drawn from its own `APP`, keeps the old name.

## Screenshots

Taken with a headless browser from these pages (dark scene unless named),
kept out of the mockup folder, in `target/update-mockups/`:

| File | States |
|---|---|
| `toolbar-states-en.webp` | Every toolbar state, English |
| `toolbar-states-bright.webp` | Available, downloading, paused, ready over the bright scene |
| `toolbar-fr.webp` | French: available, downloading, ready during a fight |
| `toolbar-after-restart.webp` | The toolbar back after the restart, with What's new under it |
| `about-en-ok-checking.webp` | About: up to date, checking, just checked |
| `about-en-download.webp` | About: available, downloading, paused, checking the download, ready |
| `about-en-errors.webp` | About: failed its check, GitHub unreachable, GitHub limiting |
| `about-fr.webp` | About in French: up to date, available, downloading, ready, failed, limiting |
| `changelog-en.webp` / `changelog-fr.webp` | Changelog: newest open, another opened, after Show older |
| `whatsnew-en.webp` / `whatsnew-fr.webp` | What's new: one version, skipped versions, scrolled, toolbar low, pointer on Full changelog |
| `whatsnew-subset.webp` | The Markdown source beside the window drawn from it |

## Decisions

### Agreed (user, 2026-10-08 and 2026-10-09)

1. **Source.** Updates come from GitHub Releases.
2. **Step 1, manual update.** An Update button appears when a new release is
   detected; a click downloads, verifies and stages it; then a Restart
   button; Restart closes the app and starts the new version.
3. **Nothing installs silently.** Installing always waits for the player's
   click on Restart.
4. **Download updates automatically: on by default, always throttled.**
   "On by default but download is always throttled not only during
   fights." The download is always speed-capped, and also paused while you
   fight.
5. **Where.** "Settings -> about, i'm ok with that but a small button in the
   tool bar with a hover tooltip could be good too no ? (to say an update it
   available or has been downloaded when auto-updates are on)". So About
   shows the status, "Check now" (at most once a minute), the switch and
   the errors; and a small toolbar icon appears only when there is
   something to do or know (available, downloading, ready), with a tooltip
   after 500 ms like the other tools, never stealing focus or flashing.
6. **Checks.** At launch, then once an hour, never more often (GitHub's
   unauthenticated limit, 60 requests an hour per address).
7. **Changelog files.** `CHANGELOG.md` and `CHANGELOG.fr.md` (a future
   language `CHANGELOG.<code>.md`), rendered natively in the app's look.
8. **What's new.** After an update, only the versions newer than the last
   one this player ran, up to the current one (skipped versions included).
9. **Settings › About › Changelog.** The full history: the newest version
   expanded, older ones collapsed to a header (version, date, summary), and
   "Show older" beyond the most recent 5–10, in a list that draws only what
   is on screen.

### Agreed (2026-10-09, "Approve all": accepted as drawn)

**The toolbar's icon**

1. **Place.** First in the end group, before fold: `… warning | update « ×`.
   It is about the app, like fold and quit, and the tools and counts keep
   their places when it comes and goes. A 26 px tool (not a 20 px end
   button) because it carries an action. Folded, the handle keeps it after
   the warning. ([toolbar.html](toolbar.html?states=available,folded))
2. **When it shows.** Available, downloading, paused, checking the download,
   ready, and after a failed download. Up to date, checking for updates,
   GitHub unreachable or limiting: no icon (About only).
3. **Looks, no new hue.** Available: Lucide `download` in text on the soft
   fill (a tool at rest has no fill, so it reads as "something waits");
   downloading: the same icon in muted with a 2 px progress line inset at the
   bottom (the reminder card's time-left line: track `gauge-empty`, fill
   `muted`); paused: the line in `faint`; checking: the line full; ready:
   Lucide `rotate-cw` in `good` on `chip-good`. Pointer: lights on enter,
   goes on leave; the line changes only when the download progresses;
   nothing pulses.
4. **Clicks.** Available: the download starts at once, no confirmation.
   Downloading, paused or checking: opens Settings › About, where Cancel and
   the details are. Ready: restarts at once, no confirmation, as quit has
   none.
5. **Tooltips** as drawn: "Update available: 1.6.0 / Click to download it.
   It installs only when you restart."; "Downloading 1.6.0 · 62% /
   Speed-capped, paused while you fight. Click for details."; "Download
   paused · 62% / Resumes when the fight ends. Click for details.";
   "Checking 1.6.0… / Making sure the download is complete and genuine.";
   "Restart to update to 1.6.0 / Closes and opens again in the new version.
   Nothing is lost."; French drafts beside them. The button's accessible
   name is the first line; the line is a progressbar.
6. **A failed download** (hash or signature mismatch: deleted, nothing
   installed) shows the available look again, with the tooltip "The last
   download failed its check and was deleted. Click to try again." No
   colour, no pop-up during play. With automatic download on, the app
   retries at the next hourly check, never in a loop.
7. **Restarting during a fight** stays possible: the tooltip warns "You are
   fighting: restarting now splits this fight in History." Not chosen:
   hiding Restart until the fight ends, or a "Restart after this fight"
   that arms itself (answers TASK-28's open question; easy to swap).
8. **After the restart** the toolbar comes back where it was, and the tools
   that were open open again in their places (the meter too, so fights keep
   being tracked); then What's new.

**The download**

9. **Speed cap 1 MB/s**, and paused while a fight is active (the core's
   activity rule), in both modes: the same rules apply when the player
   clicked Update. (TASK-28.2 suggested 1–2 MB/s.) Turning the switch off
   or Cancel during a download deletes the partial file.

**Settings › About**

10. **An "Updates" group right after the app's own group**, so it shows
    without scrolling: the status row (label, one sentence, its action on
    the stable right edge), "Download updates automatically" (on), and
    "Changelog" with "Open changelog". The switch lives here, not in
    General: everything about updates in one place, the place the user
    accepted, and General holds only the language.
11. **Status rows** as drawn ([settings.html](settings.html)): "Up to date /
    1.5.1 is the latest version. Checked at 21:02; checked again at launch
    and every hour." with "Check now"; "Checking for updates…" with the
    spinner badge; Check now disabled for a minute after any check ("Checked
    just now; you can check again in a minute."); "Version 1.6.0 is
    available" with "Release notes" (ghost) and the primary "Update to
    1.6.0" (automatic download off only: with it on, the download starts at
    once); "Downloading 1.6.0" with the Slider's track as progress, "2.6 of
    4.2 MB · at most 1 MB/s, paused while you fight" and Cancel; "Download
    paused" (the range dims); "Checking the download"; "1.6.0 is ready to
    install" with the primary "Restart to update". One bright button per
    view, as the main window's rule says.
12. **Errors**, never a pop-up: a failed check is a field message in `bad`
    under the status row ("The last download did not pass its check
    (signature mismatch), so it was deleted. Nothing was installed.") with
    "Try again"; GitHub unreachable or limiting is a field message in `warn`
    (new: field-msg had only `bad`) with the next try's time; when GitHub
    limits, Check now stays disabled until GitHub's reset time.
13. **What is contacted**, in the group title's faint note: "Asks GitHub for
    the latest release; nothing about you or your game is sent." (The README
    must say it too: TASK-28.)
14. **Notes before updating**: "Release notes" opens the GitHub release page
    in the browser. Not chosen: the release body shown in the app (GitHub's
    body is in one language, and its links need a browser anyway). Answers
    TASK-29's open question.
15. **The changelog is a sub-page of About** with a back button ("‹
    About"), as the fight report opens a player in place, not a window of
    its own. Rows follow tokens.md's expandable rows: the chevron at the
    row's end, one open at a time; collapsed: the version (display type), the
    summary cut to one line, the date. Five versions first; "Show older"
    adds five each time, with "5 of 9 versions" beside it; then "All 9
    versions".

**What's new**

16. **Its own overlay window, opened once by itself** on the first start of
    a newer version: History's frame (370 px, header with close, 26 px
    footer), 6 px under the toolbar with left edges aligned (above it when
    the toolbar sits low), like the reminder card while the meter is
    closed. No-activate like every overlay tool; scrolls with the wheel;
    stays until closed; not a toolbar tool. Not chosen: a card like the
    reminder card (too small for several versions, and a 30 s lifetime does
    not suit reading); an icon in the toolbar only (easy to miss).
17. **When not.** Never during a fight (it waits until the fight ends); a
    fresh install opens nothing; a version entry without bullets does not
    open it. The versions count as seen once the window is closed (× or
    "Full changelog"); quitting with it open shows it again at the next
    start.
18. **Layout** as drawn ([whatsnew.html](whatsnew.html)): Lucide
    `scroll-text` and "What's new", a `chip-good` "Updated to 1.6.0"; with
    skipped versions a faint line "Since 1.4.0, the last version you ran";
    each version newest first and all open (they are all new): the version
    13.5 px / 600 and its date in faint; the summary 12.5 px / 500 in text;
    the bullets 12 px / 500 in muted with a 4 px faint dot; 14 px between
    versions, no rules; the list scrolls past 318 px with the 3 px thumb;
    footer: a quiet "Full changelog" that opens Settings › About ›
    Changelog and closes the window.

**The files**

19. **The Markdown subset**: `## [1.6.0] - 2026-10-21` headings; one
    summary paragraph; `- ` bullets, one level (a bullet may continue on
    lines indented by two spaces); `**bold**` inside the summary and
    bullets. **Bold yes** (a feature or a setting path, "**Settings ›
    Reminders**"). **Inline code no** (players never need it). **Links no**
    (a click in a no-activate overlay would open a browser over the game,
    and the notes are on GitHub anyway). The app's parser fails on anything
    else; `cargo test` parses both files with it and checks parity (same
    versions, same bullet counts). The page's own parser does exactly this
    ([whatsnew.html?states=subset](whatsnew.html?states=subset)).
20. **Dates** are written ISO in the files and shown "21 Oct 2026" / "21
    oct. 2026"; sizes "4.2 MB" / "4,2 Mo"; percentages "62%" / "62 %".
21. **French copy** as drafted on the pages (« Mise à jour disponible »,
    « Redémarrer pour mettre à jour », « Télécharger les mises à jour
    automatiquement », « Nouveautés », « Journal des modifications »); the
    real text goes through Fluent.

### Agreed (2026-10-09, were open; "All OK" to the recommended answers)

1. **No switch to turn update checks off** (TASK-28). The automatic
   download can be turned off, but the hourly check always runs; the README
   says what is contacted (GitHub, for the latest release; nothing about the
   player or the game is sent).
2. **No beta channel** for now (TASK-28): the app reads `/releases/latest`.
3. **A settings file from before the version reset** (no
   `last_seen_version`, TASK-29) is not a fresh install: those players come
   from an older build and see the reset version's notes (1.0.0) once.
4. **Aether's history** (TASK-29, TASK-26.5) moves out of `CHANGELOG.md`
   into a file of its own, credited to Aether; the in-app changelog starts
   at the reset version.
5. **Signing** (TASK-28): an ed25519 key, minisign-style. The failure
   message keeps the word "signature"; the public key is in the build.
6. **A newer release while one is staged**: the hourly check goes on while
   an update is available or staged, so a newer release replaces a staged
   one (TASK-28.1's "polling stops once an update is found" does not hold).
7. **The reset version number** is 1.0.0.

## Not decided by the mockup

- The app's name: one constant (`../approved/app-name.js`), the Fluent term
  `-app-name` in the build; the rename (TASK-23) changes it in one place.
- Icons to add from Lucide (ISC) when built: `download`, `rotate-cw`,
  `scroll-text`, `cloud-off`, `pause` (all in gpui-kit-assets 0.6.6);
  `chevron-left`, `chevron-right`, `loader-circle`, `clock`,
  `triangle-alert`, `settings`, `check` are in `crates/app/assets/icons/`
  already.
- GPUI: the What's new window is an overlay window with the meter's styles
  (transparent, no-activate, tool window), sized to its content up to the
  list's maximum height; Windows scrolls inactive windows under the pointer
  by default. The progress line redraws only when the percentage changes.
- The CI release workflow (TASK-30), the lean update archive, its hash and
  signature are prerequisites, not drawn.
