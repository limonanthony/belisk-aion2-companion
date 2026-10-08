// The hub round (2026-10-08): drawing for the toolbar, its dropdowns, the
// Resources window, the meter with its "…" menu, the first-open prompt and
// the reminder card under the toolbar. Shared by the pages in this folder;
// each page mounts its states with Kit.mount (../approved/kit.js).
//
// Everything here is invented: values, times, characters. Caps and refills
// are the game's ticket table as in our notes (todo-autofill-findings.md),
// as in ../todos-autofill/. Nothing is saved or sent anywhere.
//
// Icons: Lucide (https://lucide.dev), ISC License; see ../approved/kit.js.
(function () {
  "use strict";

  const EXTRA = {
    listChecks: '<path d="m3 17 2 2 4-4"/><path d="m3 7 2 2 4-4"/><path d="M13 6h8"/><path d="M13 12h8"/><path d="M13 18h8"/>',
    users:
      '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
    gamepad:
      '<line x1="6" x2="10" y1="11" y2="11"/><line x1="8" x2="8" y1="9" y2="13"/><line x1="15" x2="15.01" y1="12" y2="12"/><line x1="18" x2="18.01" y1="10" y2="10"/><path d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"/>',
    bell: '<path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/>',
    bellOff: '<path d="M8.7 3A6 6 0 0 1 18 8a21.3 21.3 0 0 0 .6 5"/><path d="M17 17H3s3-2 3-9a4.67 4.67 0 0 1 .3-1.7"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/><path d="m2 2 20 20"/>',
    crosshair:
      '<circle cx="12" cy="12" r="10"/><line x1="22" x2="18" y1="12" y2="12"/><line x1="6" x2="2" y1="12" y2="12"/><line x1="12" x2="12" y1="6" y2="2"/><line x1="12" x2="12" y1="22" y2="18"/>',
    grip: '<circle cx="9" cy="12" r="1"/><circle cx="9" cy="5" r="1"/><circle cx="9" cy="19" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="5" r="1"/><circle cx="15" cy="19" r="1"/>',
    key: '<path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"/><circle cx="16.5" cy="7.5" r=".5" fill="currentColor"/>',
    chevronsLeft: '<path d="m11 17-5-5 5-5"/><path d="m18 17-5-5 5-5"/>',
    chevronsRight: '<path d="m6 17 5-5-5-5"/><path d="m13 17 5-5-5-5"/>',
    package:
      '<path d="M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z"/><path d="M12 22V12"/><path d="m3.3 7 7.703 4.734a2 2 0 0 0 1.994 0L20.7 7"/><path d="m7.5 4.27 9 5.15"/>',
    map: '<path d="M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"/><path d="M15 5.764v15"/><path d="M9 3.236v15"/>',
    pencil:
      '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>',
    swords:
      '<polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" x2="19" y1="19" y2="13"/><line x1="16" x2="20" y1="16" y2="20"/><line x1="19" x2="21" y1="21" y2="19"/><polyline points="14.5 6.5 18 3 21 3 21 6 17.5 9.5"/><line x1="5" x2="9" y1="14" y2="18"/><line x1="7" x2="4" y1="17" y2="20"/><line x1="3" x2="5" y1="19" y2="21"/>',
  };

  function icon(name, cls) {
    if (!EXTRA[name]) return Kit.icon(name, cls);
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${EXTRA[name]}</svg>`;
  }

  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const tag = (kind) => `<span class="dec is-${kind.toLowerCase()}">${kind}</span>`;

  // ---- Open data (as in ../todos-autofill/): ?deus=14, ?conquest=pool ------
  const params = new URLSearchParams(location.search);
  const OPEN = {
    deus: params.get("deus") === "14" ? 14 : 4,
    conquest: params.get("conquest") === "pool" ? "pool" : "dungeon",
  };

  // ---- Clock (mocked): Monday 21:48, daily reset 09:00 ----------------------
  const DAY = 1440;
  const DAYS = { en: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], fr: ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."] };
  const RESET = 9 * 60;
  const NOW = 21 * 60 + 48;
  const LOGIN = 19 * 60 + 2;
  const pad = (n) => String(n).padStart(2, "0");
  const clock = (t) => `${pad(Math.floor((((t % DAY) + DAY) % DAY) / 60))}:${pad((((t % DAY) + DAY) % DAY) % 60)}`;
  const stamp = (t, lang = "en") => `${DAYS[lang][((Math.floor(t / DAY) % 7) + 7) % 7]} ${clock(t)}`;

  function span(m, lang = "en") {
    const d = Math.floor(m / DAY);
    const h = Math.floor((m % DAY) / 60);
    const min = m % 60;
    const dd = lang === "fr" ? "j" : "d";
    if (d) return `${d} ${dd} ${h} h`;
    if (h) return `${h} h ${min} min`;
    return `${min} min`;
  }

  function nextAt(now, at) {
    let t = Math.floor(now / DAY) * DAY + at;
    if (t <= now) t += DAY;
    return t;
  }

  const refill = (r, now, k) => (r.every ? (Math.floor((now - (r.off || 0)) / r.every) + k) * r.every + (r.off || 0) : nextAt(now, r.at) + (k - 1) * DAY);

  // ---- Resources: the to-do window's Recharges rows, same order -------------
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
      : { id: "conquest", kind: "group", name: "Conquest dungeons", open: "conquest", items: CONQUEST_DUNGEONS.map(([id, name]) => ({ id: "c-" + id, name, cap: 14, add: 2, at: RESET })) };

  const RES = [
    { id: "odyle", name: "Odyle Energy", fr: "Odyle Energy", short: "Odyle", ic: "zap", cap: 560, mcap: 840, add: 15, every: 180 },
    { id: "shugo", name: "Shugo Festival keys", fr: "Clés du Shugo Festival", short: "Shugo keys", shortFr: "Clés Shugo", ic: "key", cap: 12, mcap: 21, add: 3, at: RESET, account: true },
    { id: "invasion", name: "Invasion keys", fr: "Clés de l'Invasion", cap: 7, add: 1, at: RESET, account: true },
    { id: "nightmare", name: "Nightmare", cap: 14, add: 2, at: RESET },
    CONQUEST,
    { id: "deus", name: "Deus Research Base", cap: OPEN.deus, add: 2, at: RESET, open: "deus" },
    { id: "arkanis", name: "Shattered Arkanis", cap: OPEN.deus, add: 2, at: RESET, open: "deus" },
    { id: "balaur", name: "Abandoned Balaur Fortress", cap: 7, add: 1, at: RESET },
    { id: "fafnium", name: "Fafnium Laboratory", cap: 7, add: 1, at: RESET },
  ];

  const leaves = () => RES.flatMap((r) => (r.kind === "group" ? r.items : [r]));
  const find = (id) => leaves().find((r) => r.id === id);
  const capOf = (r, member) => (member && r.mcap ? r.mcap : r.cap);

  // What the game sent (n = regular pool, x = extra pool). Mirelle, live,
  // as in ../todos-autofill/ ("Filled from the game, live").
  function snapshot(o = {}) {
    const g = {
      odyle: { n: 245, x: 380 },
      shugo: { n: 7, x: 3 },
      invasion: { n: 2, x: 0 },
      nightmare: { n: 0, x: 3 },
      conquest: { n: 16, x: 0 },
      deus: { n: 4, x: 0 },
      arkanis: { n: 4, x: 0 },
      balaur: { n: 6, x: 0 },
      fafnium: { n: 6, x: 0 },
    };
    for (const [id] of CONQUEST_DUNGEONS) g["c-" + id] = { n: 12, x: 0 };
    g["c-fire"].n = 10;
    g["c-vakron"].n = 13;
    for (const [id, val] of Object.entries(o)) g[id] = val === null ? null : { ...g[id], ...val };
    return g;
  }

  // Nothing else full: the toolbar shows no warning.
  const CALM = { deus: { n: 2 }, arkanis: { n: 3 } };

  // A resource's state: { src: "game" | "empty", n, x, cap, full, above, at, next }.
  function stateOf(r, v) {
    const cap = capOf(r, v.member);
    const g = v.g[r.id];
    if (v.over && v.over[r.id] != null) {
      const n = v.over[r.id];
      return withEstimate(r, { src: "over", n, x: g ? g.x || 0 : 0, cap }, v);
    }
    if (!g) return { src: "empty", cap };
    // A regular pool above the cap in force can only come from a membership
    // the switch does not know: shown as is, never full, no estimate.
    if (g.n > cap) return { src: "game", n: g.n, x: g.x || 0, cap, above: true };
    return withEstimate(r, { src: "game", n: g.n, x: g.x || 0, cap }, v);
  }

  function withEstimate(r, s, v) {
    const now = v.now == null ? NOW : v.now;
    if (s.n >= s.cap) return { ...s, full: true, next: refill(r, now, 1) };
    return { ...s, full: false, at: refill(r, now, Math.ceil((s.cap - s.n) / r.add)) };
  }

  const fullOthers = (v) => leaves().filter((r) => r.id !== "odyle" && r.id !== "shugo" && stateOf(r, v).full);
  const fullAll = (v) => leaves().filter((r) => stateOf(r, v).full);

  function view(o = {}) {
    return { member: true, g: snapshot(), now: NOW, src: "live", lang: "en", ...o };
  }

  // ---- Strings (English; French for the new copy, a draft for i18n) --------
  const T = {
    en: {
      meter: ["DPS meter", "Fights are tracked while it is open."],
      meterOpen: ["DPS meter", "Click to close. Fights stop being tracked."],
      pvp: "PvP meter",
      todos: "To-dos",
      history: "Fight history",
      reminders: "Reminders",
      remindersOff: ["Reminders", "Paused"],
      settings: "Settings",
      fold: "Fold the toolbar",
      unfold: "Unfold the toolbar",
      quit: ["Quit Kisk", "Closes every tool."],
      fullLabel: "Full, a refill is coming",
      refills: "refills",
      all: "All resources",
      of: "of",
      extra: "extra",
      fullIn: "full in",
      fullSpend: "Full, spend by",
      fullWaste: "Full, being wasted",
      noValues: "No value yet",
      noValuesLine: "Log in while Kisk runs.",
      click: "Click for all resources.",
      moreFull: (n) => `${n} more full`,
    },
    fr: {
      meter: ["Compteur DPS", "Les combats sont suivis tant qu'il est ouvert."],
      meterOpen: ["Compteur DPS", "Cliquer pour le fermer. Les combats ne sont plus suivis."],
      pvp: "Compteur JcJ",
      todos: "Tâches",
      history: "Historique des combats",
      reminders: "Rappels",
      remindersOff: ["Rappels", "En pause"],
      settings: "Paramètres",
      fold: "Replier la barre",
      unfold: "Déplier la barre",
      quit: ["Quitter Kisk", "Ferme tous les outils."],
      fullLabel: "Pleines, une recharge arrive",
      refills: "recharge",
      all: "Toutes les ressources",
      of: "sur",
      extra: "en plus",
      fullIn: "max en",
      fullSpend: "À vider avant",
      fullWaste: "Plein, gaspillé",
      noValues: "Pas encore de valeur",
      noValuesLine: "Connectez-vous pendant que Kisk tourne.",
      click: "Cliquer pour toutes les ressources.",
      moreFull: (n) => `${n} autres pleines`,
    },
  };

  // ---- Toolbar ---------------------------------------------------------------
  // o: { lit: {meter, pvp, todos, history, reminders, settings, resources},
  //      hover: key, tip: key, warnOpen, bellOpen, bellOff, folded, labels,
  //      lang, v (resource view), empty, focus: key, up (dropdown opens up) }
  function tipHtml(t, cls = "") {
    if (!t) return "";
    const [a, b] = Array.isArray(t) ? t : [t, ""];
    return `<span class="hb-tip${cls}" role="tooltip">${a}${b ? `<span>${b}</span>` : ""}</span>`;
  }

  function toolBtn(o, key, ic, label, { tip, cls = "" } = {}) {
    const lit = o.lit && o.lit[key];
    const c = ["hb-btn", cls, lit && "is-lit", o.hover === key && "is-hover", o.focus === key && "is-focus"].filter(Boolean).join(" ");
    const tipShown = o.tip === key ? " is-on" : "";
    const press = key === "fold" || key === "unfold" || key === "quit" ? "" : ` aria-pressed="${!!lit}"`;
    return `<button type="button" class="${c}" data-hb="${key}" aria-label="${esc(Array.isArray(label) ? label[0] : label)}"${press}>${icon(ic)}${tipHtml(tip || label, tipShown)}</button>`;
  }

  function countBtn(o, r) {
    const L = T[o.lang || "en"];
    const v = o.v;
    const s = stateOf(r, v);
    const name = (o.lang === "fr" && r.fr) || r.name;
    const short = (o.lang === "fr" && r.shortFr) || r.short;
    const c = ["hb-count", s.full && "is-full", o.lit && o.lit.resources && "is-lit", o.hover === r.id && "is-hover", o.focus === r.id && "is-focus"].filter(Boolean).join(" ");
    let fig;
    let tip;
    if (s.src === "empty") {
      fig = `<span class="hb-fig is-blank">&mdash;</span>`;
      tip = [name, `${L.noValues}. ${L.noValuesLine}`];
    } else {
      fig = `<span class="hb-fig">${s.n}<small>/${s.cap}</small></span>${s.x ? `<span class="hb-x">+${s.x}</span>` : ""}`;
      const lang = o.lang || "en";
      const when = s.above ? "" : s.full ? (r.every ? L.fullWaste : `${L.fullSpend} ${stamp(s.next, lang)}`) : `${L.fullIn} ${span(s.at - v.now, lang)}`;
      const line = `${s.n} ${L.of} ${s.cap}${s.x ? `, +${s.x} ${L.extra}` : ""}${when ? ` · ${when}` : ""}`;
      tip = [name, `${line}. ${L.click}`];
    }
    const label = o.labels ? `<span class="nm">${esc(short)}</span>` : "";
    const aria = s.src === "empty" ? `${name}, ${L.noValues}` : `${name}, ${s.n} of ${s.cap}${s.x ? `, ${s.x} extra` : ""}${s.full ? ", full" : ""}. Open resources`;
    const shown = o.tip === r.id ? " is-on" : "";
    return `<button type="button" class="${c}" data-hb="res" aria-label="${esc(aria)}">${icon(r.ic)}${label}${fig}${tipHtml(tip, shown)}</button>`;
  }

  function warnDrop(o) {
    const L = T[o.lang || "en"];
    const v = o.v;
    const items = fullOthers(v)
      .map((r) => {
        const s = stateOf(r, v);
        const name = (o.lang === "fr" && r.fr) || r.name;
        const hot = o.dropHot === r.id ? " is-hot" : "";
        return `<div class="o-menu-item${hot}" role="menuitem"><span class="nm">${esc(name)}</span><span class="fig">${s.n}<small>/${s.cap}</small></span><span class="when">${L.refills} <b>${clock(s.next)}</b></span></div>`;
      })
      .join("");
    const allHot = o.dropHot === "all" ? " is-hot" : "";
    return `<div class="o-menu hb-drop${o.up ? " is-up" : ""}" role="menu"><div class="hb-drop-label">${icon("alert")}${L.fullLabel}</div>${items}<div class="o-menu-sep"></div><div class="o-menu-item${allHot}" role="menuitem"><span class="lead">${icon("package")}</span><span class="nm">${L.all}</span>${icon("chevronRight")}</div></div>`;
  }

  function bellDrop(o) {
    const items = [
      ["Shugo Festival", "22:00"],
      ["Dimensional Invasion", "22:30"],
      ["Rift portals", "23:00"],
    ]
      .map(([n, t]) => `<div class="o-menu-item" role="menuitem"><span class="nm">${n}</span><span class="when"><b>${t}</b></span></div>`)
      .join("");
    return `<div class="o-menu hb-drop is-left" role="menu" style="width:236px"><div class="hb-drop-label">${icon("bell")}Next reminders</div>${items}<div class="o-menu-sep"></div><div class="o-menu-item" role="menuitemcheckbox" aria-checked="false"><span class="lead"></span><span class="nm">Pause reminders</span></div><div class="o-menu-item" role="menuitem"><span class="lead">${icon("settings")}</span><span class="nm">Reminder settings</span>${icon("chevronRight")}</div></div>`;
  }

  function toolbar(ctx, o = {}) {
    const L = T[o.lang || "en"];
    o = { v: view(), ...o };
    const grip = `<span class="hb-grip" title="Drag to move">${icon("grip")}</span>`;
    if (o.folded) {
      const n = fullAll(o.v).length;
      const warn = n ? `<button type="button" class="hb-warn" data-hb="unfold" aria-label="${n} resources full. Unfold the toolbar">${icon("alert")}${n}</button>` : "";
      return `<div class="ov o hb is-folded${o.live ? " is-live" : ""}" role="toolbar" aria-label="Kisk">${grip}${toolBtn(o, "unfold", "chevronsRight", L.unfold, { cls: "is-end" })}${warn}</div>`;
    }
    const meterTip = o.lit && o.lit.meter ? L.meterOpen : L.meter;
    const bellIc = o.bellOff ? "bellOff" : "bell";
    const tools =
      toolBtn(o, "meter", "swords", meterTip) +
      toolBtn(o, "pvp", "crosshair", L.pvp) +
      toolBtn(o, "todos", "listChecks", L.todos) +
      toolBtn(o, "history", "crown", L.history) +
      `<span style="position:relative;display:inline-flex">${toolBtn(o, "reminders", bellIc, o.bellOff ? L.remindersOff : L.reminders, { cls: o.bellOff ? "is-off" : "" })}${o.bellOpen ? bellDrop(o) : ""}</span>` +
      toolBtn(o, "settings", "settings", L.settings);
    const others = fullOthers(o.v).length;
    const warn = others
      ? `<span style="position:relative;display:inline-flex"><button type="button" class="hb-warn${o.warnOpen ? " is-open" : ""}${o.hover === "warn" ? " is-hover" : ""}" data-hb="warn" aria-haspopup="menu" aria-expanded="${!!o.warnOpen}" aria-label="${others} more resources full">${icon("alert")}${others}${o.tip === "warn" ? tipHtml(L.moreFull(others), " is-on") : ""}</button>${o.warnOpen ? warnDrop(o) : ""}</span>`
      : "";
    const counts = `<span class="hb-counts">${countBtn(o, find("odyle"))}${countBtn(o, find("shugo"))}${warn}</span>`;
    const end = toolBtn(o, "fold", "chevronsLeft", L.fold, { cls: "is-end" }) + toolBtn(o, "quit", "close", L.quit, { cls: "is-end", tip: L.quit });
    return `<div class="ov o hb${o.live ? " is-live" : ""}" role="toolbar" aria-label="Kisk">${grip}<span class="hb-tools">${tools}</span><span class="hb-vr" aria-hidden="true"></span>${counts}<span class="hb-vr" aria-hidden="true"></span>${end}</div>`;
  }

  // ---- Resources window ------------------------------------------------------
  // The to-do window's classes (todos.css, autofill.css): header, character
  // selector, source line, the Recharges rows, footer.
  const CHARS = {
    mirelle: { name: "Mirelle", cls: "ranger", klass: "Ranger", server: "Ossia" },
    sefa: { name: "Sefa", cls: "cleric", klass: "Cleric", server: "Lyss" },
  };

  function nameHtml(r) {
    const acct = r.account ? `<span class="t-acct">${icon("users")}Account</span>` : "";
    return `<span class="t-name">${esc(r.name)}${acct}</span>`;
  }

  function resRow(v, r, sub) {
    const s = stateOf(r, v);
    const cls = ["t-row", sub && "is-sub"];
    let gut = `<span class="t-gut"></span>`;
    let when = "";
    let right;
    const hover = v.hover === r.id ? " is-hover" : "";
    if (v.edit === r.id) {
      right = `<span class="t-field"><span class="val">${esc(v.editVal)}</span><span class="o-caret"></span><span class="t-cap">/${s.cap}</span><button type="button" class="o-btn is-sm t-clear" aria-label="Back to the game's value: ${esc(r.name)}">${icon("close")}</button></span>`;
    } else if (s.src === "empty") {
      cls.push("is-empty");
      right = `<span class="t-step"><span class="t-gap"></span><span class="t-fig is-amt is-blank${hover}" role="spinbutton" tabindex="0" aria-label="${esc(r.name)}" aria-valuetext="not tracked">&mdash;</span><span class="t-gap"></span></span>`;
    } else {
      if (s.src === "over") gut = `<span class="t-gut is-typed" title="Typed by you">${icon("pencil") || ""}</span>`;
      if (s.full) {
        cls.push("is-waste");
        gut = `<span class="t-gut is-warn">${icon("alert")}</span>`;
      }
      const w = s.above ? "" : s.full ? (r.every ? "Full, being wasted" : `Full, spend by ${stamp(s.next)}`) : `Full in ${span(s.at - v.now)}`;
      when = w ? `<span class="t-when${s.full ? " is-warn" : ""}">${w}</span>` : "";
      const inner = `${s.n}<small${r.open === "deus" ? ' class="mk-open"' : ""}>/${s.cap}</small>`;
      const aria = `${r.name}, ${s.n} of ${s.cap}${s.x ? `, ${s.x} extra` : ""}, from the game. Type over it`;
      right = `<span class="g-step"><span class="t-gap"></span><span class="t-fig g-fig${hover}" role="button" tabindex="0" aria-label="${esc(aria)}">${inner}</span><span class="g-extra">${s.x ? `+${s.x}` : ""}</span></span>`;
    }
    const tip = v.tip && v.tip.id === r.id ? `<div class="t-tip g-tip" role="tooltip" style="top:24px;right:${v.tip.right || 4}px">${v.tip.text}</div>` : "";
    const open = r.open === "conquest" && !sub ? " mk-open" : "";
    return `<div class="${cls.filter(Boolean).join(" ")}${open}">${gut}${nameHtml(r)}${when}${right}${tip}</div>`;
  }

  function conquestRow(v, it) {
    const vals = it.items.map((r) => stateOf(r, v)).filter((s) => s.src !== "empty");
    const full = it.items.filter((r) => stateOf(r, v).full).length;
    const lo = vals.length ? Math.min(...vals.map((s) => s.n)) : null;
    const hi = vals.length ? Math.max(...vals.map((s) => s.n)) : null;
    const open = v.openConquest;
    const sum = full
      ? `<span class="t-sum" style="color:var(--o-warn);font-weight:600">${full} full</span>`
      : `<span class="t-sum">${lo == null ? "6 dungeons" : `${lo === hi ? lo : `${lo}–${hi}`} of 14 each`}</span>`;
    const head = `<div class="t-row is-group mk-open"><span class="t-main" role="button" tabindex="0" aria-expanded="${!!open}"><span class="t-gut">${icon(open ? "chevronDown" : "chevronRight")}</span>${nameHtml(it)}${sum}</span></div>`;
    return open ? head + it.items.map((r) => resRow(v, r, true)).join("") : head;
  }

  function sourceLine(v) {
    const dot = '<span class="faint">·</span>';
    let text;
    if (v.src === "live") text = `From the game, live ${dot} logged in at <b>${clock(LOGIN)}</b>`;
    else if (v.src === "stale") text = `From the game at login, <b>Sun 22:40</b> ${dot} refills since estimated`;
    else text = `No game values yet. Log in with <b>${esc(v.who.name)}</b> while Kisk runs.`;
    return `<div class="g-src">${icon("gamepad")}<span>${text}</span></div>`;
  }

  function resources(ctx, o = {}) {
    const v = view({ who: CHARS.mirelle, playing: true, ...o });
    const full = fullAll(v).length;
    const chip = full ? `<span class="o-chip is-warn">${icon("alert")}${full} full</span>` : "";
    const head = `<div class="o-head"><span class="o-title">${icon("package")}Resources</span>${chip}<span class="grow"></span><button type="button" class="o-btn" aria-label="Close">${icon("close")}</button></div>`;
    const nm = esc(ctx.opts.mask ? ctx.fmt.mask(v.who.name) : v.who.name);
    const who = `<div class="t-char"><span class="o-input" role="combobox" tabindex="0" aria-expanded="false" aria-label="Character"><img src="${ctx.classIcon(v.who.cls)}" alt=""><span class="nm">${nm}</span><span class="meta">${v.who.klass} · ${v.who.server}</span>${v.playing ? '<span class="t-live">Playing</span>' : '<span class="grow" style="flex:1"></span>'}${icon("chevronDown")}</span></div>`;
    const rows = RES.map((r) => (r.kind === "group" ? conquestRow(v, r) : resRow(v, r))).join("");
    const foot = `<div class="o-foot t-foot">${icon("clock")}<span>Daily reset <b>${clock(RESET)}</b></span><span class="grow"></span><span class="faint">${v.member ? "Membership caps" : "Caps without membership"}</span></div>`;
    return `<div class="ov o td rs" role="dialog" aria-label="Resources">${head}${who}${sourceLine(v)}<div class="t-list">${rows}</div>${foot}</div>`;
  }

  // ---- The approved meter (copied from ../approved/meter.html, trimmed) -----
  const SESSION_SECS = 1122;
  const SESSION_TARGETS = 37;
  const SESSION = [
    { name: "Vaelith", cls: "sorcerer", cp: 41820, dmg: 17.84e6, dps: 41240 },
    { name: "Mirelle", cls: "ranger", cp: 38904, dmg: 14.62e6, dps: 38420, you: true },
    { name: "Korrinth", cls: "gladiator", cp: 40112, dmg: 11.05e6, dps: 29610 },
    { name: "Ilyrae", cls: "chanter", cp: 34210, dmg: 4.31e6, dps: 12870 },
    { name: "Brannoc", cls: "templar", cp: 37450, dmg: 2.18e6, dps: 24730 },
  ];
  const SESSION_TOTAL = SESSION.reduce((s, p) => s + p.dmg, 0);
  const MOB = "Thornback Boar";

  const dbtn = (name, label, extra = "") => `<span class="d-btn${extra}" role="button" tabindex="0" aria-label="${label}">${icon(name)}</span>`;
  const nameOf = (ctx, n) => ctx.fmt.esc(ctx.opts.mask ? ctx.fmt.mask(n) : n);

  function gauge(hp) {
    let segs = "";
    for (let i = 0; i < 10; i++) {
      const fill = Math.max(0, Math.min(1, hp * 10 - i));
      segs += `<span class="d-seg"><i style="width:${(fill * 100).toFixed(0)}%"></i></span>`;
    }
    return `<div class="d-gauge">${segs}<span class="d-hpv">${(hp * 100).toFixed(0)}%</span></div>`;
  }

  function bar(scale, rgb, inner) {
    return `<span class="d-track" style="--rgb:${rgb}"><span class="d-fill" style="width:${(scale * 100).toFixed(1)}%"></span>${inner}</span>`;
  }

  function drow(ctx, p, figs) {
    const meta = ctx.opts.cp && p.cp ? `<small>${ctx.fmt.int(p.cp)}</small>` : "";
    return `<div class="d-row${p.you ? " is-you" : ""}"><span class="d-rk">${p.rank}</span>${bar(p.scale, p.rgb, `<img class="d-icon" src="${p.icon}" alt="${p.cls}"><span class="d-name">${p.label}${meta}</span>${figs}`)}<span class="d-pc">${ctx.fmt.pct(p.pct)}</span></div>`;
  }

  function sessionRows(ctx) {
    const { fmt } = ctx;
    const top = SESSION[0].dmg;
    return `<div class="d-rows">${SESSION.map((p, i) =>
      drow(
        ctx,
        { ...p, rank: i + 1, label: nameOf(ctx, p.name), rgb: ctx.CLASS_RGB[p.cls], icon: ctx.classIcon(p.cls), scale: Math.max(0.06, p.dmg / top), pct: p.dmg / SESSION_TOTAL },
        `<span class="d-dmg">${fmt.compact(p.dps)}<span class="d-unit">/s</span></span><span class="d-dps">${fmt.compact(p.dmg)}</span>`
      )
    ).join("")}</div>`;
  }

  function targetRows(ctx) {
    const { fmt } = ctx;
    return `<div class="d-rows">${ctx.players.slice(0, 5).map((p) => drow(ctx, p, `<span class="d-dps">${fmt.compact(p.dps)}</span>`)).join("")}</div>`;
  }

  function dhead(name, session, right, ptr, t = {}) {
    if (ptr) {
      const more = dbtn("more", "Boss setting", t.menu ? " is-hot" : "");
      const close = dbtn("close", "Close the meter", " is-close" + (t.closeHot ? " is-hot" : ""));
      right = `<span class="d-tools">${t.reset === false ? "" : dbtn("reset", "Reset")}${dbtn("capsule", "Capsule")}${dbtn("lockOpen", "Lock (click-through)")}${t.copy === false ? "" : dbtn("copy", "Copy report")}${more}${close}</span>`;
    }
    return `<div class="d-head"><span class="d-tgt${session ? " is-session" : ""}">${name}</span>${right}</div>`;
  }

  function seg(view) {
    const opt = (id, label) => `<span class="seg-opt${view === id ? " is-on" : ""}" role="radio" aria-checked="${view === id}">${label}</span>`;
    return `<span class="seg" role="radiogroup" aria-label="Board, automatic">${opt("session", "Session")}${opt("target", "Target")}</span>`;
  }

  const dfoot = (ctx, left, view) => `<div class="d-foot">${left}<span class="d-sys d-ping is-good">${icon("signal")}${ctx.sys.pingMs} ms</span>${seg(view)}</div>`;
  const damage = (ctx, dmg) => `<span class="d-label">Damage</span><span class="d-val">${ctx.fmt.compact(dmg)}</span>`;

  // o: { board: "session" | "target" | "waiting", ptr, menu: { kind, choice, hot }, closeTip, capsule }
  function meter(ctx, o = {}) {
    const board = o.board || "session";
    const t = { menu: !!o.menu, closeHot: !!o.closeTip };
    let card;
    if (board === "waiting") {
      const h = dhead("DPS Meter", true, `<span class="d-label">Standing by</span>`, o.ptr, { ...t, reset: false, copy: false });
      card = `<div class="ov d">${h}<div class="d-idle"><strong>Waiting for the first hit</strong><span>Alt+CapsLock locks and unlocks the meter</span></div><div class="d-foot is-sys"><span class="grow"></span><span class="d-sys is-good">${icon("signal")}${ctx.sys.pingMs} ms</span></div></div>`;
    } else if (board === "target") {
      const h = dhead(ctx.boss.label, false, `<span class="d-time">${ctx.fmt.time(ctx.fightSecs)}</span>`, o.ptr, t);
      card = `<div class="ov d">${h}${gauge(ctx.boss.hp)}${targetRows(ctx)}${dfoot(ctx, damage(ctx, ctx.players.slice(0, 5).reduce((s, p) => s + p.dmg, 0)), "target")}</div>`;
    } else {
      const h = dhead(nameOf(ctx, MOB), true, `<span class="d-time">${ctx.fmt.time(SESSION_SECS)}</span>`, o.ptr, t);
      const targets = `<span class="d-sys d-count" aria-label="${SESSION_TARGETS} targets fought">${icon("target")}${SESSION_TARGETS}</span>`;
      card = `<div class="ov d">${h}${sessionRows(ctx)}${dfoot(ctx, damage(ctx, SESSION_TOTAL) + targets, "session")}</div>`;
    }
    const menu = o.menu ? meterMenu(ctx, o.menu, board) : "";
    const tip = o.closeTip ? `<div class="hb-mtip" role="tooltip">Close the meter<span>Fights are tracked only while it is open.</span></div>` : "";
    return `<div class="hb-mwrap">${card}${menu}${tip}</div>`;
  }

  // The "…" menu. New: only the boss setting. Today (built): the four links,
  // a divider, then the boss setting (crates/app/src/meter/view.rs).
  function meterMenu(ctx, m, board) {
    const target = board === "waiting" ? null : board === "target" ? ctx.boss.label : nameOf(ctx, MOB);
    const choice = m.choice || "auto";
    const item = (id, text) => {
      const on = target && choice === id;
      const cls = ["o-menu-item", on && "is-on", !target && "is-off", m.hot === id && "is-hot"].filter(Boolean).join(" ");
      return `<div class="${cls}" role="menuitemradio" aria-checked="${!!on}"${target ? "" : ' aria-disabled="true"'}><span class="lead">${on ? icon("check") : ""}</span><span class="tx">${text}</span></div>`;
    };
    const link = (text, trail) => `<div class="o-menu-item" role="menuitem"><span class="lead"></span><span class="tx">${text}</span>${trail ? `<small>${trail}</small>` : ""}</div>`;
    const links = m.today ? link("Fight history") + link("PvP meter") + link("To-dos", "1 full") + link("Settings") + `<div class="o-menu-sep"></div>` : "";
    const hint = target ? "" : `<div class="hb-hint">Hit a mob to set whether it is a boss</div>`;
    const boss = target ? `Treat ${target} as a boss` : "Treat as a boss";
    return `<div class="o-menu hb-mmenu" role="menu" aria-label="Boss setting">${links}${hint}${item("boss", boss)}${item("not", "Not a boss")}${item("auto", "Automatic")}</div>`;
  }

  // ---- First-open prompt ------------------------------------------------------
  const PROMPT = {
    en: {
      title: "Open the meter at launch?",
      body: "Kisk starts with the toolbar alone. Open the DPS meter each time Kisk starts?",
      path: ["Settings", "Overlay", "Open the meter at launch"],
      yes: "Yes, at launch",
      no: "Not now",
      done: "The meter opens at launch from now on.",
    },
    fr: {
      title: "Ouvrir le compteur au lancement ?",
      body: "Kisk démarre avec la barre d'outils seule. Ouvrir le compteur DPS à chaque lancement de Kisk ?",
      path: ["Paramètres", "Overlay", "Ouvrir le compteur au lancement"],
      yes: "Oui, au lancement",
      no: "Pas maintenant",
      done: "Le compteur s'ouvre désormais au lancement.",
    },
  };

  // o: { lang, hover: "yes" | "no", done, narrow }
  function prompt(ctx, o = {}) {
    const P = PROMPT[o.lang || "en"];
    const steps = P.path.map((s, i) => (i === P.path.length - 1 ? `<b>${s}</b>` : `<span>${s}</span>`)).join(icon("chevronRight", "sep"));
    const path = `<div class="hp-path">${icon("settings")}${steps}</div>`;
    const body = o.done
      ? `<div class="o-note is-good" role="status">${icon("check")}<span>${P.done}</span></div>${path}`
      : `<div class="hp-body">${P.body}</div>${path}<div class="hp-acts"><button type="button" class="o-tbtn is-quiet${o.hover === "no" ? " is-hover" : ""}">${P.no}</button><button type="button" class="o-tbtn${o.hover === "yes" ? " is-hover" : ""}">${P.yes}</button></div>`;
    return `<div class="ov o hp${o.narrow ? " is-narrow" : ""}" role="dialog" aria-label="${esc(P.title)}"><div class="hp-head">${icon("swords")}<span class="hp-title">${P.title}</span></div>${body}</div>`;
  }

  // ---- Reminder card (approved, ../approved/reminders.html) -----------------
  function reminder() {
    return `<div class="ov rm" role="status"><div class="rm-head">${icon("bell")}<span class="rm-when">In 1 min</span><span class="rm-dot">·</span><span class="rm-at">22:00</span></div><div class="rm-ev"><div class="rm-name">Shugo Festival</div><div class="rm-detail">3 min to join</div></div><div class="rm-left"><i style="width:62%"></i></div></div>`;
  }

  // ---- Open-data selectors (as in ../todos-autofill/index.html) -------------
  function openControls() {
    const sel = (key, lbl, opts) =>
      `<label><span class="mk-legend">${lbl}</span> <select data-open="${key}">${opts.map(([val, text]) => `<option value="${val}"${String(OPEN[key]) === val ? " selected" : ""}>${text}</option>`).join("")}</select></label>`;
    return `<div class="controls" role="region" aria-label="Open data questions"><div class="controls__inner"><span>Open data (not decided; dashed):</span>${sel("deus", "Deus / Arkanis cap", [
      ["4", "4, the ticket table"],
      ["14", "14, our list today"],
    ])}${sel("conquest", "A Conquest entry spends", [
      ["dungeon", "its dungeon's ticket (six rows)"],
      ["pool", "a shared pool (one row)"],
    ])}</div></div>`;
  }

  function mountOpenControls() {
    const main = document.querySelector("main.scenes");
    if (!main || params.has("embed")) return;
    main.insertAdjacentHTML("beforebegin", openControls());
    document.querySelectorAll("[data-open]").forEach((el) =>
      el.addEventListener("change", () => {
        const p = new URLSearchParams(location.search);
        p.set(el.dataset.open, el.value);
        location.search = p.toString();
      })
    );
  }

  window.Hub = { icon, esc, tag, toolbar, resources, meter, prompt, reminder, view, snapshot, CALM, OPEN, fullOthers, mountOpenControls };
})();
