/* Proposal E: D's model (one list per character, every figure set by the
 * player) without checkboxes and without edit mode.
 *
 *   done       the name is struck through and muted where it stands; rows
 *              never move
 *   tick       a click anywhere on the row marks it done or not done; under
 *              the pointer a 20 px check button (undo on a done row) appears
 *              at its end, "Mark done" / "Undo"
 *   count      X / N with one-less / one-more arrows, as in D; at N it gets
 *              the same done look
 *   amount     optional, unchanged from D: empty ("—") and ignored until you
 *              type what you have
 *   hide       a small × at the row's end, only while the pointer is on that
 *              row; each section ends with "N hidden · Show"
 *   add        "+ Add a to-do" at the end of the list opens a form in place
 *
 * Limits and refill rules: MetaBot's client tables (Global client
 * 0.0.4387.0) and Game8 for Duty Missions and Duty Commands. Counts, times
 * and character names are invented. The clock is mocked (Monday 21:48, or
 * Wednesday 09:12 for the fresh week). Loaded after ../approved/kit.js and
 * todos.js. Icons: Lucide (https://lucide.dev), ISC License; see
 * ../approved/kit.js. */

(function () {
  "use strict";

  const T = Todo;
  const { esc } = T;

  // Lucide undo-2 (ISC); the rest come from todos.js and kit.js.
  const LOCAL = { undo: '<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11"/>' };

  function icon(name, cls) {
    if (!LOCAL[name]) return T.icon(name, cls);
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${LOCAL[name]}</svg>`;
  }

  // ---- Clock (as in D) ----------------------------------------------------
  const DAY = 1440;
  const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const RESET = 9 * 60; // the "Daily reset" setting: 09:00 reported, 17:00 also seen
  const WEEKLY = 2 * DAY + RESET; // Wednesday at the reset hour
  const KEYS = 16 * 60; // Shugo Festival and Invasion keys
  const MON = 21 * 60 + 48; // Mon 21:48
  const WED = WEEKLY + 12; // Wed 09:12, just after the weekly reset

  const pad = (n) => String(n).padStart(2, "0");
  const stamp = (t) => `${DAYS[Math.floor(t / DAY) % 7]} ${pad(Math.floor((t % DAY) / 60))}:${pad(t % 60)}`;

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

  const refill = (r, now, k) => (r.every ? (Math.floor(now / r.every) + k) * r.every : nextAt(now, r.at) + (k - 1) * DAY);

  function estimate(r, v, now) {
    if (v >= r.cap) return { full: true, next: refill(r, now, 1) };
    return { full: false, at: refill(r, now, Math.ceil((r.cap - v) / r.add)) };
  }

  // ---- The default list (as in D) -----------------------------------------
  const TODAY = [{ id: "duty", kind: "count", name: "Duty Missions", of: 5, account: true }];

  const CHARGES = [
    { id: "odyle", name: "Odyle Energy", cap: 560, add: 15, every: 180, step: 10, rule: "+15 every 3 h, up to 560" },
    { id: "shugo", name: "Shugo Festival keys", cap: 28, add: 4, at: KEYS, rule: "+4 at 16:00, up to 28" },
    { id: "invasion", name: "Invasion keys", cap: 7, add: 1, at: KEYS, rule: "+1 at 16:00, up to 7" },
    { id: "nightmare", name: "Nightmare", cap: 14, add: 2, at: RESET, rule: "+2 at the daily reset, up to 14" },
    { id: "conquest", name: "Conquest dungeons", cap: 14, add: 2, at: RESET, rule: "+2 a day for each dungeon, up to 14" },
    { id: "deus", name: "Deus Research Base", cap: 14, add: 2, at: RESET, rule: "+2 at the daily reset, up to 14" },
    { id: "arkanis", name: "Shattered Arkanis", cap: 14, add: 2, at: RESET, rule: "+2 at the daily reset, up to 14" },
    { id: "balaur", name: "Abandoned Balaur Fortress", cap: 7, add: 1, at: RESET, rule: "+1 at the daily reset, up to 7" },
    { id: "fafnium", name: "Fafnium Laboratory", cap: 7, add: 1, at: RESET, rule: "+1 at the daily reset, up to 7" },
  ];

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
    { id: "fissure", kind: "count", name: "Daily Dungeon", sub: "Unknown Fissure", of: 14 },
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
    { id: "dutycmd", kind: "count", name: "Duty Commands", of: 20 },
  ];

  // Hidden until the player shows them (Game8's figure, not the client's).
  const HIDDEN = ["dutycmd"];

  const LEGION = { id: "legion", kind: "tick", name: "Legion check-in", custom: true, reset: "daily" };

  // ---- Lookups -----------------------------------------------------------
  const flat = (list) => list.flatMap((it) => (it.kind === "group" ? [it, ...it.items] : [it]));
  const findItem = (v, id) => flat(TODAY.concat(WEEK, v.custom)).find((it) => it.id === id);
  const todayList = (v) => TODAY.concat(v.custom.filter((c) => c.reset === "daily"));
  const weekList = (v) => WEEK.concat(v.custom.filter((c) => c.reset === "weekly"));
  // Hidden rows never count; "Show" lists them in place, faded.
  const active = (list, v) => list.filter((it) => !v.hidden.has(it.id));
  const hiddenIn = (list, v) => list.filter((it) => v.hidden.has(it.id));
  const listed = (list, v, key) => (v.reveal[key] ? list : active(list, v));
  const isDone = (it, v) => (it.kind === "group" ? active(it.items, v).every((s) => isDone(s, v)) : (v.n[it.id] || 0) >= (it.of || 1));
  const left = (list, v) => active(list, v).filter((it) => !isDone(it, v)).length;
  const label = (it) => (it.sub ? `${it.name}, ${it.sub}` : it.name);
  const clamp = (x, lo, hi) => Math.max(lo, Math.min(hi, x));

  function view(o = {}) {
    return {
      now: MON,
      n: {},
      amt: {},
      custom: [],
      hidden: new Set(HIDDEN),
      reveal: {},
      fold: { today: false, charges: false, week: false },
      open: { sanct: false, explore: false },
      ui: {},
      live: false,
      menu: false,
      max: 430,
      scroll: "",
      note: "",
      removed: null,
      adding: false,
      add: { name: "", kind: "tick", of: 3, reset: "daily" },
      alt: "", // "over": a line over the name instead of through it
      ...o,
    };
  }

  // ---- Pieces ------------------------------------------------------------
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
    return `<button type="button" class="o-btn${sm ? " is-sm" : ""}${cls}" ${attr({ "data-act": act, "data-id": id, "aria-label": lbl, title: lbl, disabled, tabindex: tab })}>${icon(name)}</button>`;
  }

  function nameHtml(it) {
    const acct = it.account ? `<span class="t-acct">${icon("users")}Account</span>` : "";
    return `<span class="t-name">${esc(it.name)}${it.sub ? `<small>${esc(it.sub)}</small>` : ""}${acct}${it.custom ? '<small class="e-tag">Yours</small>' : ""}</span>`;
  }

  // A tooltip: `title` and `text` for an explanation, `text` alone for a
  // control's name ("Mark done").
  function tip(v, id) {
    const t = v.ui.tip;
    if (!t || t.id !== id) return "";
    if (!t.title) return `<div class="t-tip e-tip" role="tooltip" style="${t.at || "top:24px;right:20px"}">${t.text}</div>`;
    return `<div class="t-tip d-tip" role="tooltip" style="${t.at || "top:27px;right:0"}"><b>${t.title}</b>${t.text}</div>`;
  }

  function rowCls(v, it, extra) {
    const c = ["t-item", "d-row", "e-row", ...extra.filter(Boolean)];
    if (v.hidden.has(it.id)) c.push("is-off");
    if (has(v.ui.hover, `row:${it.id}`)) c.push("is-hover");
    if (has(v.ui.focus, `row:${it.id}`)) c.push("is-focus");
    return c.join(" ");
  }

  // The row's last slot, 20 px, kept on every row so nothing moves when the
  // × appears. A hidden row listed by "Show" puts it back instead. Your own
  // to-dos are removed (with Undo), not hidden.
  function xSlot(v, it) {
    if (v.hidden.has(it.id)) return `<span class="t-edit e-x is-on">${button("eye", `Put back ${label(it)}`, { act: "unhide", id: it.id, cls: mark(v, `x:${it.id}`) })}</span>`;
    const lbl = it.custom ? `Remove ${label(it)}` : `Hide ${label(it)}`;
    return `<span class="t-edit e-x">${button("close", lbl, { act: it.custom ? "remove" : "hide", id: it.id, cls: mark(v, `x:${it.id}`), tab: -1 })}</span>`;
  }

  // One less, the figure, one more (D's stepper). At the limit the figure
  // reads "5 of 5, done".
  function stepper(v, id, n, of, lbl, { amt = false, step = 1 } = {}) {
    const less = step > 1 ? `${step} less` : "One less";
    const more = step > 1 ? `${step} more` : "One more";
    const dn = button("chevronDown", `${less}: ${lbl}`, { act: "dec", id, cls: " d-arrow" + mark(v, `dec:${id}`), disabled: n <= 0 });
    const up = button("chevronUp", `${more}: ${lbl}`, { act: "inc", id, cls: " d-arrow" + mark(v, `inc:${id}`), disabled: n >= of });
    const a = attr({
      role: "spinbutton",
      tabindex: 0,
      "aria-label": lbl,
      "aria-valuemin": 0,
      "aria-valuemax": of,
      "aria-valuenow": n,
      "aria-valuetext": `${n} of ${of}${!amt && n >= of ? ", done" : ""}`,
      "data-act": amt ? "amt-edit" : "bump",
      "data-id": id,
      title: amt ? "Click to type the amount" : "Click to add one",
    });
    return `<span class="d-step">${dn}<span class="d-count${amt ? " is-amt" : ""}${mark(v, `count:${id}`)}" ${a}>${n}<small>/${of}</small></span>${up}</span>`;
  }

  // A hidden row listed by "Show": its figure without controls.
  const ghost = (inner) => `<span class="d-step e-ghost"><span class="d-gap"></span><span class="d-count">${inner}</span><span class="d-gap"></span></span>`;

  function countRow(v, it, isSub) {
    const n = Math.min(v.n[it.id] || 0, it.of);
    const done = n >= it.of;
    const right = v.hidden.has(it.id) ? ghost(`${n}<small>/${it.of}</small>`) : stepper(v, it.id, n, it.of, label(it));
    return `<div class="${rowCls(v, it, [isSub && "is-sub", done && "is-done"])}" data-row="${it.id}"><span class="e-gut"></span>${nameHtml(it)}${right}${xSlot(v, it)}${tip(v, it.id)}</div>`;
  }

  // The whole row is the button. Under the pointer (or keyboard focus) the
  // check button at its end appears, undo on a done row. It does what a click
  // on the row does; the row stays the keyboard's target (the check, like
  // the ×, is not a tab stop).
  function tickRow(v, it, isSub) {
    const done = (v.n[it.id] || 0) >= 1;
    const off = v.hidden.has(it.id);
    const word = done ? "Undo" : "Mark done";
    const cue = off
      ? ""
      : `<button type="button" class="o-btn is-sm e-cue${done ? " is-undo" : ""}${mark(v, `cue:${it.id}`)}" ${attr({ "data-act": "tick", "data-id": it.id, "aria-label": `${word}: ${label(it)}`, title: word, tabindex: -1 })}>${icon(done ? "undo" : "check")}</button>`;
    const a = off ? "" : attr({ role: "button", tabindex: 0, "aria-label": `${label(it)}, ${done ? "done" : "not done"}`, "data-act": "tick", "data-id": it.id });
    return `<div class="${rowCls(v, it, ["is-tick", isSub && "is-sub", done && "is-done"])}" data-row="${it.id}"><span class="e-main${mark(v, `main:${it.id}`)}" ${a}><span class="e-gut"></span>${nameHtml(it)}</span>${cue}${xSlot(v, it)}${tip(v, it.id)}</div>`;
  }

  function hiddenLine(v, list, key, sub = false) {
    const h = hiddenIn(list, v).length;
    if (!h) return "";
    const on = !!v.reveal[key];
    const lbl = on ? `Hide again: ${h} hidden` : `Show ${h} hidden`;
    return `<div class="e-hid${sub ? " is-sub" : ""}"><span>${h} hidden</span><span aria-hidden="true">·</span><button type="button" class="d-link${mark(v, `show:${key}`)}" data-act="reveal" data-id="${key}" aria-expanded="${on}" aria-label="${lbl}">${on ? "Hide again" : "Show"}</button></div>`;
  }

  function groupRow(v, it) {
    const act = active(it.items, v);
    const done = act.filter((s) => isDone(s, v)).length;
    const open = !!v.open[it.id];
    const all = act.length > 0 && done === act.length;
    const off = v.hidden.has(it.id);
    const a = off ? "" : attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "aria-label": `${it.name}, ${done} of ${act.length} done`, "data-act": "group", "data-id": it.id });
    const head = `<div class="${rowCls(v, it, ["is-group", all && "is-done"])}" data-row="${it.id}"><span class="e-main${mark(v, `main:${it.id}`)}" ${a}><span class="e-gut">${icon(open && !off ? "chevronDown" : "chevronRight")}</span>${nameHtml(it)}<span class="d-sum${all ? " is-good" : ""}">${done} of ${act.length} done</span></span>${xSlot(v, it)}</div>`;
    if (!open || off) return head;
    return head + listed(it.items, v, it.id).map((s) => rowFor(v, s, true)).join("") + hiddenLine(v, it.items, it.id, true);
  }

  function rowFor(v, it, isSub = false) {
    if (it.kind === "group") return groupRow(v, it);
    if (it.kind === "tick") return tickRow(v, it, isSub);
    return countRow(v, it, isSub);
  }

  function field(v, r) {
    const val = v.ui.editVal == null ? "" : String(v.ui.editVal);
    const input = v.live
      ? `<input class="d-input" data-id="${r.id}" inputmode="numeric" autocomplete="off" placeholder="Amount" aria-label="${esc(r.name)}: how many you have now" value="${esc(val)}">`
      : `<span class="val">${esc(val)}</span><span class="o-caret"></span>`;
    return `<span class="d-field">${input}<span class="d-cap">/${r.cap}</span>${button("close", `Stop tracking ${r.name}`, { act: "amt-clear", id: r.id, cls: " d-clear" })}</span>`;
  }

  // Amounts, unchanged from D except that the hourglass goes: the section
  // says what they are, and the gutter keeps the warning for a full one.
  function amountRow(v, r) {
    const a = v.amt[r.id];
    const extra = [];
    let gut = `<span class="e-gut"></span>`;
    let right;
    if (v.hidden.has(r.id)) {
      if (a == null) extra.push("is-empty");
      right = ghost(a == null ? "&mdash;" : `${a}<small>/${r.cap}</small>`);
    } else if (v.ui.editAmt === r.id) {
      right = field(v, r);
    } else if (a == null) {
      extra.push("is-empty");
      const st = mark(v, `count:${r.id}`);
      const at = attr({ role: "spinbutton", tabindex: 0, "aria-label": r.name, "aria-valuetext": "not tracked", "data-act": "amt-edit", "data-id": r.id, title: "Type how many you have to track it" });
      right = `<span class="d-step"><span class="d-gap"></span><span class="d-count is-amt is-blank${st}" ${at}>&mdash;</span><span class="d-gap"></span></span>`;
    } else {
      const e = estimate(r, a, v.now);
      if (e.full) {
        extra.push("is-waste");
        gut = `<span class="e-gut is-warn">${icon("alert")}</span>`;
      }
      const when = e.full ? (r.every ? "Full, being wasted" : `Full, spend by ${stamp(e.next)}`) : `Full in ${span(e.at - v.now)}`;
      right = `<span class="d-when${e.full ? " is-warn" : ""}">${when}</span>${stepper(v, r.id, a, r.cap, r.name, { amt: true, step: r.step || 1 })}`;
    }
    return `<div class="${rowCls(v, r, extra)}" data-row="${r.id}">${gut}${nameHtml(r)}${right}${xSlot(v, r)}${tip(v, r.id)}</div>`;
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
    const list = CHARGES;
    const tracked = list.filter((r) => v.amt[r.id] != null && !v.hidden.has(r.id));
    const full = tracked.filter((r) => v.amt[r.id] >= r.cap).length;
    const filling = tracked.filter((r) => v.amt[r.id] < r.cap).map((r) => estimate(r, v.amt[r.id], v.now).at);
    const at = filling.length ? `next full in <b>${span(Math.min(...filling) - v.now)}</b>` : tracked.length ? "" : "amounts optional";
    const head = sec(v, "charges", "Recharges", { n: tracked.length ? `${tracked.length} tracked` : "", chip: full ? T.warnChip(`${full} full`) : "", at });
    if (v.fold.charges) return head;
    return head + `<div class="t-list">${listed(list, v, "charges").map((r) => amountRow(v, r)).join("")}${hiddenLine(v, list, "charges")}</div>`;
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
    if (!v.adding) return `<button type="button" class="e-addline${mark(v, "addline")}" data-act="add-open">${icon("plus")}<span>Add a to-do</span></button>`;
    const a = v.add;
    const ok = !!a.name.trim();
    const name = v.live
      ? `<span class="o-input"><input class="d-input e-add-name" placeholder="Name" autocomplete="off" aria-label="New to-do" value="${esc(a.name)}"></span>`
      : a.name
        ? `<span class="o-input is-focus"><span class="val">${esc(a.name)}</span><span class="o-caret"></span></span>`
        : `<span class="o-input is-focus"><span class="o-caret"></span><span class="ph">Name</span></span>`;
    const seg = (key, lbl, opts) =>
      `<span class="t-seg" role="radiogroup" aria-label="${lbl}">${opts
        .map(([val, text]) => `<button type="button" role="radio" aria-checked="${a[key] === val}" class="${a[key] === val ? "is-on" : ""}" data-act="add-${key}" data-id="${val}">${text}</button>`)
        .join("")}</span>`;
    const of =
      a.kind === "count"
        ? `<span class="d-of"><span>of</span><span class="d-step">${button("chevronDown", "One less", { act: "add-dec", cls: " d-arrow", disabled: a.of <= 2 })}<span class="d-count is-fixed" ${attr({ role: "spinbutton", tabindex: 0, "aria-label": "How many", "aria-valuenow": a.of, "aria-valuemin": 2, "data-act": "add-of" })}>${a.of}</span>${button("chevronUp", "One more", { act: "add-inc", cls: " d-arrow" })}</span></span>`
        : "";
    const add = `<button type="button" class="o-tbtn t-small${ok ? "" : " is-disabled"}" data-act="add"${ok ? "" : " disabled"}>${icon("plus")}Add</button>`;
    return `<div class="e-add" role="group" aria-label="Add a to-do"><div class="e-add-top">${name}${add}${button("close", "Cancel", { act: "add-cancel" })}</div><div class="d-add-row">${seg("kind", "Kind", [["tick", "Tick"], ["count", "Count"]])}${of}<span class="grow"></span>${seg("reset", "Resets", [["daily", "Daily"], ["weekly", "Weekly"]])}</div></div>`;
  }

  function head(v) {
    const l = left(todayList(v), v) + left(weekList(v), v);
    const full = CHARGES.filter((r) => v.amt[r.id] != null && v.amt[r.id] >= r.cap && !v.hidden.has(r.id)).length;
    const chip = full ? T.warnChip(`${full} full`) : "";
    return `<div class="o-head"><span class="o-title">${icon("listChecks")}To-dos</span><span class="o-count">${l ? `${l} left` : "all done"}</span>${chip}<span class="grow"></span>${button("close", "Close", { sm: false })}</div>`;
  }

  function charSelect(v, ctx) {
    const me = T.CHARS[0];
    const open = v.menu;
    const a = attr({ role: "combobox", tabindex: 0, "aria-expanded": String(open), "aria-label": "Character", "data-act": "menu" });
    const box = `<span class="o-input${open ? " is-focus" : ""}" ${a}><img src="${ctx.classIcon(me.cls)}" alt=""><span class="nm">${T.nm(ctx, me.name)}</span><span class="meta">${me.klass} · ${me.server}</span><span class="t-live">Playing</span>${icon(open ? "chevronUp" : "chevronDown")}</span>`;
    if (!open) return `<div class="t-char">${box}</div>`;
    const l = left(todayList(v), v) + left(weekList(v), v);
    const status = { mirelle: [l ? `${l} left` : "all done", ""], tavrin: ["1 full", "is-warn"], sefa: ["9 left", ""] };
    const items = T.CHARS.map(
      (c, i) =>
        `<div class="o-menu-item t-who${i === 1 ? " is-hot" : ""}" role="option" aria-selected="${i === 0}"><img src="${ctx.classIcon(c.cls)}" alt=""><b>${T.nm(ctx, c.name)}</b><span class="srv">${c.klass} · ${c.server}</span><span class="left ${status[c.id][1]}">${status[c.id][0]}</span></div>`
    ).join("");
    return `<div class="t-char">${box}<div class="o-menu" role="listbox">${items}<div class="o-menu-sep"></div><div class="t-menu-note">Characters the meter has seen, with what each has left. Duty Missions are shared by your characters on one server.</div></div></div>`;
  }

  function noteHtml(v) {
    if (v.removed) return `<div class="o-note t-note e-note" role="status">${icon("trash")}<span>Removed <b class="t-strong">${esc(v.removed.it.name)}</b>.</span><button type="button" class="d-link" data-act="undo-remove">Undo</button></div>`;
    if (v.note) return `<div class="o-note is-good t-note" role="status">${icon("check")}<span>${v.note}</span></div>`;
    return "";
  }

  function foot() {
    return `<div class="o-foot d-foot">${icon("clock")}<span>Daily reset <b>${pad(RESET / 60)}:00</b></span><span class="faint">·</span><button type="button" class="d-link" data-act="reset-hour" aria-label="Change the daily reset hour in Settings, To-dos">Change</button><span class="grow"></span><span class="faint">Weekly on Wednesday</span></div>`;
  }

  function windowHtml(v, ctx) {
    const body = todaySec(v) + chargeSec(v) + weekSec(v) + addHtml(v);
    const scroll = `<div class="d-scrollwrap"><div class="d-scroll" style="max-height:${v.max}px"${v.scroll ? ` data-scroll="${v.scroll}"` : ""}>${body}</div><span class="o-thumb d-thumb" aria-hidden="true"></span></div>`;
    const alt = v.alt ? ` is-alt-${v.alt}` : "";
    const still = v.ui.still ? " is-still" : "";
    return `<div class="ov o td is-d is-e${alt}${still}" role="dialog" aria-label="To-dos"${v.live ? " data-live" : ""}>${head(v)}${charSelect(v, ctx)}${noteHtml(v)}${scroll}${foot()}</div>`;
  }

  // ---- Scroll thumbs (GPUI draws its own 3 px thumb) ----------------------
  function thumb(s) {
    const wrap = s.parentElement;
    const t = wrap.querySelector(".d-thumb");
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
    document.querySelectorAll(".d-scroll").forEach((s) => {
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

  document.addEventListener("scroll", (e) => e.target.classList && e.target.classList.contains("d-scroll") && thumb(e.target), true);

  // ---- The live window ----------------------------------------------------
  // Shared by the "Try it" tiles of every scene; nothing is stored.
  const MID = { duty: 2, legion: 1, fissure: 9, trial: 3, subj: 1, forge: 2, forgeboss: 1, transc: 12, conqk: 35, cradle: 4, ordeal: 2, "x-fire": 7, "x-krao": 7, "x-draupnir": 3 };
  const LIVE = view({ live: true, n: { ...MID }, custom: [{ ...LEGION }] });
  let lastCtx = null;
  let lastTile = 0;
  let redrawing = false;
  let noteTimer = 0;

  // One polite live region for the page: what a screen reader says after a
  // change ("Legion check-in: done").
  const sr = document.createElement("div");
  sr.className = "e-sr";
  sr.setAttribute("aria-live", "polite");
  document.body.appendChild(sr);

  function say(text) {
    sr.textContent = "";
    requestAnimationFrame(() => (sr.textContent = text));
  }

  const FOCUSABLE = '.e-main[tabindex="0"], [role="spinbutton"][tabindex="0"], button:not([tabindex="-1"]):not(:disabled)';

  // `focus`: [act, id] to refocus a control, { row } for the first control
  // of a row, { sel } for a selector inside the tile.
  function redraw(focus) {
    const tiles = [...document.querySelectorAll("[data-live]")];
    const active = document.activeElement;
    const at = tiles.findIndex((t) => t.contains(active));
    const key = focus || (at >= 0 && active.dataset && active.dataset.act ? [active.dataset.act, active.dataset.id] : null);
    if (at >= 0) lastTile = at;
    const tops = tiles.map((t) => t.querySelector(".d-scroll").scrollTop);
    redrawing = true;
    tiles.forEach((t) => (t.outerHTML = windowHtml(LIVE, lastCtx)));
    redrawing = false;
    const fresh = [...document.querySelectorAll("[data-live]")];
    fresh.forEach((t, i) => (t.querySelector(".d-scroll").scrollTop = tops[i] || 0));
    syncThumbs();
    const home = fresh[lastTile];
    if (!home) return;
    if (LIVE.ui.editAmt) {
      const input = home.querySelector(".d-field .d-input");
      if (input) {
        input.focus();
        input.select();
      }
      return;
    }
    if (LIVE.adding && key && key[0] === "add-open") {
      const input = home.querySelector(".e-add-name");
      if (input) input.focus();
      return;
    }
    if (!key || (at < 0 && !focus)) return;
    let el = null;
    if (Array.isArray(key)) el = home.querySelector(`[data-act="${key[0]}"]` + (key[1] ? `[data-id="${key[1]}"]` : ""));
    else if (key.row) {
      const row = home.querySelector(`[data-row="${key.row}"]`);
      el = row && row.querySelector(FOCUSABLE);
    } else if (key.sel) el = home.querySelector(key.sel);
    if (el && !el.disabled) el.focus({ preventScroll: true });
  }

  // Where focus goes when a row leaves the list: the next row, else the
  // one before.
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

  // Enter keeps, Esc cancels; from the keyboard, focus returns to the figure.
  function commit(input, save, refocus) {
    const v = LIVE;
    const id = v.ui.editAmt;
    if (!id) return;
    if (save) {
      const r = CHARGES.find((x) => x.id === id);
      const s = input.value.trim();
      if (s === "") delete v.amt[id];
      else {
        const num = parseInt(s.replace(/[^0-9]/g, ""), 10);
        if (!Number.isNaN(num)) v.amt[id] = clamp(num, 0, r.cap);
      }
    }
    v.ui.editAmt = null;
    redraw(refocus ? ["amt-edit", id] : null);
  }

  function act(name, id, { keyboard = false } = {}) {
    const v = LIVE;
    const r = CHARGES.find((x) => x.id === id);
    const it = findItem(v, id);
    let focus;
    switch (name) {
      case "inc":
      case "dec": {
        const d = name === "inc" ? 1 : -1;
        if (r) v.amt[id] = clamp((v.amt[id] || 0) + d * (r.step || 1), 0, r.cap);
        else if (it) {
          const was = (v.n[id] || 0) >= it.of;
          v.n[id] = clamp((v.n[id] || 0) + d, 0, it.of);
          const now = v.n[id] >= it.of;
          if (was !== now) say(`${label(it)}: ${now ? "done" : "not done"}`);
          // The up arrow goes dim at the limit; keep the keyboard on the row.
          if (now && name === "inc") focus = ["bump", id];
        }
        break;
      }
      case "bump":
        if (it) {
          const was = (v.n[id] || 0) >= it.of;
          v.n[id] = clamp((v.n[id] || 0) + 1, 0, it.of);
          if (!was && v.n[id] >= it.of) say(`${label(it)}: done`);
        }
        break;
      case "tick":
        v.n[id] = v.n[id] ? 0 : 1;
        say(`${label(it)}: ${v.n[id] ? "done" : "not done"}`);
        break;
      case "fold":
        v.fold[id] = !v.fold[id];
        break;
      case "group":
        v.open[id] = !v.open[id];
        break;
      case "amt-edit":
        v.ui.editAmt = id;
        v.ui.editVal = v.amt[id] == null ? "" : String(v.amt[id]);
        break;
      case "amt-clear":
        delete v.amt[id];
        v.ui.editAmt = null;
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
        say(`${label(findItem(v, id) || r)} hidden`);
        break;
      case "unhide":
        v.hidden.delete(id);
        say(`${label(findItem(v, id) || r)} put back`);
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
        const nm = v.add.name.trim();
        if (!nm) return;
        const item = { id: "c" + Date.now(), kind: v.add.kind, name: nm, of: v.add.kind === "count" ? v.add.of : undefined, custom: true, reset: v.add.reset };
        v.custom.push(item);
        const where = item.reset === "daily" ? "today" : "week";
        v.fold[where] = false;
        v.adding = false;
        v.removed = null;
        flash(`Added <b class="t-strong">${esc(nm)}</b> to ${item.reset === "daily" ? "Today" : "This week"}.`);
        focus = ["add-open"];
        break;
      }
      default:
        return; // close, reset hour: not part of the live window
    }
    redraw(focus);
  }

  document.addEventListener("mousedown", (e) => {
    // Keep the amount field focused while its × is pressed.
    if (e.target.closest && e.target.closest(".d-clear")) e.preventDefault();
  });

  document.addEventListener("mousemove", () => {
    if (!LIVE.ui.still) return;
    LIVE.ui.still = false;
    document.querySelectorAll("[data-live].is-still").forEach((t) => t.classList.remove("is-still"));
  });

  document.addEventListener("click", (e) => {
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
    if (!el && !e.target.closest(".e-x")) {
      const row = e.target.closest(".e-row.is-tick:not(.is-off)");
      if (row) el = row.querySelector(".e-main");
    }
    if (v.menu && !(el && el.dataset.act === "menu")) {
      v.menu = false;
      if (!el) return redraw();
    }
    // A click outside the form closes it while the name is empty.
    if (v.adding && !e.target.closest(".e-add") && !v.add.name.trim()) {
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
    if (el.classList.contains("d-input") && el.closest(".d-field")) {
      if (e.key === "Enter" || e.key === "Escape") {
        e.preventDefault();
        commit(el, e.key === "Enter", true);
      }
      return;
    }
    if (el.classList.contains("e-add-name")) {
      if (e.key === "Enter" || e.key === "Escape") {
        e.preventDefault();
        act(e.key === "Enter" ? "add" : "add-cancel");
      }
      return;
    }
    const a = el.dataset && el.dataset.act;
    // Delete on a focused row hides it (removes your own), like its ×.
    if (e.key === "Delete") {
      const row = el.closest(".e-row:not(.is-off)");
      const x = row && row.querySelector('.e-x [data-act="hide"], .e-x [data-act="remove"]');
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
    if (e.target.classList && e.target.classList.contains("e-add-name")) {
      LIVE.add.name = e.target.value;
      const add = e.target.closest(".e-add").querySelector('[data-act="add"]');
      const ok = !!LIVE.add.name.trim();
      add.disabled = !ok;
      add.classList.toggle("is-disabled", !ok);
    }
  });

  document.addEventListener("focusout", (e) => {
    if (redrawing) return;
    const el = e.target;
    if (el.classList && el.classList.contains("d-input") && el.closest(".d-field") && el.isConnected) {
      if (e.relatedTarget && e.relatedTarget.classList.contains("d-clear")) return;
      commit(el, true);
    }
  });

  // ---- States -------------------------------------------------------------
  const allDone = () => {
    const n = {};
    flat(TODAY.concat(WEEK, [LEGION])).forEach((it) => {
      if (it.kind !== "group") n[it.id] = it.of || 1;
    });
    return n;
  };

  const UNDER = { n: { ...MID }, custom: [{ ...LEGION }], amt: { odyle: 425 } };
  const HID = new Set(["dutycmd", "ordeal", "deus", "arkanis", "balaur", "fafnium"]);

  const STATES = {
    fresh: () => view({ now: WED, custom: [{ ...LEGION }], max: 452 }),
    progress: () => view({ ...UNDER, fold: { today: false, charges: true, week: false }, open: { sanct: true, explore: false }, max: 452 }),
    hover: () =>
      view({
        ...UNDER,
        n: { ...MID, legion: 0 },
        fold: { today: false, charges: true, week: false },
        ui: { hover: ["row:legion", "main:legion", "cue:legion"], tip: { id: "legion", text: "Mark done" } },
      }),
    hoverDone: () =>
      view({
        ...UNDER,
        fold: { today: true, charges: true, week: false },
        open: { sanct: true },
        ui: { hover: ["row:forgeboss", "main:forgeboss", "cue:forgeboss"], tip: { id: "forgeboss", text: "Undo" } },
      }),
    reach: () =>
      view({
        ...UNDER,
        n: { ...MID, duty: 5 },
        fold: { today: false, charges: true, week: false },
        ui: {
          hover: "row:duty",
          tip: { id: "duty", title: "Duty Missions: 5 of 5, done", text: "Struck through and muted where it stands; rows never move. The down arrow takes one back." },
        },
      }),
    hidden: () =>
      view({
        ...UNDER,
        hidden: new Set(HID),
        fold: { today: true, charges: false, week: false },
        ui: { hover: "show:charges" },
        max: 452,
      }),
    reveal: () =>
      view({
        ...UNDER,
        hidden: new Set(HID),
        reveal: { charges: true },
        fold: { today: true, charges: false, week: true },
        ui: { hover: ["row:deus", "x:deus"] },
      }),
    adding: () =>
      view({
        ...UNDER,
        fold: { today: true, charges: true, week: false },
        open: { sanct: false, explore: false },
        adding: true,
        add: { name: "Arena of Tactics wins", kind: "count", of: 3, reset: "weekly" },
        max: 330,
        scroll: "end",
      }),
    removed: () =>
      view({
        ...UNDER,
        custom: [],
        removed: { it: { ...LEGION }, at: 0 },
        fold: { today: false, charges: true, week: false },
      }),
    done: () =>
      view({
        n: allDone(),
        custom: [{ ...LEGION }],
        amt: { odyle: 425 },
        fold: { today: true, charges: true, week: true },
        note: `<b class="t-strong">Everything is done on ${"Mirelle"}.</b> Odyle Energy is full in ${span(estimate(CHARGES[0], 425, MON).at - MON)}.`,
      }),
    focus: () =>
      view({
        ...UNDER,
        n: { ...MID, legion: 0 },
        fold: { today: false, charges: true, week: false },
        ui: {
          focus: "row:legion",
          tip: { id: "legion", title: "Read out as \"Legion check-in, not done, button\"", text: "Space or Enter: done (then \"Legion check-in: done\"). Delete: remove it (your own) or hide it (the others). Tab: the next row." },
        },
      }),
    waste: () => view({ ...UNDER, amt: { odyle: 560, invasion: 7, shugo: 20, nightmare: 10 }, fold: { today: true, charges: false, week: true } }),
    altOver: () => view({ ...UNDER, fold: { today: false, charges: true, week: false }, open: { sanct: true, explore: false }, alt: "over" }),
  };

  function render(state, ctx) {
    lastCtx = ctx;
    if (state === "try") return windowHtml(LIVE, ctx);
    // The meter's "…" menu: To-dos between PvP meter and Settings, with
    // "1 full" while an amount you typed is full (as drawn in A).
    if (state === "menu") return `<div class="t-stack">${T.meter(ctx)}</div><div style="height:96px"></div>`;
    const make = STATES[state];
    return make ? windowHtml(make(), ctx) : "";
  }

  const main = document.querySelector("main.scenes");
  if (main) new MutationObserver(() => requestAnimationFrame(syncThumbs)).observe(main, { childList: true });
  if (document.fonts) document.fonts.ready.then(syncThumbs);

  window.TodoE = { render };
})();
