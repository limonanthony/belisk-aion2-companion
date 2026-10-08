## AION 2 DPS Meter, English (the fallback language). Every message exists in
## fr/aion_2_dps_meter.ftl too, with the same arguments; a test checks it.
## Keys are prefixed by their area: common-, unit-, duration-, weekday-,
## month-, day-, list-, todos-, reminder-, settings-, meter-, history-,
## detail-, pvp-.

## Common

common-you = You
common-target-numbered = Target { $id }
common-mob-numbered = Mob { $code }
common-session = Session
common-target = Target
common-player = Player
common-players = Players
common-players-count =
    { $count ->
        [one] { $count } player
       *[other] { $count } players
    }
common-damage = Damage
common-dps = DPS
common-cancel = Cancel
common-close = Close
common-dismiss = Dismiss
common-character = Character
common-server = Server

## Sizes (the value is already formatted)

unit-bytes = { $value } B
unit-kilobytes = { $value } KB
unit-megabytes = { $value } MB
unit-gigabytes = { $value } GB

## Durations, dates and lists

duration-days-hours = { $days } d { $hours } h
duration-hours-minutes = { $hours } h { $minutes } min
duration-minutes = { $minutes } min

weekday-short-mon = Mon
weekday-short-tue = Tue
weekday-short-wed = Wed
weekday-short-thu = Thu
weekday-short-fri = Fri
weekday-short-sat = Sat
weekday-short-sun = Sun

month-short-1 = Jan
month-short-2 = Feb
month-short-3 = Mar
month-short-4 = Apr
month-short-5 = May
month-short-6 = Jun
month-short-7 = Jul
month-short-8 = Aug
month-short-9 = Sep
month-short-10 = Oct
month-short-11 = Nov
month-short-12 = Dec

day-stamp = { $weekday } { $time }
list-and = { $rest } and { $last }

## To-dos

todos-section-today = Today
todos-section-recharges = Recharges
todos-section-this-week = This week

todos-sub-attempts = attempts
todos-sub-final-boss = final boss
todos-sub-final-boss-kills = final boss kills
todos-sub-runs-per-dungeon = 7 runs a dungeon

todos-left = { $count } left
todos-full = { $count } full
todos-hidden = { $count } hidden
todos-tracked = { $count } tracked
todos-done-of = { $done } of { $total } done
todos-all-done = all done
todos-done = done
todos-amounts-optional = amounts optional
todos-weekly-on-wednesday = Weekly on Wednesday
todos-not-tracked = not tracked
todos-value-of = { $value } of { $cap }

todos-resets-in = resets in { $time }
todos-next-full-in = next full in { $time }
todos-weekly-reset-when = { $day } · in { $time }
todos-amount-full-in = Full in { $time }
todos-full-being-wasted = Full, being wasted
todos-full-spend-by = Full, spend by { $stamp }

todos-everything-done = Everything is done on { $name }.
todos-amount-full-note = { $label } is full in { $time }.
todos-removed-note = Removed { $name }.
todos-added-note = Added { $name } to { $section }.

todos-back =
    { $count ->
        [one] { $count } to-do back
       *[other] { $count } to-dos back
    }
todos-restore-keeps = Restoring the defaults keeps your own to-dos and all progress.
todos-hidden-description-none = Nothing is hidden. { todos-restore-keeps }
todos-hidden-description =
    { $hidden } hidden, on { $characters ->
        [one] { $characters } character
       *[other] { $characters } characters
    }. { todos-restore-keeps }
todos-reset-dialog-body = On every character: every hidden to-do comes back, and every count and amount you entered is emptied.
todos-reset-dialog-body-with-own = On every character: your own to-dos ({ $own }) are removed, every hidden to-do comes back, and every count and amount you entered is emptied.
todos-reset-dialog-keeps = The reset hour and the membership stay. This cannot be undone.

