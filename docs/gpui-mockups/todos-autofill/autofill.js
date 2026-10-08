// To-dos filled from the game (TASK-20.2): the approved to-do window
// (../approved/todos.html, direction E) with values the game sends. The
// window code is a copy of the approved page's script, trimmed to what these
// states need, plus the game values. Nothing here is saved or sent anywhere;
// counts, times and character names are invented. Caps and refills are the
// game's ticket table as in our notes (todo-autofill-findings.md).
(function () {
  "use strict";

  // ---- Icons: Lucide (https://lucide.dev), ISC License; see kit.js ----------
  const EXTRA = {
    listChecks: '<path d="m3 17 2 2 4-4"/><path d="m3 7 2 2 4-4"/><path d="M13 6h8"/><path d="M13 12h8"/><path d="M13 18h8"/>',
    users:
      '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
    undo: '<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11"/>',
    pencil:
      '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>',
    gamepad:
      '<line x1="6" x2="10" y1="11" y2="11"/><line x1="8" x2="8" y1="9" y2="13"/><line x1="15" x2="15.01" y1="12" y2="12"/><line x1="18" x2="18.01" y1="10" y2="10"/><path d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"/>',
  };

  function icon(name, cls) {
    if (!EXTRA[name]) return Kit.icon(name, cls);
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${EXTRA[name]}</svg>`;
  }

  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const warnChip = (text) => `<span class="o-chip is-warn">${icon("alert")}${text}</span>`;
  const tag = (kind, text) => `<span class="dec is-${kind.toLowerCase()}">${text || kind}</span>`;

  // ---- Open data questions: shown both ways, never decided here -------------
  const params = new URLSearchParams(location.search);
  const OPEN = {
    deus: params.get("deus") === "14" ? 14 : 4,
    conquest: params.get("conquest") === "pool" ? "pool" : "dungeon",
  };

  // ---- Clock (mocked): Monday 21:48, daily reset 09:00, weekly Wednesday ----
  const DAY = 1440;
  const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const RESET = 9 * 60;
  const WEEKLY = 2 * DAY + RESET;
  const MON = 21 * 60 + 48; // now
  const LOGIN = 19 * 60 + 2; // Mirelle logged in today at 19:02
  const SUN = 6 * DAY + 22 * 60 + 40 - 7 * DAY; // yesterday, Sun 22:40

  const pad = (n) => String(n).padStart(2, "0");
  const clock = (t) => `${pad(Math.floor((((t % DAY) + DAY) % DAY) / 60))}:${pad((((t % DAY) + DAY) % DAY) % 60)}`;
  const stamp = (t) => `${DAYS[((Math.floor(t / DAY) % 7) + 7) % 7]} ${clock(t)}`;

  function span(m) {
    const d = Math.floor(m / DAY);
    const h = Math.floor((m % DAY) / 60);
    const min = m % 60;
    if (d) return `${d} d ${h} h`;
    if (h) return `${h} h ${min} min`;
    return `${min} min`;
  }

  function nextAt(now, at) {
    let t = Math.floor(now / DAY) * DAY + at;
    if (t <= now) t += DAY;
    return t;
  }

  function nextWeekly(now) {
    let t = Math.floor(now / (7 * DAY)) * 7 * DAY + WEEKLY;
    if (t <= now) t += 7 * DAY;
    return t;
  }

  // The k-th refill from now: every `every` minutes (from `off` past
  // midnight), or once a day at `at`.
  const refill = (r, now, k) => (r.every ? (Math.floor((now - (r.off || 0)) / r.every) + k) * r.every + (r.off || 0) : nextAt(now, r.at) + (k - 1) * DAY);

  // Refills between two moments (the projection of a value seen earlier).
  function refillsBetween(r, from, to) {
    if (r.every) return Math.floor((to - (r.off || 0)) / r.every) - Math.floor((from - (r.off || 0)) / r.every);
    let k = 0;
    for (let t = nextAt(from, r.at); t <= to; t += DAY) k++;
    return k;
  }

  // Full when the regular pool reaches the cap; extras never count.
  function estimate(r, v, cap, now) {
    if (v >= cap) return { full: true, next: refill(r, now, 1) };
    return { full: false, at: refill(r, now, Math.ceil((cap - v) / r.add)) };
  }

  // ---- The list ----------------------------------------------------------------
  const TODAY = [{ id: "duty", kind: "count", name: "Duty Missions", of: 5, account: true, manual: true }];

  const CONQUEST_DUNGEONS = [
    ["krao", "Krao Cave"],
    ["urugugu", "Urugugu Canyon"],
    ["fire", "Fire Temple"],
    ["draupnir", "Draupnir"],
    ["vakron", "Vakron Sky Island"],
    ["horn", "Ferocious Horn Den"],
  ];

  const CONQUEST =
    OPEN.conquest === "pool"
      ? { id: "conquest", name: "Conquest dungeons", cap: 21, add: 1, every: 480, off: 60, open: "conquest" }
      : {
          id: "conquest",
          kind: "cgroup",
          name: "Conquest dungeons",
          open: "conquest",
          items: CONQUEST_DUNGEONS.map(([id, name]) => ({ id: "c-" + id, name, cap: 14, add: 2, at: RESET })),
        };

  // Settled data (2026-10-07/08): Shugo Festival keys are ticket 1, 12 (21
  // with membership), +3 a day; Shugo, Invasion and the Unknown Fissure are
  // shared by the characters of one server. Refills follow the table's
  // daily clock (16:00 KST = the 09:00 daily reset here).
  const CHARGES = [
    { id: "odyle", name: "Odyle Energy", cap: 560, mcap: 840, add: 15, every: 180, step: 10 },
    { id: "shugo", name: "Shugo Festival keys", cap: 12, mcap: 21, add: 3, at: RESET, account: true },
    { id: "invasion", name: "Invasion keys", cap: 7, add: 1, at: RESET, account: true },
    { id: "nightmare", name: "Nightmare", cap: 14, add: 2, at: RESET },
    CONQUEST,
    { id: "deus", name: "Deus Research Base", cap: OPEN.deus, add: 2, at: RESET, open: "deus" },
    { id: "arkanis", name: "Shattered Arkanis", cap: OPEN.deus, add: 2, at: RESET, open: "deus" },
    { id: "balaur", name: "Abandoned Balaur Fortress", cap: 7, add: 1, at: RESET },
    { id: "fafnium", name: "Fafnium Laboratory", cap: 7, add: 1, at: RESET },
  ];

  const chargeLeaves = () => CHARGES.flatMap((r) => (r.kind === "cgroup" ? r.items : [r]));
  const findCharge = (id) => chargeLeaves().find((r) => r.id === id);

  const EXPLORE = [
    ["azure", "Azure Breath Island"],
    ["cradle", "Cradle of Nihility"],
    ["draupnir", "Draupnir"],
    ["dramata", "Dying Dramata's Nest"],
    ["horn", "Ferocious Horn Den"],
    ["fire", "Fire Temple"],
    ["illusion", "Hall of Illusion"],
    ["krao", "Krao Cave"],
    ["urugugu", "Urugugu Canyon"],
    ["vakron", "Vakron Sky Island"],
  ];

  const WEEK = [
    { id: "fissure", kind: "count", name: "Daily Dungeon", sub: "Unknown Fissure", of: 14, account: true },
    { id: "trial", kind: "count", name: "Ascension Trial", of: 3 },
    { id: "subj", kind: "count", name: "Subjugation", of: 3 },
    {
      id: "sanct",
      kind: "group",
      name: "Sanctuary",
      items: [
        { id: "forge", kind: "count", name: "Abyssal Forge", sub: "attempts", of: 4 },
        { id: "forgeboss", kind: "tick", name: "Abyssal Forge", sub: "final boss" },
        { id: "corr", kind: "count", name: "Corroded Facility", sub: "attempts", of: 4 },
        { id: "corrboss", kind: "count", name: "Corroded Facility", sub: "final boss", of: 2 },
        { id: "chal", kind: "count", name: "Chalice of Muspel", sub: "attempts", of: 4 },
        { id: "chalboss", kind: "count", name: "Chalice of Muspel", sub: "final boss", of: 2 },
      ],
    },
    { id: "transc", kind: "count", name: "Transcendence", sub: "final boss kills", of: 28 },
    { id: "conqk", kind: "count", name: "Conquest", sub: "final boss kills", of: 35 },
    { id: "cradle", kind: "count", name: "Cradle of Nihility", sub: "Conquest", of: 7 },
    { id: "dramata", kind: "count", name: "Dying Dramata's Nest", sub: "Conquest", of: 7 },
    { id: "ordeal", kind: "count", name: "Ordeal", sub: "final boss kills", of: 7 },
    { id: "explore", kind: "group", name: "Exploration", sub: "7 runs a dungeon", items: EXPLORE.map(([id, name]) => ({ id: "x-" + id, kind: "count", name, of: 7 })) },
    { id: "dutycmd", kind: "count", name: "Duty Commands", of: 20, manual: true },
  ];

  const HIDDEN = ["dutycmd"];
  const LEGION = { id: "legion", kind: "tick", name: "Legion check-in", custom: true, reset: "daily", manual: true };

  const CHARS = [
    { id: "mirelle", name: "Mirelle", cls: "ranger", klass: "Ranger", server: "Ossia" },
    { id: "tavrin", name: "Tavrin", cls: "templar", klass: "Templar", server: "Ossia" },
    { id: "sefa", name: "Sefa", cls: "cleric", klass: "Cleric", server: "Lyss" },
  ];

  // Rows the game fills for every character of a server.
  const SERVER_ROWS = new Set(["shugo", "invasion", "fissure"]);

  // ---- What the game sent (count = regular pool or entries left; x = extra) --
  function snapshot(o = {}) {
    const g = {
      odyle: { n: 245, x: 380 },
      shugo: { n: 7, x: 3 },
      invasion: { n: 2, x: 0 },
      nightmare: { n: 0, x: 3 },
      conquest: { n: 16, x: 0 },
      "c-krao": { n: 12, x: 0 },
      "c-urugugu": { n: 12, x: 0 },
      "c-fire": { n: 10, x: 0 },
      "c-draupnir": { n: 12, x: 0 },
      "c-vakron": { n: 13, x: 0 },
      "c-horn": { n: 12, x: 0 },
      deus: { n: 4, x: 0 },
      arkanis: { n: 4, x: 0 },
      balaur: { n: 6, x: 0 },
      fafnium: { n: 6, x: 0 },
      fissure: { n: 9, x: 7 },
      trial: { n: 0, x: 1 },
      subj: { n: 2, x: 0 },
      forge: { n: 2, x: 0 },
      forgeboss: { n: 0, x: 0 },
      corr: { n: 4, x: 0 },
      corrboss: { n: 2, x: 0 },
      chal: { n: 3, x: 0 },
      chalboss: { n: 2, x: 0 },
      transc: { n: 16, x: 0 },
      conqk: { n: 0, x: 0 },
      cradle: { n: 3, x: 0 },
      dramata: { n: 7, x: 0 },
      ordeal: { n: 5, x: 0 },
    };
    EXPLORE.forEach(([id]) => (g["x-" + id] = { n: 7, x: 0 }));
    g["x-fire"].n = 0;
    g["x-krao"].n = 0;
    g["x-draupnir"].n = 4;
    for (const [id, val] of Object.entries(o)) g[id] = { ...g[id], ...val };
    return g;
  }

  // ---- Lookups -------------------------------------------------------------------
  const flat = (list) => list.flatMap((it) => (it.kind === "group" ? [it, ...it.items] : [it]));
  const findItem = (v, id) => flat(TODAY.concat(WEEK, v.custom)).find((it) => it.id === id);
  const todayList = (v) => TODAY.concat(v.custom.filter((c) => c.reset === "daily"));
  const weekList = (v) => WEEK.concat(v.custom.filter((c) => c.reset === "weekly"));
  const active = (list, v) => list.filter((it) => !v.hidden.has(it.id));
  const hiddenIn = (list, v) => list.filter((it) => v.hidden.has(it.id));
  const listed = (list, v, key) => (v.reveal[key] ? list : active(list, v));
  const label = (it) => (it.sub ? `${it.name}, ${it.sub}` : it.name);
  const clamp = (x, lo, hi) => Math.max(lo, Math.min(hi, x));
  const capOf = (v, r) => (v.member && r.mcap ? r.mcap : r.cap);

  // The game's value for a row. Live values are the game's latest (it sends
  // every spend, refill and gain); a value from an earlier login is
  // projected to now with the refills the table knows (never the spends).
  function gameOf(v, id) {
    const g = v.g[id];
    if (!g) return null;
    const at = g.at == null ? v.seen : g.at;
    if (v.src.kind !== "stale") return { ...g, at, seen: g.n, est: 0 };
    const r = findCharge(id);
    if (r) {
      const cap = capOf(v, r);
      const k = refillsBetween(r, at, v.now);
      const n = g.n >= cap ? g.n : Math.min(cap, g.n + k * r.add);
      return { ...g, at, seen: g.n, n, est: n - g.n };
    }
    const it = findItem(v, id);
    if (it && at < nextWeekly(v.now) - 7 * DAY) return { ...g, at, seen: g.n, n: it.of || 1, est: 0, weekly: true };
    return { ...g, at, seen: g.n, est: 0 };
  }

  // Done count of a count or tick row: typed over, from the game, or manual.
  function countOf(v, it) {
    if (v.over[it.id] != null) return v.over[it.id];
    const g = gameOf(v, it.id);
    if (g) return it.kind === "tick" ? (g.n <= 0 ? 1 : 0) : Math.max(0, it.of - g.n);
    return v.n[it.id] || 0;
  }

  const isDone = (it, v) => (it.kind === "group" ? active(it.items, v).every((s) => isDone(s, v)) : countOf(v, it) >= (it.of || 1));
  const left = (list, v) => active(list, v).filter((it) => !isDone(it, v)).length;

  // An amount's state: { src: "game" | "over" | "typed" | "empty", n, x, full, e }.
  function amountOf(v, r) {
    const cap = capOf(v, r);
    if (v.over[r.id] != null) {
      const n = v.over[r.id];
      const g = gameOf(v, r.id);
      return { src: "over", n, x: g ? g.x || 0 : 0, cap, e: estimate(r, n, cap, v.now) };
    }
    const g = gameOf(v, r.id);
    // A regular pool above the cap in force can only come from a membership
    // the switch does not know: shown as is, never full, no estimate.
    if (g && g.n > cap) return { src: "game", n: g.n, x: g.x || 0, g, cap, e: { full: false, above: true } };
    if (g) return { src: "game", n: g.n, x: g.x || 0, g, cap, e: estimate(r, g.n, cap, v.now) };
    if (v.amt[r.id] != null) {
      const n = v.amt[r.id];
      return { src: "typed", n, x: 0, cap, e: estimate(r, n, cap, v.now) };
    }
    return { src: "empty", cap };
  }

  const isFull = (v, r) => !v.hidden.has(r.id) && !v.hidden.has(parentOf(r.id)) && (amountOf(v, r).e || {}).full;
  const parentOf = (id) => (id.startsWith("c-") ? "conquest" : "");

  function view(o = {}) {
    return {
      now: MON,
      seen: LOGIN,
      who: CHARS[0],
      playing: true,
      member: true,
      src: { kind: "live" },
      n: { duty: 2, legion: 1 },
      g: snapshot(),
      over: {},
      amt: {},
      custom: [{ ...LEGION }],
      hidden: new Set(HIDDEN),
      reveal: {},
      fold: { today: false, charges: false, week: true },
      open: { sanct: false, explore: false, conquest: false },
      ui: {},
      live: false,
      max: 452,
      scroll: "",
      note: "",
      ...o,
    };
  }

  // ---- Pieces --------------------------------------------------------------------
  let VAR = { arrows: "none", weekly: "done" };

  const attr = (o) =>
    Object.entries(o)
      .filter(([, x]) => x !== undefined && x !== false && x !== null)
      .map(([k, x]) => (x === true ? k : `${k}="${esc(x)}"`))
      .join(" ");

  const has = (list, key) => [].concat(list || []).includes(key);

  function mark(v, key) {
    return (has(v.ui.hover, key) ? " is-hover" : "") + (has(v.ui.press, key) ? " is-press" : "") + (has(v.ui.focus, key) ? " is-focus" : "");
  }

  function button(name, lbl, { act, id, cls = "", disabled = false, sm = true, tab } = {}) {
    return `<button type="button" class="o-btn${sm ? " is-sm" : ""}${cls}" ${attr({ "data-act": act, "data-id": id, "aria-label": lbl, disabled, tabindex: tab })}>${icon(name)}</button>`;
  }

  function nameHtml(it) {
    const acct = it.account ? `<span class="t-acct">${icon("users")}Account</span>` : "";
    return `<span class="t-name">${esc(it.name)}${it.sub ? `<small>${esc(it.sub)}</small>` : ""}${acct}${it.custom ? '<small class="t-yours">Yours</small>' : ""}</span>`;
  }

  // A control's tooltip, under it.
  function tip(v, id) {
    const t = v.ui.tip;
    if (!t || t.id !== id) return "";
    return `<div class="t-tip g-tip" role="tooltip" style="top:${t.top || 24}px;right:${t.right == null ? 20 : t.right}px">${t.text}</div>`;
  }

  const openCls = (r) => (r && r.open ? " mk-open" : "");

  function rowCls(v, it, extra) {
    const c = ["t-row", ...extra.filter(Boolean)];
    if (v.hidden.has(it.id)) c.push("is-off");
    if (has(v.ui.hover, `row:${it.id}`)) c.push("is-hover");
    if (has(v.ui.focus, `row:${it.id}`)) c.push("is-focus");
    return c.join(" ");
  }

  function xSlot(v, it) {
    if (v.hidden.has(it.id)) return `<span class="t-x is-on">${button("eye", `Put back ${label(it)}`, { act: "unhide", id: it.id, cls: mark(v, `x:${it.id}`) })}</span>`;
    const lbl = it.custom ? `Remove ${label(it)}` : `Hide ${label(it)}`;
    return `<span class="t-x">${button("close", lbl, { act: it.custom ? "remove" : "hide", id: it.id, cls: mark(v, `x:${it.id}`), tab: -1 })}</span>`;
  }

  // One less, the figure, one more. Amounts have no upper stop any more: a
  // typed amount may go above the cap.
  function stepper(v, id, n, of, lbl, { amt = false, step = 1, inner, after = "" } = {}) {
    const less = step > 1 ? `${step} less` : "One less";
    const more = step > 1 ? `${step} more` : "One more";
    const dn = button("chevronDown", `${less}: ${lbl}`, { act: "dec", id, cls: " t-arrow" + mark(v, `dec:${id}`), disabled: n <= 0 });
    const up = button("chevronUp", `${more}: ${lbl}`, { act: "inc", id, cls: " t-arrow" + mark(v, `inc:${id}`), disabled: !amt && n >= of });
    const a = attr({
      role: "spinbutton",
      tabindex: 0,
      "aria-label": lbl,
      "aria-valuemin": 0,
      "aria-valuemax": amt ? undefined : of,
      "aria-valuenow": n,
      "aria-valuetext": `${n} of ${of}${!amt && n >= of ? ", done" : ""}`,
      "data-act": amt ? "amt-edit" : "bump",
      "data-id": id,
    });
    return `<span class="t-step">${dn}<span class="t-fig${amt ? " is-amt" : ""}${mark(v, `count:${id}`)}" ${a}>${inner || `${n}<small>/${of}</small>`}</span>${up}${after}</span>`;
  }

  // A value from the game: the figure where a manual figure sits, no arrows;
  // the extra pool after it. A click (or Enter) types over it.
  function gameFig(v, id, inner, lbl, valuetext, extra, { cls = "" } = {}) {
    const src = v.over[id] != null ? "typed by you" : "from the game";
    const a = attr({ role: "button", tabindex: 0, "aria-label": `${lbl}, ${valuetext}, ${src}. Type over it`, "data-act": "g-edit", "data-id": id });
    const x = extra ? `+${extra}` : "";
    return `<span class="g-step"><span class="t-gap"></span><span class="t-fig g-fig${cls}${mark(v, `count:${id}`)}" ${a}>${inner}</span><span class="g-extra">${x}</span></span>`;
  }

  // The same value with the approved arrows kept (variant "keep"): the extra
  // pool after the one-more arrow.
  function gameStepper(v, id, n, of, lbl, extra, opts = {}) {
    const after = `<span class="g-extra is-after">${extra ? `+${extra}` : ""}</span>`;
    return stepper(v, id, n, of, lbl, { ...opts, after });
  }

  const ghost = (inner) => `<span class="t-step t-ghost"><span class="t-gap"></span><span class="t-fig">${inner}</span><span class="t-gap"></span></span>`;

  const typedGut = (v, id) => `<span class="t-gut is-typed" title="Typed by you">${icon("pencil")}</span>`;

  // Done of target (the approved figure), or entries left (variant "left").
  function weekInner(n, of) {
    if (VAR.weekly === "left") return `${Math.max(0, of - n)}<small class="g-left">left</small>`;
    return `${n}<small>/${of}</small>`;
  }

  function countRow(v, it, isSub) {
    const n = countOf(v, it);
    const done = n >= it.of;
    const over = v.over[it.id] != null;
    // A game row keeps the game's layout when typed over; the pencil says so.
    const g = gameOf(v, it.id);
    let right;
    if (v.hidden.has(it.id)) right = ghost(`${Math.min(n, it.of)}<small>/${it.of}</small>`);
    else if (v.ui.editAmt === it.id) right = field(v, it, it.of, gameOf(v, it.id));
    else if (g && VAR.arrows === "none") right = gameFig(v, it.id, weekInner(n, it.of), label(it), VAR.weekly === "left" ? `${Math.max(0, it.of - n)} left` : `${n} of ${it.of} done`, g.x);
    else if (g) right = gameStepper(v, it.id, n, it.of, label(it), g.x, { inner: weekInner(n, it.of) });
    else right = stepper(v, it.id, Math.min(n, it.of), it.of, label(it));
    const gut = over ? typedGut(v, it.id) : `<span class="t-gut"></span>`;
    return `<div class="${rowCls(v, it, [isSub && "is-sub", done && "is-done"])}" data-row="${it.id}">${gut}${nameHtml(it)}${right}${xSlot(v, it)}${tip(v, it.id)}</div>`;
  }

  function tickRow(v, it, isSub) {
    const done = countOf(v, it) >= 1;
    const off = v.hidden.has(it.id);
    const word = done ? "Undo" : "Mark done";
    const check = off
      ? ""
      : `<button type="button" class="o-btn is-sm t-check${done ? " is-undo" : ""}${mark(v, `check:${it.id}`)}" ${attr({ "data-act": "tick", "data-id": it.id, "aria-label": `${word}: ${label(it)}`, title: word, tabindex: -1 })}>${icon(done ? "undo" : "check")}</button>`;
    const a = off ? "" : attr({ role: "button", tabindex: 0, "aria-label": `${label(it)}, ${done ? "done" : "not done"}`, "data-act": "tick", "data-id": it.id });
    const gut = v.over[it.id] != null ? typedGut(v, it.id) : `<span class="t-gut"></span>`;
    return `<div class="${rowCls(v, it, ["is-tick", isSub && "is-sub", done && "is-done"])}" data-row="${it.id}"><span class="t-main" ${a}>${gut}${nameHtml(it)}</span>${check}${xSlot(v, it)}${tip(v, it.id)}</div>`;
  }

  function hiddenLine(v, list, key, sub = false) {
    const h = hiddenIn(list, v).length;
    if (!h) return "";
    const on = !!v.reveal[key];
    const lbl = on ? `Hide again: ${h} hidden` : `Show ${h} hidden`;
    return `<div class="t-hid${sub ? " is-sub" : ""}"><span>${h} hidden</span><span aria-hidden="true">·</span><button type="button" class="t-link${mark(v, `show:${key}`)}" data-act="reveal" data-id="${key}" aria-expanded="${on}" aria-label="${lbl}">${on ? "Hide again" : "Show"}</button></div>`;
  }

  function groupRow(v, it) {
    const act = active(it.items, v);
    const done = act.filter((s) => isDone(s, v)).length;
    const open = !!v.open[it.id];
    const all = act.length > 0 && done === act.length;
    const off = v.hidden.has(it.id);
    const a = off ? "" : attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "aria-label": `${it.name}, ${done} of ${act.length} done`, "data-act": "group", "data-id": it.id });
    const head = `<div class="${rowCls(v, it, ["is-group", all && "is-done"])}" data-row="${it.id}"><span class="t-main" ${a}><span class="t-gut">${icon(open && !off ? "chevronDown" : "chevronRight")}</span>${nameHtml(it)}<span class="t-sum${all ? " is-good" : ""}">${done} of ${act.length} done</span></span>${xSlot(v, it)}</div>`;
    if (!open || off) return head;
    return head + listed(it.items, v, it.id).map((s) => rowFor(v, s, true)).join("") + hiddenLine(v, it.items, it.id, true);
  }

  function rowFor(v, it, isSub = false) {
    if (it.kind === "group") return groupRow(v, it);
    if (it.kind === "tick") return tickRow(v, it, isSub);
    return countRow(v, it, isSub);
  }

  // Typing: the field takes the figure's place. On a game value its × goes
  // back to the game's value; elsewhere it stops tracking, as approved.
  function field(v, r, cap, g) {
    const val = v.ui.editVal == null ? "" : String(v.ui.editVal);
    const input = v.live
      ? `<input class="t-input" data-id="${r.id}" inputmode="numeric" autocomplete="off" placeholder="Amount" aria-label="${esc(r.name)}: how many you have now" value="${esc(val)}">`
      : `<span class="val">${esc(val)}</span><span class="o-caret"></span>`;
    const back = g ? `Back to the game's value: ${r.name}` : `Stop tracking ${r.name}`;
    return `<span class="t-field">${input}<span class="t-cap">/${cap}</span>${button("close", back, { act: g ? "g-back" : "amt-clear", id: r.id, cls: " t-clear" + mark(v, `clear:${r.id}`) })}</span>`;
  }

  function amountRow(v, r, isSub) {
    const s = amountOf(v, r);
    const extra = [isSub && "is-sub"];
    let gut = `<span class="t-gut"></span>`;
    let right;
    let when = "";
    if (v.hidden.has(r.id)) {
      if (s.src === "empty") extra.push("is-empty");
      right = ghost(s.src === "empty" ? "&mdash;" : `${s.n}<small>/${s.cap}</small>`);
    } else if (v.ui.editAmt === r.id) {
      right = field(v, r, s.cap, s.src === "game" || s.src === "over" ? gameOf(v, r.id) : null);
    } else if (s.src === "empty") {
      extra.push("is-empty");
      const at = attr({ role: "spinbutton", tabindex: 0, "aria-label": r.name, "aria-valuetext": "not tracked", "data-act": "amt-edit", "data-id": r.id });
      right = `<span class="t-step"><span class="t-gap"></span><span class="t-fig is-amt is-blank${mark(v, `count:${r.id}`)}" ${at}>&mdash;</span><span class="t-gap"></span></span>`;
    } else {
      const e = s.e;
      if (s.src === "over") gut = typedGut(v, r.id);
      if (e.full) {
        extra.push("is-waste");
        gut = `<span class="t-gut is-warn">${icon("alert")}</span>`;
      }
      const w = e.above ? "" : e.full ? (r.every ? "Full, being wasted" : `Full, spend by ${stamp(e.next)}`) : `Full in ${span(e.at - v.now)}`;
      when = w ? `<span class="t-when${e.full ? " is-warn" : ""}">${w}</span>` : "";
      const inner = `${s.n}<small${r.open === "deus" ? ' class="mk-open"' : ""}>/${s.cap}</small>`;
      const fromGame = s.src === "game" || s.src === "over";
      if (fromGame && VAR.arrows === "none") right = gameFig(v, r.id, inner, r.name, `${s.n} of ${s.cap}${s.x ? `, ${s.x} extra` : ""}`, s.x);
      else if (fromGame) right = gameStepper(v, r.id, s.n, s.cap, r.name, s.x, { amt: true, step: r.step || 1, inner });
      else right = stepper(v, r.id, s.n, s.cap, r.name, { amt: true, step: r.step || 1, inner });
    }
    const rowOpen = r.open === "conquest" && !isSub ? " mk-open" : "";
    return `<div class="${rowCls(v, r, extra)}${rowOpen}" data-row="${r.id}">${gut}${nameHtml(r)}${when}${right}${xSlot(v, r)}${tip(v, r.id)}</div>`;
  }

  // Per-dungeon Conquest tickets (open question 2): a folding group like
  // Sanctuary, each dungeon its own amount.
  function conquestGroup(v, it) {
    const open = !!v.open[it.id];
    const vals = it.items.map((r) => amountOf(v, r)).filter((s) => s.src !== "empty");
    const full = it.items.filter((r) => isFull(v, r)).length;
    const lo = vals.length ? Math.min(...vals.map((s) => s.n)) : null;
    const hi = vals.length ? Math.max(...vals.map((s) => s.n)) : null;
    const sum = full ? `<span class="t-sum" style="color:var(--o-warn);font-weight:600">${full} full</span>` : `<span class="t-sum">${lo == null ? "6 dungeons" : `${lo === hi ? lo : `${lo}–${hi}`} of 14 each`}</span>`;
    const a = attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "aria-label": `${it.name}, ${it.items.length} dungeons`, "data-act": "group", "data-id": it.id });
    const head = `<div class="${rowCls(v, it, ["is-group"])} mk-open" data-row="${it.id}"><span class="t-main" ${a}><span class="t-gut">${icon(open ? "chevronDown" : "chevronRight")}</span>${nameHtml(it)}${sum}</span>${xSlot(v, it)}</div>`;
    if (!open) return head;
    return head + it.items.map((r) => amountRow(v, r, true)).join("");
  }

  function sec(v, id, title, { n = "", good = false, at = "", chip = "" } = {}) {
    const open = !v.fold[id];
    const a = attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "data-act": "fold", "data-id": id });
    return `<div class="t-sec${mark(v, `sec:${id}`)}" ${a}>${icon(open ? "chevronDown" : "chevronRight")}<span>${title}</span>${n ? `<span class="n${good ? " is-good" : ""}">${n}</span>` : ""}${chip}<span class="at">${at}</span></div>`;
  }

  function todaySec(v) {
    const list = todayList(v);
    const l = left(list, v);
    const head = sec(v, "today", "Today", { n: l ? `${l} left` : "done", good: !l, at: `resets in <b>${span(nextAt(v.now, RESET) - v.now)}</b>` });
    if (v.fold.today) return head;
    return head + `<div class="t-list">${listed(list, v, "today").map((it) => rowFor(v, it)).join("")}${hiddenLine(v, list, "today")}</div>`;
  }

  function chargeSec(v) {
    const leaves = chargeLeaves().filter((r) => !v.hidden.has(r.id) && !v.hidden.has(parentOf(r.id)));
    const tracked = leaves.filter((r) => amountOf(v, r).src !== "empty");
    const full = tracked.filter((r) => amountOf(v, r).e.full).length;
    const filling = tracked.map((r) => amountOf(v, r).e).filter((e) => !e.full && !e.above).map((e) => e.at);
    const at = filling.length ? `next full in <b>${span(Math.min(...filling) - v.now)}</b>` : tracked.length ? "" : "amounts optional";
    const head = sec(v, "charges", "Recharges", { n: tracked.length ? `${tracked.length} tracked` : "", chip: full ? warnChip(`${full} full`) : "", at });
    if (v.fold.charges) return head;
    const rows = listed(CHARGES, v, "charges").map((r) => (r.kind === "cgroup" ? conquestGroup(v, r) : amountRow(v, r)));
    return head + `<div class="t-list">${rows.join("")}${hiddenLine(v, CHARGES, "charges")}</div>`;
  }

  function weekSec(v) {
    const list = weekList(v);
    const l = left(list, v);
    const w = nextWeekly(v.now);
    const head = sec(v, "week", "This week", { n: l ? `${l} left` : "done", good: !l, at: `${stamp(w)} · in <b>${span(w - v.now)}</b>` });
    if (v.fold.week) return head;
    return head + `<div class="t-list">${listed(list, v, "week").map((it) => rowFor(v, it)).join("")}${hiddenLine(v, list, "week")}</div>`;
  }

  const addLine = (v) => `<button type="button" class="t-addline${mark(v, "addline")}" data-act="add-open">${icon("plus")}<span>Add a to-do</span></button>`;

  const fullCount = (v) => chargeLeaves().filter((r) => isFull(v, r)).length;

  function head(v) {
    const l = left(todayList(v), v) + left(weekList(v), v);
    const full = fullCount(v);
    const chip = full ? warnChip(`${full} full`) : "";
    return `<div class="o-head"><span class="o-title">${icon("listChecks")}To-dos</span><span class="o-count">${l ? `${l} left` : "all done"}</span>${chip}<span class="grow"></span>${button("close", "Close", { sm: false })}</div>`;
  }

  function charSelect(v, ctx) {
    const me = v.who;
    const a = attr({ role: "combobox", tabindex: 0, "aria-expanded": "false", "aria-label": "Character", "data-act": "menu" });
    const nm = esc(ctx.opts.mask ? ctx.fmt.mask(me.name) : me.name);
    return `<div class="t-char"><span class="o-input" ${a}><img src="${ctx.classIcon(me.cls)}" alt=""><span class="nm">${nm}</span><span class="meta">${me.klass} · ${me.server}</span>${v.playing ? '<span class="t-live">Playing</span>' : '<span class="grow" style="flex:1"></span>'}${icon("chevronDown")}</span></div>`;
  }

  // Proposed: one line under the selector says where the values come from.
  function sourceLine(v) {
    const s = v.src;
    const dot = '<span class="faint">·</span>';
    let text;
    if (s.kind === "live") text = `From the game, live ${dot} logged in at <b>${clock(v.seen)}</b>`;
    else if (s.kind === "stale") text = `From the game at login, <b>${stamp(v.seen)}</b> ${dot} refills since estimated`;
    else if (s.kind === "shared") text = `<b>${esc(v.who.name)}</b> not seen at login yet ${dot} Account rows from ${esc(s.from)}`;
    else text = `No game values yet. Log in with <b>${esc(v.who.name)}</b> while Kisk runs.`;
    return `<div class="g-src">${icon("gamepad")}<span>${text}</span></div>`;
  }

  function noteHtml(v) {
    if (v.note) return `<div class="o-note is-good t-note" role="status">${icon("check")}<span>${v.note}</span></div>`;
    return "";
  }

  function foot() {
    return `<div class="o-foot t-foot">${icon("clock")}<span>Daily reset <b>${pad(RESET / 60)}:00</b></span><span class="faint">·</span><button type="button" class="t-link" data-act="reset-hour" aria-label="Change the daily reset hour in Settings, To-dos">Change</button><span class="grow"></span><span class="faint">Weekly on Wednesday</span></div>`;
  }

  function windowHtml(v, ctx) {
    const body = todaySec(v) + chargeSec(v) + weekSec(v) + addLine(v);
    const scroll = `<div class="t-scrollwrap"><div class="t-scroll" style="max-height:${v.max}px"${v.scroll ? ` data-scroll="${v.scroll}"` : ""}>${body}</div><span class="o-thumb t-thumb" aria-hidden="true"></span></div>`;
    return `<div class="ov o td" role="dialog" aria-label="To-dos"${v.live ? " data-live" : ""}>${head(v)}${charSelect(v, ctx)}${sourceLine(v)}${noteHtml(v)}${scroll}${foot()}</div>`;
  }

  // ---- Scroll thumbs (GPUI draws its own 3 px thumb) ------------------------
  function thumb(s) {
    const wrap = s.parentElement;
    const t = wrap.querySelector(".t-thumb");
    const over = s.scrollHeight > s.clientHeight + 1;
    wrap.classList.toggle("is-over", over);
    if (!over || !t) return;
    const h = Math.max(24, (s.clientHeight / s.scrollHeight) * (s.clientHeight - 6));
    const room = s.scrollHeight - s.clientHeight;
    const top = 3 + (room > 0 ? (s.scrollTop / room) * (s.clientHeight - 6 - h) : 0);
    t.style.setProperty("--h", h.toFixed(1) + "px");
    t.style.setProperty("--t", top.toFixed(1) + "px");
  }

  function syncThumbs() {
    document.querySelectorAll(".t-scroll").forEach((s) => {
      thumb(s);
      if (s.dataset.scroll && !s.dataset.placed) {
        const row = s.querySelector(`[data-row="${s.dataset.scroll}"]`);
        if (s.dataset.scroll === "end") s.scrollTop = s.scrollHeight;
        else if (row) s.scrollTop = row.offsetTop - 3;
        s.dataset.placed = "1";
      }
      thumb(s);
    });
  }

  document.addEventListener("scroll", (e) => e.target.classList && e.target.classList.contains("t-scroll") && thumb(e.target), true);

  // ---- The working window ("Try it"); nothing is stored ----------------------
  const LIVE = view({ live: true, fold: { today: false, charges: false, week: false }, open: { sanct: false, explore: false, conquest: false } });
  let lastCtx = null;
  let redrawing = false;
  let noteTimer = 0;

  const sr = document.createElement("div");
  sr.className = "t-sr";
  sr.setAttribute("aria-live", "polite");
  document.body.appendChild(sr);

  function say(text) {
    sr.textContent = "";
    requestAnimationFrame(() => (sr.textContent = text));
  }

  function redraw(focus) {
    const tiles = [...document.querySelectorAll("[data-live]")];
    const activeEl = document.activeElement;
    const at = tiles.findIndex((t) => t.contains(activeEl));
    const key = focus || (at >= 0 && activeEl.dataset && activeEl.dataset.act ? [activeEl.dataset.act, activeEl.dataset.id] : null);
    const tops = tiles.map((t) => t.querySelector(".t-scroll").scrollTop);
    redrawing = true;
    tiles.forEach((t) => (t.outerHTML = windowHtml(LIVE, lastCtx)));
    redrawing = false;
    const fresh = [...document.querySelectorAll("[data-live]")];
    fresh.forEach((t, i) => (t.querySelector(".t-scroll").scrollTop = tops[i] || 0));
    syncThumbs();
    const home = fresh[Math.max(0, at)];
    if (!home) return;
    if (LIVE.ui.editAmt) {
      const input = home.querySelector(".t-field .t-input");
      if (input) {
        input.focus();
        input.select();
      }
      return;
    }
    if (!key || (at < 0 && !focus)) return;
    const el = home.querySelector(`[data-act="${key[0]}"]` + (key[1] ? `[data-id="${key[1]}"]` : ""));
    if (el && !el.disabled) el.focus({ preventScroll: true });
  }

  function flash(note) {
    clearTimeout(noteTimer);
    LIVE.note = note;
    noteTimer = setTimeout(() => {
      LIVE.note = "";
      redraw();
    }, 4000);
  }

  // The value the field opens with: the figure as shown.
  function shownValue(v, id) {
    const r = findCharge(id);
    if (r) {
      const s = amountOf(v, r);
      return s.src === "empty" ? "" : String(s.n);
    }
    const it = findItem(v, id);
    return String(countOf(v, it));
  }

  // Enter keeps (numbers above the cap too), Esc cancels. An empty field
  // goes back to the game's value on a game row, else stops tracking.
  function commit(input, save, refocus) {
    const v = LIVE;
    const id = v.ui.editAmt;
    if (!id) return;
    if (save) {
      const s = input.value.trim();
      const fromGame = !!v.g[id];
      const it = findItem(v, id);
      if (s === "") {
        if (fromGame) delete v.over[id];
        else delete v.amt[id];
      } else {
        let num = parseInt(s.replace(/[^0-9]/g, ""), 10);
        if (!Number.isNaN(num)) {
          if (it) num = clamp(num, 0, it.of);
          if (fromGame) v.over[id] = num;
          else if (it) v.n[id] = num;
          else v.amt[id] = num;
        }
      }
    }
    v.ui.editAmt = null;
    redraw(refocus ? [v.g[id] ? "g-edit" : "amt-edit", id] : null);
  }

  function act(name, id) {
    const v = LIVE;
    const r = findCharge(id);
    const it = findItem(v, id);
    let focus;
    switch (name) {
      case "inc":
      case "dec": {
        const d = name === "inc" ? 1 : -1;
        if (r) {
          const s = amountOf(v, r);
          const next = Math.max(0, (s.n || 0) + d * (r.step || 1));
          if (v.g[id]) v.over[id] = next;
          else v.amt[id] = next;
        } else if (it) {
          const next = clamp(countOf(v, it) + d, 0, it.of);
          if (v.g[id]) v.over[id] = next;
          else v.n[id] = next;
          if (next >= it.of && name === "inc") focus = ["bump", id];
        }
        break;
      }
      case "bump":
        if (it) {
          const next = clamp(countOf(v, it) + 1, 0, it.of);
          if (v.g[id]) v.over[id] = next;
          else v.n[id] = next;
        }
        break;
      case "tick": {
        const done = countOf(v, it) >= 1;
        if (v.g[id]) v.over[id] = done ? 0 : 1;
        else v.n[id] = done ? 0 : 1;
        say(`${label(it)}: ${done ? "not done" : "done"}`);
        break;
      }
      case "fold":
        v.fold[id] = !v.fold[id];
        break;
      case "group":
        v.open[id] = !v.open[id];
        break;
      case "g-edit":
      case "amt-edit":
        v.ui.editAmt = id;
        v.ui.editVal = shownValue(v, id);
        break;
      case "g-back":
        delete v.over[id];
        v.ui.editAmt = null;
        focus = ["g-edit", id];
        say(`${(r || it).name}: back to the game's value`);
        break;
      case "amt-clear":
        delete v.amt[id];
        v.ui.editAmt = null;
        break;
      case "hide":
        v.hidden.add(id);
        break;
      case "unhide":
        v.hidden.delete(id);
        break;
      case "reveal":
        v.reveal[id] = !v.reveal[id];
        focus = ["reveal", id];
        break;
      default:
        return; // close, menu, reset hour, add: unchanged from the approved window
    }
    redraw(focus);
  }

  // Simulated game messages (mockup only): a spend, a refill, a fresh login.
  // The game's new value always replaces a value typed over it.
  function simulate(kind) {
    const v = LIVE;
    const g = v.g;
    const spend = (id) => {
      if (g[id].n > 0) g[id].n -= 1;
      else if (g[id].x > 0) g[id].x -= 1;
      delete v.over[id];
    };
    if (kind === "shugo") spend("shugo");
    if (kind === "fissure") spend("fissure");
    if (kind === "odyle") {
      g.odyle.n = Math.min(capOf(v, findCharge("odyle")), g.odyle.n + 15);
      delete v.over.odyle;
    }
    if (kind === "gain") {
      g.shugo.x += 2;
      delete v.over.shugo;
    }
    if (kind === "login") {
      v.g = snapshot();
      v.over = {};
      v.ui.editAmt = null;
      flash("Filled from the game at login.");
    }
    redraw();
  }

  document.addEventListener("mousedown", (e) => {
    if (e.target.closest && e.target.closest(".t-clear")) e.preventDefault();
  });

  document.addEventListener("click", (e) => {
    const sim = e.target.closest && e.target.closest("[data-sim]");
    if (sim) return simulate(sim.dataset.sim);
    const tile = e.target.closest && e.target.closest("[data-live]");
    if (!tile) return;
    let el = e.target.closest("[data-act]");
    if (!el && !e.target.closest(".t-x")) {
      const row = e.target.closest(".t-row.is-tick:not(.is-off)");
      if (row) el = row.querySelector(".t-main");
    }
    if (!el || el.disabled || el.tagName === "INPUT") return;
    act(el.dataset.act, el.dataset.id);
  });

  document.addEventListener("keydown", (e) => {
    const el = e.target;
    if (!el.closest || !el.closest("[data-live]")) return;
    if (el.classList.contains("t-input") && el.closest(".t-field")) {
      if (e.key === "Enter" || e.key === "Escape") {
        e.preventDefault();
        commit(el, e.key === "Enter", true);
      }
      return;
    }
    const a = el.dataset && el.dataset.act;
    if (!a) return;
    if (el.getAttribute("role") === "spinbutton" && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      act(e.key === "ArrowUp" ? "inc" : "dec", el.dataset.id);
      return;
    }
    if (el.tagName !== "BUTTON" && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      act(a, el.dataset.id);
    }
  });

  document.addEventListener("focusout", (e) => {
    if (redrawing) return;
    const el = e.target;
    if (el.classList && el.classList.contains("t-input") && el.closest(".t-field") && el.isConnected) {
      if (e.relatedTarget && e.relatedTarget.classList.contains("t-clear")) return;
      commit(el, true);
    }
  });

  // ---- States --------------------------------------------------------------------
  const ONLY_CHARGES = { today: true, charges: false, week: true };
  const tipText = (main, sub) => `${main}${sub ? `<br><span class="faint">${sub}</span>` : ""}`;

  // Yesterday's login: what the game said then; the window adds the refills
  // since (one daily reset, Monday 09:00; eight Odyle ticks).
  const STALE_G = snapshot({
    odyle: { n: 125, x: 380 },
    shugo: { n: 4, x: 3 },
    invasion: { n: 1, x: 0 },
    nightmare: { n: 0, x: 3 },
    conquest: { n: 13, x: 0 },
    "c-krao": { n: 10 },
    "c-urugugu": { n: 10 },
    "c-fire": { n: 8 },
    "c-draupnir": { n: 10 },
    "c-vakron": { n: 11 },
    "c-horn": { n: 10 },
    deus: { n: 2 },
    arkanis: { n: 2 },
    balaur: { n: 5 },
    fafnium: { n: 5 },
  });

  // Tavrin, never seen at login: only the rows his server shares come from
  // the game (Mirelle's login today).
  const SHARED_G = Object.fromEntries(Object.entries(snapshot()).filter(([id]) => SERVER_ROWS.has(id)));

  const STATES = {
    live: () => view({ fold: ONLY_CHARGES }),
    hoverGame: () =>
      view({
        fold: ONLY_CHARGES,
        ui: {
          hover: ["row:shugo", "count:shugo"],
          tip: { id: "shugo", right: 24, text: tipText("From the game, live.", "+3 extra from items: they never refill and are not part of the 21.") },
        },
      }),
    full: () =>
      view({
        fold: ONLY_CHARGES,
        g: snapshot({ odyle: { n: 840, x: 245 }, shugo: { n: 21, x: 3 }, invasion: { n: 2, x: 12 }, deus: { n: 2 }, arkanis: { n: 2 } }),
      }),
    week: () => view({ fold: { today: true, charges: true, week: false }, open: { sanct: true, explore: false } }),
    compare: () =>
      view({
        fold: { today: false, charges: false, week: true },
        ui: { hover: ["row:duty", "count:duty"], tip: { id: "duty", right: 44, text: tipText("Counted by you.", "The game does not send Duty Missions.") } },
      }),
    stale: () =>
      view({
        playing: false,
        seen: SUN,
        src: { kind: "stale" },
        g: STALE_G,
        fold: ONLY_CHARGES,
        ui: {
          hover: ["row:odyle", "count:odyle"],
          tip: { id: "odyle", right: 24, text: tipText("125 at your login Sun 22:40.", "+120 since: eight refills of 15, estimated.") },
        },
      }),
    typing: () =>
      view({
        fold: ONLY_CHARGES,
        ui: { editAmt: "shugo", editVal: "9", hover: ["clear:shugo"], tip: { id: "shugo", right: 21, text: tipText("Back to the game's value, 7") } },
      }),
    typed: () =>
      view({
        fold: ONLY_CHARGES,
        over: { shugo: 9 },
        ui: { hover: ["row:shugo", "count:shugo"], tip: { id: "shugo", right: 44, text: tipText("Typed by you. The game said 7.", "Its next value replaces yours: a spend, a refill or your next login.") } },
      }),
    login: () => view({ fold: ONLY_CHARGES, note: "Filled from the game at login. Values you typed were replaced." }),
    first: () =>
      view({
        who: CHARS[2],
        playing: false,
        src: { kind: "none" },
        g: {},
        n: { duty: 0 },
        amt: { odyle: 425, shugo: 24 },
        fold: { today: false, charges: false, week: true },
      }),
    firstShared: () =>
      view({
        who: CHARS[1],
        playing: false,
        src: { kind: "shared", from: "Mirelle" },
        g: SHARED_G,
        n: { duty: 2 },
        custom: [],
        fold: { today: true, charges: false, week: false },
        max: 520,
      }),
    openData: () => view({ fold: ONLY_CHARGES, open: { conquest: true } }),
    memberOff: () =>
      view({
        member: false,
        fold: ONLY_CHARGES,
        g: snapshot({ shugo: { n: 18, x: 3 }, odyle: { n: 605, x: 380 } }),
        ui: { hover: ["row:shugo", "count:shugo"], tip: { id: "shugo", right: 24, text: tipText("From the game, live.", "18 is more than 12: only a membership allows that. Settings, To-dos asks about it.") } },
      }),
  };

  function simStrip() {
    return `<div class="mk-sim" role="group" aria-label="Simulated game messages"><b>Mockup only: messages from the game</b><button type="button" data-sim="shugo">Spend a Shugo key</button><button type="button" data-sim="gain">+2 Shugo keys from an item</button><button type="button" data-sim="odyle">Odyle refill +15</button><button type="button" data-sim="fissure">Run the Unknown Fissure</button><button type="button" data-sim="login">Log in again</button></div>`;
  }

  function render(state, ctx) {
    lastCtx = ctx;
    VAR = ctx.v;
    if (state === "try") return `<div style="position:relative;z-index:1">${windowHtml(LIVE, ctx)}${simStrip()}</div>`;
    const make = STATES[state];
    return make ? windowHtml(make(), ctx) : "";
  }

  // ---- Open data controls (mockup only): reload with ?deus= / ?conquest= --
  function openControls() {
    const sel = (key, lbl, opts) =>
      `<label><span class="mk-legend">${lbl}</span> <select data-open="${key}">${opts.map(([val, text]) => `<option value="${val}"${String(OPEN[key]) === val ? " selected" : ""}>${text}</option>`).join("")}</select></label>`;
    return `<div class="controls" role="region" aria-label="Open data questions"><div class="controls__inner"><span>Open data (not decided; dashed in the window):</span>${sel("deus", "1. Deus / Arkanis cap", [
      ["4", "4, the ticket table"],
      ["14", "14, our list today"],
    ])}${sel("conquest", "2. A Conquest entry spends", [
      ["dungeon", "its dungeon's ticket (six rows)"],
      ["pool", "a shared pool (one row, 21, +1 every 8 h)"],
    ])}</div></div>`;
  }

  const main = document.querySelector("main.scenes");
  if (main) new MutationObserver(() => requestAnimationFrame(syncThumbs)).observe(main, { childList: true });
  if (document.fonts) document.fonts.ready.then(syncThumbs);

  const A = tag("Agreed");
  const P = tag("Proposed");
  const O = tag("Open");

  Kit.mount({
    states: [
      { id: "live", title: "Filled from the game, live", sub: `${A} "7/21" and "+3": the regular pool against its cap, the extra pool beside it; membership on, so Shugo keys 21 and Odyle 840. ${P} a game value has no arrows (the arrows mean "you count this"); one line under the selector says where values come from; Shugo, Invasion keys get "Account". ${O} Deus and Arkanis at 4/4 are full only if their cap is 4 (dashed)` },
      { id: "hoverGame", title: "Pointer on a game value", sub: `${P} the figure reads as a field (a click types over it); its tooltip says where it comes from and what "+3" is` },
      { id: "full", title: "Full, being wasted", sub: `${A} only when the regular pool is at its cap: Odyle 840/840 +245 and Shugo 21/21 +3 are full; Invasion keys 2/7 +12 (14 in all, above the cap) is normal, never an error or a warning` },
      { id: "week", title: "This week from the game", sub: `${P} the game sends entries left; the row reads done = target − left (Unknown Fissure: 9 left of 14 → 5/14), its extra entries beside it (+7). Done at 0 left, struck through as approved. Switch "Weekly rows" above to see "9 left" instead` },
      { id: "compare", title: "A game value next to a manual one", sub: `${P} Duty Missions keep the approved arrows: the game does not send them, so you count them; pointer on its figure. Game rows below have no arrows` },
      { id: "stale", title: "From yesterday's login", sub: `${P} Mirelle is not in game; values are from Sunday 22:40, plus the refills the table knows since (Monday 09:00 reset, Odyle every 3 h), said once on the line under the selector; pointer on Odyle's figure` },
      { id: "typing", title: "Typing over a game value", sub: `${A} a typed number may be above the cap. ${P} a click on the figure opens the approved field; its × (or an empty field) goes back to the game's value instead of "stop tracking"` },
      { id: "typed", title: "After typing over it", sub: `${P} your number takes the figure's place with a pencil in the gutter; the extra pool stays the game's (+3). The game's next value for that ticket (spend, refill, item, login) replaces yours` },
      { id: "login", title: "Just logged in", sub: `${P} the snapshot at login fills every row and replaces values you typed; a note for 4 s` },
      { id: "first", title: "Before the first login is seen", sub: `${P} Sefa, alone on Lyss: nothing from the game yet; the line says how to get values; everything works by hand as approved. ${A} a typed 24 Shugo keys (above 21) is kept, not cut. ${P} a typed number is your total, so at or above the cap it counts as full` },
      { id: "firstShared", title: "New character, server already seen", sub: `${P} Tavrin has never logged in with the meter, but Mirelle shares his server: Shugo keys, Invasion keys and the Unknown Fissure come from her login, the rest waits for his` },
      { id: "openData", title: "Open data, drawn as placeholders", sub: `${O} dashed: which ticket a Conquest entry spends (here: one ticket per dungeon, a folding group like Sanctuary; switch above for one shared pool) and the Deus / Arkanis cap (4 or 14). Not decided by this mockup` },
      { id: "memberOff", title: "Membership off, the game shows more", sub: `${A} a total above the cap is normal. ${P} with the switch off, 18 Shugo keys (above 12) and 605 Odyle Energy (above 560) can only come from a membership: shown as they are, never full, no estimate; Settings, To-dos asks about the membership` },
      { id: "try", title: "Try it", sub: "a working window: click a game figure to type over it (above the cap too), × or an empty field goes back to the game's value; the buttons under it play messages from the game. Nothing is saved" },
    ],
    variants: [
      {
        key: "arrows",
        label: "Game rows",
        def: "none",
        options: [
          { value: "none", label: "figure only, no arrows" },
          { value: "keep", label: "keep the arrows" },
        ],
      },
      {
        key: "weekly",
        label: "Weekly rows",
        def: "done",
        options: [
          { value: "done", label: "done of target (5/14)" },
          { value: "left", label: "entries left (9 left)" },
        ],
      },
    ],
    controls: ["alpha", "scale", "mask"],
    tileMin: 430,
    stageMin: 600,
    render,
  });

  if (main && !new URLSearchParams(location.search).has("embed")) {
    main.insertAdjacentHTML("beforebegin", openControls());
    document.querySelectorAll("[data-open]").forEach((el) =>
      el.addEventListener("change", () => {
        const p = new URLSearchParams(location.search);
        p.set(el.dataset.open, el.value);
        location.search = p.toString();
      })
    );
  }
})();
