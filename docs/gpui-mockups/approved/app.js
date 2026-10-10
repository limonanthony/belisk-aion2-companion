/* Harness for the approved main-window mockups: the window shell (title bar,
 * sidebar), Lucide icons, English and French strings for the shell and Home,
 * the page controls (language only, a real app setting) and the state grid.
 *
 * Everything shown is invented: character, server numbers, addresses,
 * recordings and personal bests. Nothing comes from a recording.
 *
 * Icons: Lucide (https://lucide.dev), ISC License,
 * Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part
 * of Feather (MIT). All other copyright (c) for Lucide are held by Lucide
 * Contributors 2022. */

(function () {
  "use strict";

  const ICONS = {
    gauge: '<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>',
    minus: '<path d="M5 12h14"/>',
    square: '<rect width="18" height="18" x="3" y="3" rx="2"/>',
    close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    sliders:
      '<line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/>',
    activity:
      '<path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/>',
    keyboard:
      '<path d="M10 8h.01"/><path d="M12 12h.01"/><path d="M14 8h.01"/><path d="M16 12h.01"/><path d="M18 8h.01"/><path d="M6 8h.01"/><path d="M7 16h10"/><path d="M8 12h.01"/><rect width="20" height="16" x="2" y="4" rx="2"/>',
    radio:
      '<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/><path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/><path d="M19.1 4.9C23 8.8 23 15.1 19.1 19"/>',
    trophy:
      '<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6"/><path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18"/><path d="M4 22h16"/><path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22"/><path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22"/><path d="M18 2H6v7a6 6 0 0 0 12 0V2Z"/>',
    bell:
      '<path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/>',
    listChecks: '<path d="m3 17 2 2 4-4"/><path d="m3 7 2 2 4-4"/><path d="M13 6h8"/><path d="M13 12h8"/><path d="M13 18h8"/>',
    languages:
      '<path d="m5 8 6 6"/><path d="m4 14 6-6 2-3"/><path d="M2 5h12"/><path d="M7 2h1"/><path d="m22 22-5-10-5 10"/><path d="M14 18h6"/>',
    info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
    play: '<polygon points="6 3 20 12 6 21 6 3" fill="currentColor"/>',
    stop: '<rect x="6" y="6" width="12" height="12" rx="1.5" fill="currentColor"/>',
    loader: '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>',
    check: '<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',
    alert:
      '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
    download:
      '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>',
    refresh:
      '<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>',
    shield:
      '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>',
    folder:
      '<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>',
    rec: '<circle cx="12" cy="12" r="6" fill="currentColor"/>',
    user: '<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
    server:
      '<rect width="20" height="8" x="2" y="2" rx="2" ry="2"/><rect width="20" height="8" x="2" y="14" rx="2" ry="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/>',
    cpu: '<rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/>',
    file: '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
    scale:
      '<path d="m16 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z"/><path d="m2 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z"/><path d="M7 21h10"/><path d="M12 3v18"/><path d="M3 7h2c2 0 5-1 7-2 2 1 5 2 7 2h2"/>',
    drive:
      '<line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/>',
    reset: '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>',
    external:
      '<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>',
    chevronDown: '<path d="m6 9 6 6 6-6"/>',
    chevronRight: '<path d="m9 18 6-6-6-6"/>',
    trash:
      '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>',
    lock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
    tick: '<path d="M20 6 9 17l-5-5"/>',
    dash: '<path d="M8 12h8"/>',
    // Settings › Pets (2026-10-10, the pets round): Lucide paw-print, the
    // toolbar tool's icon, from gpui-kit-assets 0.6.6.
    paw: '<circle cx="11" cy="4" r="2"/><circle cx="18" cy="8" r="2"/><circle cx="20" cy="16" r="2"/><path d="M9 10a5 5 0 0 1 5 5v3.5a3.5 3.5 0 0 1-6.84 1.045Q6.52 17.48 4.46 16.84A3.5 3.5 0 0 1 5.5 10Z"/>',
  };

  function icon(name, cls) {
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name]}</svg>`;
  }

  // ---- Strings: the shell and Home in English and French --------------------

  const STRINGS = {
    en: {
      meter: "Meter",
      settings: "Settings",
      overlay: "Overlay",
      behaviour: "Meter behaviour",
      shortcuts: "Shortcuts",
      capture: "Capture",
      bests: "Personal bests",
      reminders: "Reminders",
      todos: "To-dos",
      pets: "Pets",
      general: "General",
      about: "About",
      ready: "Ready",
      readyDetail: "Finds the game and the server on its own. Nothing to set up.",
      start: "Start meter",
      starting: "Starting…",
      stop: "Stop meter",
      waiting: "Waiting for the game",
      waitingDetail: "Start AION 2 and log in. The meter connects by itself.",
      running: "Running",
      closeAfter: "Close this window once it starts",
      closeAfterDesc: "The meter keeps running from the tray. Alt+E shows the overlay.",
      recheck: "Re-run capture check",
      failTitle: "The meter could not start",
      failBody: "No capture backend would start. Npcap: no network adapter answered. WinDivert: the driver could not be loaded (access denied).",
      whilePlaying: "While you play",
      kShow: "Show or hide the overlay",
      kReset: "Reset the meter",
      kLock: "Lock or unlock the overlay",
      change: "Change",
      stopped: "Stopped",
    },
    fr: {
      meter: "Compteur",
      settings: "Paramètres",
      overlay: "Overlay",
      behaviour: "Comportement",
      shortcuts: "Raccourcis",
      capture: "Capture",
      bests: "Records perso",
      reminders: "Rappels",
      todos: "Tâches",
      pets: "Familiers",
      general: "Général",
      about: "À propos",
      ready: "Prêt",
      readyDetail: "Trouve le jeu et le serveur tout seul. Rien à configurer.",
      start: "Démarrer le compteur",
      starting: "Démarrage…",
      stop: "Arrêter le compteur",
      waiting: "En attente du jeu",
      waitingDetail: "Lancez AION 2 et connectez-vous. Le compteur se connecte tout seul.",
      running: "En cours",
      closeAfter: "Fermer cette fenêtre au démarrage",
      closeAfterDesc: "Le compteur continue depuis la zone de notification. Alt+E affiche l'overlay.",
      recheck: "Relancer la vérification de capture",
      failTitle: "Le compteur n'a pas pu démarrer",
      failBody: "Aucune méthode de capture n'a démarré. Npcap : aucune carte réseau n'a répondu. WinDivert : le pilote n'a pas pu être chargé (accès refusé).",
      whilePlaying: "En jeu",
      kShow: "Afficher ou masquer l'overlay",
      kReset: "Réinitialiser le compteur",
      kLock: "Verrouiller ou déverrouiller l'overlay",
      change: "Modifier",
      stopped: "Arrêté",
    },
  };

  // ---- Shell ----------------------------------------------------------------

  const SECTIONS = [
    ["overlay", "sliders"],
    ["behaviour", "activity"],
    ["shortcuts", "keyboard"],
    ["capture", "radio"],
    ["bests", "trophy"],
    ["reminders", "bell"],
    ["todos", "listChecks"],
    ["pets", "paw"],
    ["general", "languages"],
    ["about", "info"],
  ];

  // The name comes from app-name.js (window.APP_NAME, Fluent `-app-name`).
  // Earlier rounds' pages load this file without it and keep the name they
  // were drawn with.
  function titleBar(ctx, { gate = false } = {}) {
    const ctl = gate
      ? `<span class="tb-ctl" title="Close">${icon("close")}</span>`
      : `<span class="tb-ctl" title="Minimise">${icon("minus")}</span><span class="tb-ctl" title="Maximise">${icon("square")}</span><span class="tb-ctl" title="Close">${icon("close")}</span>`;
    return `<div class="tb"><span class="tb-mark"><span class="logo">${icon("gauge")}</span>${window.APP_NAME || "AION 2 DPS Meter"}</span><span class="grow"></span>${ctl}</div>`;
  }

  /* The main window: title bar, sidebar (Meter, then the settings sections,
   * then the meter's status line) and `content`.
   *   page    "home" or a settings section id
   *   meter   { tone: "idle" | "wait" | "live", label } for the status line
   *   thumb   scroll thumb height in px when the page scrolls
   *   scroll  shows the page scrolled down by that many pixels */
  function shell(ctx, page, content, { meter, overlayHtml = "", thumb = 0, scroll = 0 } = {}) {
    const t = ctx.t;
    const status = meter
      ? `<div class="side-status"><span class="dot${meter.tone === "live" ? " is-good" : meter.tone === "wait" ? " is-warn" : ""}"></span><span>${meter.label}</span></div>`
      : "";
    const thumbCls = thumb ? ` has-thumb" style="--thumb:${thumb}px;--thumb-top:${8 + Math.round(scroll * 0.45)}px` : "";
    if (scroll) content = `<div style="margin-top:-${scroll}px">${content}</div>`;
    const side = `<aside class="side">
  <div class="side-item${page === "home" ? " is-on" : ""}">${icon("gauge")}${t.meter}</div>
  <div class="side-group">${t.settings}</div>
  ${SECTIONS.map(([id, ic]) => `<div class="side-item${id === page ? " is-on" : ""}">${icon(ic)}${t[id]}</div>`).join("")}
  ${status}
</aside>`;
    return `<div class="win">${titleBar(ctx)}<div class="shell">${side}<div class="content${thumbCls}">${content}</div></div>${overlayHtml}</div>`;
  }

  // ---- Small component helpers ------------------------------------------------

  const C = {
    sw: (on) => `<span class="sw${on ? " is-on" : ""}" role="switch" aria-checked="${on}"></span>`,
    seg: (opts, on) => `<span class="seg">${opts.map((o) => `<span class="${o === on ? "is-on" : ""}">${o}</span>`).join("")}</span>`,
    sel: (val, { open = false, items = [], hot = "" } = {}) =>
      `<span class="sel${open ? " is-open" : ""}"><span class="val">${val}</span>${icon("chevronDown")}${
        open ? `<span class="menu">${items.map((it) => `<span class="menu-item${it === hot ? " is-hot" : ""}">${it}${it === val ? icon("tick") : ""}</span>`).join("")}</span>` : ""
      }</span>`,
    slider: (pct, label) =>
      `<span class="sl"><span class="sl-track"><span class="sl-range" style="width:${pct}%"></span><span class="sl-thumb" style="left:${pct}%"></span></span><span class="sl-val">${label}</span></span>`,
    row: (label, desc, ctl, extra = "") =>
      `<div class="row${extra}"><div class="row-text"><div class="row-label">${label}</div>${desc ? `<div class="row-desc">${desc}</div>` : ""}</div><div class="row-ctl">${ctl}</div></div>`,
    group: (title, rows, first = false) => `<div class="grp-title${first ? " is-first" : ""}">${title}</div><div class="grp">${rows}</div>`,
    kbd: (combo) => combo.split("+").map((k) => `<span class="kbd">${k}</span>`).join('<span class="faint">+</span>'),
    btn: (label, { kind = "", ic = "", size = "" } = {}) =>
      `<span class="btn${kind ? " is-" + kind : ""}${size ? " is-" + size : ""}" role="button">${ic ? icon(ic) : ""}${label}</span>`,
    badge: (label, tone = "", ic = "") => `<span class="badge${tone ? " is-" + tone : ""}">${ic ? icon(ic) : ""}${label}</span>`,
  };

  // ---- Mount ----------------------------------------------------------------

  /* page: { states: [{id, title, sub}], render(stateId, ctx),
   *         lang: true to offer English/French, gate: true for the small window } */
  function mount(page) {
    const params = new URLSearchParams(location.search);
    const embed = params.has("embed");
    let lang = params.get("lang") === "fr" ? "fr" : "en";
    const stateIds = (params.get("states") || page.states.map((s) => s.id).join(",")).split(",");
    if (embed) document.body.classList.add("is-embed");

    const main = document.querySelector("main.scenes");
    if (!embed && page.lang) {
      main.insertAdjacentHTML(
        "beforebegin",
        `<div class="controls" role="region" aria-label="Mockup controls"><div class="controls__inner"><label>Language <select id="v-lang"><option value="en"${lang === "en" ? " selected" : ""}>English</option><option value="fr"${lang === "fr" ? " selected" : ""}>Français</option></select></label></div></div>`
      );
    }

    const render = () => {
      const ctx = { lang, t: STRINGS[lang], icon, C, shell, titleBar };
      main.innerHTML = `<section class="scene-group"><div class="tiles" style="--tile-min:${page.gate ? 660 : 1020}px">${page.states
        .filter((s) => stateIds.includes(s.id))
        .map(
          (s) => `<figure class="tile" style="margin:0"><figcaption class="tile__cap"><b>${s.title}</b><span>${s.sub}</span></figcaption><div class="tile__stage desk">${page.render(s.id, ctx)}</div></figure>`
        )
        .join("")}</div></section>`;
    };

    if (!embed) {
      const le = document.getElementById("v-lang");
      if (le) {
        le.addEventListener("input", () => {
          lang = le.value;
          params.set("lang", lang);
          history.replaceState(null, "", "?" + params.toString());
          render();
        });
      }
    }
    render();
  }

  window.App = { mount, icon, C };
})();