todos-window-title = To-dos
todos-window-playing = Playing
todos-window-picker-note = Characters the meter has seen, with what each has left. Duty Missions are shared by your characters on one server.
todos-window-no-character-title = No character yet
todos-window-no-character-line = Your to-dos appear once the meter has seen your character. Start it before login, or relog via character select.
todos-window-undo = Undo
todos-window-change = Change
todos-window-change-name = Change the daily reset hour in Settings, To-dos
todos-window-daily-reset = Daily reset
todos-window-account = Account
todos-window-yours = Yours
todos-window-hidden-announce = { $label } hidden
todos-window-removed-announce = { $name } removed. Undo is at the top of the window.
todos-window-put-back-announce = { $label } put back
todos-window-announce-done = { $label }: done
todos-window-announce-not-done = { $label }: not done
todos-window-row-done = { $label }, done
todos-window-row-not-done = { $label }, not done
todos-window-remove = Remove { $label }
todos-window-hide = Hide { $label }
todos-window-put-back = Put back { $label }
todos-window-mark-done = Mark done
todos-window-mark-done-row = Mark done: { $label }
todos-window-undo-row = Undo: { $label }
todos-window-one-less = One less
todos-window-one-more = One more
todos-window-one-less-row = One less: { $label }
todos-window-one-more-row = One more: { $label }
todos-window-step-less-row = { $step } less: { $label }
todos-window-step-more-row = { $step } more: { $label }
todos-window-count-done = { $target } of { $target }, done
todos-window-show = Show
todos-window-show-name = Show { $hidden }
todos-window-hide-again = Hide again
todos-window-hide-again-name = Hide again: { $hidden }
todos-window-add = Add a to-do
todos-window-new = New to-do
todos-window-name = Name
todos-window-add-button = Add
todos-window-kind = Kind
todos-window-tick = Tick
todos-window-count = Count
todos-window-resets = Resets
todos-window-daily = Daily
todos-window-weekly = Weekly
todos-window-how-many = How many
todos-window-of = of
todos-window-stop-tracking = Stop tracking { $label }
todos-window-amount-input = { $label }: how many you have now

## Event reminders

reminder-event-shugo-festival = Shugo Festival
reminder-event-dimensional-invasion = Dimensional Invasion
reminder-event-rift-portals = Rift portals
reminder-event-watcher-kaira = Watcher Kaira
reminder-event-artifact-siege = Artifact Siege
reminder-event-abyss-field-bosses = Abyss field bosses
reminder-event-guardian-lord-nahma = Guardian Lord Nahma
reminder-event-arena-of-tactics = Arena of Tactics
reminder-event-reward-keys = Reward keys refill

reminder-detail-shugo-festival = 3 min to join
reminder-detail-dimensional-invasion = Verteron · Altgard · 3 min to join
reminder-detail-rift-portals = Altgard · Verteron · Morheim · Eltnen
reminder-detail-watcher-kaira = Field boss · Chaotic Lower Reshanta
reminder-detail-artifact-siege = Chaotic Lower Reshanta
reminder-detail-abyss-field-bosses = Lower and Middle Reshanta
reminder-detail-guardian-lord-nahma = Lv 65, and the Enraged Lv 80
reminder-detail-arena-of-tactics-early = 10 vs 10 queue open until 14:00
reminder-detail-arena-of-tactics-late = 10 vs 10 queue open until 21:00
reminder-detail-reward-keys = Shugo Festival and Invasion keys

reminder-test-title = Test reminder
reminder-test-event = Reminders look like this
reminder-starting-now = Starting now
reminder-in-minutes = In { $minutes } min
reminder-label = Event reminder: { $when }
reminder-label-event = { $name }: { $detail }
reminder-next = Next: { $names } at { $time }
reminder-off = Event reminders are off
reminder-none-on = No event is switched on

## Settings

settings-end-fight-never = Never

settings-window-title = AION 2 DPS Meter Settings
settings-sidebar-title = Settings
settings-sections = Settings sections
settings-section-overlay = Overlay
settings-section-behaviour = Meter behaviour
settings-section-capture = Capture
settings-section-bests = Personal bests
settings-section-reminders = Reminders
settings-section-todos = To-dos
settings-section-general = General
settings-section-about = About
settings-meter-status = Meter: { $status }
settings-status-running = Running
settings-status-stopped = Stopped
settings-status-not-running = Not running
settings-set-aside = The stored settings were set aside
settings-not-saved = Settings could not be saved

