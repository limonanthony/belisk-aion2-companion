# Changelog

## [1.1.0] - 2026-10-11

Pet tracking, and smoother windows.

- **Pets**: a new window, opened from the paw in the toolbar, lists every pet with its level and the souls you have toward the next one, read from the game. Tabs follow the game's families, a search finds a pet by name, and pinned pets stay at the top.
- **Soul tracker**: a small card shows a pet's progress as soon as you get its souls, from a kill or the black market, and goes away after 10 seconds. Choose when it appears in **Settings › Pets**.
- The game sends your pet collection when you log in, so start Belisk before you log in. If you start it while already playing a character it knows, new souls and the to-do counts still follow the game, and the Pets window shows the collection from your last login.
- **Cards**: the reminder and soul tracker cards show their time left as a small ring next to the ×, with even spacing on every side.
- **Smoother windows**: History and Pets stay fast with long lists, dragging a window no longer stutters, and the scrollbars of History, Pets, the to-dos and the other tool windows can now be dragged.
- **Alt+Shift+CapsLock**: showing everything again brings back only the windows that are still open.

## [1.0.2] - 2026-10-10

The first update delivered by the app itself.

- **Releases**: each release now checks its version and its notes before it is published, so the same version can never be published twice.

## [1.0.1] - 2026-10-10

A fix for the update check.

- **Updates**: the app can now check for new versions and download them. In 1.0.0 the check stopped at its first request, so 1.0.0 cannot update itself: download this version by hand once, and the next ones arrive on their own.
- If an update check ever fails, **Settings › About** says so and the app tries again an hour later.

## [1.0.0] - 2026-10-10

Belisk is a combat meter and companion for AION 2 on the global servers: a live DPS meter with the tools around it, all opened from one small toolbar.

- **Toolbar**: a small floating bar opens at launch and holds every tool: the DPS meter, PvP, to-dos, fight history, event reminders and settings, with your Odyle Energy and Shugo keys counts beside them. The meter opens when you ask for it, or at launch if you choose so in **Settings › Overlay**.
- **DPS meter**: the Session board counts everything you fight and the Target board follows one monster. In a dungeon it stays on the boss until the boss is dead. It shows damage, DPS, healing, damage taken, deaths and buffs for every player.
- **Boss fights**: a summary appears when a boss dies, with your rank and whether it is a new personal best. A wipe is noticed when the boss returns to full health, and the failed attempt is kept too. Each boss fight has a **fight report** with every player's damage, DPS, healing, damage taken and deaths, and one click on a name shows that player's skills and buffs.
- **History and personal bests**: every fight you finish is saved. Your best DPS against each boss is kept for each character, and the meter shows your pace against it during the fight.
- **PvP**: a window follows the players you watch, and your last attacker shows up by itself.
- **To-dos and Resources**: daily and weekly to-dos for each character, and a Resources window with Odyle Energy, Shugo keys, Invasion keys, Nightmare and the Daily Dungeon, filled in from the game while the app runs. A resource that is full just before it refills is flagged, so you can spend it first.
- **Event reminders**: a small card appears shortly before the Shugo Festival, Dimensional Invasions, rift portals and other events. Pick the events in **Settings › Reminders**.
- **English and French**: the app follows your Windows language, or the one you choose in **Settings › General**.
- **Safe next to the game**: it only listens to the game's network traffic and never opens, reads or changes the game itself. There is no account, and nothing from your fights is uploaded.
