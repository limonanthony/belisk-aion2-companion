/* Proposal: shared data and pieces for the to-do window mockups (a.html,
 * b.html, c.html). Loaded after ../approved/kit.js.
 *
 * Content: the reset research (MetaBot client tables for the Global client,
 * Game8 for Duty Missions and Duty Commands). Limits and refill rules are the
 * research's; the counts each character has reached, the times you used a
 * recharge and every name of a character are invented.
 *
 * The mocked clock is Monday 21:48. Daily reset 09:00 (reported, not
 * confirmed), weekly reset Wednesday 09:00, reward keys refill at 16:00.
 *
 * Icons: Lucide (https://lucide.dev), ISC License; see ../approved/kit.js. */

(function () {
  "use strict";

  // ---- Icons the approved kit does not carry (Lucide, ISC) ----------------
  const EXTRA = {
    listChecks:
      '<path d="m3 17 2 2 4-4"/><path d="m3 7 2 2 4-4"/><path d="M13 6h8"/><path d="M13 12h8"/><path d="M13 18h8"/>',
    pencil:
      '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>',
    eyeOff:
      '<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/>',
    users:
      '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
    minus: '<path d="M5 12h14"/>',
    activity:
      '<path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/>',
    rotate: '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>',
  };

  function icon(name, cls) {
    if (!EXTRA[name]) return Kit.icon(name, cls);
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${EXTRA[name]}</svg>`;
  }

  // ---- Clock ----------------------------------------------------------------
  const CLOCK = {
    now: "Mon 21:48",
    daily: { left: "11 h 12 min", at: "Tue 09:00" },
    weekly: { left: "1 d 11 h", at: "Wed 09:00" },
    keys: { left: "18 h 12 min", at: "Tue 16:00" },
  };

  // ---- Characters the meter remembers (invented) ---------------------------
  const CHARS = [
    { id: "mirelle", name: "Mirelle", cls: "ranger", klass: "Ranger", server: "Ossia", playing: true },
    { id: "tavrin", name: "Tavrin", cls: "templar", klass: "Templar", server: "Ossia" },
    { id: "sefa", name: "Sefa", cls: "cleric", klass: "Cleric", server: "Lyss" },
  ];

  // ---- To-dos -----------------------------------------------------------------
  // Lost at the daily reset. Duty Missions are per account and server.
  const TODAY = [{ id: "duty", name: "Duty Missions", fact: "5 a day", n: 3, of: 5, account: true }];

  // Bank up to a cap; past it, new charges are lost. "lvl" is the estimate
  // from when you last used them up (A, B) or the counter (C).
  const RECHARGES = [
    { id: "deus", name: "Deus Research Base", fact: "+2 a day, up to 4", est: 2, cap: 4, when: "Full in 11 h", at: "Tue 09:00" },
    { id: "shugo", name: "Shugo Festival keys", fact: "+4 at 16:00, up to 28", est: 24, cap: 28, when: "Full in 18 h", at: "Tue 16:00" },
    { id: "invasion", name: "Invasion keys", fact: "+1 at 16:00, up to 7", est: 6, cap: 7, when: "Full in 18 h", at: "Tue 16:00" },
    { id: "arkanis", name: "Shattered Arkanis", fact: "+2 a day, up to 4", est: 2, cap: 4, when: "Full in 11 h", at: "Tue 09:00" },
    { id: "balaur", name: "Abandoned Balaur Fortress", fact: "+1 a day, up to 7", est: 5, cap: 7, when: "Full in 1 d 11 h", at: "Wed 09:00" },
    { id: "fafnium", name: "Fafnium Laboratory", fact: "+1 a day, up to 7", est: 5, cap: 7, when: "Full in 1 d 11 h", at: "Wed 09:00" },
    { id: "conquest", name: "Conquest dungeons", fact: "+2 a day each, up to 14", est: 8, cap: 14, when: "Full in 2 d 11 h", at: "Thu 09:00" },
    { id: "odyle", name: "Odyle Energy", fact: "+15 every 3 h, up to 560", est: 180, cap: 560, when: "Full in 2 d 16 h", at: "Thu 14:00" },
    { id: "nightmare", name: "Nightmare", fact: "+2 a day, up to 14", est: 4, cap: 14, when: "Full in 4 d 11 h", at: "Sat 09:00" },
  ];

  // Lost at the weekly reset (Wednesday).
  const WEEKLY = [
    { id: "trial", name: "Ascension Trial", fact: "3 a week · solo", n: 2, of: 3, seen: true },
    { id: "subj", name: "Subjugation", fact: "3 tickets a week · 4 players", n: 3, of: 3 },
    { id: "ludra", name: "Abyssal Forge: Ludra", fact: "1 boss kill a week · 10 players", n: 1, of: 1 },
    { id: "corroded", name: "Corroded Decontamination Facility", fact: "2 boss kills a week · 10 players", n: 0, of: 2 },
    { id: "muspel", name: "Chalice of Muspel", fact: "2 boss kills a week · item level 4,300", n: 0, of: 2, hidden: true },
    { id: "fissure", name: "Unknown Fissure", fact: "14 a week · solo", n: 9, of: 14 },
    { id: "transc", name: "Transcendence boss kills", fact: "28 a week", n: 17, of: 28 },
    { id: "conqk", name: "Conquest boss kills", fact: "35 a week", n: 35, of: 35 },
    { id: "ordeal", name: "Ordeal boss kills", fact: "7 a week", n: 3, of: 7 },
    { id: "cradle", name: "Cradle of Nihility", fact: "7 rewards a week", n: 4, of: 7 },
    { id: "dramata", name: "Dying Dramata's Nest", fact: "7 rewards a week", n: 0, of: 7 },
    { id: "dutycmd", name: "Duty Commands", fact: "Up to 20 a week", n: 12, of: 20 },
    { id: "explore", name: "Exploration runs", fact: "7 a week per dungeon", n: 0, of: 1 },
    { id: "growth", name: "Growth dungeons", fact: "7 h a week each, counted while inside", n: 0, of: 1 },
  ];

  // Added by the player in edit mode (a row the defaults leave out).
  const CUSTOM = [{ id: "legion", name: "Legion donation", fact: "Yours · daily", n: 0, of: 1, custom: true }];

  const clone = (list) => list.map((x) => ({ ...x }));

  // ---- Pieces ---------------------------------------------------------------
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  function nm(ctx, name) {
    return esc(ctx.opts.mask ? ctx.fmt.mask(name) : name);
  }

  const btn = (name, label, extra = "") =>
    `<span class="o-btn${extra}" role="button" tabindex="0" title="${label}" aria-label="${label}">${icon(name)}</span>`;

  function box(kind) {
    if (kind === "done") return `<span class="t-box is-done" aria-hidden="true">${icon("check")}</span>`;
    if (kind === "suggest") return `<span class="t-box is-suggest" aria-hidden="true"></span>`;
    return `<span class="t-box" aria-hidden="true"></span>`;
  }

  function pips(n, of, { seenAt = -1 } = {}) {
    let s = "";
    for (let i = 0; i < of; i++) s += `<span class="t-pip${i < n ? " is-on" : ""}${i === seenAt ? " is-seen" : ""}"></span>`;
    return `<span class="t-pips" aria-hidden="true">${s}</span>`;
  }

  function bar(frac, { warn = false, width } = {}) {
    const w = width ? ` style="width:${width}px"` : "";
    return `<span class="t-bar${warn ? " is-warn" : ""}"${w} aria-hidden="true"><i style="width:${(Math.min(1, frac) * 100).toFixed(0)}%"></i></span>`;
  }

  const fig = (n, of) => `<span class="t-fig">${n}<small>/${of}</small></span>`;

  // What sits at the right of a counted to-do: segments up to 7, a bar and
  // a figure above that, nothing for a single tick.
  function progress(it, { seen = false } = {}) {
    if (it.of <= 1) return "";
    if (it.of <= 7) return pips(it.n, it.of, { seenAt: seen ? it.n : -1 }) + fig(it.n, it.of);
    return bar(it.n / it.of) + fig(it.n, it.of);
  }

  function ariaFor(it) {
    const done = it.n >= it.of;
    const label = it.of > 1 ? `${it.name}, ${it.n} of ${it.of}` : it.name;
    return `role="checkbox" aria-checked="${done ? "true" : it.n > 0 ? "mixed" : "false"}" aria-label="${esc(label)}" tabindex="0"`;
  }

  function head(title, { count = "", chip = "", editing = false, tools = true } = {}) {
    const edit = tools ? btn("pencil", editing ? "Done editing" : "Edit the list", editing ? " is-hot" : "") : "";
    return `<div class="o-head"><span class="o-title">${icon("listChecks")}${title}</span>${count ? `<span class="o-count">${count}</span>` : ""}${chip}<span class="grow"></span>${edit}${btn("close", "Close")}</div>`;
  }

  const warnChip = (text) => `<span class="o-chip is-warn">${icon("alert")}${text}</span>`;

  function resetLine(kind) {
    if (kind === "weekly") return `<div class="t-reset">${icon("clock")}<span>Weekly reset in <b>${CLOCK.weekly.left}</b></span><span class="at">${CLOCK.weekly.at}</span></div>`;
    return `<div class="t-reset">${icon("clock")}<span>Daily reset in <b>${CLOCK.daily.left}</b></span><span class="at">${CLOCK.daily.at}</span></div>`;
  }

  // Footer: the reset hour is reported by a fan site, not by the game.
  function foot(right = "") {
    return `<div class="o-foot"><span class="faint" style="display:inline-flex;align-items:center;gap:5px">${icon("info")}Reset at 09:00 is reported, not confirmed</span><span class="grow"></span>${right}</div>`;
  }

  function editFoot() {
    return `<div class="o-foot"><span class="o-tbtn is-quiet t-small" role="button">${icon("rotate")}Restore defaults</span><span class="grow"></span><span class="o-tbtn t-small" role="button">${icon("check")}Done</span></div>`;
  }

  function addRow({ typed = "", kind = "daily" } = {}) {
    const field = typed
      ? `<span class="o-input is-focus"><span class="val">${esc(typed)}</span><span class="o-caret"></span></span>`
      : `<span class="o-input"><span class="ph">Add a to-do</span></span>`;
    return `<div class="t-add">${field}<span class="t-seg" role="radiogroup" aria-label="Resets"><span class="${kind === "daily" ? "is-on" : ""}" role="radio">Daily</span><span class="${kind === "weekly" ? "is-on" : ""}" role="radio">Weekly</span></span><span class="o-tbtn t-small${typed ? "" : " is-disabled"}" role="button">${icon("plus")}Add</span></div>`;
  }

  // ---- The meter with its "…" menu (opening the window) ------------------
  const METER_ROWS = [
    { name: "Vaelith", cls: "sorcerer", cp: 41820, dmg: 17.84e6, dps: 41240 },
    { name: "Mirelle", cls: "ranger", cp: 38904, dmg: 14.62e6, dps: 38420, you: true },
    { name: "Korrinth", cls: "gladiator", cp: 40112, dmg: 11.05e6, dps: 29610 },
    { name: "Ilyrae", cls: "chanter", cp: 34210, dmg: 4.31e6, dps: 12870 },
  ];

  function meter(ctx, { menu = true } = {}) {
    const { fmt } = ctx;
    const total = METER_ROWS.reduce((s, p) => s + p.dmg, 0);
    const top = METER_ROWS[0].dmg;
    const rows = METER_ROWS.map(
      (p, i) => `<div class="o-row${p.you ? " is-you" : ""}"><span class="o-rk">${i + 1}</span><span class="o-track" style="--rgb:${ctx.CLASS_RGB[p.cls]}"><span class="o-fill" style="width:${((p.dmg / top) * 100).toFixed(1)}%"></span><img class="o-icon" src="${ctx.classIcon(p.cls)}" alt="${p.cls}"><span class="o-name">${nm(ctx, p.name)}<small>${fmt.int(p.cp)}</small></span><span class="o-num muted" style="font-size:11.5px">${fmt.compact(p.dps)}<span class="faint">/s</span></span><span class="o-num" style="font-size:14px;font-weight:600">${fmt.compact(p.dmg)}</span></span><span class="o-pc">${fmt.pct(p.dmg / total)}</span></div>`
    ).join("");
    const tools = ["reset", "capsule", "lockOpen", "copy"].map((n) => btn(n, n)).join("") + btn("more", "More", menu ? " is-hot" : "") + btn("close", "Close");
    const lead = (i) => `<span class="lead">${i ? icon(i) : ""}</span>`;
    const m = menu
      ? `<div class="o-menu" role="menu"><div class="o-menu-item" role="menuitem">${lead()}Fight history</div><div class="o-menu-item" role="menuitem">${lead()}PvP meter</div><div class="o-menu-item is-hot is-new" role="menuitem">${lead()}To-dos<small>1 full</small></div><div class="o-menu-item" role="menuitem">${lead()}Settings</div><div class="o-menu-sep"></div><div class="o-menu-item" role="menuitemradio">${lead()}Treat Thornback Boar as a boss</div><div class="o-menu-item" role="menuitemradio">${lead()}Not a boss</div><div class="o-menu-item" role="menuitemradio" aria-checked="true">${lead("check")}<b style="font-weight:600">Automatic</b></div></div>`
      : "";
    return `<div class="ov o t-meter"><div class="t-mhead"><span class="tgt">Thornback Boar</span>${tools}</div><div class="o-rows">${rows}</div><div class="t-mfoot"><span class="lbl">Damage</span><span class="val">${fmt.compact(total)}</span><span class="ping">${icon("signal")}${ctx.sys.pingMs} ms</span><span class="t-seg2"><span class="is-on">Session</span><span>Target</span></span></div>${m}</div>`;
  }

  window.Todo = {
    icon,
    CLOCK,
    CHARS,
    today: () => clone(TODAY),
    recharges: () => clone(RECHARGES),
    weekly: () => clone(WEEKLY),
    custom: () => clone(CUSTOM),
    esc,
    nm,
    btn,
    box,
    pips,
    bar,
    fig,
    progress,
    ariaFor,
    head,
    warnChip,
    resetLine,
    foot,
    editFoot,
    addRow,
    meter,
  };
})();