settings-overlay-lead = How the meter overlay looks. Changes show in the overlay at once.
settings-overlay-look = Look
settings-overlay-opacity = Background opacity
settings-overlay-opacity-text = Lower is more see-through. The text keeps its own plates.
settings-overlay-scale = Content scale
settings-overlay-scale-text = 70 to 150%.
settings-overlay-layout = Layout
settings-overlay-layout-text = Capsule is one line: your DPS, place and time. It opens on pointer over.
settings-overlay-layout-full = Full
settings-overlay-layout-capsule = Capsule
settings-overlay-percentage = Percentage
settings-overlay-percentage-text = Contribution is based on the visible list; share on the whole fight.
settings-overlay-contribution = Contribution
settings-overlay-share = Share
settings-overlay-shown = Shown on each row
settings-overlay-boss-hp = Boss HP gauge
settings-overlay-player-name = Player name
settings-overlay-player-name-text = Off hides names; icons and figures stay.
settings-overlay-mask = Mask nicknames
settings-overlay-mask-text = Shows Mi****e instead of the name, for streams and screenshots.
settings-overlay-combat-power = Combat power
settings-overlay-combat-power-text = Beside the name, where the game reported it.
settings-overlay-total-damage = Total damage
settings-overlay-total-damage-text = On the Target board. The Session board always shows it.
settings-overlay-boss-fights = Boss fights
settings-overlay-fight-summary = Fight summary
settings-overlay-fight-summary-text = When a boss dies: your DPS and place, top skills and personal best. One click copies it.
settings-overlay-pace = Pace against your personal best
settings-overlay-pace-text = After 10 s of a boss fight, ahead or behind your best on that boss.
settings-overlay-window = Window
settings-overlay-lock = Lock overlay
settings-overlay-lock-text = Clicks pass through to the game and the controls hide. Alt+CapsLock switches it.
settings-overlay-lock-unavailable = Alt+CapsLock could not be registered, so nothing could unlock the overlay.
settings-overlay-detail-opens = Player detail opens
settings-overlay-detail-opens-text = Beside the overlay, or in the middle of the screen.
settings-overlay-beside = Beside
settings-overlay-centred = Centred
settings-preview = Preview
settings-preview-note = Sample data. Follows every change on this page.

settings-behaviour-lead = What the meter counts, and what it does between fights.
settings-behaviour-counted = What is counted
settings-behaviour-count = Count
settings-behaviour-count-tip = With Boss only on and no boss in front of you, every hit is ignored and the overlay stays empty.
settings-behaviour-count-text = All targets records every mob you hit; Boss only ignores everything that is not a boss.
settings-behaviour-all-targets = All targets
settings-behaviour-boss-only = Boss only
settings-behaviour-possible-bosses = Detect possible bosses
settings-behaviour-possible-bosses-text = Treats very high-HP monsters missing from the boss table as bosses. Useful in new dungeons.
settings-behaviour-dummy = Training dummy only
settings-behaviour-dummy-text = Counts only damage on your own training dummy.
settings-behaviour-hide-unknown = Hide unknown players
settings-behaviour-hide-unknown-text = Leaves out players the meter could not name.
settings-behaviour-players-shown = Players shown
settings-behaviour-players-shown-text = 5 to 20.
settings-behaviour-between = Between fights
settings-behaviour-end-fight = End a fight after
settings-behaviour-end-fight-text = With no hit of yours for this long, the fight is saved to history and the meter clears.

settings-capture-lead = How the meter sees the game's traffic. Everything here is detected; there is nothing to enter.
settings-capture-connection = Connection
settings-capture-meter = Meter
settings-capture-via = via { $backend }
settings-capture-game = Game connection
settings-capture-found = Found
settings-capture-waiting-game = Waiting for the game
settings-capture-not-identified = Not identified yet
settings-capture-backend = Backend priority
settings-capture-backend-text = Which capture method is tried first; the other is the fallback. Npcap first: some anti-cheats dislike a loaded WinDivert driver.
settings-capture-next-start = Applies at next start.
settings-capture-npcap-first = Npcap first
settings-capture-windivert-first = WinDivert first
settings-capture-check = Capture check
settings-capture-check-text = Administrator rights, Npcap and WinDivert, as at start-up.
settings-capture-checking = Checking…
settings-capture-rerun = Re-run
settings-check-ready = Everything the meter needs is in place.
settings-check-blocked = To sort out before the meter can run: { $names }.
settings-check-line = { $label }: { $status }
settings-check-elevation = Administrator rights
settings-check-elevated = Running elevated, which packet capture requires.
settings-check-not-elevated = The meter is not running as Administrator, so capture drivers cannot be opened. Close the meter and start it again.
settings-check-npcap-ok =
    { $count ->
        [one] Installed and answering, { $count } network adapter visible.
       *[other] Installed and answering, { $count } network adapters visible.
    }
