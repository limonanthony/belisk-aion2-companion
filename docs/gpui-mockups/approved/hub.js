// The hub, approved 2026-10-08: drawing for the toolbar, its dropdowns, the
// Resources window, the first-open prompt and the reminder card under the
// toolbar, shared by toolbar.html, resources.html, prompt.html, whatsnew.html
// and the "Opening it" state of todos.html. The toolbar's update slot was
// approved on 2026-10-09 with the updates round (../updates/README.md); the
// Pets tool (paw-print, after To-dos) on 2026-10-10 with the pets round
// (../pets/README.md).
// Styles: hub.css; the Resources window's rows use todos.css; the meter drawn
// under the toolbar uses meter.css. The app's name comes from app-name.js
// (window.APP_NAME, the Fluent term -app-name), loaded before this file on
// every page that uses it.
//
// Everything here is invented: values, times, characters. Caps and refills
// are the game's ticket table. Nothing is saved or sent
// anywhere. No rows for Conquest dungeons, Deus Research Base or Shattered
// Arkanis: on the Global servers only Odyle Energy limits them. No rows for
// Abandoned Balaur Fortress or Fafnium Laboratory: the Daily Dungeon menu
// lists only the Unknown Fissure. Both settled in game, 2026-10-08.
//
// Icons: Lucide (https://lucide.dev), ISC License; see kit.js. `download` and
// `rotate-cw` are copied from gpui-kit-assets 0.6.6.
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
    grip: '<circle cx="9" cy="12" r="1"/><circle cx="9" cy="5" r="1"/><circle cx="9" cy="19" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="5" r="1"/><circle cx="15" cy="19" r="1"/>',
    key: '<path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"/><circle cx="16.5" cy="7.5" r=".5" fill="currentColor"/>',
    chevronsLeft: '<path d="m11 17-5-5 5-5"/><path d="m18 17-5-5 5-5"/>',
    chevronsRight: '<path d="m6 17 5-5-5-5"/><path d="m13 17 5-5-5-5"/>',
    package:
      '<path d="M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z"/><path d="M12 22V12"/><path d="m3.3 7 7.703 4.734a2 2 0 0 0 1.994 0L20.7 7"/><path d="m7.5 4.27 9 5.15"/>',
    pencil:
      '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>',
    // The DPS meter's icon (the PvP meter takes Lucide crosshair, Kit "pvp").
    swords:
      '<polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" x2="19" y1="19" y2="13"/><line x1="16" x2="20" y1="16" y2="20"/><line x1="19" x2="21" y1="21" y2="19"/><polyline points="14.5 6.5 18 3 21 3 21 6 17.5 9.5"/><line x1="5" x2="9" y1="14" y2="18"/><line x1="7" x2="4" y1="17" y2="20"/><line x1="3" x2="5" y1="19" y2="21"/>',
    // The update slot (2026-10-09): available, downloading; ready.
    download: '<path d="M12 15V3"/><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/>',
    rotateCw: '<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/>',
    // The Pets window's tool (2026-10-10, the pets round): Lucide
    // paw-print, from gpui-kit-assets 0.6.6.
    paw: '<circle cx="11" cy="4" r="2"/><circle cx="18" cy="8" r="2"/><circle cx="20" cy="16" r="2"/><path d="M9 10a5 5 0 0 1 5 5v3.5a3.5 3.5 0 0 1-6.84 1.045Q6.52 17.48 4.46 16.84A3.5 3.5 0 0 1 5.5 10Z"/>',
  };

  function icon(name, cls) {
    if (!EXTRA[name]) return Kit.icon(name, cls);
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${EXTRA[name]}</svg>`;
  }

  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  // ---- Clock (mocked): Monday 21:48, daily reset 09:00, weekly Wednesday ----
  const DAY = 1440;
  const DAYS = { en: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], fr: ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."] };
  const RESET = 9 * 60;
  const WEEKLY = 2 * DAY + RESET; // Wednesday at the reset hour
  const NOW = 21 * 60 + 48;
  const SOON = DAY + 8 * 60 + 12; // Tue 08:12, 48 min before the refills
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

  function nextWeekly(now) {
    let t = Math.floor(now / (7 * DAY)) * 7 * DAY + WEEKLY;
    if (t <= now) t += 7 * DAY;
    return t;
  }

  const refill = (r, now, k) =>
    r.every ? (Math.floor((now - (r.off || 0)) / r.every) + k) * r.every + (r.off || 0) : r.weekly ? nextWeekly(now) + (k - 1) * 7 * DAY : nextAt(now, r.at) + (k - 1) * DAY;

  // A full pool warns only when the refill it loses is near: 1 h before
  // Odyle's 3-hour refill, 3 h before a daily one, 24 h before a weekly one.
  const nearOf = (r) => (r.every ? 60 : r.weekly ? DAY : 180);

  // ---- Resources: only here and on the toolbar since 2026-10-08 ("I think we
  // can remove the recharges from the todos now no ?"); the to-do window has
  // no Recharges section any more. Shugo keys (the Shugo Festival's): 12 (21
  // with membership), +3 a day, extra pool up to 30. Shugo keys, Invasion
  // keys and the Daily Dungeon are shared by the characters of one server.
  // Daily refills follow the ticket table's daily clock (16:00 KST = the
  // 09:00 daily reset here). The Daily Dungeon is ticket 101: 14, +14 at the
  // weekly reset; its menu lists one dungeon, so the row has no dungeon name.
  const RES = [
    { id: "odyle", name: "Odyle Energy", fr: "Odyle Energy", ic: "zap", cap: 560, mcap: 840, add: 15, every: 180, step: 10 },
    { id: "shugo", name: "Shugo keys", fr: "Shugo keys", ic: "key", cap: 12, mcap: 21, add: 3, at: RESET, account: true },
    { id: "invasion", name: "Invasion keys", fr: "Clés de l'Invasion", cap: 7, add: 1, at: RESET, account: true },
    { id: "nightmare", name: "Nightmare", cap: 14, add: 2, at: RESET },
    { id: "dungeon", name: "Daily Dungeon", fr: "Donjon quotidien", cap: 14, add: 14, weekly: true, account: true },
  ];

  const find = (id) => RES.find((r) => r.id === id);
  const capOf = (r, member) => (member && r.mcap ? r.mcap : r.cap);

  // What the game sent (n = regular pool, x = extra pool). Mirelle, live
  // ("From the game, live"). `null` leaves a row without a game value.
  function snapshot(o = {}) {
    const g = {
      odyle: { n: 245, x: 380 },
      shugo: { n: 7, x: 3 },
      invasion: { n: 2, x: 0 },
      nightmare: { n: 0, x: 3 },
      dungeon: { n: 9, x: 7 },
    };
    for (const [id, val] of Object.entries(o)) g[id] = val === null ? null : { ...g[id], ...val };
    return g;
  }

  // A resource's state: { src: "game" | "over" | "typed" | "empty", n, x, cap, full, warn, above, at, next };
  // warn: full and the refill it loses is near. Rows hidden earlier in the
  // to-do window have no hidden state here: every row shows and counts.
  function stateOf(r, v) {
    const cap = capOf(r, v.member);
    const g = v.g[r.id];
    if (v.over && v.over[r.id] != null) {
      const n = v.over[r.id];
      return withEstimate(r, { src: "over", n, x: g ? g.x || 0 : 0, cap }, v);
    }
    // A regular pool above the cap in force can only come from a membership
    // the switch does not know: shown as is, never full, no estimate.
    if (g && g.n > cap) return { src: "game", n: g.n, x: g.x || 0, cap, above: true };
    if (g) return withEstimate(r, { src: "game", n: g.n, x: g.x || 0, cap }, v);
    // Before the game fills a row, a typed number is your total: at or above
    // the cap it counts as full.
    if (v.amt && v.amt[r.id] != null) return withEstimate(r, { src: "typed", n: v.amt[r.id], x: 0, cap }, v);
    return { src: "empty", cap };
  }

  function withEstimate(r, s, v) {
    const now = v.now == null ? NOW : v.now;
    if (s.n >= s.cap) {
      const next = refill(r, now, 1);
      return { ...s, full: true, warn: next - now <= nearOf(r), next };
    }
    return { ...s, full: false, at: refill(r, now, Math.ceil((s.cap - s.n) / r.add)) };
  }

  const fullOthers = (v) => RES.filter((r) => r.id !== "odyle" && r.id !== "shugo" && stateOf(r, v).warn);
  const fullAll = (v) => RES.filter((r) => stateOf(r, v).warn);

  function view(o = {}) {
    return { member: true, g: snapshot(), now: NOW, src: "live", lang: "en", ...o };
  }

  // ---- Strings (English; French drawn to check the room, through i18n) -----
  // `app` is the product name (window.APP_NAME, Fluent `-app-name`), as the
  // build's `toolbar-quit = Quit { -app-name }`.
  const T = {
    en: {
      meter: ["DPS meter", "Fights are tracked while it is open."],
      meterOpen: ["DPS meter", "Click to close. Fights stop being tracked."],
      meterAside: ["DPS meter", "Hidden between fights. Click to show it for 10 s."],
      pvp: "PvP meter",
      todos: "To-dos",
      pets: "Pets",
      history: "Fight history",
      reminders: "Reminders",
      remindersOff: ["Reminders", "Paused"],
      settings: "Settings",
      fold: "Fold the toolbar",
      unfold: "Unfold the toolbar",
      quit: (app) => [`Quit ${app}`, "Closes every tool."],
      fullLabel: "Full, a refill is coming",
      refills: "refills",
      all: "All resources",
      of: "of",
      extra: "extra",
      fullIn: "full in",
      fullSpend: "Full, spend by",
      fullWaste: "Full, being wasted",
      fullRefills: "Full · refills",
      noValues: "No value yet",
      noValuesLine: (app) => `Log in while ${app} runs.`,
      click: "Click for all resources.",
      moreFull: (n) => `${n} more full`,
      // The update slot (2026-10-09): the first line is the button's name.
      upAvailable: (v) => [`Update available: ${v}`, "Click to download it. It installs only when you restart."],
      upFailed: (v) => [`Update available: ${v}`, "The last download failed its check and was deleted. Click to try again."],
      upDownloading: (v, p) => [`Downloading ${v} · ${p}`, "Speed-capped, paused while you fight. Click for details."],
      upPaused: (v, p) => [`Download paused · ${p}`, "Resumes when the fight ends. Click for details."],
      upVerifying: (v) => [`Checking ${v}…`, "Making sure the download is complete and genuine."],
      upReady: (v) => [`Restart to update to ${v}`, "Closes and opens again in the new version. Nothing is lost."],
      upReadyFight: (v) => [`Restart to update to ${v}`, "You are fighting: restarting now splits this fight in History."],
    },
    fr: {
      meter: ["Compteur DPS", "Les combats sont suivis tant qu'il est ouvert."],
      meterOpen: ["Compteur DPS", "Cliquer pour le fermer. Les combats ne sont plus suivis."],
      meterAside: ["Compteur DPS", "Masqué entre les combats. Cliquer pour l'afficher 10 s."],
      pvp: "Compteur JcJ",
      todos: "Tâches",
      pets: "Familiers",
      history: "Historique des combats",
      reminders: "Rappels",
      remindersOff: ["Rappels", "En pause"],
      settings: "Paramètres",
      fold: "Replier la barre",
      unfold: "Déplier la barre",
      quit: (app) => [`Quitter ${app}`, "Ferme tous les outils."],
      fullLabel: "Pleines, une recharge arrive",
      refills: "recharge",
      all: "Toutes les ressources",
      of: "sur",
      extra: "en plus",
      fullIn: "max en",
      fullSpend: "À vider avant",
      fullWaste: "Plein, gaspillé",
      fullRefills: "Plein · rech.",
      noValues: "Pas encore de valeur",
      noValuesLine: (app) => `Connectez-vous pendant que ${app} tourne.`,
      click: "Cliquer pour toutes les ressources.",
      moreFull: (n) => `${n} autres pleines`,
      upAvailable: (v) => [`Mise à jour disponible : ${v}`, "Cliquer pour la télécharger. Elle s'installe seulement au redémarrage."],
      upFailed: (v) => [`Mise à jour disponible : ${v}`, "Le dernier téléchargement n'a pas passé la vérification. Cliquer pour réessayer."],
      upDownloading: (v, p) => [`Téléchargement de ${v} · ${p}`, "Débit limité, en pause pendant les combats. Cliquer pour les détails."],
      upPaused: (v, p) => [`Téléchargement en pause · ${p}`, "Reprend à la fin du combat. Cliquer pour les détails."],
      upVerifying: (v) => [`Vérification de ${v}…`, "On vérifie que le fichier est complet et authentique."],
      upReady: (v) => [`Redémarrer pour passer à ${v}`, "Ferme et rouvre dans la nouvelle version. Rien n'est perdu."],
      upReadyFight: (v) => [`Redémarrer pour passer à ${v}`, "Combat en cours : redémarrer maintenant le coupe en deux dans l'historique."],
    },
  };

  // ---- Toolbar ---------------------------------------------------------------
  // o: { lit: {meter, pvp, todos, history, reminders, settings, resources},
  //      aside (the open meter steps aside between fights), hover: key,
  //      tip: key, warnOpen, dropHot, bellOpen, bellOff, folded, lang,
  //      v (resource view), focus: key, up (dropdown opens up), live,
  //      update: { state, version, pct, fight } (the update slot; none: no
  //      slot) }
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
    const c = ["hb-count", s.warn && "is-full", o.lit && o.lit.resources && "is-lit", o.hover === r.id && "is-hover", o.focus === r.id && "is-focus"].filter(Boolean).join(" ");
    let fig;
    let tip;
    if (s.src === "empty") {
      fig = `<span class="hb-fig is-blank">&mdash;</span>`;
      tip = [name, `${L.noValues}. ${L.noValuesLine(window.APP_NAME)}`];
    } else {
      fig = `<span class="hb-fig">${s.n}<small>/${s.cap}</small></span>${s.x ? `<span class="hb-x">+${s.x}</span>` : ""}`;
      const lang = o.lang || "en";
      const when = s.above ? "" : s.warn ? (r.every ? L.fullWaste : `${L.fullSpend} ${stamp(s.next, lang)}`) : s.full ? `${L.fullRefills} ${stamp(s.next, lang)}` : `${L.fullIn} ${span(s.at - v.now, lang)}`;
      const line = `${s.n} ${L.of} ${s.cap}${s.x ? `, +${s.x} ${L.extra}` : ""}${when ? ` · ${when}` : ""}`;
      tip = [name, `${line}. ${L.click}`];
    }
    const aria = s.src === "empty" ? `${name}, ${L.noValues}` : `${name}, ${s.n} of ${s.cap}${s.x ? `, ${s.x} extra` : ""}${s.full ? ", full" : ""}. Open resources`;
    const shown = o.tip === r.id ? " is-on" : "";
    return `<button type="button" class="${c}" data-hb="res" aria-label="${esc(aria)}">${icon(r.ic)}${fig}${tipHtml(tip, shown)}</button>`;
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

  // The bell's dropdown: the next three reminders switched on, "Pause
  // reminders" (the Settings master switch) and "Reminder settings".
  function bellDrop() {
    const items = [
      ["Shugo Festival", "22:00"],
      ["Dimensional Invasion", "22:30"],
      ["Rift portals", "23:00"],
    ]
      .map(([n, t]) => `<div class="o-menu-item" role="menuitem"><span class="nm">${n}</span><span class="when"><b>${t}</b></span></div>`)
      .join("");
    return `<div class="o-menu hb-drop is-left" role="menu" style="width:236px"><div class="hb-drop-label">${icon("bell")}Next reminders</div>${items}<div class="o-menu-sep"></div><div class="o-menu-item" role="menuitemcheckbox" aria-checked="false"><span class="lead"></span><span class="nm">Pause reminders</span></div><div class="o-menu-item" role="menuitem"><span class="lead">${icon("settings")}</span><span class="nm">Reminder settings</span>${icon("chevronRight")}</div></div>`;
  }

  // ---- The update slot (approved 2026-10-09) -----------------------------------
  // Drawn only while there is something to do or know: first in the end
  // group, before fold (after the warning when folded). A 26 px tool:
  // available (and after a failed download) Lucide download on soft in text;
  // downloading, paused, checking the download: the icon in muted with a
  // 2 px progress line; ready: Lucide rotate-cw in good on chip-good.
  // o.update: { state: "available" | "failed" | "downloading" | "paused" |
  //   "verifying" | "ready", version, pct, fight }; hover, tip, focus: "update".
  function updateBtn(o) {
    const u = o.update;
    const L = T[o.lang || "en"];
    const p = o.lang === "fr" ? `${u.pct || 0}\u202f%` : `${u.pct || 0}%`;
    const busy = u.state === "downloading" || u.state === "paused" || u.state === "verifying";
    const tip = {
      available: () => L.upAvailable(u.version),
      failed: () => L.upFailed(u.version),
      downloading: () => L.upDownloading(u.version, p),
      paused: () => L.upPaused(u.version, p),
      verifying: () => L.upVerifying(u.version),
      ready: () => (u.fight ? L.upReadyFight(u.version) : L.upReady(u.version)),
    }[u.state]();
    const c = [
      "hb-btn",
      "hb-up",
      busy && "is-busy",
      u.state === "paused" && "is-paused",
      u.state === "ready" && "is-ready",
      o.hover === "update" && "is-hover",
      o.focus === "update" && "is-focus",
    ]
      .filter(Boolean)
      .join(" ");
    const fill = u.state === "verifying" ? 100 : u.pct || 0;
    const prog = busy ? `<span class="hb-prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${fill}"><i style="width:${fill}%"></i></span>` : "";
    return `<button type="button" class="${c}" data-hb="update" data-up="${u.state}" aria-label="${esc(tip[0])}">${icon(u.state === "ready" ? "rotateCw" : "download")}${prog}${tipHtml(tip, o.tip === "update" ? " is-on" : "")}</button>`;
  }

  function toolbar(ctx, o = {}) {
    const L = T[o.lang || "en"];
    const app = window.APP_NAME;
    const slot = o.update ? updateBtn(o) : "";
    o = { v: view(), ...o };
    const grip = `<span class="hb-grip" title="Drag to move">${icon("grip")}</span>`;
    if (o.folded) {
      const n = fullAll(o.v).length;
      const warn = n ? `<button type="button" class="hb-warn" data-hb="unfold" aria-label="${n} resources full. Unfold the toolbar">${icon("alert")}${n}</button>` : "";
      return `<div class="ov o hb is-folded${o.live ? " is-live" : ""}" role="toolbar" aria-label="${esc(app)}">${grip}${toolBtn(o, "unfold", "chevronsRight", L.unfold, { cls: "is-end" })}${warn}${slot}</div>`;
    }
    const meterTip = o.lit && o.lit.meter ? (o.aside ? L.meterAside : L.meterOpen) : L.meter;
    const bellIc = o.bellOff ? "bellOff" : "bell";
    const tools =
      toolBtn(o, "meter", "swords", meterTip) +
      toolBtn(o, "pvp", "pvp", L.pvp) +
      toolBtn(o, "todos", "listChecks", L.todos) +
      toolBtn(o, "pets", "paw", L.pets) +
      toolBtn(o, "history", "crown", L.history) +
      `<span style="position:relative;display:inline-flex">${toolBtn(o, "reminders", bellIc, o.bellOff ? L.remindersOff : L.reminders, { cls: o.bellOff ? "is-off" : "" })}${o.bellOpen ? bellDrop(o) : ""}</span>` +
      toolBtn(o, "settings", "settings", L.settings);
    const others = fullOthers(o.v).length;
    const warn = others
      ? `<span style="position:relative;display:inline-flex"><button type="button" class="hb-warn${o.warnOpen ? " is-open" : ""}${o.hover === "warn" ? " is-hover" : ""}" data-hb="warn" aria-haspopup="menu" aria-expanded="${!!o.warnOpen}" aria-label="${others} more resources full">${icon("alert")}${others}${o.tip === "warn" ? tipHtml(L.moreFull(others), " is-on") : ""}</button>${o.warnOpen ? warnDrop(o) : ""}</span>`
      : "";
    const counts = `<span class="hb-counts">${countBtn(o, find("odyle"))}${countBtn(o, find("shugo"))}${warn}</span>`;
    const end = slot + toolBtn(o, "fold", "chevronsLeft", L.fold, { cls: "is-end" }) + toolBtn(o, "quit", "close", L.quit(app), { cls: "is-end" });
    return `<div class="ov o hb${o.live ? " is-live" : ""}" role="toolbar" aria-label="${esc(app)}">${grip}<span class="hb-tools">${tools}</span><span class="hb-vr" aria-hidden="true"></span>${counts}<span class="hb-vr" aria-hidden="true"></span>${end}</div>`;
  }

  // ---- Resources window ------------------------------------------------------
  // The to-do window's classes (todos.css): header, character selector,
  // source line, rows, footer. These rows are drawn here only.
  const CHARS = {
    mirelle: { name: "Mirelle", cls: "ranger", klass: "Ranger", server: "Ossia" },
    tavrin: { name: "Tavrin", cls: "templar", klass: "Templar", server: "Ossia" },
    sefa: { name: "Sefa", cls: "cleric", klass: "Cleric", server: "Lyss" },
  };

  // "Account" is its Lucide users icon only; the word shows under the pointer.
  function nameHtml(v, r) {
    const acct = r.account ? `<span class="t-acct${v.acctHover === r.id ? " is-hover" : ""}" role="img" aria-label="Account" title="Account">${icon("users")}</span>` : "";
    return `<span class="t-name">${esc(r.name)}${acct}</span>`;
  }

  // An amount you count yourself before the game fills it: one less, the
  // figure (it reads as a field: a click types), one more; no upper stop.
  function stepper(r, s, mark) {
    const step = r.step || 1;
    const less = step > 1 ? `${step} less` : "One less";
    const more = step > 1 ? `${step} more` : "One more";
    const dn = `<button type="button" class="o-btn is-sm t-arrow" aria-label="${less}: ${esc(r.name)}"${s.n <= 0 ? " disabled" : ""}>${icon("chevronDown")}</button>`;
    const up = `<button type="button" class="o-btn is-sm t-arrow" aria-label="${more}: ${esc(r.name)}">${icon("chevronUp")}</button>`;
    const fig = `<span class="t-fig is-amt${mark}" role="spinbutton" tabindex="0" aria-label="${esc(r.name)}" aria-valuemin="0" aria-valuenow="${s.n}" aria-valuetext="${s.n} of ${s.cap}">${s.n}<small>/${s.cap}</small></span>`;
    return `<span class="t-step">${dn}${fig}${up}</span>`;
  }

  function resRow(v, r) {
    const s = stateOf(r, v);
    const cls = ["t-row"];
    let gut = `<span class="t-gut"></span>`;
    let when = "";
    let right;
    const mark = (v.hover === r.id ? " is-hover" : "") + (v.focus === r.id ? " is-focus" : "");
    if (v.edit === r.id) {
      // On a game value the field's × goes back to the game's value; on a
      // value only you have, it stops tracking.
      const back = s.src === "game" || s.src === "over" ? `Back to the game's value: ${r.name}` : `Stop tracking ${r.name}`;
      right = `<span class="t-field"><span class="val">${esc(v.editVal)}</span><span class="o-caret"></span><span class="t-cap">/${s.cap}</span><button type="button" class="o-btn is-sm t-clear" aria-label="${esc(back)}">${icon("close")}</button></span>`;
    } else if (s.src === "empty") {
      cls.push("is-empty");
      right = `<span class="t-step"><span class="t-gap"></span><span class="t-fig is-amt is-blank${mark}" role="spinbutton" tabindex="0" aria-label="${esc(r.name)}" aria-valuetext="not tracked">&mdash;</span><span class="t-gap"></span></span>`;
    } else {
      if (s.src === "over") gut = `<span class="t-gut is-typed" title="Typed by you">${icon("pencil")}</span>`;
      if (s.warn) {
        cls.push("is-waste");
        gut = `<span class="t-gut is-warn">${icon("alert")}</span>`;
      }
      // Full far from the refill: a quiet "Full · refills …", no warning.
      const w = s.above ? "" : s.warn ? (r.every ? "Full, being wasted" : `Full, spend by ${stamp(s.next)}`) : s.full ? `Full · refills ${stamp(s.next)}` : `Full in ${span(s.at - v.now)}`;
      when = w ? `<span class="t-when${s.warn ? " is-warn" : ""}">${w}</span>` : "";
      if (s.src === "typed") right = stepper(r, s, mark);
      else {
        const inner = `${s.n}<small>/${s.cap}</small>`;
        const aria = `${r.name}, ${s.n} of ${s.cap}${s.x ? `, ${s.x} extra` : ""}, ${s.src === "over" ? "typed by you" : "from the game"}. Type over it`;
        right = `<span class="g-step"><span class="t-gap"></span><span class="t-fig g-fig${mark}" role="button" tabindex="0" aria-label="${esc(aria)}">${inner}</span><span class="g-extra">${s.x ? `+${s.x}` : ""}</span></span>`;
      }
    }
    const at = v.tip && v.tip.left != null ? `left:${v.tip.left}px` : `right:${(v.tip && v.tip.right) || 4}px`;
    const tip = v.tip && v.tip.id === r.id ? `<div class="t-tip g-tip" role="tooltip" style="top:24px;${at}">${v.tip.text}</div>` : "";
    return `<div class="${cls.filter(Boolean).join(" ")}">${gut}${nameHtml(v, r)}${when}${right}${tip}</div>`;
  }

  function sourceLine(v) {
    const dot = '<span class="faint">·</span>';
    let text;
    if (v.src === "live") text = `From the game, live ${dot} logged in at <b>${clock(v.seen == null ? LOGIN : v.seen)}</b>`;
    else if (v.src === "stale") text = `From the game at login, <b>Sun 22:40</b> ${dot} refills since estimated`;
    else if (v.src === "shared") text = `<b>${esc(v.who.name)}</b> not seen at login yet ${dot} Account rows from ${esc(v.from)}`;
    else text = `No game values yet. Log in with <b>${esc(v.who.name)}</b> while ${window.APP_NAME} runs.`;
    return `<div class="g-src">${icon("gamepad")}<span>${text}</span></div>`;
  }

  // A note under the source line for 4 s ("Filled from the game at login.").
  const noteHtml = (v) => (v.note ? `<div class="o-note is-good t-note" role="status">${icon("check")}<span>${v.note}</span></div>` : "");

  function resources(ctx, o = {}) {
    const v = view({ who: CHARS.mirelle, playing: true, ...o });
    const full = fullAll(v).length;
    const chip = full ? `<span class="o-chip is-warn">${icon("alert")}${full} full</span>` : "";
    const head = `<div class="o-head"><span class="o-title">${icon("package")}Resources</span>${chip}<span class="grow"></span><button type="button" class="o-btn" aria-label="Close">${icon("close")}</button></div>`;
    const nm = esc(ctx.opts.mask ? ctx.fmt.mask(v.who.name) : v.who.name);
    const who = `<div class="t-char"><span class="o-input" role="combobox" tabindex="0" aria-expanded="false" aria-label="Character"><img src="${ctx.classIcon(v.who.cls)}" alt=""><span class="nm">${nm}</span><span class="meta">${v.who.klass} · ${v.who.server}</span>${v.playing ? '<span class="t-live">Playing</span>' : '<span class="grow" style="flex:1"></span>'}${icon("chevronDown")}</span></div>`;
    const rows = RES.map((r) => resRow(v, r)).join("");
    const foot = `<div class="o-foot t-foot">${icon("clock")}<span>Daily reset <b>${clock(RESET)}</b></span><span class="grow"></span><span class="faint">${v.member ? "Membership caps" : "Caps without membership"}</span></div>`;
    return `<div class="ov o td rs" role="dialog" aria-label="Resources">${head}${who}${sourceLine(v)}${noteHtml(v)}<div class="t-list">${rows}</div>${foot}</div>`;
  }

  // ---- The meter, opened from the toolbar --------------------------------------
  // A trimmed copy of meter.html's Session and Waiting boards (meter.css),
  // so the meter can be judged beside the toolbar and the prompt. Its "…"
  // menu and × tooltip are drawn in meter.html.
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

  const nameOf = (ctx, n) => ctx.fmt.esc(ctx.opts.mask ? ctx.fmt.mask(n) : n);

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

  function seg(board) {
    const opt = (id, label) => `<span class="seg-opt${board === id ? " is-on" : ""}" role="radio" aria-checked="${board === id}">${label}</span>`;
    return `<span class="seg" role="radiogroup" aria-label="Board, automatic">${opt("session", "Session")}${opt("target", "Target")}</span>`;
  }

  // o: { board: "session" | "waiting" }
  function meter(ctx, o = {}) {
    let card;
    if (o.board === "waiting") {
      const h = `<div class="d-head"><span class="d-tgt is-session">DPS meter</span><span class="d-label">Standing by</span></div>`;
      card = `<div class="ov d">${h}<div class="d-idle"><strong>Waiting for the first hit</strong><span>Alt+E hides the meter · Alt+Q resets it</span></div><div class="d-foot is-sys"><span class="grow"></span><span class="d-sys is-good">${icon("signal")}${ctx.sys.pingMs} ms</span><span class="d-sys">CPU ${ctx.sys.cpu}%</span><span class="d-sys">${ctx.sys.memGb} GB</span></div></div>`;
    } else {
      const h = `<div class="d-head"><span class="d-tgt is-session">${nameOf(ctx, MOB)}</span><span class="d-time">${ctx.fmt.time(SESSION_SECS)}</span></div>`;
      const targets = `<span class="d-sys d-count" aria-label="${SESSION_TARGETS} targets fought">${icon("target")}${SESSION_TARGETS}</span>`;
      const damage = `<span class="d-label">Damage</span><span class="d-val">${ctx.fmt.compact(SESSION_TOTAL)}</span>`;
      card = `<div class="ov d">${h}${sessionRows(ctx)}<div class="d-foot">${damage}${targets}<span class="d-sys d-ping is-good">${icon("signal")}${ctx.sys.pingMs} ms</span>${seg("session")}</div></div>`;
    }
    return `<div class="hb-mwrap">${card}</div>`;
  }

  // ---- First-open prompt ------------------------------------------------------
  // `body` takes the product name (window.APP_NAME), as the build's
  // `prompt-body`.
  const PROMPT = {
    en: {
      title: "Open the meter at launch?",
      body: (app) => `${app} starts with the toolbar alone. Open the DPS meter each time ${app} starts?`,
      path: ["Settings", "Overlay", "Open the meter at launch"],
      yes: "Yes, at launch",
      no: "Not now",
      done: "The meter opens at launch from now on.",
    },
    fr: {
      title: "Ouvrir le compteur au lancement ?",
      body: (app) => `${app} démarre avec la barre d'outils seule. Ouvrir le compteur DPS à chaque lancement de ${app} ?`,
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
      : `<div class="hp-body">${P.body(window.APP_NAME)}</div>${path}<div class="hp-acts"><button type="button" class="o-tbtn is-quiet${o.hover === "no" ? " is-hover" : ""}">${P.no}</button><button type="button" class="o-tbtn${o.hover === "yes" ? " is-hover" : ""}">${P.yes}</button></div>`;
    return `<div class="ov o hp${o.narrow ? " is-narrow" : ""}" role="dialog" aria-label="${esc(P.title)}"><div class="hp-head">${icon("swords")}<span class="hp-title">${P.title}</span></div>${body}</div>`;
  }

  // ---- Reminder card (reminders.html) -----------------------------------------
  // The timer ring (approved 2026-10-10, ../card-timer/README.md): a 12 px
  // hollow ring left of the ×; `left` is the time left (0..1), the arc
  // emptying clockwise from 12 o'clock. `solo`: no × (locked), so the ring
  // takes the ×'s slot. Shared with the soul tracker card (pets.js).
  function ring(left, solo) {
    const off = (-(1 - Math.max(0, Math.min(1, left))) * 100).toFixed(2);
    return `<span class="rm-timer${solo ? " is-solo" : ""}" role="timer" aria-label="Time left"><svg class="rm-ring" width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><circle class="tr" cx="6" cy="6" r="5"/><circle class="arc" cx="6" cy="6" r="5" pathLength="100" transform="rotate(-90 6 6)" style="stroke-dashoffset:${off}"/></svg></span>`;
  }

  function reminder() {
    const x = `<span class="rm-x" role="button" aria-label="Dismiss">${icon("close")}</span>`;
    return `<div class="ov rm" role="status"><div class="rm-head">${icon("bell")}<span class="rm-when">In 1 min</span><span class="rm-dot">·</span><span class="rm-at">22:00</span>${ring(0.62)}${x}</div><div class="rm-ev"><div class="rm-name">Shugo Festival</div><div class="rm-detail">3 min to join</div></div></div>`;
  }

  window.Hub = { icon, esc, toolbar, resources, meter, prompt, reminder, ring, view, snapshot, SOON };
})();
