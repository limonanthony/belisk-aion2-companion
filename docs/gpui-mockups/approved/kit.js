/* Harness for the approved overlay mockups: invented data for the meter,
 * PvP, detail and history overlays, Lucide icons, the page controls (real
 * overlay settings and a scene filter) and the scene/state grid. Pages may
 * pass `variants` for a future open choice; approved pages pass none.
 *
 * Everything here is fake. Player, boss, dungeon, skill and buff names are
 * invented; nothing comes from a recording. Skill icons are the game's PNGs
 * in crates/app/assets/game/skills/, used only as pictures.
 *
 * Icons: Lucide (https://lucide.dev), ISC License,
 * Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part
 * of Feather (MIT). All other copyright (c) for Lucide are held by Lucide
 * Contributors 2022. */

(function () {
  "use strict";

  // ---- Formatting -----------------------------------------------------------

  const int = (n) => Math.round(n).toLocaleString("en-US");

  function compact(n) {
    if (n < 10000) return int(n);
    if (n < 1e6) return (n / 1e3).toFixed(1) + "K";
    return (n / 1e6).toFixed(2) + "M";
  }

  function pct(n) {
    const p = n * 100;
    return p >= 99.95 ? "100%" : p.toFixed(1) + "%";
  }

  function time(secs) {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  function mask(name) {
    const t = name.trim();
    if (t.length <= 1) return t ? "*" : "";
    if (t.length === 2) return t[0] + "*";
    return t[0] + "*".repeat(t.length - 2) + t[t.length - 1];
  }

  const esc = (s) =>
    String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  // ---- Icons (Lucide, single colour, stroke = currentColor) ----------------

  const ICONS = {
    reset: '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>',
    history:
      '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/>',
    settings:
      '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>',
    capsule: '<path d="m7 20 5-5 5 5"/><path d="m7 4 5 5 5-5"/>',
    lockOpen: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>',
    lock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
    // Lucide crosshair (hub, approved 2026-10-08): the crossed swords are
    // the DPS meter's now, in the toolbar.
    pvp: '<circle cx="12" cy="12" r="10"/><line x1="22" x2="18" y1="12" y2="12"/><line x1="6" x2="2" y1="12" y2="12"/><line x1="12" x2="12" y1="6" y2="2"/><line x1="12" x2="12" y1="22" y2="18"/>',
    close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    copy: '<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>',
    up: '<polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/>',
    down: '<polyline points="22 17 13.5 8.5 8.5 13.5 2 7"/><polyline points="16 17 22 17 22 11"/>',
    award: '<circle cx="12" cy="8" r="6"/><path d="M15.477 12.89 17 22l-5-3-5 3 1.523-9.11"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    hourglass:
      '<path d="M5 22h14"/><path d="M5 2h14"/><path d="M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22"/><path d="M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2"/>',
    signal: '<path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V8"/>',
    more: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
    plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
    chevronDown: '<path d="m6 9 6 6 6-6"/>',
    chevronRight: '<path d="m9 18 6-6-6-6"/>',
    chevronUp: '<path d="m18 15-6-6-6 6"/>',
    trash:
      '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>',
    info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
    eye: '<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/>',
    arrowDown: '<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>',
    external:
      '<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>',
    clock: '<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>',
    alert:
      '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
    target: '<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/>',
    heart:
      '<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>',
    user: '<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
    zap: '<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/>',
    skull:
      '<circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><path d="M8 20v2h8v-2"/><path d="m12.5 17-.5-1-.5 1h1z"/><path d="M16 20a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20"/>',
    fileText:
      '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
    chevronLeft: '<path d="m15 18-6-6 6-6"/>',
    mousePointer:
      '<path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"/>',
    // The boss-record mark in History (fight report, approved 2026-10-07).
    crown:
      '<path d="M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.246a1 1 0 0 1-.956.734H5.81a1 1 0 0 1-.957-.734L2.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z"/><path d="M5 21h14"/>',
  };

  function icon(name, cls) {
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name]}</svg>`;
  }

  // ---- Shared data ----------------------------------------------------------

  // Same colours as today's meter, so class identity carries over.
  const CLASS_RGB = {
    assassin: "168, 85, 247",
    chanter: "52, 211, 153",
    cleric: "251, 191, 36",
    elementalist: "34, 211, 238",
    fighter: "248, 113, 113",
    gladiator: "251, 146, 60",
    ranger: "163, 230, 53",
    sorcerer: "232, 121, 249",
    templar: "56, 189, 248",
  };

  const classIcon = (cls) => `../../../crates/app/assets/game/class/${cls}.png`;
  const skillIcon = (code) => `../../../crates/app/assets/game/skills/${code}.png`;

  const ROSTER = [
    { name: "Vaelith", cls: "sorcerer", server: "Lyss", cp: 41820, dps: 61240 },
    { name: "Mirelle", cls: "ranger", server: "Ossia", cp: 38904, dps: 48210, you: true },
    { name: "Korrinth", cls: "gladiator", server: "Lyss", cp: 40112, dps: 39875 },
    { name: "Saebrin", cls: "assassin", server: "Vandor", cp: 36455, dps: 34602 },
    { name: "Tobiah", cls: "elementalist", server: "Ossia", cp: 35020, dps: 28940 },
    { name: "Quenna", cls: "templar", server: "Lyss", cp: 39670, dps: 22318 },
    { name: "Ilyrae", cls: "chanter", server: "Vandor", cp: 34210, dps: 17455 },
    { name: "Dovrek", cls: "cleric", server: "Ossia", cp: 33880, dps: 12936 },
    { name: "Pellam", cls: "fighter", server: "Vandor", cp: 31275, dps: 9820 },
    { name: "Ysolde", cls: "cleric", server: "Lyss", cp: 30940, dps: 6104 },
  ];

  const BOSS = { name: "Veilwarden Orsikh", where: "Hollow of Ashen Tides", hp: 0.624 };
  const FIGHT_SECS = 221;
  const SUMMARY_SECS = 248;
  const QUIET_SECS = 177;
  const SKILLS_TOP = [
    { name: "Piercing Volley", share: 0.31 },
    { name: "Gale Arrow", share: 0.22 },
    { name: "Hunter's Mark", share: 0.14 },
  ];
  const SYS = { pingMs: 42, cpu: 18, memGb: 11.2 };
  const PB_DPS = 51740;

  // ---- PvP ------------------------------------------------------------------

  const PVP_STATS = [
    { name: "Grimsalt", server: "Vandor", damage: 1840000, kills: 4, assists: 2, deaths: 1 },
    { name: "Tessaly", server: "Lyss", damage: 1520000, kills: 2, assists: 5, deaths: 0 },
    { name: "Kaedric", server: "Ossia", damage: 1310000, kills: 3, assists: 1, deaths: 2 },
    { name: "Mirelle", server: "Ossia", damage: 1120000, kills: 1, assists: 3, deaths: 1, you: true },
    { name: "Orrin", server: "Vandor", damage: 860000, kills: 1, assists: 0, deaths: 3 },
    { name: "Selvane", server: "Lyss", damage: 410000, kills: 0, assists: 2, deaths: 2 },
    { name: "Juno", server: "Ossia", damage: 220000, kills: 0, assists: 1, deaths: 4 },
  ];

  const PVP_STATS_MORE = [
    { name: "Brannoc", server: "Lyss", damage: 198000, kills: 0, assists: 1, deaths: 1 },
    { name: "Ferelith", server: "Vandor", damage: 176000, kills: 1, assists: 0, deaths: 2 },
    { name: "Hask", server: "Ossia", damage: 152000, kills: 0, assists: 0, deaths: 1 },
    { name: "Iridel", server: "Lyss", damage: 131000, kills: 0, assists: 2, deaths: 0 },
    { name: "Morrow", server: "Vandor", damage: 98000, kills: 0, assists: 0, deaths: 2 },
    { name: "Nalla", server: "Ossia", damage: 64000, kills: 0, assists: 1, deaths: 1 },
    { name: "Quill", server: "Lyss", damage: 31000, kills: 0, assists: 0, deaths: 1 },
  ];

  const PVP_DEALT = [
    { name: "Kaedric", cls: "gladiator", server: "Ossia", dmg: 412000, dps: 3210, pct: 0.412 },
    { name: "Selvane", cls: "cleric", server: "Lyss", dmg: 288000, dps: 2410, pct: 0.288 },
    { name: "Juno", cls: "templar", server: "Ossia", dmg: 176000, dps: 1380, pct: 0.176 },
    { name: "Orrin", cls: "assassin", server: "Vandor", dmg: 121000, dps: 990, pct: 0.121 },
  ];

  const PVP_TAKEN = [
    { name: "Grimsalt", cls: "sorcerer", server: "Vandor", dmg: 356000, dps: 2870, pct: 0.54 },
    { name: "Kaedric", cls: "gladiator", server: "Ossia", dmg: 204000, dps: 1650, pct: 0.31 },
    { name: "Tessaly", cls: "ranger", server: "Lyss", dmg: 98000, dps: 790, pct: 0.15 },
  ];

  const PVP_WATCH = [
    { name: "Grimsalt", cls: "sorcerer", server: "Vandor", hp: 18240, max: 41900, auto: true },
    { name: "Kaedric", cls: "gladiator", server: "Ossia", hp: 37105, max: 52300 },
    { name: "Tessaly", missing: true },
  ];

  const PVP_SUGGEST = ["Kaedric", "Kaelis", "Karrow"];

  // ---- Detail ---------------------------------------------------------------

  // hits, crit, perfect, double, front, back, parry, multi rates; multi-hit
  // damage, min, max hit; total. Spec: which of the five spec slots are taken.
  const DETAIL_SKILLS = [
    ["Piercing Volley", 1101, 212, 0.46, 0.05, 0.08, 0.1, 0.14, 0.01, 0.12, 412800, 8120, 41900, 3708000, [1, 3, 4]],
    ["Gale Arrow", 1102, 318, 0.39, 0.03, 0.07, 0.09, 0.12, 0.02, 0.0, 0, 4210, 19870, 2631000, [2, 5]],
    ["Hunter's Mark", 1103, 96, 0.44, 0.04, 0.06, 0.08, 0.13, 0.0, 0.0, 0, 9880, 38200, 1674000, [1]],
    ["Rapid Shot", 1104, 241, 0.41, 0.02, 0.07, 0.07, 0.12, 0.01, 0.18, 198400, 2380, 12450, 1196000, [3]],
    ["Arrow Storm", 1105, 38, 0.52, 0.06, 0.1, 0.11, 0.16, 0.0, 0.0, 0, 14300, 61200, 958000, [2, 4]],
    ["Snare Trap", 1106, 44, 0.34, 0.01, 0.05, 0.06, 0.1, 0.03, 0.0, 0, 6020, 22100, 598000, []],
    ["Eagle Eye", 1108, 27, 0.48, 0.03, 0.05, 0.09, 0.11, 0.0, 0.0, 0, 9100, 31800, 431000, [5]],
    ["Quick Draw", 1504, 133, 0.38, 0.02, 0.06, 0.08, 0.1, 0.02, 0.09, 41200, 1840, 7020, 359000, []],
    ["Wind Step", 1505, 22, 0.31, 0.0, 0.04, 0.05, 0.09, 0.0, 0.0, 0, 4400, 11900, 216000, []],
    ["Basic Attack", 1506, 153, 0.29, 0.0, 0.03, 0.06, 0.08, 0.04, 0.0, 0, 520, 2380, 189820, []],
  ].map((r) => ({
    name: r[0],
    icon: skillIcon(r[1]),
    hits: r[2],
    crit: r[3],
    perfect: r[4],
    double: r[5],
    front: r[6],
    back: r[7],
    parry: r[8],
    multi: r[9],
    multiDmg: r[10],
    min: r[11],
    max: r[12],
    total: r[13],
    spec: r[14],
  }));

  const DETAIL_TOTAL = DETAIL_SKILLS.reduce((s, r) => s + r.total, 0);
  const DETAIL_HITS = DETAIL_SKILLS.reduce((s, r) => s + r.hits, 0);

  const PLAYER_BUFFS = [
    { name: "Blessing of Wind", icon: skillIcon(1508), caster: "Ilyrae", cls: "chanter", cover: 0.92, active: true },
    { name: "War Hymn", icon: skillIcon(1509), caster: "Ilyrae", cls: "chanter", cover: 0.78, active: true },
    { name: "Focused Aim", icon: skillIcon(1510), caster: "Mirelle", cls: "ranger", cover: 0.64, active: false },
    { name: "Bulwark Aura", icon: skillIcon(1511), caster: "Quenna", cls: "templar", cover: 0.41, active: false },
    { name: "Swift Feet", icon: skillIcon(100010), caster: "Dovrek", cls: "cleric", cover: 0.23, active: false },
  ];

  const BOSS_BUFFS = [
    { name: "Armor Break", icon: skillIcon(100020), caster: "Korrinth", cls: "gladiator", cover: 0.88, active: true },
    { name: "Weakening Mark", icon: skillIcon(100030), caster: "Mirelle", cls: "ranger", cover: 0.71, active: true },
    { name: "Frost Bind", icon: skillIcon(100040), caster: "Tobiah", cls: "elementalist", cover: 0.32, active: false },
  ];

  // ---- History --------------------------------------------------------------

  // Boss records get the crown and a report button in History; `mob` marks a
  // record whose target was not a boss (fight report, approved 2026-10-07).
  const HISTORY = [
    { target: "Veilwarden Orsikh", when: "Today 21:42", main: "Mirelle", players: 8, dmg: 65.9e6, secs: 248 },
    { target: "Veilwarden Orsikh", when: "Today 21:18", main: "Mirelle", players: 8, dmg: 61.2e6, secs: 262 },
    { target: "Ashmaw the Hollow", when: "Today 20:55", main: "Mirelle", players: 6, dmg: 38.4e6, secs: 191 },
    { target: "Tidecaller Vennis", when: "Today 20:31", main: "Saebrin", players: 4, dmg: 21.7e6, secs: 166 },
    { target: "Unknown target #4412", when: "Today 20:12", main: "Saebrin", players: 1, dmg: 0.42e6, secs: 38, mob: true },
    { target: "Veilwarden Orsikh", when: "Yesterday 22:07", main: "Mirelle", players: 8, dmg: 58.8e6, secs: 281 },
    { target: "Ashmaw the Hollow", when: "Yesterday 21:40", main: "Mirelle", players: 6, dmg: 36.1e6, secs: 204 },
    { target: "Tidecaller Vennis", when: "Yesterday 21:02", main: "Saebrin", players: 4, dmg: 20.3e6, secs: 171 },
    { target: "Veilwarden Orsikh", when: "Mon 21:55", main: "Mirelle", players: 7, dmg: 55.0e6, secs: 299 },
    { target: "Ashmaw the Hollow", when: "Mon 21:20", main: "Mirelle", players: 6, dmg: 35.2e6, secs: 210 },
    { target: "Veilwarden Orsikh", when: "Sun 20:48", main: "Mirelle", players: 8, dmg: 52.6e6, secs: 305 },
    { target: "Tidecaller Vennis", when: "Sun 20:10", main: "Saebrin", players: 4, dmg: 19.9e6, secs: 182 },
    { target: "Ashmaw the Hollow", when: "Sat 22:31", main: "Mirelle", players: 5, dmg: 31.7e6, secs: 226 },
    { target: "Veilwarden Orsikh", when: "Sat 21:58", main: "Mirelle", players: 8, dmg: 50.1e6, secs: 318 },
  ];

  function countBy(list, key) {
    const m = new Map();
    for (const r of list) m.set(r[key], (m.get(r[key]) || 0) + 1);
    return [...m.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  }

  // ---- Options --------------------------------------------------------------

  const opts = {
    alpha: 150,
    scale: 100,
    rows: 8,
    mask: false,
    server: false,
    damage: false,
    cp: true,
    pctMode: "contribution",
    scene: "all",
  };

  function buildCtx(variant) {
    const nm = (n) => esc(opts.mask ? mask(n) : n);
    const roster = ROSTER.slice(0, Math.max(1, Math.min(10, opts.rows)));
    if (!roster.some((p) => p.you)) roster[roster.length - 1] = ROSTER[1];
    const partyDps = roster.reduce((s, p) => s + p.dps, 0);
    const top = Math.max(...roster.map((p) => p.dps));
    const bossTaken = partyDps * 1.035;
    const players = roster
      .slice()
      .sort((a, b) => b.dps - a.dps)
      .map((p, i) => ({
        ...p,
        rank: i + 1,
        dmg: p.dps * FIGHT_SECS,
        rgb: CLASS_RGB[p.cls],
        icon: classIcon(p.cls),
        label: nm(p.name),
        scale: Math.max(0.06, p.dps / top),
        pct: opts.pctMode === "share" ? p.dps / partyDps : p.dps / bossTaken,
      }));
    const you = players.find((p) => p.you);
    const withClass = (list) => list.map((p) => ({ ...p, label: p.name ? nm(p.name) : "", rgb: p.cls ? CLASS_RGB[p.cls] : "", icon: p.cls ? classIcon(p.cls) : "" }));
    return {
      opts,
      v: variant,
      players,
      you,
      count: players.length,
      partyDps,
      boss: { ...BOSS, label: nm(BOSS.name) },
      fightSecs: FIGHT_SECS,
      summarySecs: SUMMARY_SECS,
      quietSecs: QUIET_SECS,
      quietHp: 0.183,
      skills: SKILLS_TOP,
      crit: 0.41,
      sys: SYS,
      pb: {
        dps: PB_DPS,
        aheadPace: 0.042,
        behindPace: -0.068,
        newPace: 0.061,
        quietPace: (you.dps - PB_DPS) / PB_DPS,
      },
      pvp: {
        stats: withClass(PVP_STATS),
        statsLong: withClass(PVP_STATS.concat(PVP_STATS_MORE)),
        dealt: withClass(PVP_DEALT),
        taken: withClass(PVP_TAKEN),
        watch: withClass(PVP_WATCH.filter((w) => !w.missing)).concat(PVP_WATCH.filter((w) => w.missing).map((w) => ({ ...w, label: nm(w.name) }))),
        suggest: PVP_SUGGEST.map(nm),
      },
      detail: {
        actor: { ...ROSTER[1], rgb: CLASS_RGB.ranger, icon: classIcon("ranger"), label: nm(ROSTER[1].name) },
        skills: DETAIL_SKILLS,
        total: DETAIL_TOTAL,
        hits: DETAIL_HITS,
        secs: SUMMARY_SECS,
        rates: { crit: 0.412, back: 0.124, front: 0.081, double: 0.069, perfect: 0.032, parry: 0.011, multi: 0.098 },
        playerBuffs: withClass(PLAYER_BUFFS.map((b) => ({ ...b, name: b.name, casterLabel: nm(b.caster) }))),
        bossBuffs: withClass(BOSS_BUFFS.map((b) => ({ ...b, casterLabel: nm(b.caster) }))),
      },
      history: {
        records: HISTORY.map((r, i) => ({ ...r, id: i, mainLabel: nm(r.main), targetLabel: nm(r.target) })),
        targets: countBy(HISTORY, "target"),
        mains: countBy(HISTORY, "main"),
      },
      fmt: { int, compact, pct, time, esc, mask },
      icon,
      classIcon,
      CLASS_RGB,
    };
  }

  // ---- Scenes ---------------------------------------------------------------

  const SCENES = [
    { id: "busy", title: "Busy scene", sub: "spell bursts, light and dark under one card. Floating numbers and the red nameplate are the game, behind the overlay" },
    { id: "bright", title: "Bright scene", sub: "snowfield at noon, the worst case for a translucent card" },
    { id: "dark", title: "Dark scene", sub: "night dungeon with torchlight" },
  ];

  function fx(seed) {
    let s = seed * 9301 + 49297;
    const rnd = () => ((s = (s * 9301 + 49297) % 233280) / 233280);
    const kinds = [
      ["hit", () => int(2000 + rnd() * 18000)],
      ["crit", () => int(20000 + rnd() * 40000)],
      ["heal", () => "+" + int(800 + rnd() * 4000)],
    ];
    let out = "";
    for (let i = 0; i < 9; i++) {
      const [cls, text] = kinds[Math.floor(rnd() * kinds.length)];
      out += `<span class="${cls}" style="left:${(rnd() * 88).toFixed(1)}%;top:${(rnd() * 90).toFixed(1)}%">${text()}</span>`;
    }
    out += `<span class="plate" style="left:${(40 + rnd() * 30).toFixed(1)}%;top:${(8 + rnd() * 20).toFixed(1)}%">${esc(BOSS.name)}</span>`;
    return `<div class="fx" aria-hidden="true">${out}</div>`;
  }

  // ---- Controls -------------------------------------------------------------

  const ALL_CONTROLS = ["alpha", "scale", "rows", "mask", "server", "damage", "cp", "pct"];

  function controlsHtml(controls, variants, variant) {
    const has = (k) => controls.includes(k);
    const parts = [];
    if (has("alpha")) parts.push(`<label>Background opacity <input type="range" id="c-alpha" min="0" max="255" value="${opts.alpha}"><output id="o-alpha">${opts.alpha}</output></label>`);
    if (has("scale")) parts.push(`<label>Content scale <input type="range" id="c-scale" min="70" max="150" step="5" value="${opts.scale}"><output id="o-scale">${opts.scale}%</output></label>`);
    if (has("rows")) parts.push(`<label>Rows <input type="range" id="c-rows" min="1" max="10" value="${opts.rows}"><output id="o-rows">${opts.rows}</output></label>`);
    parts.push(`<label>Scene <select id="c-scene"><option value="all">All three</option><option value="busy">Busy</option><option value="bright">Bright</option><option value="dark">Dark</option></select></label>`);
    parts.push(`<span class="controls__sep" aria-hidden="true"></span>`);
    if (has("mask")) parts.push(`<label><input type="checkbox" id="c-mask"> Mask names</label>`);
    if (has("server")) parts.push(`<label><input type="checkbox" id="c-server"> Server</label>`);
    if (has("damage")) parts.push(`<label><input type="checkbox" id="c-damage"> Total damage</label>`);
    if (has("cp")) parts.push(`<label><input type="checkbox" id="c-cp" checked> Combat power</label>`);
    if (has("pct")) parts.push(`<label>Percent <select id="c-pct"><option value="contribution">Contribution</option><option value="share">Share</option></select></label>`);
    if (variants.length) {
      parts.push(`<span class="controls__sep" aria-hidden="true"></span>`);
      for (const v of variants) {
        parts.push(
          `<label>${v.label} <select id="v-${v.key}">${v.options
            .map((o) => `<option value="${o.value}"${o.value === variant[v.key] ? " selected" : ""}>${o.label}${o.value === v.def ? " (recommended)" : ""}</option>`)
            .join("")}</select></label>`
        );
      }
    }
    return `<div class="controls" role="region" aria-label="Mockup controls"><div class="controls__inner">${parts.join("\n")}</div></div>`;
  }

  function applyVars() {
    const root = document.documentElement.style;
    root.setProperty("--bg-a", (opts.alpha / 255).toFixed(3));
    root.setProperty("--scale", (opts.scale / 100).toFixed(2));
  }

  /* Mount a page.
   *   states    [{ id, title, sub }]
   *   render    (stateId, ctx) => html
   *   controls  subset of ALL_CONTROLS (real overlay settings shown)
   *   variants  [{ key, label, def, options: [{ value, label }] }]: open
   *             design choices; body gets "<key>-<value>" classes and the
   *             page re-renders on change. URL ?<key>=<value> overrides.
   *   tileMin, stageMin  tile sizing in px */
  function mount(page) {
    const params = new URLSearchParams(location.search);
    const embed = params.has("embed");
    const controls = page.controls || ALL_CONTROLS;
    const variants = page.variants || [];
    const variant = {};
    for (const v of variants) {
      const asked = params.get(v.key);
      variant[v.key] = v.options.some((o) => o.value === asked) ? asked : v.def;
    }
    if (params.get("scenes")) opts.scene = params.get("scenes");
    const stateIds = (params.get("states") || page.states.map((s) => s.id).join(",")).split(",");
    if (embed) document.body.classList.add("is-embed");

    const main = document.querySelector("main.scenes");
    if (page.tileMin) main.style.setProperty("--tile-min", page.tileMin + "px");
    if (page.stageMin) main.style.setProperty("--stage-min", page.stageMin + "px");
    if (!embed) main.insertAdjacentHTML("beforebegin", controlsHtml(controls, variants, variant));

    const applyVariantClasses = () => {
      for (const v of variants) {
        for (const o of v.options) document.body.classList.remove(`${v.key}-${o.value}`);
        document.body.classList.add(`${v.key}-${variant[v.key]}`);
      }
    };

    const render = () => {
      const ctx = buildCtx(variant);
      const sceneIds = opts.scene === "all" ? SCENES.map((s) => s.id) : opts.scene.split(",");
      let html = "";
      let seed = 1;
      for (const scene of SCENES.filter((s) => sceneIds.includes(s.id))) {
        html += `<section class="scene-group"><h2>${scene.title}<span>${scene.sub}</span></h2><div class="tiles">`;
        for (const state of page.states.filter((s) => stateIds.includes(s.id))) {
          html += `<figure class="tile" style="margin:0">
  <figcaption class="tile__cap"><b>${state.title}</b><span>${state.sub}</span></figcaption>
  <div class="tile__stage scene--${scene.id}">${fx(seed++)}${page.render(state.id, ctx)}</div>
</figure>`;
        }
        html += `</div></section>`;
      }
      main.innerHTML = html;
    };

    applyVars();
    applyVariantClasses();

    if (!embed) {
      const bind = (id, key, read, show) => {
        const el = document.getElementById(id);
        if (!el) return;
        el.addEventListener("input", () => {
          opts[key] = read(el);
          if (show) document.getElementById(show[0]).textContent = show[1](opts[key]);
          applyVars();
          render();
        });
      };
      bind("c-alpha", "alpha", (e) => Number(e.value), ["o-alpha", (v) => v]);
      bind("c-scale", "scale", (e) => Number(e.value), ["o-scale", (v) => v + "%"]);
      bind("c-rows", "rows", (e) => Number(e.value), ["o-rows", (v) => v]);
      bind("c-scene", "scene", (e) => e.value);
      bind("c-mask", "mask", (e) => e.checked);
      bind("c-server", "server", (e) => e.checked);
      bind("c-damage", "damage", (e) => e.checked);
      bind("c-cp", "cp", (e) => e.checked);
      bind("c-pct", "pctMode", (e) => e.value);
      for (const v of variants) {
        const el = document.getElementById("v-" + v.key);
        el.addEventListener("input", () => {
          variant[v.key] = el.value;
          params.set(v.key, el.value);
          history.replaceState(null, "", "?" + params.toString());
          applyVariantClasses();
          render();
        });
      }
    }
    render();
  }

  window.Kit = { mount, icon };
})();