settings-check-available = Available.
settings-check-unavailable = Not available.
settings-check-windivert-fallback = Bundled as a fallback. Its driver is loaded only if Npcap cannot capture, and unloaded again when the meter is done with it.
settings-check-windivert-used = Bundled driver answering, and used for capture.
settings-check-windivert-absent = Not beside the meter, which is fine while Npcap works. It is only a fallback.
settings-check-windivert-missing = Install Npcap, or put WinDivert.dll and WinDivert64.sys beside the meter.
settings-capture-recording = Recording
settings-capture-record-unfamiliar = Record unfamiliar servers
settings-capture-record-unfamiliar-text = The first two minutes on a server no profile matches. The newest five are kept and never leave this PC.
settings-capture-record-every-session = Record every session
settings-capture-record-every-session-text = Keeps this session's game traffic in 30-minute files; the newest are kept up to 2 GB. Files never leave this PC.
settings-capture-packet-recording = Packet recording
settings-capture-packet-recording-text = Writes this session's raw packets to a file, for protocol work.
settings-capture-recording-progress =
    { $count ->
        [one] { $packets } packet
       *[other] { $packets } packets
    } · { $size }
settings-capture-stop-recording = Stop recording
settings-capture-start-recording = Start recording
settings-capture-recordings = Recordings
settings-capture-recordings-note = Replay runs one through the live meter, which has to be running.
settings-capture-unfamiliar-server = unfamiliar server
settings-capture-rolling-recording = whole session
settings-capture-replaying = Replaying
settings-capture-cancel-replay = Cancel replay
settings-capture-replay = Replay
settings-capture-no-recordings = No recordings yet
settings-capture-no-recordings-line = Start one above, or let an unfamiliar server start one.
settings-capture-replay-failed = The replay failed: { $error }
settings-capture-replayed =
    { $count ->
        [one] Replayed { $packets } packet.
       *[other] Replayed { $packets } packets.
    }
settings-capture-mob-data = Mob data
settings-capture-collect-mobs = Collect mob data
settings-capture-collect-mobs-text = Records every mob you fight (code, HP, counts) into mobs.json, to help name the ones the bundled list misses. No player data. Boss corrections are saved either way.
settings-capture-log = Log
settings-capture-decision-log = Decision log
settings-capture-decision-log-text = Writes the meter's decisions (which board, which boss, who you are) to the log file. Lines may include player names; the file stays on this PC.
settings-capture-meter-log = Meter log
settings-capture-meter-log-text = For when something does not look right.
settings-capture-open-log-folder = Open log folder

settings-bests-lead = Your best DPS per boss and character. Boss fights of 15 s or more count. Kept when history is deleted.
settings-bests-none = None yet
settings-bests-none-line = Beat a boss for 15 s or more and your best appears here, per character.
settings-bests-count =
    { $bosses ->
        [one] { $bosses } boss
       *[other] { $bosses } bosses
    }, { $characters ->
        [one] { $characters } character
       *[other] { $characters } characters
    }
settings-bests-confirm = Click again to reset
settings-bests-reset-all = Reset all
settings-bests-boss = Boss
settings-bests-best-dps = Best DPS
settings-bests-date = Date
settings-bests-row = { $boss }, { $character }, { $dps } per second, { $when }

