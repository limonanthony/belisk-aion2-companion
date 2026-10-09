// The approved to-do window: direction E (2026-10-06) with values from the
// game (approved 2026-10-08). Drawn by todos.html; its styles are todos.css.
//
// Everything here is invented: counts, times, character names. Limits and
// reset rules: the reset research (MetaBot's client tables for the Global
// client 0.0.4387.0; Game8 for Duty Missions and Duty Commands) and the
// game's ticket table (which ticket a row reads). Nothing is saved or sent
// anywhere.
//
// No Recharges (decided 2026-10-08, "I think we can remove the recharges
// from the todos now no ?"): Odyle Energy, Shugo keys, Invasion keys,
// Nightmare and the Daily Dungeon are drawn only in the Resources window
// (hub.js) and the toolbar. The to-do window keeps Today, This week and
// your own to-dos.
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
  const nm = (ctx, name) => esc(ctx.opts.mask ? ctx.fmt.mask(name) : name);

  // ---- Clock (mocked): Monday 21:48, daily reset 09:00, weekly Wednesday ----
  const DAY = 1440;
  const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const RESET = 9 * 60; // the "Daily reset" setting, 09:00 by default
  const WEEKLY = 2 * DAY + RESET; // Wednesday at the reset hour
  const MON = 21 * 60 + 48; // Mon 21:48, now
  const LOGIN = 19 * 60 + 2; // Mirelle logged in today at 19:02
  const SUN = 22 * 60 + 40 - DAY; // yesterday, Sun 22:40
  const WED = WEEKLY + 12; // Wed 09:12, just after the weekly reset

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

  // ---- The default list ----------------------------------------------------
  const TODAY = [{ id: "duty", kind: "count", name: "Duty Missions", of: 5, account: true, manual: true }];

  // This week: only what the game's menu has a weekly limit for (settled in
  // game, 2026-10-08: no cap on Transcendence, Exploration or Conquest; no
  // Subjugation, Sanctuary or Ordeal entry in the menu).
  const WEEK = [
    { id: "trial", kind: "count", name: "Ascension Trial", of: 3 },
    { id: "dutycmd", kind: "count", name: "Duty Commands", of: 20, manual: true },
  ];

  // Hidden until the player shows them (Game8's figure, not the client's).
  const HIDDEN = ["dutycmd"];

  // A to-do of the player's own (invented).
  const LEGION = { id: "legion", kind: "tick", name: "Legion check-in", custom: true, reset: "daily", manual: true };

  // Characters the meter remembers (invented).
  const CHARS = [
    { id: "mirelle", name: "Mirelle", cls: "ranger", klass: "Ranger", server: "Ossia" },
    { id: "tavrin", name: "Tavrin", cls: "templar", klass: "Templar", server: "Ossia" },
    { id: "sefa", name: "Sefa", cls: "cleric", klass: "Cleric", server: "Lyss" },
  ];

  // ---- What the game sent (n = entries left; x = extra entries) ------------
  function snapshot(o = {}) {
    const g = { trial: { n: 1, x: 1 } };
    for (const [id, val] of Object.entries(o)) g[id] = { ...g[id], ...val };
    return g;
  }

  // Every weekly row's entries left set to `left(item)`: a fresh week (all
  // of them) or everything done (0).
  function weekly(g, left) {
    WEEK.forEach((it) => {
      if (!it.manual && g[it.id]) g[it.id] = { ...g[it.id], n: left(it) };
    });
    return g;
  }

  // ---- Lookups ---------------------------------------------------------------
  const findItem = (v, id) => TODAY.concat(WEEK, v.custom).find((it) => it.id === id);
  const todayList = (v) => TODAY.concat(v.custom.filter((c) => c.reset === "daily"));
  const weekList = (v) => WEEK.concat(v.custom.filter((c) => c.reset === "weekly"));
  // Hidden rows never count; "Show" lists them in place, faded.
  const active = (list, v) => list.filter((it) => !v.hidden.has(it.id));
  const hiddenIn = (list, v) => list.filter((it) => v.hidden.has(it.id));
  const listed = (list, v, key) => (v.reveal[key] ? list : active(list, v));
  const label = (it) => (it.sub ? `${it.name}, ${it.sub}` : it.name);
  const clamp = (x, lo, hi) => Math.max(lo, Math.min(hi, x));

  // The game's value for a row. Live values are the game's latest (it sends
  // every run and gain); a value from an earlier login is the game's then,
  // except that a weekly row seen before this week's reset starts over.
  function gameOf(v, id) {
    const g = v.g[id];
    if (!g) return null;
    const at = g.at == null ? v.seen : g.at;
    if (v.src.kind !== "stale") return { ...g, at, seen: g.n };
    const it = findItem(v, id);
    if (it && at < nextWeekly(v.now) - 7 * DAY) return { ...g, at, seen: g.n, n: it.of || 1, weekly: true };
    return { ...g, at, seen: g.n };
  }

  // Done count of a count or tick row: typed over, from the game (done =
  // target − entries left), or counted by hand.
  function countOf(v, it) {
    if (v.over[it.id] != null) return v.over[it.id];
    const g = gameOf(v, it.id);
    if (g) return Math.max(0, it.of - g.n);
    return v.n[it.id] || 0;
  }

  const isDone = (it, v) => countOf(v, it) >= (it.of || 1);
  const left = (list, v) => active(list, v).filter((it) => !isDone(it, v)).length;

  function view(o = {}) {
    return {
      now: MON,
      seen: LOGIN,
      who: CHARS[0],
      playing: true,
      src: { kind: "live" },
      n: { duty: 2, legion: 1 },
      g: snapshot(),
      over: {},
      custom: [{ ...LEGION }],
      hidden: new Set(HIDDEN),
      reveal: {},
      fold: { today: false, week: false },
      ui: {},
      live: false,
      menu: false,
      max: 452,
      scroll: "",
      note: "",
      removed: null,
      adding: false,
      add: { name: "", kind: "tick", of: 3, reset: "daily" },
      ...o,
    };
  }

  // ---- Pieces ------------------------------------------------------------------
  const attr = (o) =>
    Object.entries(o)
      .filter(([, x]) => x !== undefined && x !== false && x !== null)
      .map(([k, x]) => (x === true ? k : `${k}="${esc(x)}"`))
      .join(" ");

  const has = (list, key) => [].concat(list || []).includes(key);

  // A drawn pointer or keyboard moment: "hover", "press" or "focus" in v.ui,
  // keyed "<part>:<id>".
  function mark(v, key) {
    return (has(v.ui.hover, key) ? " is-hover" : "") + (has(v.ui.press, key) ? " is-press" : "") + (has(v.ui.focus, key) ? " is-focus" : "");
  }

  function button(name, lbl, { act, id, cls = "", disabled = false, sm = true, tab } = {}) {
    return `<button type="button" class="o-btn${sm ? " is-sm" : ""}${cls}" ${attr({ "data-act": act, "data-id": id, "aria-label": lbl, disabled, tabindex: tab })}>${icon(name)}</button>`;
  }

  // "Account" is its Lucide users icon only, 3 px after the name; the word
  // shows under the pointer (2026-10-08: the word left no room for warned
  // names).
  function nameHtml(v, it) {
    const acct = it.account ? `<span class="t-acct${mark(v, `acct:${it.id}`)}" role="img" aria-label="Account" title="Account">${icon("users")}</span>` : "";
    return `<span class="t-name">${esc(it.name)}${it.sub ? `<small>${esc(it.sub)}</small>` : ""}${acct}${it.custom ? '<small class="t-yours">Yours</small>' : ""}</span>`;
  }

  // A control's tooltip, under it: right-aligned to a control at the row's
  // end, or from `left` under the Account icon.
  function tip(v, id) {
    const t = v.ui.tip;
    if (!t || t.id !== id) return "";
    const at = t.left != null ? `left:${t.left}px` : `right:${t.right == null ? 20 : t.right}px`;
    return `<div class="t-tip g-tip" role="tooltip" style="top:${t.top || 24}px;${at}">${t.text}</div>`;
  }

  function rowCls(v, it, extra) {
    const c = ["t-row", ...extra.filter(Boolean)];
    if (v.hidden.has(it.id)) c.push("is-off");
    if (has(v.ui.hover, `row:${it.id}`)) c.push("is-hover");
    if (has(v.ui.focus, `row:${it.id}`)) c.push("is-focus");
    return c.join(" ");
  }

  // The row's last slot, 20 px, kept on every row so nothing moves when the
  // × appears. A hidden row listed by "Show" puts it back instead. Your own
  // to-dos are removed (with Undo), not hidden.
  function xSlot(v, it) {
    if (v.hidden.has(it.id)) return `<span class="t-x is-on">${button("eye", `Put back ${label(it)}`, { act: "unhide", id: it.id, cls: mark(v, `x:${it.id}`) })}</span>`;
    const lbl = it.custom ? `Remove ${label(it)}` : `Hide ${label(it)}`;
    return `<span class="t-x">${button("close", lbl, { act: it.custom ? "remove" : "hide", id: it.id, cls: mark(v, `x:${it.id}`), tab: -1 })}</span>`;
  }

  // One less, the figure, one more: what you count yourself (Duty Missions,
  // Duty Commands, your own, every row before the game has filled it).
  function stepper(v, id, n, of, lbl) {
    const dn = button("chevronDown", `One less: ${lbl}`, { act: "dec", id, cls: " t-arrow" + mark(v, `dec:${id}`), disabled: n <= 0 });
    const up = button("chevronUp", `One more: ${lbl}`, { act: "inc", id, cls: " t-arrow" + mark(v, `inc:${id}`), disabled: n >= of });
    const a = attr({
      role: "spinbutton",
      tabindex: 0,
      "aria-label": lbl,
      "aria-valuemin": 0,
      "aria-valuemax": of,
      "aria-valuenow": n,
      "aria-valuetext": `${n} of ${of}${n >= of ? ", done" : ""}`,
      "data-act": "bump",
      "data-id": id,
    });
    return `<span class="t-step">${dn}<span class="t-fig${mark(v, `count:${id}`)}" ${a}>${n}<small>/${of}</small></span>${up}</span>`;
  }

  // A value from the game: the figure where a manual figure sits, no arrows;
  // the extra pool after it. A click (or Enter) types over it.
  function gameFig(v, id, inner, lbl, valuetext, extra) {
    const src = v.over[id] != null ? "typed by you" : "from the game";
    const a = attr({ role: "button", tabindex: 0, "aria-label": `${lbl}, ${valuetext}, ${src}. Type over it`, "data-act": "g-edit", "data-id": id });
    const x = extra ? `+${extra}` : "";
    return `<span class="g-step"><span class="t-gap"></span><span class="t-fig g-fig${mark(v, `count:${id}`)}" ${a}>${inner}</span><span class="g-extra">${x}</span></span>`;
  }

  // A hidden row listed by "Show": its figure without controls.
  const ghost = (inner) => `<span class="t-step t-ghost"><span class="t-gap"></span><span class="t-fig">${inner}</span><span class="t-gap"></span></span>`;

  const typedGut = () => `<span class="t-gut is-typed" title="Typed by you">${icon("pencil")}</span>`;

  function countRow(v, it) {
    const n = countOf(v, it);
    const done = n >= it.of;
    const g = gameOf(v, it.id);
    let right;
    if (v.hidden.has(it.id)) right = ghost(`${Math.min(n, it.of)}<small>/${it.of}</small>`);
    else if (v.ui.editAmt === it.id) right = field(v, it, it.of);
    else if (g) right = gameFig(v, it.id, `${n}<small>/${it.of}</small>`, label(it), `${n} of ${it.of} done${g.x ? `, ${g.x} extra` : ""}`, g.x);
    else right = stepper(v, it.id, Math.min(n, it.of), it.of, label(it));
    const gut = v.over[it.id] != null ? typedGut() : `<span class="t-gut"></span>`;
    return `<div class="${rowCls(v, it, [done && "is-done"])}" data-row="${it.id}">${gut}${nameHtml(v, it)}${right}${xSlot(v, it)}${tip(v, it.id)}</div>`;
  }

  // The whole row is the button. Under the pointer (or keyboard focus) the
  // check at its end appears, undo on a done row. It does what a click on
  // the row does; the row stays the keyboard's target.
  function tickRow(v, it) {
    const done = countOf(v, it) >= 1;
    const off = v.hidden.has(it.id);
    const word = done ? "Undo" : "Mark done";
    const check = off
      ? ""
      : `<button type="button" class="o-btn is-sm t-check${done ? " is-undo" : ""}${mark(v, `check:${it.id}`)}" ${attr({ "data-act": "tick", "data-id": it.id, "aria-label": `${word}: ${label(it)}`, title: word, tabindex: -1 })}>${icon(done ? "undo" : "check")}</button>`;
    const a = off ? "" : attr({ role: "button", tabindex: 0, "aria-label": `${label(it)}, ${done ? "done" : "not done"}`, "data-act": "tick", "data-id": it.id });
    const gut = v.over[it.id] != null ? typedGut() : `<span class="t-gut"></span>`;
    return `<div class="${rowCls(v, it, ["is-tick", done && "is-done"])}" data-row="${it.id}"><span class="t-main" ${a}>${gut}${nameHtml(v, it)}</span>${check}${xSlot(v, it)}${tip(v, it.id)}</div>`;
  }

  function hiddenLine(v, list, key) {
    const h = hiddenIn(list, v).length;
    if (!h) return "";
    const on = !!v.reveal[key];
    const lbl = on ? `Hide again: ${h} hidden` : `Show ${h} hidden`;
    return `<div class="t-hid"><span>${h} hidden</span><span aria-hidden="true">·</span><button type="button" class="t-link${mark(v, `show:${key}`)}" data-act="reveal" data-id="${key}" aria-expanded="${on}" aria-label="${lbl}">${on ? "Hide again" : "Show"}</button></div>`;
  }

  const rowFor = (v, it) => (it.kind === "tick" ? tickRow(v, it) : countRow(v, it));

  // Typing over a game value: the field takes the figure's place; its ×
  // goes back to the game's value.
  function field(v, it, of) {
    const val = v.ui.editVal == null ? "" : String(v.ui.editVal);
    const input = v.live
      ? `<input class="t-input" data-id="${it.id}" inputmode="numeric" autocomplete="off" placeholder="Done" aria-label="${esc(it.name)}: how many done" value="${esc(val)}">`
      : `<span class="val">${esc(val)}</span><span class="o-caret"></span>`;
    return `<span class="t-field">${input}<span class="t-cap">/${of}</span>${button("close", `Back to the game's value: ${it.name}`, { act: "g-back", id: it.id, cls: " t-clear" + mark(v, `clear:${it.id}`) })}</span>`;
  }

  function sec(v, id, title, { n = "", good = false, at = "" } = {}) {
    const open = !v.fold[id];
    const a = attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "data-act": "fold", "data-id": id });
    return `<div class="t-sec${mark(v, `sec:${id}`)}" ${a}>${icon(open ? "chevronDown" : "chevronRight")}<span>${title}</span>${n ? `<span class="n${good ? " is-good" : ""}">${n}</span>` : ""}<span class="at">${at}</span></div>`;
  }

  function todaySec(v) {
    const list = todayList(v);
    const l = left(list, v);
    const head = sec(v, "today", "Today", { n: l ? `${l} left` : "done", good: !l, at: `resets in <b>${span(nextAt(v.now, RESET) - v.now)}</b>` });
    if (v.fold.today) return head;
    return head + `<div class="t-list">${listed(list, v, "today").map((it) => rowFor(v, it)).join("")}${hiddenLine(v, list, "today")}</div>`;
  }

  function weekSec(v) {
    const list = weekList(v);
    const l = left(list, v);
    const w = nextWeekly(v.now);
    const head = sec(v, "week", "This week", { n: l ? `${l} left` : "done", good: !l, at: `${stamp(w)} · in <b>${span(w - v.now)}</b>` });
    if (v.fold.week) return head;
    return head + `<div class="t-list">${listed(list, v, "week").map((it) => rowFor(v, it)).join("")}${hiddenLine(v, list, "week")}</div>`;
  }

  // "+ Add a to-do" at the end of the list; the form opens in its place.
  function addHtml(v) {
    if (!v.adding) return `<button type="button" class="t-addline${mark(v, "addline")}" data-act="add-open">${icon("plus")}<span>Add a to-do</span></button>`;
    const a = v.add;
    const ok = !!a.name.trim();
    const name = v.live
      ? `<span class="o-input"><input class="t-input t-add-name" placeholder="Name" autocomplete="off" aria-label="New to-do" value="${esc(a.name)}"></span>`
      : a.name
        ? `<span class="o-input is-focus"><span class="val">${esc(a.name)}</span><span class="o-caret"></span></span>`
        : `<span class="o-input is-focus"><span class="o-caret"></span><span class="ph">Name</span></span>`;
    const seg = (key, lbl, opts) =>
      `<span class="t-seg" role="radiogroup" aria-label="${lbl}">${opts
        .map(([val, text]) => `<button type="button" role="radio" aria-checked="${a[key] === val}" class="${a[key] === val ? "is-on" : ""}" data-act="add-${key}" data-id="${val}">${text}</button>`)
        .join("")}</span>`;
    const of =
      a.kind === "count"
        ? `<span class="t-of"><span>of</span><span class="t-step">${button("chevronDown", "One less", { act: "add-dec", cls: " t-arrow", disabled: a.of <= 2 })}<span class="t-fig is-fixed" ${attr({ role: "spinbutton", tabindex: 0, "aria-label": "How many", "aria-valuenow": a.of, "aria-valuemin": 2, "aria-valuemax": 99, "data-act": "add-of" })}>${a.of}</span>${button("chevronUp", "One more", { act: "add-inc", cls: " t-arrow", disabled: a.of >= 99 })}</span></span>`
        : "";
    const add = `<button type="button" class="o-tbtn t-small${ok ? "" : " is-disabled"}" data-act="add"${ok ? "" : " disabled"}>${icon("plus")}Add</button>`;
    return `<div class="t-add" role="group" aria-label="Add a to-do"><div class="t-add-top">${name}${add}${button("close", "Cancel", { act: "add-cancel" })}</div><div class="t-add-row">${seg("kind", "Kind", [["tick", "Tick"], ["count", "Count"]])}${of}<span class="grow"></span>${seg("reset", "Resets", [["daily", "Daily"], ["weekly", "Weekly"]])}</div></div>`;
  }

  function head(v) {
    const l = left(todayList(v), v) + left(weekList(v), v);
    return `<div class="o-head"><span class="o-title">${icon("listChecks")}To-dos</span><span class="o-count">${l ? `${l} left` : "all done"}</span><span class="grow"></span>${button("close", "Close", { sm: false })}</div>`;
  }

  function charSelect(v, ctx) {
    const me = v.who;
    const open = v.menu;
    const a = attr({ role: "combobox", tabindex: 0, "aria-expanded": String(open), "aria-label": "Character", "data-act": "menu" });
    const live = v.playing ? '<span class="t-live">Playing</span>' : '<span class="grow" style="flex:1"></span>';
    const box = `<span class="o-input${open ? " is-focus" : ""}" ${a}><img src="${ctx.classIcon(me.cls)}" alt=""><span class="nm">${nm(ctx, me.name)}</span><span class="meta">${me.klass} · ${me.server}</span>${live}${icon(open ? "chevronUp" : "chevronDown")}</span>`;
    if (!open) return `<div class="t-char">${box}</div>`;
    const l = left(todayList(v), v) + left(weekList(v), v);
    const status = { mirelle: l ? `${l} left` : "all done", tavrin: "3 left", sefa: "2 left" };
    const items = CHARS.map(
      (c, i) =>
        `<div class="o-menu-item t-who${i === 1 ? " is-hot" : ""}" role="option" aria-selected="${c.id === me.id}"><img src="${ctx.classIcon(c.cls)}" alt=""><b>${nm(ctx, c.name)}</b><span class="srv">${c.klass} · ${c.server}</span><span class="left">${status[c.id]}</span></div>`
    ).join("");
    return `<div class="t-char">${box}<div class="o-menu" role="listbox">${items}<div class="o-menu-sep"></div><div class="t-menu-note">Characters the meter has seen, with what each has left. Duty Missions are shared by your characters on one server.</div></div></div>`;
  }

  // One line under the selector says where the game's values come from. The
  // to-do window's are weekly entries: nothing refills between logins.
  function sourceLine(v) {
    const s = v.src;
    const dot = '<span class="faint">·</span>';
    let text;
    if (s.kind === "live") text = `From the game, live ${dot} logged in at <b>${clock(v.seen)}</b>`;
    else if (s.kind === "stale") text = `From the game at login, <b>${stamp(v.seen)}</b>`;
    else text = `No game values yet. Log in with <b>${esc(v.who.name)}</b> while ${window.APP_NAME} runs.`;
    return `<div class="g-src">${icon("gamepad")}<span>${text}</span></div>`;
  }

  function noteHtml(v) {
    if (v.removed) return `<div class="o-note t-note is-removed" role="status">${icon("trash")}<span>Removed <b class="t-strong">${esc(v.removed.it.name)}</b>.</span><button type="button" class="t-link" data-act="undo-remove">Undo</button></div>`;
    if (v.note) return `<div class="o-note is-good t-note" role="status">${icon("check")}<span>${v.note}</span></div>`;
    return "";
  }

  function foot() {
    return `<div class="o-foot t-foot">${icon("clock")}<span>Daily reset <b>${pad(RESET / 60)}:00</b></span><span class="faint">·</span><button type="button" class="t-link" data-act="reset-hour" aria-label="Change the daily reset hour in Settings, To-dos">Change</button><span class="grow"></span><span class="faint">Weekly on Wednesday</span></div>`;
  }

  function windowHtml(v, ctx) {
    const body = todaySec(v) + weekSec(v) + addHtml(v);
    const scroll = `<div class="t-scrollwrap"><div class="t-scroll" style="max-height:${v.max}px"${v.scroll ? ` data-scroll="${v.scroll}"` : ""}>${body}</div><span class="o-thumb t-thumb" aria-hidden="true"></span></div>`;
    const still = v.ui.still ? " is-still" : "";
    return `<div class="ov o td${still}" role="dialog" aria-label="To-dos"${v.live ? " data-live" : ""}>${head(v)}${charSelect(v, ctx)}${sourceLine(v)}${noteHtml(v)}${scroll}${foot()}</div>`;
  }

  // ---- Scroll thumbs (GPUI draws its own 3 px thumb) ----------------------------
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

  // ---- The working window ("Try it"); nothing is stored ------------------------
  const LIVE = view({ live: true });
  let lastCtx = null;
  let lastTile = 0;
  let redrawing = false;
  let noteTimer = 0;

  // One polite live region for the page: what a screen reader says after a
  // change ("Legion check-in: done").
  const sr = document.createElement("div");
  sr.className = "t-sr";
  sr.setAttribute("aria-live", "polite");
  document.body.appendChild(sr);

  function say(text) {
    sr.textContent = "";
    requestAnimationFrame(() => (sr.textContent = text));
  }

  const FOCUSABLE = '.t-main[tabindex="0"], [role="spinbutton"][tabindex="0"], [role="button"][tabindex="0"], button:not([tabindex="-1"]):not(:disabled)';

  // `focus`: [act, id] to refocus a control, { row } for the first control
  // of a row.
  function redraw(focus) {
    const tiles = [...document.querySelectorAll("[data-live]")];
    const activeEl = document.activeElement;
    const at = tiles.findIndex((t) => t.contains(activeEl));
    const key = focus || (at >= 0 && activeEl.dataset && activeEl.dataset.act ? [activeEl.dataset.act, activeEl.dataset.id] : null);
    if (at >= 0) lastTile = at;
    const tops = tiles.map((t) => t.querySelector(".t-scroll").scrollTop);
    redrawing = true;
    tiles.forEach((t) => (t.outerHTML = windowHtml(LIVE, lastCtx)));
    redrawing = false;
    const fresh = [...document.querySelectorAll("[data-live]")];
    fresh.forEach((t, i) => (t.querySelector(".t-scroll").scrollTop = tops[i] || 0));
    syncThumbs();
    const home = fresh[lastTile];
    if (!home) return;
    if (LIVE.ui.editAmt) {
      const input = home.querySelector(".t-field .t-input");
      if (input) {
        input.focus();
        input.select();
      }
      return;
    }
    if (LIVE.adding && key && key[0] === "add-open") {
      const input = home.querySelector(".t-add-name");
      if (input) input.focus();
      return;
    }
    if (!key || (at < 0 && !focus)) return;
    let el = null;
    if (Array.isArray(key)) el = home.querySelector(`[data-act="${key[0]}"]` + (key[1] ? `[data-id="${key[1]}"]` : ""));
    else if (key.row) {
      const row = home.querySelector(`[data-row="${key.row}"]`);
      el = row && row.querySelector(FOCUSABLE);
    }
    if (el && !el.disabled) el.focus({ preventScroll: true });
  }

  // Where focus goes when a row leaves the list: the next row, else the one
  // before.
  function neighbour(id) {
    const tile = document.querySelectorAll("[data-live]")[lastTile];
    if (!tile) return null;
    const rows = [...tile.querySelectorAll("[data-row]")].filter((r) => r.querySelector(FOCUSABLE));
    const i = rows.findIndex((r) => r.dataset.row === id);
    const next = rows[i + 1] || rows[i - 1];
    return next ? { row: next.dataset.row } : null;
  }

  function flash(note) {
    clearTimeout(noteTimer);
    LIVE.note = note;
    noteTimer = setTimeout(() => {
      LIVE.note = "";
      LIVE.removed = null;
      redraw();
    }, 4000);
  }

  // The value the field opens with: the figure as shown.
  const shownValue = (v, id) => String(countOf(v, findItem(v, id)));

  // Enter keeps (0 to the target), Esc cancels; from the keyboard, focus
  // returns to the figure. An empty field goes back to the game's value.
  function commit(input, save, refocus) {
    const v = LIVE;
    const id = v.ui.editAmt;
    if (!id) return;
    if (save) {
      const s = input.value.trim();
      const it = findItem(v, id);
      if (s === "") delete v.over[id];
      else {
        const num = parseInt(s.replace(/[^0-9]/g, ""), 10);
        if (!Number.isNaN(num)) v.over[id] = clamp(num, 0, it.of);
      }
    }
    v.ui.editAmt = null;
    redraw(refocus ? ["g-edit", id] : null);
  }

  // A value you change on a game row is typed over it; elsewhere it is yours.
  function setCount(v, id, next) {
    if (v.g[id]) v.over[id] = next;
    else v.n[id] = next;
  }

  function act(name, id, { keyboard = false } = {}) {
    const v = LIVE;
    const it = findItem(v, id);
    let focus;
    switch (name) {
      case "inc":
      case "dec": {
        const d = name === "inc" ? 1 : -1;
        if (it) {
          const was = countOf(v, it) >= it.of;
          const next = clamp(countOf(v, it) + d, 0, it.of);
          setCount(v, id, next);
          if (was !== next >= it.of) say(`${label(it)}: ${next >= it.of ? "done" : "not done"}`);
          // The up arrow goes dim at the limit; keep the keyboard on the row.
          if (next >= it.of && name === "inc") focus = ["bump", id];
        }
        break;
      }
      case "bump":
        if (it) {
          const was = countOf(v, it) >= it.of;
          const next = clamp(countOf(v, it) + 1, 0, it.of);
          setCount(v, id, next);
          if (!was && next >= it.of) say(`${label(it)}: done`);
        }
        break;
      case "tick": {
        const done = countOf(v, it) >= 1;
        setCount(v, id, done ? 0 : 1);
        say(`${label(it)}: ${done ? "not done" : "done"}`);
        break;
      }
      case "fold":
        v.fold[id] = !v.fold[id];
        break;
      case "g-edit":
        v.ui.editAmt = id;
        v.ui.editVal = shownValue(v, id);
        break;
      case "g-back":
        delete v.over[id];
        v.ui.editAmt = null;
        focus = ["g-edit", id];
        say(`${it.name}: back to the game's value`);
        break;
      case "menu":
        v.menu = !v.menu;
        break;
      case "hide":
        if (keyboard) focus = neighbour(id);
        // The row below slides under the pointer: its × waits for the
        // pointer to move, so a double click cannot hide two rows.
        else v.ui.still = true;
        v.hidden.add(id);
        say(`${label(it)} hidden`);
        break;
      case "unhide":
        v.hidden.delete(id);
        say(`${label(it)} put back`);
        focus = { row: id };
        break;
      case "reveal":
        v.reveal[id] = !v.reveal[id];
        focus = ["reveal", id];
        break;
      case "remove": {
        const at = v.custom.findIndex((c) => c.id === id);
        if (at < 0) return;
        if (keyboard) focus = neighbour(id);
        else v.ui.still = true;
        v.removed = { it: v.custom[at], at };
        v.custom.splice(at, 1);
        flash("");
        say(`${v.removed.it.name} removed. Undo is at the top of the window.`);
        break;
      }
      case "undo-remove":
        if (!v.removed) return;
        v.custom.splice(v.removed.at, 0, v.removed.it);
        focus = { row: v.removed.it.id };
        v.removed = null;
        clearTimeout(noteTimer);
        break;
      case "add-open":
        v.adding = true;
        v.add = { name: "", kind: "tick", of: 3, reset: "daily" };
        focus = ["add-open"];
        break;
      case "add-cancel":
        v.adding = false;
        focus = ["add-open"];
        break;
      case "add-kind":
        v.add.kind = id;
        break;
      case "add-reset":
        v.add.reset = id;
        break;
      case "add-inc":
        v.add.of = clamp(v.add.of + 1, 2, 99);
        break;
      case "add-dec":
        v.add.of = clamp(v.add.of - 1, 2, 99);
        break;
      case "add": {
        const name = v.add.name.trim();
        if (!name) return;
        const item = { id: "c" + Date.now(), kind: v.add.kind, name, of: v.add.kind === "count" ? v.add.of : undefined, custom: true, reset: v.add.reset, manual: true };
        v.custom.push(item);
        const where = item.reset === "daily" ? "today" : "week";
        v.fold[where] = false;
        v.adding = false;
        v.removed = null;
        flash(`Added <b class="t-strong">${esc(name)}</b> to ${item.reset === "daily" ? "Today" : "This week"}.`);
        focus = ["add-open"];
        break;
      }
      default:
        return; // close, reset hour: not part of the working window
    }
    redraw(focus);
  }

  // Messages from the game, played by the buttons under the working window.
  // The game's new value always replaces a value typed over it.
  function simulate(kind) {
    const v = LIVE;
    const g = v.g;
    if (kind === "trial") {
      if (g.trial.n > 0) g.trial.n -= 1;
      else if (g.trial.x > 0) g.trial.x -= 1;
      delete v.over.trial;
    }
    if (kind === "gain") {
      g.trial.x += 1;
      delete v.over.trial;
    }
    if (kind === "login") {
      v.g = snapshot();
      const typed = Object.keys(v.over).length;
      v.over = {};
      v.ui.editAmt = null;
      flash(typed ? "Filled from the game at login. Values you typed were replaced." : "Filled from the game at login.");
    }
    redraw();
  }

  document.addEventListener("mousedown", (e) => {
    // Keep the field focused while its × is pressed.
    if (e.target.closest && e.target.closest(".t-clear")) e.preventDefault();
  });

  document.addEventListener("mousemove", () => {
    if (!LIVE.ui.still) return;
    LIVE.ui.still = false;
    document.querySelectorAll("[data-live].is-still").forEach((t) => t.classList.remove("is-still"));
  });

  document.addEventListener("click", (e) => {
    const sim = e.target.closest && e.target.closest("[data-sim]");
    if (sim) return simulate(sim.dataset.sim);
    const tile = e.target.closest && e.target.closest("[data-live]");
    const v = LIVE;
    if (!tile) {
      let dirty = false;
      if (v.menu) (v.menu = false), (dirty = true);
      if (v.adding && !v.add.name.trim()) (v.adding = false), (dirty = true);
      if (dirty) redraw();
      return;
    }
    const tiles = [...document.querySelectorAll("[data-live]")];
    if (tiles.includes(tile)) lastTile = tiles.indexOf(tile);
    let el = e.target.closest("[data-act]");
    // A click on a tick row's edge still counts as a click on the row; one
    // on the × slot while its button is held back does nothing.
    if (!el && !e.target.closest(".t-x")) {
      const row = e.target.closest(".t-row.is-tick:not(.is-off)");
      if (row) el = row.querySelector(".t-main");
    }
    if (v.menu && !(el && el.dataset.act === "menu")) {
      v.menu = false;
      if (!el) return redraw();
    }
    // A click outside the form closes it while the name is empty.
    if (v.adding && !e.target.closest(".t-add") && !v.add.name.trim()) {
      v.adding = false;
      if (!el) return redraw();
    }
    if (!el || el.disabled || el.tagName === "INPUT") return;
    act(el.dataset.act, el.dataset.id);
  });

  document.addEventListener("keydown", (e) => {
    const el = e.target;
    if (!el.closest || !el.closest("[data-live]")) return;
    // preventDefault: Enter must not go on to click the control that takes
    // focus after the redraw.
    if (el.classList.contains("t-input") && el.closest(".t-field")) {
      if (e.key === "Enter" || e.key === "Escape") {
        e.preventDefault();
        commit(el, e.key === "Enter", true);
      }
      return;
    }
    if (el.classList.contains("t-add-name")) {
      if (e.key === "Enter" || e.key === "Escape") {
        e.preventDefault();
        act(e.key === "Enter" ? "add" : "add-cancel");
      }
      return;
    }
    const a = el.dataset && el.dataset.act;
    // Delete on a focused row hides it (removes your own), like its ×.
    if (e.key === "Delete") {
      const row = el.closest(".t-row:not(.is-off)");
      const x = row && row.querySelector('.t-x [data-act="hide"], .t-x [data-act="remove"]');
      if (x) {
        e.preventDefault();
        act(x.dataset.act, x.dataset.id, { keyboard: true });
      }
      return;
    }
    if (!a) return;
    if (el.getAttribute("role") === "spinbutton" && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      if (a === "add-of") act(e.key === "ArrowUp" ? "add-inc" : "add-dec");
      else act(e.key === "ArrowUp" ? "inc" : "dec", el.dataset.id);
      return;
    }
    if (el.tagName !== "BUTTON" && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      act(a, el.dataset.id, { keyboard: true });
    }
  });

  document.addEventListener("input", (e) => {
    if (e.target.classList && e.target.classList.contains("t-add-name")) {
      LIVE.add.name = e.target.value;
      const add = e.target.closest(".t-add").querySelector('[data-act="add"]');
      const ok = !!LIVE.add.name.trim();
      add.disabled = !ok;
      add.classList.toggle("is-disabled", !ok);
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

  // ---- States ------------------------------------------------------------------
  const tipText = (main, sub) => `${main}${sub ? `<br><span class="faint">${sub}</span>` : ""}`;

  // Counts you keep yourself while the week is under way.
  const MID = { duty: 2, legion: 1 };
  const HID = new Set(["duty", "dutycmd"]);

  // Sefa, alone on Lyss, never seen at login: everything by hand.
  const SEFA = { who: CHARS[2], playing: false, src: { kind: "none" }, g: {}, n: { duty: 0 } };

  const STATES = {
    live: () => view(),
    // The word "Account" under the pointer, on Duty Missions.
    hoverAcct: () => view({ ui: { hover: ["acct:duty"], tip: { id: "duty", left: 95, text: "Account" } } }),
    hoverGame: () =>
      view({
        ui: {
          hover: ["row:trial", "count:trial"],
          tip: { id: "trial", right: 24, text: tipText("From the game, live.", "+1 extra entry from an item: not part of the 3.") },
        },
      }),
    progress: () => view({ n: { ...MID }, g: snapshot({ trial: { n: 0 } }) }),
    compare: () => view({ ui: { hover: ["row:duty", "count:duty"], tip: { id: "duty", right: 44, text: tipText("Counted by you.", "The game does not send Duty Missions.") } } }),
    hover: () => view({ n: { ...MID, legion: 0 }, ui: { hover: ["row:legion", "check:legion"], tip: { id: "legion", text: "Mark done" } } }),
    hoverDone: () => view({ fold: { today: false, week: true }, ui: { hover: ["row:legion", "check:legion"], tip: { id: "legion", text: "Undo" } } }),
    reach: () => view({ n: { ...MID, duty: 5 }, ui: { hover: "row:duty" } }),
    press: () => view({ n: { ...MID, duty: 3 }, ui: { hover: "row:duty", press: "inc:duty" } }),
    stale: () =>
      view({
        playing: false,
        seen: SUN,
        src: { kind: "stale" },
        ui: {
          hover: ["row:trial", "count:trial"],
          tip: { id: "trial", right: 24, text: tipText("2/3 at your login Sun 22:40.", "A run since shows at your next login.") },
        },
      }),
    typeOver: () => view({ ui: { editAmt: "trial", editVal: "3", hover: ["clear:trial"], tip: { id: "trial", right: 21, text: tipText("Back to the game's value, 2") } } }),
    typed: () =>
      view({
        over: { trial: 3 },
        ui: { hover: ["row:trial", "count:trial"], tip: { id: "trial", right: 44, text: tipText("Typed by you. The game said 2.", "Its next value replaces yours: a run, an entry from an item or your next login.") } },
      }),
    login: () => view({ note: "Filled from the game at login. Values you typed were replaced." }),
    first: () => view({ ...SEFA }),
    hidden: () => view({ n: { ...MID, legion: 0 }, hidden: new Set(HID), ui: { hover: "show:today" } }),
    reveal: () => view({ n: { ...MID, legion: 0 }, hidden: new Set(HID), reveal: { today: true }, fold: { today: false, week: true }, ui: { hover: ["row:duty", "x:duty"] } }),
    adding: () =>
      view({
        n: { ...MID },
        fold: { today: true, week: false },
        adding: true,
        add: { name: "Arena of Tactics wins", kind: "count", of: 3, reset: "weekly" },
        scroll: "end",
      }),
    removed: () => view({ n: { ...MID }, custom: [], removed: { it: { ...LEGION }, at: 0 } }),
    done: () => view({ n: { duty: 5, legion: 1 }, g: weekly(snapshot(), () => 0), fold: { today: true, week: true }, note: '<b class="t-strong">Everything is done on Mirelle.</b>' }),
    fresh: () =>
      view({
        now: WED,
        seen: WEEKLY + 5,
        n: { duty: 0, legion: 0 },
        g: weekly(snapshot(), (it) => it.of || 1),
      }),
    focus: () => view({ n: { ...MID, legion: 0 }, ui: { focus: "row:legion" } }),
    focusFig: () => view({ n: { ...MID }, fold: { today: true, week: false }, ui: { focus: "count:trial" } }),
    switch: () => view({ n: { ...MID }, menu: true }),
  };

  function simStrip() {
    return `<div class="mk-sim" role="group" aria-label="Messages from the game, played by hand"><b>Page only: messages from the game</b><button type="button" data-sim="trial">Run the Ascension Trial</button><button type="button" data-sim="gain">+1 Ascension Trial entry from an item</button><button type="button" data-sim="login">Log in again</button></div>`;
  }

  // Opening it: the toolbar's checklist (hub.js), lit while the window is
  // open. Nothing full, so the toolbar shows no warning.
  function opening(ctx) {
    const bar = Hub.toolbar(ctx, { lit: { todos: true }, hover: "todos" });
    return `<div class="hb-col">${bar}${windowHtml(view(), ctx)}</div>`;
  }

  function render(state, ctx) {
    lastCtx = ctx;
    if (state === "try") return `<div style="position:relative;z-index:1">${windowHtml(LIVE, ctx)}${simStrip()}</div>`;
    if (state === "opening") return opening(ctx);
    const make = STATES[state];
    return make ? windowHtml(make(), ctx) : "";
  }

  const main = document.querySelector("main.scenes");
  if (main) new MutationObserver(() => requestAnimationFrame(syncThumbs)).observe(main, { childList: true });
  if (document.fonts) document.fonts.ready.then(syncThumbs);

  Kit.mount({
    states: [
      { id: "live", title: "Filled from the game, live", sub: "Today and This week, no Recharges (they are in the Resources window only). Ascension Trial from the game: 1 entry left of 3 reads 2/3, the extra entry beside it (+1), no arrows; the line under the selector says where it comes from. Duty Missions and your own to-dos are yours to count; Duty Missions carry the Account icon; the title counts what is left" },
      { id: "hoverAcct", title: "Pointer on the Account icon", sub: "Duty Missions, shared by your characters on the server: \"Account\" is the icon alone; under the pointer it lights and the tooltip says \"Account\"" },
      { id: "hoverGame", title: "Pointer on a game value", sub: "the figure reads as a field (a click types over it); its tooltip says where it comes from and what \"+1\" is" },
      { id: "progress", title: "This week from the game, done", sub: "Ascension Trial at 0 entries left: 3/3, struck through and muted where it stands; the extra entry (+1) does not stop it from being done" },
      { id: "compare", title: "A game value next to a manual one", sub: "Duty Missions keep the arrows: the game does not send them, so you count them; pointer on its figure. Ascension Trial below has no arrows" },
      { id: "hover", title: "Pointer on a to-do", sub: "the row lights, the check and the × appear at its end; the pointer on the check: \"Mark done\"" },
      { id: "hoverDone", title: "Pointer on a done to-do", sub: "Legion check-in, done: the undo arrow in place of the check; the pointer on it: \"Undo\"" },
      { id: "reach", title: "A count at its limit", sub: "Duty Missions 5/5: done like a tick; the up arrow dims, the down arrow takes one back; no check on counts" },
      { id: "press", title: "Pressing one-more", sub: "Duty Missions going from 2 to 3; a click on the figure does the same" },
      { id: "stale", title: "From yesterday's login", sub: "Mirelle is not in game: Ascension Trial is the game's value at Sunday 22:40, said once on the line under the selector; nothing refills in this window, and a weekly row seen before Wednesday's reset starts over at the reset; pointer on its figure" },
      { id: "typeOver", title: "Typing over a game value", sub: "a click on the figure opens the field (0 to 3); its × (or an empty field) goes back to the game's value" },
      { id: "typed", title: "After typing over it", sub: "your number takes the figure's place with a pencil in the gutter; 3/3 is done; the extra entry stays the game's (+1). The game's next value (a run, an entry from an item, your next login) replaces yours" },
      { id: "login", title: "Just logged in", sub: "the snapshot at login fills Ascension Trial and replaces a value you typed there; a note for 4 s" },
      { id: "first", title: "Before the first login is seen", sub: "Sefa, alone on Lyss: nothing from the game yet; the line says how to get values; Ascension Trial keeps the arrows until the game fills it" },
      { id: "hidden", title: "Hidden rows", sub: "Duty Missions and Duty Commands hidden: \"1 hidden · Show\" under Today and under This week; hidden rows never count; the pointer on Show" },
      { id: "reveal", title: "After Show", sub: "the hidden row listed in place, faded, with a put-back button; \"Hide again\" folds it away" },
      { id: "adding", title: "Adding a to-do", sub: "\"+ Add a to-do\" opened in place: a name, Count of 3, Weekly; Enter adds, Esc cancels" },
      { id: "removed", title: "Removing your own", sub: "the × on a to-do of your own removes it, with Undo for 4 s" },
      { id: "done", title: "Everything done", sub: "both sections folded, \"done\" in good; the title reads \"all done\"" },
      { id: "fresh", title: "Fresh week", sub: "Wednesday 09:12, just after the reset: the game's entries are back to their targets, so nothing is done; Duty Commands start hidden" },
      { id: "focus", title: "Keyboard focus on a row", sub: "read as \"Legion check-in, not done, button\": the ring 1 px outside the row; the check and × show; Space or Enter marks it done, Delete removes it" },
      { id: "focusFig", title: "Keyboard focus on a game figure", sub: "read as \"Ascension Trial, 2 of 3 done, 1 extra, from the game. Type over it\"; Enter types over it" },
      { id: "switch", title: "Character selector", sub: "the characters the meter remembers, with what each has left (to-dos only: full resources are the Resources window's)" },
      { id: "opening", title: "Opening it", sub: "the toolbar's checklist opens and closes the window; it is lit while the window is open" },
      { id: "try", title: "Try it", sub: "a working window: rows, the check, arrows, ×, Show, add and remove; click Ascension Trial's figure to type over it, × or an empty field goes back to the game's value; the buttons under it play messages from the game. Nothing is saved" },
    ],
    controls: ["alpha", "scale", "mask"],
    tileMin: 430,
    stageMin: 600,
    render,
  });
})();
