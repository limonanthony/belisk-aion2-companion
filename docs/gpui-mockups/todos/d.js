/* Proposal D, the chosen model: one list per character (B's sections) with
 * one line per to-do (A's rows), and every figure set by the player. The
 * meter cannot read any of these counts from the game (packet research,
 * 2026-10-06), so nothing here is filled in from a packet.
 *
 * Three kinds of rows:
 *   tick    done or not done
 *   count   X / N with one-less / one-more arrows; back to 0 at each reset
 *   amount  optional: empty ("—") and ignored until you type what you have;
 *           then the meter adds the refill on schedule and says when it is
 *           full
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
  const { icon, esc } = T;

  // ---- Clock -------------------------------------------------------------
  // Minutes since Monday 00:00 of the mocked week.
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

  // The k-th refill after `now`: every few hours, or once a day at a time.
  const refill = (r, now, k) => (r.every ? (Math.floor(now / r.every) + k) * r.every : nextAt(now, r.at) + (k - 1) * DAY);

  function estimate(r, v, now) {
    if (v >= r.cap) return { full: true, next: refill(r, now, 1) };
    return { full: false, at: refill(r, now, Math.ceil((r.cap - v) / r.add)) };
  }

  // ---- The default list ---------------------------------------------------
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
  const capOf = (v, r) => (r.id === "odyle" && v.member ? 840 : r.cap);
  const chargesOf = (v) => CHARGES.map((r) => ({ ...r, cap: capOf(v, r), rule: r.id === "odyle" && v.member ? "+15 every 3 h, up to 840 (membership)" : r.rule }));
  const todayList = (v) => TODAY.concat(v.custom.filter((c) => c.reset === "daily"));
  const weekList = (v) => WEEK.concat(v.custom.filter((c) => c.reset === "weekly"));
  const shown = (list, v) => list.filter((it) => v.edit || !v.hidden.has(it.id));
  const isDone = (it, v) => (it.kind === "group" ? shown(it.items, v).every((s) => isDone(s, v)) : (v.n[it.id] || 0) >= (it.of || 1));
  const left = (list, v) => shown(list, v).filter((it) => !isDone(it, v)).length;
  const label = (it) => (it.sub ? `${it.name}, ${it.sub}` : it.name);
  const clamp = (x, lo, hi) => Math.max(lo, Math.min(hi, x));

  function view(o = {}) {
    return {
      now: MON,
      n: {},
      amt: {},
      custom: [],
      hidden: new Set(HIDDEN),
      fold: { today: false, charges: false, week: false },
      open: { sanct: false, explore: false },
      ui: {},
      edit: false,
      live: false,
      member: false,
      menu: false,
      max: 430,
      scroll: "",
      note: "",
      add: { name: "", kind: "tick", of: 3, reset: "daily" },
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

  // `state` marks a drawn pointer or keyboard moment: "hover", "press" or
  // "focus" in v.ui, keyed "<part>:<id>".
  function mark(v, key) {
    return (has(v.ui.hover, key) ? " is-hover" : "") + (has(v.ui.press, key) ? " is-press" : "") + (has(v.ui.focus, key) ? " is-focus" : "");
  }

  function button(name, lbl, { act, id, cls = "", disabled = false, sm = true } = {}) {
    return `<button type="button" class="o-btn${sm ? " is-sm" : ""}${cls}" ${attr({ "data-act": act, "data-id": id, "aria-label": lbl, title: lbl, disabled })}>${icon(name)}</button>`;
  }

  function nameHtml(it) {
    const acct = it.account ? `<span class="t-acct">${icon("users")}Account</span>` : "";
    return `<span class="t-name">${esc(it.name)}${it.sub ? `<small>${esc(it.sub)}</small>` : ""}${acct}${it.custom ? "<small>Yours</small>" : ""}</span>`;
  }

  function tip(v, id) {
    const t = v.ui.tip;
    if (!t || t.id !== id) return "";
    return `<div class="t-tip d-tip" role="tooltip" style="${t.at || "top:27px;right:0"}"><b>${t.title}</b>${t.text}</div>`;
  }

  function rowCls(v, it, extra) {
    const c = ["t-item", "d-row", ...extra.filter(Boolean)];
    if (v.edit && v.hidden.has(it.id)) c.push("is-off");
    if (has(v.ui.hover, `row:${it.id}`)) c.push("is-hover");
    if (has(v.ui.focus, `row:${it.id}`)) c.push("is-focus");
    return c.join(" ");
  }

  function editBtns(v, it) {
    if (it.custom) return `<span class="t-edit">${button("trash", `Delete ${label(it)}`, { act: "delete", id: it.id })}</span>`;
    const off = v.hidden.has(it.id);
    return `<span class="t-edit">${button(off ? "eyeOff" : "eye", `${off ? "Show" : "Hide"} ${label(it)}`, { act: "hide", id: it.id })}</span>`;
  }

  // One less, the figure, one more. A click on a count's figure adds one;
  // a click on an amount's figure opens the field.
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
      "aria-valuetext": `${n} of ${of}`,
      "data-act": amt ? "amt-edit" : "bump",
      "data-id": id,
      title: amt ? "Click to type the amount" : "Click to add one",
    });
    return `<span class="d-step">${dn}<span class="d-count${amt ? " is-amt" : ""}${mark(v, `count:${id}`)}" ${a}>${n}<small>/${of}</small></span>${up}</span>`;
  }

  function countRow(v, it, isSub) {
    const n = Math.min(v.n[it.id] || 0, it.of);
    const done = n >= it.of;
    const sug = v.ui.suggest === it.id && !done;
    let right;
    if (v.edit) right = editBtns(v, it);
    else {
      const ask = sug
        ? `<span class="d-sug"><button type="button" class="o-tbtn t-small is-good" data-act="accept" data-id="${it.id}" aria-label="Add one to ${esc(label(it))}: the meter saw its last boss fall">${icon("activity")}+1?</button>${button("close", "Not this time", { act: "dismiss", id: it.id })}</span>`
        : "";
      right = ask + stepper(v, it.id, n, it.of, label(it));
    }
    const box = T.box(done ? "done" : sug ? "suggest" : "");
    return `<div class="${rowCls(v, it, [isSub && "is-sub", done && "is-done", sug && "is-suggest"])}" data-row="${it.id}">${box}${nameHtml(it)}${right}${tip(v, it.id)}</div>`;
  }

  function tickRow(v, it, isSub) {
    const done = (v.n[it.id] || 0) >= 1;
    const a = v.edit ? "" : attr({ role: "checkbox", tabindex: 0, "aria-checked": String(done), "aria-label": label(it), "data-act": "tick", "data-id": it.id });
    return `<div class="${rowCls(v, it, ["is-tick", isSub && "is-sub", done && "is-done"])}" data-row="${it.id}" ${a}>${T.box(done ? "done" : "")}${nameHtml(it)}${v.edit ? editBtns(v, it) : ""}${tip(v, it.id)}</div>`;
  }

  function groupRow(v, it) {
    const subs = shown(it.items, v);
    const done = subs.filter((s) => isDone(s, v)).length;
    const open = !!v.open[it.id];
    const all = subs.length > 0 && done === subs.length;
    const a = attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "aria-label": `${it.name}, ${done} of ${subs.length} done`, "data-act": "group", "data-id": it.id });
    const right = v.edit ? editBtns(v, it) : `<span class="d-sum${all ? " is-good" : ""}">${done} of ${subs.length} done</span>`;
    const head = `<div class="${rowCls(v, it, ["is-group", all && "is-done"])}" data-row="${it.id}" ${a}><span class="t-glyph">${icon(open ? "chevronDown" : "chevronRight")}</span>${nameHtml(it)}${right}</div>`;
    return head + (open ? subs.map((s) => rowFor(v, s, true)).join("") : "");
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

  function amountRow(v, r) {
    const a = v.amt[r.id];
    const extra = [];
    let glyph = `<span class="t-glyph">${icon("hourglass")}</span>`;
    let right;
    if (v.edit) {
      right = editBtns(v, r);
      if (a == null) extra.push("is-empty");
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
        glyph = `<span class="t-glyph is-warn">${icon("alert")}</span>`;
      }
      const when = e.full ? (r.every ? "Full, being wasted" : `Full, spend by ${stamp(e.next)}`) : `Full in ${span(e.at - v.now)}`;
      right = `<span class="d-when${e.full ? " is-warn" : ""}">${when}</span>${stepper(v, r.id, a, r.cap, r.name, { amt: true, step: r.step || 1 })}`;
    }
    return `<div class="${rowCls(v, r, extra)}" data-row="${r.id}">${glyph}${nameHtml(r)}${right}${tip(v, r.id)}</div>`;
  }

  function sec(v, id, title, { n = "", good = false, at = "", chip = "" } = {}) {
    const open = !v.fold[id];
    const a = attr({ role: "button", tabindex: 0, "aria-expanded": String(open), "data-act": "fold", "data-id": id });
    return `<div class="t-sec${mark(v, `sec:${id}`)}" ${a}>${icon(open ? "chevronDown" : "chevronRight")}<span>${title}</span>${n ? `<span class="n${good ? " is-good" : ""}">${n}</span>` : ""}${chip}<span class="at">${at}</span></div>`;
  }

  function todaySec(v) {
    const list = todayList(v);
    const l = left(list, v);
    const head = sec(v, "today", "Today", { n: v.edit ? "" : l ? `${l} left` : "done", good: !l, at: `resets in <b>${span(nextAt(v.now, RESET) - v.now)}</b>` });
    return head + (v.fold.today ? "" : `<div class="t-list">${shown(list, v).map((it) => rowFor(v, it)).join("")}</div>`);
  }

  function chargeSec(v) {
    const list = chargesOf(v);
    const tracked = list.filter((r) => v.amt[r.id] != null && !v.hidden.has(r.id));
    const full = tracked.filter((r) => v.amt[r.id] >= r.cap).length;
    const filling = tracked.filter((r) => v.amt[r.id] < r.cap).map((r) => estimate(r, v.amt[r.id], v.now).at);
    let at = "";
    if (!v.edit) at = filling.length ? `next full in <b>${span(Math.min(...filling) - v.now)}</b>` : tracked.length ? "" : "amounts optional";
    const head = sec(v, "charges", "Recharges", {
      n: tracked.length && !v.edit ? `${tracked.length} tracked` : "",
      chip: full && !v.edit ? T.warnChip(`${full} full`) : "",
      at,
    });
    if (v.fold.charges) return head;
    const member = v.edit
      ? `<div class="t-item d-row is-tick${v.member ? " is-done" : ""}" ${attr({ role: "checkbox", tabindex: 0, "aria-checked": String(v.member), "data-act": "member" })}>${T.box(v.member ? "done" : "")}<span class="t-name">Membership<small>Odyle Energy up to 840</small></span></div>`
      : "";
    return head + `<div class="t-list">${member}${shown(list, v).map((r) => amountRow(v, r)).join("")}</div>`;
  }

  function weekSec(v) {
    const list = weekList(v);
    const l = left(list, v);
    const w = nextWeekly(v.now);
    const head = sec(v, "week", "This week", { n: v.edit ? "" : l ? `${l} left` : "done", good: !l, at: `${stamp(w)} · in <b>${span(w - v.now)}</b>` });
    return head + (v.fold.week ? "" : `<div class="t-list">${shown(list, v).map((it) => rowFor(v, it)).join("")}</div>`);
  }

  function head(v) {
    const l = left(todayList(v), v) + left(weekList(v), v);
    const full = chargesOf(v).filter((r) => v.amt[r.id] != null && v.amt[r.id] >= r.cap && !v.hidden.has(r.id)).length;
    const count = v.edit ? "" : `<span class="o-count">${l ? `${l} left` : "all done"}</span>`;
    const chip = full && !v.edit ? T.warnChip(`${full} full`) : "";
    const edit = button("pencil", v.edit ? "Done editing" : "Edit the list", { act: "edit", cls: v.edit ? " is-hot" : "", sm: false });
    return `<div class="o-head"><span class="o-title">${icon("listChecks")}${v.edit ? "Edit to-dos" : "To-dos"}</span>${count}${chip}<span class="grow"></span>${edit}${button("close", "Close", { sm: false })}</div>`;
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

  function addForm(v) {
    const a = v.add;
    const name = v.live
      ? `<span class="o-input"><input class="d-input d-add-name" placeholder="Add a to-do" autocomplete="off" aria-label="New to-do" value="${esc(a.name)}"></span>`
      : a.name
        ? `<span class="o-input is-focus"><span class="val">${esc(a.name)}</span><span class="o-caret"></span></span>`
        : `<span class="o-input"><span class="ph">Add a to-do</span></span>`;
    const seg = (key, lbl, opts) =>
      `<span class="t-seg" role="radiogroup" aria-label="${lbl}">${opts
        .map(([val, text]) => `<button type="button" role="radio" aria-checked="${a[key] === val}" class="${a[key] === val ? "is-on" : ""}" data-act="add-${key}" data-id="${val}">${text}</button>`)
        .join("")}</span>`;
    const of =
      a.kind === "count"
        ? `<span class="d-of"><span>of</span><span class="d-step">${button("chevronDown", "One less", { act: "add-dec", cls: " d-arrow", disabled: a.of <= 2 })}<span class="d-count is-fixed" ${attr({ role: "spinbutton", tabindex: 0, "aria-label": "How many", "aria-valuenow": a.of, "aria-valuemin": 2, "data-act": "add-of" })}>${a.of}</span>${button("chevronUp", "One more", { act: "add-inc", cls: " d-arrow" })}</span></span>`
        : "";
    return `<div class="d-add">${name}<div class="d-add-row">${seg("kind", "Kind", [["tick", "Tick"], ["count", "Count"]])}${of}${seg("reset", "Resets", [["daily", "Daily"], ["weekly", "Weekly"]])}<span class="grow"></span><button type="button" class="o-tbtn t-small${a.name.trim() ? "" : " is-disabled"}" data-act="add"${a.name.trim() ? "" : " disabled"}>${icon("plus")}Add</button></div></div>`;
  }

  function foot(v) {
    if (v.edit) {
      return `<div class="o-foot"><button type="button" class="o-tbtn is-quiet t-small" data-act="restore">${icon("rotate")}Restore defaults</button><span class="grow"></span><button type="button" class="o-tbtn t-small" data-act="edit">${icon("check")}Done</button></div>`;
    }
    return `<div class="o-foot d-foot">${icon("clock")}<span>Daily reset <b>${pad(RESET / 60)}:00</b></span><span class="faint">·</span><button type="button" class="d-link" data-act="reset-hour" aria-label="Change the daily reset hour">Change</button><span class="grow"></span><span class="faint">Weekly on Wednesday</span></div>`;
  }

  function windowHtml(v, ctx) {
    const note = v.note ? `<div class="o-note is-good t-note">${icon("check")}<span>${v.note}</span></div>` : "";
    const hint = v.edit ? `<div class="t-hint" style="margin:0 0 6px">Hide what ${T.nm(ctx, T.CHARS[0].name)} doesn't do; hidden to-dos stay here, faded. Each character keeps its own list.</div>` : "";
    const body = todaySec(v) + chargeSec(v) + weekSec(v);
    const scroll = `<div class="d-scrollwrap"><div class="d-scroll" style="max-height:${v.max}px"${v.scroll ? ` data-scroll="${v.scroll}"` : ""}>${body}</div><span class="o-thumb d-thumb" aria-hidden="true"></span></div>`;
    return `<div class="ov o td is-d" role="dialog" aria-label="To-dos"${v.live ? " data-live" : ""}>${head(v)}${charSelect(v, ctx)}${note}${hint}${scroll}${v.edit ? addForm(v) : ""}${foot(v)}</div>`;
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

  function redraw(focusKey) {
    const tiles = [...document.querySelectorAll("[data-live]")];
    const active = document.activeElement;
    const at = tiles.findIndex((t) => t.contains(active));
    const key = focusKey || (at >= 0 && active.dataset && active.dataset.act ? [active.dataset.act, active.dataset.id] : null);
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
    if (key) {
      const sel = `[data-act="${key[0]}"]` + (key[1] ? `[data-id="${key[1]}"]` : "");
      const el = home.querySelector(sel);
      if (el && !el.disabled) el.focus({ preventScroll: true });
    }
  }

  // Enter keeps, Esc cancels; from the keyboard, focus returns to the figure.
  function commit(input, save, refocus) {
    const v = LIVE;
    const id = v.ui.editAmt;
    if (!id) return;
    if (save) {
      const r = chargesOf(v).find((x) => x.id === id);
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

  function act(name, id) {
    const v = LIVE;
    const r = chargesOf(v).find((x) => x.id === id);
    const it = findItem(v, id);
    switch (name) {
      case "inc":
      case "dec": {
        const d = name === "inc" ? 1 : -1;
        if (r) v.amt[id] = clamp((v.amt[id] || 0) + d * (r.step || 1), 0, r.cap);
        else if (it) v.n[id] = clamp((v.n[id] || 0) + d, 0, it.of);
        break;
      }
      case "bump":
        if (it) v.n[id] = clamp((v.n[id] || 0) + 1, 0, it.of);
        break;
      case "tick":
        v.n[id] = v.n[id] ? 0 : 1;
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
      case "edit":
        v.edit = !v.edit;
        v.ui.editAmt = null;
        v.menu = false;
        break;
      case "menu":
        v.menu = !v.menu;
        break;
      case "hide":
        if (v.hidden.has(id)) v.hidden.delete(id);
        else v.hidden.add(id);
        break;
      case "delete":
        v.custom = v.custom.filter((c) => c.id !== id);
        break;
      case "member":
        v.member = !v.member;
        if (v.amt.odyle != null) v.amt.odyle = Math.min(v.amt.odyle, capOf(v, CHARGES[0]));
        break;
      case "restore":
        v.hidden = new Set(HIDDEN);
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
        v.custom.push({ id: "c" + Date.now(), kind: v.add.kind, name: nm, of: v.add.kind === "count" ? v.add.of : undefined, custom: true, reset: v.add.reset });
        v.add.name = "";
        break;
      }
      default:
        return; // close, reset hour, suggestions: not part of the live window
    }
    redraw();
  }

  document.addEventListener("mousedown", (e) => {
    // Keep the amount field focused while its × is pressed.
    if (e.target.closest && e.target.closest(".d-clear")) e.preventDefault();
  });

  document.addEventListener("click", (e) => {
    const tile = e.target.closest && e.target.closest("[data-live]");
    if (!tile) {
      if (LIVE.menu) {
        LIVE.menu = false;
        redraw();
      }
      return;
    }
    const tiles = [...document.querySelectorAll("[data-live]")];
    if (tiles.includes(tile)) lastTile = tiles.indexOf(tile);
    const el = e.target.closest("[data-act]");
    if (LIVE.menu && !(el && el.dataset.act === "menu")) {
      LIVE.menu = false;
      if (!el) return redraw();
    }
    if (!el || el.disabled || el.tagName === "INPUT") return;
    act(el.dataset.act, el.dataset.id);
  });

  document.addEventListener("keydown", (e) => {
    const el = e.target;
    if (!el.closest || !el.closest("[data-live]")) return;
    if (el.classList.contains("d-input") && el.closest(".d-field")) {
      if (e.key === "Enter") commit(el, true, true);
      else if (e.key === "Escape") commit(el, false, true);
      return;
    }
    if (el.classList.contains("d-add-name")) {
      if (e.key === "Enter") act("add");
      return;
    }
    const a = el.dataset && el.dataset.act;
    if (!a) return;
    if (el.getAttribute("role") === "spinbutton" && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      if (a === "add-of") act(e.key === "ArrowUp" ? "add-inc" : "add-dec");
      else act(e.key === "ArrowUp" ? "inc" : "dec", el.dataset.id);
      return;
    }
    if (el.tagName !== "BUTTON" && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      act(a, el.dataset.id);
    }
  });

  document.addEventListener("input", (e) => {
    if (e.target.classList && e.target.classList.contains("d-add-name")) {
      LIVE.add.name = e.target.value;
      const add = e.target.closest(".d-add").querySelector('[data-act="add"]');
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

  function odyleNote(amt) {
    const r = CHARGES[0];
    const e = estimate(r, amt, MON);
    return `Full ${stamp(e.at)} (estimate)`;
  }

  const STATES = {
    fresh: () => view({ now: WED, max: 452 }),
    progress: () => view({ n: { ...MID }, custom: [{ ...LEGION }], amt: { odyle: 425 }, fold: { today: false, charges: true, week: false }, open: { sanct: true, explore: false }, max: 452 }),
    amount: () =>
      view({
        n: { ...MID },
        custom: [{ ...LEGION }],
        amt: { odyle: 425, nightmare: 10 },
        fold: { today: true, charges: false, week: true },
        ui: {
          hover: "row:odyle",
          tip: { id: "odyle", title: odyleNote(425), text: "+15 every 3 h, up to 560. After you spend some, set what the game shows: the arrows (10 at a time), or click the figure and type." },
        },
      }),
    typing: () =>
      view({
        n: { ...MID },
        amt: { odyle: 425, nightmare: 10 },
        fold: { today: true, charges: false, week: true },
        ui: { editAmt: "shugo", editVal: "24", tip: { id: "shugo", title: "Shugo Festival keys", text: "Enter keeps 24 (Esc cancels). To stop tracking, empty the field or click ×: the row goes back to the dash and is ignored again." } },
      }),
    waste: () => view({ n: { ...MID }, amt: { odyle: 560, invasion: 7, shugo: 20, nightmare: 10 }, fold: { today: true, charges: false, week: true } }),
    done: () =>
      view({
        n: allDone(),
        custom: [{ ...LEGION }],
        amt: { odyle: 425 },
        fold: { today: true, charges: true, week: true },
        note: `<b class="t-strong">Everything is done on ${"Mirelle"}.</b> Odyle Energy is full in ${span(estimate(CHARGES[0], 425, MON).at - MON)}.`,
      }),
    edit: () =>
      view({
        edit: true,
        n: { ...MID },
        custom: [{ ...LEGION, reset: "weekly", name: "Weekly Legion quest" }],
        hidden: new Set(["dutycmd", "x-azure", "x-illusion", "x-urugugu", "x-vakron"]),
        fold: { today: true, charges: true, week: false },
        open: { sanct: false, explore: true },
        max: 300,
        scroll: "end",
        add: { name: "Arena of Tactics wins", kind: "count", of: 3, reset: "weekly" },
      }),
    hover: () =>
      view({
        n: { ...MID },
        fold: { today: true, charges: true, week: false },
        ui: { hover: ["dec:fissure", "row:fissure"], tip: { id: "fissure", title: "One less: Daily Dungeon", text: "9 of 14 now. The game calls it a daily dungeon, but its 14 entries reset on Wednesday." } },
      }),
    press: () => view({ n: { ...MID, duty: 3 }, fold: { today: false, charges: true, week: false }, ui: { hover: "row:duty", press: "inc:duty" } }),
    count: () =>
      view({
        n: { ...MID },
        fold: { today: true, charges: true, week: false },
        ui: { hover: ["count:subj", "row:subj"], tip: { id: "subj", title: "Subjugation: 1 of 3", text: "Click adds one. The arrows correct a slip." } },
      }),
    focus: () => view({ n: { ...MID }, fold: { today: true, charges: true, week: false }, ui: { focus: "count:fissure" } }),
    suggest: () =>
      view({
        n: { ...MID, trial: 1 },
        fold: { today: true, charges: true, week: false },
        ui: { suggest: "trial", tip: { id: "trial", title: "Seen by the meter at 21:32", text: "Ascension Trial's last boss fell. +1? adds one; × lets it go. An option, off by default." } },
      }),
    switch: () => view({ n: { ...MID }, custom: [{ ...LEGION }], amt: { odyle: 425 }, menu: true, fold: { today: false, charges: true, week: false } }),
  };

  function render(state, ctx) {
    lastCtx = ctx;
    if (state === "try") return windowHtml(LIVE, ctx);
    const make = STATES[state];
    return make ? windowHtml(make(), ctx) : "";
  }

  const main = document.querySelector("main.scenes");
  if (main) new MutationObserver(() => requestAnimationFrame(syncThumbs)).observe(main, { childList: true });
  if (document.fonts) document.fonts.ready.then(syncThumbs);

  window.TodoD = { render };
})();