settings-reminders-lead = A reminder before timed events, so you can get there in time. Times follow this PC's clock and are set for the European servers, which run on Berlin time.
settings-reminders-how = How you are reminded
settings-reminders-master = Event reminders
settings-reminders-master-text = A small card above the meter before each event switched on below. It goes by itself after 30 s; a click dismisses it.
settings-reminders-lead-time = Remind me
settings-reminders-lead-time-text = How long before the event starts.
settings-reminders-minutes-before = { $minutes } min before
settings-reminders-sound = Sound
settings-reminders-sound-text = Windows' Chimes sound, once per reminder.
settings-reminders-test-text = Shows a reminder above the meter now, with its sound.
settings-reminders-test-shown = Shown above the meter
settings-reminders-show-test = Show a test reminder
settings-reminders-every-hour = Every hour
settings-reminders-every-3-hours = Every 3 hours
settings-reminders-weekly = Weekly
settings-reminders-daily = Daily
settings-event-arena-opens = Arena of Tactics opens
settings-event-shugo-text = At :00. The minigames, with 3 min to join; which games are open shows only in the game.
settings-event-invasion-text = At :30. One of three invasions, at a random spot in Verteron or Altgard, with 3 min to join; check the map, it may not be near you.
settings-event-rifts-text = 02:00, 05:00, 08:00, 11:00, 14:00, 17:00, 20:00 and 23:00. Altgard, Verteron, Morheim and Eltnen.
settings-event-kaira-text = 01:00, 04:00, 07:00, 10:00, 13:00, 16:00, 19:00 and 22:00. Field boss in Chaotic Lower Reshanta.
settings-event-siege-text = Monday, Thursday and Saturday at 21:00. Chaotic Lower Reshanta.
settings-event-abyss-text = Monday, Thursday and Saturday at 21:30. Three Executors in Lower Reshanta, three more bosses in Middle Reshanta.
settings-event-nahma-text = Friday and Sunday at 21:00. Lv 65, and the Enraged Lv 80.
settings-event-arena-text = 11:00 and 19:00, when the 10 vs 10 queue opens (until 14:00 and 21:00).
settings-event-keys-text = 16:00. Shugo Festival and Dimensional Invasion keys.

settings-todos-lead = The to-do window, opened from the meter's "…" menu.
settings-todos-resets = Resets
settings-todos-daily-reset-text = Reported, not confirmed. Check the countdown in game.
settings-todos-weekly-reset = Weekly reset
settings-todos-weekly-reset-text = This week's to-dos start over on Wednesday, at the daily reset hour.
settings-todos-weekly-at = Wednesday { $time }
settings-todos-membership = Membership
settings-todos-membership-text = Odyle Energy fills up to 840 instead of 560, on every character.
settings-todos-defaults = Default to-dos
settings-todos-hidden = Hidden to-dos
settings-todos-restore = Restore default to-dos
settings-todos-start-over = Start over
settings-todos-start-over-text = Removes your own to-dos, brings back every hidden one and empties every count and amount.
settings-todos-reset-completely = Reset to-dos completely…
settings-todos-reset-done = To-dos reset
settings-todos-set-aside = The stored to-dos were set aside
settings-todos-not-saved = The to-dos could not be saved
settings-todos-reset-title = Reset the to-dos completely?
settings-todos-reset-confirm = Reset to-dos

settings-general-app = App
settings-language = Language
settings-language-text = Game data (skill, boss and monster names) stays in English.
settings-language-system = System ({ $language })

settings-about-description = A real-time DPS meter for AION 2, built for the global servers.
settings-about-source = Source and notice
settings-about-licence = Licence: GPL-3.0-only
settings-about-licence-text = Free software: you may share and change it under the GNU General Public License, version 3.
settings-about-read-licence = Read the licence
settings-about-credits = Credits
settings-about-forked-from = Forked from
settings-about-upstream = Upstream
settings-about-aether = This meter is a fork of Aether, by Helveticxa. GPL-3.0-only.
settings-about-noia2 = By zdyoung. Aether, which this meter is forked from, is itself a fork of NOIA2: the capture pipeline, packet parsers, overlay and game-data catalogues come from it. GPL-3.0-only.
settings-about-kuroukihime = English monster and boss names, the Fighter's skill names, and the server codes. GPL-3.0.
settings-about-references = References
settings-about-built-with = Built with
settings-about-built-with-text = GPUI (Apache-2.0) · Lucide icons (ISC) · Barlow Semi Condensed (SIL OFL 1.1) · WinDivert, the bundled fallback capture driver
settings-about-npcap = Not bundled with the meter: you install it from npcap.com, under its own licence.
settings-about-storage = Storage
settings-about-stored = Stored on this PC
settings-about-counting = Counting…
settings-about-storage-line =
    Settings { $settings } · Fight history { $records ->
        [one] { $records } record
       *[other] { $records } records
    }, { $history } · Recordings { $files ->
        [one] { $files } file
       *[other] { $files } files
    }, { $recordings }
