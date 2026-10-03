/* Shared harness for the meter mockups: invented data, Lucide icons, the page
 * controls, and the scene/state grid. Each direction page supplies only
 * `render(stateId, ctx)` and its own styles.
 *
 * Everything here is fake. Player, boss, dungeon and skill names are invented;
 * nothing comes from a recording.
 *
 * Icons: Lucide (https://lucide.dev), ISC License,
 * Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part
 * of Feather (MIT). All other copyright (c) for Lucide are held by Lucide
 * Contributors 2022. */

(function () {
  "use strict";

  // ---- Data ---------------------------------------------------------------

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

  const BOSS = {
    name: "Veilwarden Orsikh",
    where: "Hollow of Ashen Tides",
    hp: 0.624,
  };

  const FIGHT_SECS = 221; // 3:41 live
  const SUMMARY_SECS = 248; // 4:08 at the kill
  const QUIET_SECS = 177; // 2:57 when it went quiet

  const SKILLS = [
    { name: "Piercing Volley", share: 0.31 },
    { name: "Gale Arrow", share: 0.22 },
    { name: "Hunter's Mark", share: 0.14 },
  ];

  const SYS = { pingMs: 42, cpu: 18, memGb: 11.2 };

  const PB_DPS = 51740; // your best on this boss

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

  function ordinal(n) {
    const v = n % 100;
    const suffix = v >= 11 && v <= 13 ? "th" : ["th", "st", "nd", "rd"][n % 10] || "th";
    return n + suffix;
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
    pvp:
      '<polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" x2="19" y1="19" y2="13"/><line x1="16" x2="20" y1="16" y2="20"/><line x1="19" x2="21" y1="21" y2="19"/><polyline points="14.5 6.5 18 3 21 3 21 6 17.5 9.5"/><line x1="5" x2="9" y1="14" y2="18"/><line x1="7" x2="4" y1="17" y2="20"/><line x1="3" x2="5" y1="19" y2="21"/>',
    close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    copy: '<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>',
    up: '<polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/>',
    down: '<polyline points="22 17 13.5 8.5 8.5 13.5 2 7"/><polyline points="16 17 22 17 22 11"/>',
    award: '<circle cx="12" cy="8" r="6"/><path d="M15.477 12.89 17 22l-5-3-5 3 1.523-9.11"/>',
    timer: '<line x1="10" x2="14" y1="2" y2="2"/><line x1="12" x2="15" y1="14" y2="11"/><circle cx="12" cy="14" r="8"/>',
    skull:
      '<circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><path d="M8 20v2h8v-2"/><path d="m12.5 17-.5-1-.5 1h1z"/><path d="M16 20a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    hourglass:
      '<path d="M5 22h14"/><path d="M5 2h14"/><path d="M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22"/><path d="M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2"/>',
    signal: '<path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V8"/>',
    cpu: '<rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/>',
    crosshair:
      '<circle cx="12" cy="12" r="10"/><line x1="22" x2="18" y1="12" y2="12"/><line x1="6" x2="2" y1="12" y2="12"/><line x1="12" x2="12" y1="6" y2="2"/><line x1="12" x2="12" y1="22" y2="18"/>',
    back: '<path d="m15 18-6-6 6-6"/>',
  };

  function icon(name, cls) {
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name]}</svg>`;
  }

  // ---- Options (the meter's row and column settings) ------------------------

  const opts = {
    alpha: 150, // background alpha 0-255, app default
    scale: 100, // content scale %, 70-150
    rows: 8,
    mask: false,
    server: false,
    damage: false,
    cp: true,
    pctMode: "contribution",
  };

  function buildCtx() {
    const roster = ROSTER.slice(0, Math.max(1, Math.min(10, opts.rows)));
    if (!roster.some((p) => p.you)) roster[roster.length - 1] = ROSTER[1];
    const partyDps = roster.reduce((s, p) => s + p.dps, 0);
    const top = Math.max(...roster.map((p) => p.dps));
    // Contribution: share of everything the boss took, outsiders included.
    // Share: share of this party's damage.
    const bossTaken = partyDps * 1.035;
    const players = roster
      .slice()
      .sort((a, b) => b.dps - a.dps)
      .map((p, i) => {
        const dmg = p.dps * FIGHT_SECS;
        return {
          ...p,
          rank: i + 1,
          dmg,
          rgb: CLASS_RGB[p.cls],
          icon: `../../../public/aion2/class/${p.cls}.png`,
          label: esc(opts.mask ? mask(p.name) : p.name),
          scale: Math.max(0.06, p.dps / top),
          pct: opts.pctMode === "share" ? p.dps / partyDps : (p.dps * 1.0) / bossTaken,
        };
      });
    const you = players.find((p) => p.you);
    const summaryDmg = you.dps * SUMMARY_SECS;
    return {
      opts,
      players,
      you,
      count: players.length,
      partyDps,
      boss: { ...BOSS, label: esc(opts.mask ? mask(BOSS.name) : BOSS.name) },
      fightSecs: FIGHT_SECS,
      summarySecs: SUMMARY_SECS,
      quietSecs: QUIET_SECS,
      quietHp: 0.183,
      skills: SKILLS,
      crit: 0.41,
      sys: SYS,
      pb: {
        dps: PB_DPS,
        aheadPace: 0.042, // live, pointer over
        behindPace: -0.068, // live, locked
        newPace: 0.061, // final summary: a new best
        quietPace: (you.dps - PB_DPS) / PB_DPS, // provisional summary
      },
      summaryDmg,
      fmt: { int, compact, pct, time, ordinal, esc },
      icon,
    };
  }

  // ---- Scenes and states ----------------------------------------------------

  const SCENES = [
    { id: "busy", title: "Busy scene", sub: "spell bursts, light and dark under one card. Floating numbers and the red nameplate are the game, behind the meter" },
    { id: "bright", title: "Bright scene", sub: "snowfield at noon, the worst case for a translucent card" },
    { id: "dark", title: "Dark scene", sub: "night dungeon with torchlight" },
  ];

  const STATES = [
    { id: "idle", title: "Waiting", sub: "meter running, nothing hit yet" },
    { id: "hover", title: "Live, pointer over", sub: "unlocked: controls and system figures show" },
    { id: "locked", title: "Live, locked", sub: "click-through, no controls: the clean look" },
    { id: "capsule", title: "Capsule", sub: "one line; pointer over opens the full view" },
    { id: "final", title: "Summary, defeated", sub: "final: the boss died" },
    { id: "provisional", title: "Summary, fight ended", sub: "provisional: the boss went quiet" },
  ];

  // Deterministic floating combat text so every reload looks the same.
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

  function controlsHtml() {
    return `
<div class="controls" role="region" aria-label="Mockup controls">
  <div class="controls__inner">
    <label>Background opacity <input type="range" id="c-alpha" min="0" max="255" value="${opts.alpha}"><output id="o-alpha">${opts.alpha}</output></label>
    <label>Content scale <input type="range" id="c-scale" min="70" max="150" step="5" value="${opts.scale}"><output id="o-scale">${opts.scale}%</output></label>
    <label>Rows <input type="range" id="c-rows" min="1" max="10" value="${opts.rows}"><output id="o-rows">${opts.rows}</output></label>
    <span class="controls__sep" aria-hidden="true"></span>
    <label><input type="checkbox" id="c-mask"> Mask names</label>
    <label><input type="checkbox" id="c-server"> Server</label>
    <label><input type="checkbox" id="c-damage"> Total damage</label>
    <label><input type="checkbox" id="c-cp" checked> Combat power</label>
    <label>Percent <select id="c-pct"><option value="contribution">Contribution</option><option value="share">Share</option></select></label>
  </div>
</div>`;
  }

  function wireControls(rerender) {
    const bind = (id, key, read, show) => {
      const el = document.getElementById(id);
      el.addEventListener("input", () => {
        opts[key] = read(el);
        if (show) document.getElementById(show[0]).textContent = show[1](opts[key]);
        applyVars();
        rerender();
      });
    };
    bind("c-alpha", "alpha", (e) => Number(e.value), ["o-alpha", (v) => v]);
    bind("c-scale", "scale", (e) => Number(e.value), ["o-scale", (v) => v + "%"]);
    bind("c-rows", "rows", (e) => Number(e.value), ["o-rows", (v) => v]);
    bind("c-mask", "mask", (e) => e.checked);
    bind("c-server", "server", (e) => e.checked);
    bind("c-damage", "damage", (e) => e.checked);
    bind("c-cp", "cp", (e) => e.checked);
    bind("c-pct", "pctMode", (e) => e.value);
  }

  function applyVars() {
    const root = document.documentElement.style;
    root.setProperty("--bg-a", (opts.alpha / 255).toFixed(3));
    root.setProperty("--scale", (opts.scale / 100).toFixed(2));
  }

  function mount(dir) {
    const params = new URLSearchParams(location.search);
    const embed = params.has("embed");
    const sceneIds = (params.get("scenes") || SCENES.map((s) => s.id).join(",")).split(",");
    const stateIds = (params.get("states") || STATES.map((s) => s.id).join(",")).split(",");
    if (embed) document.body.classList.add("is-embed");

    const main = document.querySelector("main.scenes");
    if (!embed) main.insertAdjacentHTML("beforebegin", controlsHtml());

    const render = () => {
      const ctx = buildCtx();
      let html = "";
      let seed = 1;
      for (const scene of SCENES.filter((s) => sceneIds.includes(s.id))) {
        html += `<section class="scene-group"><h2>${scene.title}<span>${scene.sub}</span></h2><div class="tiles">`;
        for (const state of STATES.filter((s) => stateIds.includes(s.id))) {
          html += `<figure class="tile" style="margin:0">
  <figcaption class="tile__cap"><b>${state.title}</b><span>${state.sub}</span></figcaption>
  <div class="tile__stage scene--${scene.id}">${fx(seed++)}${dir.render(state.id, ctx)}</div>
</figure>`;
        }
        html += `</div></section>`;
      }
      main.innerHTML = html;
    };

    applyVars();
    if (!embed) wireControls(render);
    render();
  }

  window.Mock = { mount, icon };
})();