settings-about-open-folder = Open folder
settings-about-reset = Reset settings
settings-about-reset-text = Settings go back to their defaults.
settings-about-reset-button = Reset settings…
settings-reset-title = Reset settings?
settings-reset-body = Settings go back to their defaults, including the Capture switches. Fight history, personal bests, recordings and boss corrections are kept.

## Meter

meter-report-header = Duration { $duration } · Total damage { $damage } · Party DPS { $dps }
meter-copy-defeated = { $boss }{ $place } · defeated in { $duration }
meter-copy-ended = { $boss }{ $place } · fight ended in { $duration }
meter-copy-rank = #{ $rank } of { $total }
meter-copy-crit = crit { $percent }
meter-copy-first-record = First record
meter-copy-new-best = New PB { $pace }
meter-copy-best-behind = PB { $best }/s { $pace }
meter-copy-party = Party { $dps }/s · { $damage } damage
meter-copy-top = Top: { $skills }
meter-copy-party-list = Party: { $players }

meter-status-standing-by = Standing by
meter-status-stopped = Stopped
meter-waiting-title = Waiting for the first hit
meter-stopped-title = The meter is not running
meter-lock-hint = Alt+CapsLock locks and unlocks the meter
meter-unidentified-hint = Start before login, or relog via character select
meter-copy = Copy
meter-copied = Copied
meter-pace-versus-best = { $pace } vs PB
meter-capsule-damage = damage
meter-capsule-party-damage = party damage
meter-capsule-party-dps = party DPS

meter-switch-tip-target = Show the target board
meter-switch-tip-target-line = Follows your target until reset or the next boss.
meter-switch-tip-session = Show the session board
meter-switch-tip-session-line = Stays until reset or the next boss.
meter-switch-tip-pinned-session = Pinned to Session
meter-switch-tip-pinned-target = Pinned to Target
meter-switch-tip-unpin = Click again to switch by itself. Reset and the next boss also unpin.

meter-menu-fight-history = Fight history
meter-menu-pvp = PvP meter
meter-menu-todos = To-dos
meter-menu-settings = Settings
meter-menu-hit-a-mob = Hit a mob to set whether it is a boss
meter-menu-treat-as-boss = Treat { $name } as a boss
meter-menu-treat-as-boss-unnamed = Treat as a boss
meter-menu-not-a-boss = Not a boss
meter-menu-automatic = Automatic

meter-summary-defeated = Defeated
meter-summary-ended = Fight ended
meter-summary-may-resume = may resume
meter-summary-party-dps = Party DPS
meter-summary-rank = Rank
meter-summary-share = Share
meter-summary-crit = Crit
meter-summary-party = Party
meter-summary-open-report = Open the fight report
meter-best-first = First record
meter-best-new = New personal best
meter-best-behind = Best { $best }

## Fight history

history-unknown-player = Unknown
history-records =
    { $count ->
        [one] { $count } record
       *[other] { $count } records
    }
history-records-filtered =
    { $shown } / { $total } { $total ->
        [one] record
       *[other] records
    }
history-deleted =
    { $count ->
        [one] Deleted { $count } record.
       *[other] Deleted { $count } records.
    } Personal bests are kept.
history-when-today = Today { $time }
history-when-yesterday = Yesterday { $time }
history-when-weekday = { $weekday } { $time }
history-when-date = { $day } { $month } { $time }

history-title = Fight history
history-empty-title = No records yet
history-empty-line = Reset the meter after a fight to save it here.
history-no-match = No record matches the filters.
history-all-targets = All targets
history-all-players = All players
history-delete-one = Delete?
history-damage = damage
history-player-hint = Click a player to open their details from this record
history-saved-when = Saved when you reset the meter
history-delete-all = Delete all
history-delete-all-confirm =
    { $count ->
        [one] Delete all { $count } record?
       *[other] Delete all { $count } records?
    }
history-report = Fight report
history-boss-record = Boss, { $name }

## Player detail

detail-no-player = No player selected
detail-pick-title = Pick a player
detail-pick-line = Click a player in the meter, a row in the PvP window or a player inside a history record to see their skills and buffs here.
detail-no-skills-title = No skill data
detail-no-skills-live = This player has not damaged the current target. Buffs they cast still show below.
detail-no-skills-session = This player has not damaged anything since the last reset. Buffs they cast still show below.
detail-no-skills-history = This player did not damage the target of this record. Buffs they cast still show below.
detail-chip-history = History
detail-chip-live = Live
detail-no-target = No target
detail-current-target = current target
detail-every-target = every target since the last reset
detail-unknown-target = Unknown target
detail-saved-record = saved record
detail-saved-record-at = { $time }, saved record

detail-skill = Skill
detail-hits = Hits
detail-crit = Crit
detail-total = Total
detail-fight = Fight
detail-critical = Critical
detail-back = Back
detail-front = Front
detail-double = Double
detail-perfect = Perfect
detail-parry = Parry
detail-multi = Multi
detail-multi-damage = Multi dmg
detail-spec = Spec
detail-min = Min
detail-max = Max
detail-average = Average
detail-multi-hits = { $hits } hits: { $count } ({ $share })

detail-player-buffs = Player buffs
detail-boss-buffs = Boss buffs
detail-target-buffs = Target buffs
detail-buffs-up-now = { $count } up now
detail-buff-up-now = Up now
detail-buff-ended = Ended

## Fight report

report-title = Fight report
report-healing = Healing
report-damage-taken = Damage taken
report-deaths = Deaths
report-taken = Taken
report-sort-by = Sort by { $column }
report-open-player = { $name }, open details
report-death-label = { $name } died at { $time }, killed by { $killer }, { $skill }
report-time = Time
report-killed-by = Killed by
report-last-hit = Last hit
report-by = by
report-nobody-died = Nobody died
report-deaths-not-recorded = Not recorded for this fight
report-not-recorded-note = Healing, damage taken and deaths were not recorded for this fight: it was saved before the meter kept them. Damage, skills and buffs are complete.
report-hint = Click a player for their skills, buffs, healing and damage taken
report-unknown = Unknown

report-back = Report
report-tab-damage = Damage
report-tab-buffs = Buffs
report-tab-healing = Healing
report-tab-taken = Damage taken
report-multi-hit = Multi-hit
report-multi-hit-damage = Multi-hit damage
report-crits = Crits
report-crit-rate = Crit %
report-heals = Heals
report-healed = Healed
report-largest = Largest
report-monster-skill = Monster skill
report-buff-and-caster = Buff and caster
report-up-for = Up for
report-of-fight = of { $time }
report-on = On { $name }
report-died-at = Died at { $time }
report-last-hit-amount = last hit { $amount }
report-no-healing-title = No healing
report-no-healing-line = { $name } cast no heals in this fight.
report-no-taken-title = No damage taken
report-no-taken-line = Nothing hit { $name } in this fight.
report-not-recorded-title = Not recorded
report-not-recorded-healing = This fight was saved before the meter kept healing.
report-not-recorded-taken = This fight was saved before the meter kept damage taken.

## PvP

pvp-title = PvP
pvp-watching = Watching
pvp-watch-empty = No one watched yet. Add a name, or get hit: your last attacker appears here on its own.
pvp-add-placeholder = Add a player to watch
pvp-seen-this-session = seen this session
pvp-not-in-range = Not in range
pvp-last-attacker = Last attacker
pvp-tab-kills = Kill stats
pvp-passive-note = Capture is passive: players out of range are not counted, and damage settles when the target dies.
pvp-clear = Clear
pvp-this-session = this session
pvp-kills-empty-title = No kill stats yet
pvp-kills-empty-line = Kills, assists and deaths appear as fights settle.
pvp-kills = Kills
pvp-assists = Assists
pvp-deaths = Deaths
pvp-damage-dealt = Damage dealt
pvp-damage-taken = Damage taken
pvp-dealt-empty = No damage dealt yet.
pvp-taken-empty = No damage taken yet.
