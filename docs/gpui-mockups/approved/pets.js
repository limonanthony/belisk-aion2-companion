// Pets, approved 2026-10-10 (the pets round, ../pets/README.md): drawing
// for the Pets window (layout A, meter rows, level colours on the badge)
// and the soul tracker card (design A, the reminder card's twin), shared by
// pets.html and the "Pets open" states of toolbar.html. Styles: pets.css
// (with overlay.css, todos.css and hub.css). The toolbar's pets tool is
// drawn by hub.js.
//
// Everything here is invented: the pet list, families, levels and souls,
// times. Seven names come from public sources (Fossa, Kalgolem, Splendent
// Armadon, Hill Coradon, Forest Spirit, Slime, Krall Laborer); the rest are
// public creature words combined to fill 290 rows, and the families they
// sit in are guesses. The pet catalogue (TASK-34.2) supplies the real names
// (English and French), families and order. French pet names are drafts
// for five pets only. Nothing is saved or sent anywhere.
//
// Icons: Lucide (https://lucide.dev), ISC License; see kit.js. paw-print,
// pin, search, list-filter, arrow-right, chevrons-up and sparkles are
// copied from gpui-kit-assets 0.6.6.
(function () {
  "use strict";

  const esc = Hub.esc;

  const PI = {
    paw: '<circle cx="11" cy="4" r="2"/><circle cx="18" cy="8" r="2"/><circle cx="20" cy="16" r="2"/><path d="M9 10a5 5 0 0 1 5 5v3.5a3.5 3.5 0 0 1-6.84 1.045Q6.52 17.48 4.46 16.84A3.5 3.5 0 0 1 5.5 10Z"/>',
    pin: '<path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>',
    search: '<path d="m21 21-4.34-4.34"/><circle cx="11" cy="11" r="8"/>',
    listFilter: '<path d="M2 5h20"/><path d="M6 12h12"/><path d="M9 19h6"/>',
    arrowRight: '<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>',
    chevronsUp: '<path d="m17 11-5-5-5 5"/><path d="m17 18-5-5-5 5"/>',
    sparkles:
      '<path d="M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z"/><path d="M20 2v4"/><path d="M22 4h-4"/><circle cx="4" cy="20" r="2"/>',
  };

  function icon(name, cls) {
    if (!PI[name]) return Hub.icon(name, cls);
    return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${PI[name]}</svg>`;
  }

  // ---- Clock (mocked, as the hub): Monday 21:48, logged in at 19:02 --------
  const LOGIN = "19:02";
  const STALE = { en: "Sun 22:40", fr: "dim. 22:40" };
  const SERVER = "Ossia";

  // ---- The Global rule: 5 souls for Lv 0 -> 1, 25 for 1 -> 2, 75 for 2 -> 3;
  // Lv 3 is the cap; souls beyond a level carry over. A pet at the cap reads
  // 75/75.
  const NEED = [5, 25, 75];
  const CAP = 3;
  const needOf = (p) => (p.lv >= CAP ? NEED[CAP - 1] : NEED[p.lv]);
  const progress = (p) => (p.lv >= CAP ? 1 : p.souls / needOf(p));
  const leftOf = (p) => (p.lv >= CAP ? Infinity : needOf(p) - p.souls);

  function addSouls(p, n) {
    let lv = p.lv;
    let s = p.souls + n;
    while (lv < CAP && s >= NEED[lv]) {
      s -= NEED[lv];
      lv++;
    }
    if (lv >= CAP) {
      lv = CAP;
      s = NEED[CAP - 1];
    }
    return { lv, souls: s };
  }

  // ---- Catalogue (invented, see the header) --------------------------------
  const FAM = [
    { id: "cogni", en: "Cogni", fr: "Cogni", n: 58 },
    { id: "fera", en: "Fera", fr: "Fera", n: 92 },
    { id: "natura", en: "Natura", fr: "Natura", n: 64 },
    { id: "varian", en: "Varian", fr: "Varian", n: 52 },
    { id: "special", en: "Special", fr: "Spécial", n: 24 },
  ];
  const famName = (id, lang) => FAM.find((f) => f.id === id)[lang];

  function rng(seed) {
    return function () {
      seed = (seed + 0x6d2b79f5) | 0;
      let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  // Every modifier-base pair in a seeded shuffle, so each family reaches its
  // size without repeating a name.
  function build(n, first, mods, bases) {
    const out = first.slice();
    const seen = new Set(out);
    const pairs = [];
    for (const m of mods) for (const b of bases) pairs.push(`${m} ${b}`);
    const r = rng(n * 7 + mods.length);
    for (let i = pairs.length - 1; i > 0; i--) {
      const j = Math.floor(r() * (i + 1));
      [pairs[i], pairs[j]] = [pairs[j], pairs[i]];
    }
    for (const name of pairs) {
      if (out.length >= n) break;
      if (!seen.has(name)) {
        seen.add(name);
        out.push(name);
      }
    }
    return out;
  }

  const NAMES = {
    cogni: build(
      58,
      ["Krall Laborer"],
      ["Krall", "Lepharist", "Mau", "Shugo", "Kobold", "Brohum", "Lizardman", "Naga", "Drakan", "Goblin", "Ettin", "Mookie"],
      ["Laborer", "Hunter", "Shaman", "Scout", "Guard", "Elder", "Merchant", "Miner", "Raider", "Seer", "Courier", "Brawler", "Scribe", "Tinker", "Forager"]
    ),
    fera: build(
      92,
      ["Fossa", "Hill Coradon", "Splendent Armadon", "Thornback Boar"],
      ["Hill", "Swamp", "Frost", "Ash", "Dune", "Young", "Elder", "Striped", "Moss", "Shadow", "Ember", "Coral", "Cliff", "Storm", "Golden", "Spotted", "Thornback", "Splendent"],
      ["Fossa", "Coradon", "Armadon", "Boar", "Lynx", "Wolf", "Bear", "Hound", "Raptor", "Kirrin", "Hamerun", "Stag", "Fox", "Ram", "Jackal", "Weasel", "Hawk", "Owl", "Toad", "Tortoise"]
    ),
    natura: build(
      64,
      ["Forest Spirit"],
      ["Forest", "Water", "Fire", "Wind", "Earth", "Moss", "Lotus", "Thorn", "Dew", "Ember", "Frost", "Bloom"],
      ["Spirit", "Sprite", "Wisp", "Sporeling", "Mandrake", "Treant", "Seedling", "Glowcap", "Dryad"]
    ),
    varian: build(
      52,
      ["Slime", "Kalgolem"],
      ["Green", "Crystal", "Molten", "Bog", "Ancient", "Rune", "Hollow", "Iron", "Pale", "Twin"],
      ["Slime", "Kalgolem", "Golem", "Mimic", "Gargoyle", "Chimera", "Wraith", "Ooze", "Homunculus"]
    ),
    special: [
      "Lantern Shugo", "Festival Pinwheel", "Moonlit Rabbit", "Starfall Kitten", "Snowdrift Pup", "Harvest Scarecrow",
      "Paper Dragon", "Lucky Coin Toad", "Bell Sprite", "Ribbon Cat", "Candy Golem", "Firework Owl",
      "Blossom Fox", "Pumpkin Imp", "Frost Penguin", "Sunflower Mookie", "Tea Kettle Mau", "Plush Krall",
      "Carnival Slime", "Moon Jelly", "Clockwork Bird", "Star Whale", "Cloud Lamb", "Gift Box Mimic",
    ],
  };

  // Drafts, to check the room in French; the catalogue supplies the names.
  const FR = {
    "Forest Spirit": "Esprit de la forêt",
    "Hill Coradon": "Coradon des collines",
    "Splendent Armadon": "Armadon resplendissant",
    "Krall Laborer": "Ouvrier krall",
    "Thornback Boar": "Sanglier dos-d'épines",
  };

  // The pets named in the states, with the values the states need.
  const FIXED = {
    Fossa: { lv: 2, souls: 27 },
    Kalgolem: { lv: 3, souls: 75 },
    "Hill Coradon": { lv: 1, souls: 24 },
    "Forest Spirit": { lv: 0, souls: 4 },
    "Splendent Armadon": { lv: 2, souls: 70 },
    Slime: { lv: 1, souls: 12 },
    "Krall Laborer": { lv: 0, souls: 0 },
    "Thornback Boar": { lv: 0, souls: 0 },
  };

  const PETS = (function () {
    const r = rng(34);
    const out = [];
    let id = 1001;
    for (const f of FAM) {
      for (const name of NAMES[f.id]) {
        let st = FIXED[name];
        if (!st) {
          const x = r();
          if (f.id === "special") st = x < 0.5 ? { lv: 0, souls: 0 } : x < 0.8 ? { lv: 1, souls: Math.floor(r() * 25) } : { lv: 3, souls: 75 };
          else if (x < 0.22) st = { lv: 0, souls: 0 };
          else if (x < 0.28) st = { lv: 0, souls: 1 + Math.floor(r() * 4) };
          else if (x < 0.48) st = { lv: 1, souls: Math.floor(r() * 25) };
          else if (x < 0.78) st = { lv: 2, souls: Math.floor(r() * 75) };
          else st = { lv: 3, souls: 75 };
        }
        out.push({ id: id++, fam: f.id, name, fr: FR[name], ...st });
      }
    }
    return out;
  })();

  const byName = (name, pets = PETS) => pets.find((p) => p.name === name);
  const idOf = (name) => byName(name).id;
  const nameOf = (p, lang) => (lang === "fr" && p.fr) || p.name;
  const DEFAULT_PINS = [idOf("Fossa"), idOf("Kalgolem")];

  // ---- Strings (English; French drawn to check the room, through i18n) -----
  const S = {
    en: {
      title: "Pets",
      close: "Close",
      search: "Search pets",
      clearSearch: "Clear the search",
      filter: "Show and order",
      tabAll: "All",
      owned: (a, b) => `<b>${a}</b> of ${b} owned`,
      maxed: (n) => `<b>${n}</b> at Lv 3`,
      levels: "Levels",
      pinned: "Pinned",
      allPets: "All pets",
      noStats: "no stats",
      srcLive: (t, srv, dot) => `From the game, live ${dot} logged in at <b>${t}</b> ${dot} <b>${srv}</b>`,
      srcStale: (t, dot) => `As of your login <b>${t}</b> ${dot} relog to refresh`,
      srcNone: (app) => `No pets read yet. Log in while ${app} runs.`,
      staleTip: (t, app) => [`Read at your login ${t}; souls gained since, while ${app} ran, are added.`, `The game sends your pets only at login: log in again while ${app} runs for exact values.`],
      perLevel: "Souls per level",
      capLine: "Lv 3 is the cap",
      max: "Max",
      pin: "Pin",
      unpin: "Unpin",
      pinTip: "Pin to the top",
      unpinTip: "Unpin",
      show: "Show",
      order: "Order",
      showAll: "All pets",
      showUnowned: "Not owned yet",
      showNotMax: "Not at Lv 3",
      orderGame: "Game order",
      orderClose: "Closest to the next level",
      closeFirst: "closest first",
      shown: (n) => `${n} shown`,
      clear: "Clear",
      emptyTitle: "No pets yet",
      emptyText: (app) => `The game sends your pets once, when you log in. Log in with a character while ${app} runs, and every pet shows here with its level and souls.`,
      noMatchT: (q) => `No pet matches “${esc(q)}”`,
      noMatchIn: (tab) => `Nothing in ${tab}.`,
      searchAll: "Search all families",
      noMatchAll: "No pet in any family has that in its name.",
      card: "Pet souls",
      testCard: "Test card",
      reached: (lv) => `Lv ${lv} reached`,
      unlocked: "Unlocked",
      dismiss: "Dismiss",
    },
    fr: {
      title: "Familiers",
      close: "Fermer",
      search: "Rechercher un familier",
      clearSearch: "Effacer la recherche",
      filter: "Afficher et trier",
      tabAll: "Tous",
      owned: (a, b) => `<b>${a}</b> sur ${b} obtenus`,
      maxed: (n) => `<b>${n}</b> au Lv 3`,
      levels: "Niveaux",
      pinned: "Épinglés",
      allPets: "Tous les familiers",
      noStats: "sans stats",
      srcLive: (t, srv, dot) => `Du jeu, en direct ${dot} connexion à <b>${t}</b> ${dot} <b>${srv}</b>`,
      srcStale: (t, dot) => `À votre connexion <b>${t}</b> ${dot} reconnectez-vous pour actualiser`,
      srcNone: (app) => `Aucun familier lu. Connectez-vous pendant que ${app} tourne.`,
      staleTip: (t, app) => [`Lu à votre connexion ${t} ; les âmes gagnées depuis, pendant que ${app} tournait, sont ajoutées.`, `Le jeu n'envoie vos familiers qu'à la connexion : reconnectez-vous pendant que ${app} tourne pour des valeurs exactes.`],
      perLevel: "Âmes par niveau",
      capLine: "Lv 3 au maximum",
      max: "Max",
      pin: "Épingler",
      unpin: "Désépingler",
      pinTip: "Épingler en haut",
      unpinTip: "Désépingler",
      show: "Afficher",
      order: "Ordre",
      showAll: "Tous les familiers",
      showUnowned: "Pas encore obtenus",
      showNotMax: "Pas au Lv 3",
      orderGame: "Ordre du jeu",
      orderClose: "Plus près du niveau suivant",
      closeFirst: "les plus proches d'abord",
      shown: (n) => `${n} affichés`,
      clear: "Effacer",
      emptyTitle: "Aucun familier pour l'instant",
      emptyText: (app) => `Le jeu envoie vos familiers une fois, à la connexion. Connectez-vous avec un personnage pendant que ${app} tourne : chaque familier s'affiche ici avec son niveau et ses âmes.`,
      noMatchT: (q) => `Aucun familier ne correspond à « ${esc(q)} »`,
      noMatchIn: (tab) => `Rien dans ${tab}.`,
      searchAll: "Chercher dans toutes les familles",
      noMatchAll: "Aucun familier ne porte ce nom, toutes familles confondues.",
      card: "Âmes de familier",
      testCard: "Carte de test",
      reached: (lv) => `Lv ${lv} atteint`,
      unlocked: "Débloqué",
      dismiss: "Fermer",
    },
  };

  // ---- Window ------------------------------------------------------------------
  // o: { lang, tab, q, searchFocus, interactive (a real input), show "all" |
  //      "unowned" | "notmax", order "game" | "close", filterOpen, pinned
  //      [ids], pets, src "live" | "stale" | "none", srcTip, hover id,
  //      pinHover id, tip id, focus id, scroll px }
  const figInner = (p) => `${p.souls}<small>/${needOf(p)}</small>`;

  function pinBtn(p, o, L) {
    const on = o.pinned.includes(p.id);
    const c = ["o-btn is-sm pt-pin", on && "is-on", o.pinHover === p.id && "is-hover", o.focus === p.id && "is-focus"].filter(Boolean).join(" ");
    const label = `${on ? L.unpin : L.pin} ${nameOf(p, o.lang)}`;
    return `<button type="button" class="${c}" data-pt-pin="${p.id}" aria-pressed="${on}" aria-label="${esc(label)}">${icon("pin")}</button>`;
  }

  const tipFor = (p, o, L) => (o.tip === p.id ? `<div class="t-tip" role="tooltip">${o.pinned.includes(p.id) ? L.unpinTip : L.pinTip}</div>` : "");

  // A row: the level badge (coloured by level), the track with the
  // progress fill, the name, "27/75", "→ Lv 3" ("Max" at the cap), the pin.
  function row(p, o, L) {
    const max = p.lv >= CAP;
    const none = p.lv === 0 && p.souls === 0;
    const cls = ["pt-row", `lv-${p.lv}`, max && "is-max", none && "is-none", o.hover === p.id && "is-hover"].filter(Boolean).join(" ");
    const pct = progress(p);
    const fill = pct > 0 ? `<span class="pt-fill" style="width:${Math.max(4, pct * 100).toFixed(1)}%"></span>` : "";
    const to = max ? `<span class="pt-to is-max">${L.max}</span>` : `<span class="pt-to">${icon("arrowRight")}Lv ${p.lv + 1}</span>`;
    const tag = p.fam === "special" ? `<small>${L.noStats}</small>` : "";
    return `<div class="${cls}"><span class="pt-lv">Lv ${p.lv}</span><span class="pt-track">${fill}<span class="pt-name">${esc(nameOf(p, o.lang))}${tag}</span><span class="pt-fig">${figInner(p)}</span>${to}</span>${pinBtn(p, o, L)}${tipFor(p, o, L)}</div>`;
  }

  const tabPets = (o) => o.pets.filter((p) => o.tab === "all" || p.fam === o.tab);

  // Closest to the next level: fewest souls left first (Lv 3 last), then
  // the larger share done, then game order.
  function visible(o) {
    let list = tabPets(o);
    if (o.q) {
      const q = o.q.toLowerCase();
      list = list.filter((p) => nameOf(p, o.lang).toLowerCase().includes(q));
    }
    if (o.show === "unowned") list = list.filter((p) => p.lv === 0);
    if (o.show === "notmax") list = list.filter((p) => p.lv < CAP);
    if (o.order === "close") list = list.slice().sort((a, b) => leftOf(a) - leftOf(b) || progress(b) - progress(a) || a.id - b.id);
    const pins = o.pinned.map((id) => list.find((p) => p.id === id)).filter(Boolean);
    const rest = list.filter((p) => !o.pinned.includes(p.id));
    return { pins, rest, count: list.length };
  }

  const label = (ic, text, n) => `<div class="t-sec is-static">${icon(ic)}<span>${text}</span><span class="n">${n}</span></div>`;

  function list(o, vis, L) {
    const rows = (arr) => `<div class="pt-rows">${arr.map((p) => row(p, o, L)).join("")}</div>`;
    let h;
    if (vis.pins.length) {
      const restLabel = o.tab === "all" ? L.allPets : famName(o.tab, o.lang);
      h = label("pin", L.pinned, vis.pins.length) + rows(vis.pins) + (vis.rest.length ? label("paw", restLabel, vis.rest.length) + rows(vis.rest) : "");
    } else h = rows(vis.rest);
    return `<div class="pt-scrollwrap" data-scroll="${o.scroll || 0}"><div class="pt-scroll" role="list">${h}</div><span class="o-thumb" aria-hidden="true"></span></div>`;
  }

  function summary(pets, L) {
    const owned = pets.filter((p) => p.lv >= 1).length;
    const max = pets.filter((p) => p.lv >= CAP).length;
    const lv = pets.reduce((a, p) => a + p.lv, 0);
    const all = pets.length * CAP;
    return `<div class="pt-sum"><span>${L.owned(owned, pets.length)}</span><span class="faint">·</span><span>${L.maxed(max)}</span><span class="grow"></span><span>${L.levels} <b>${lv}</b><small>/${all}</small></span></div><div class="pt-prog" role="progressbar" aria-label="${L.levels}" aria-valuemin="0" aria-valuemax="${all}" aria-valuenow="${lv}"><i style="width:${((lv / all) * 100).toFixed(1)}%"></i></div>`;
  }

  function filterMenu(o, L) {
    const item = (key, val, text, cur) =>
      `<div class="o-menu-item${cur ? " is-cur" : ""}" role="menuitemradio" aria-checked="${cur}" data-pt-${key}="${val}"><span class="lead">${cur ? icon("check") : ""}</span><span>${text}</span></div>`;
    return `<div class="o-menu pt-menu" role="menu" aria-label="${L.filter}"><div class="hb-drop-label">${L.show}</div>${item("show", "all", L.showAll, o.show === "all")}${item("show", "unowned", L.showUnowned, o.show === "unowned")}${item("show", "notmax", L.showNotMax, o.show === "notmax")}<div class="o-menu-sep"></div><div class="hb-drop-label">${L.order}</div>${item("order", "game", L.orderGame, o.order === "game")}${item("order", "close", L.orderClose, o.order === "close")}</div>`;
  }

  const filterOn = (o) => o.show !== "all" || o.order !== "game";

  function filterLine(o, L, count) {
    const parts = [];
    if (o.show === "unowned") parts.push(L.showUnowned);
    if (o.show === "notmax") parts.push(L.showNotMax);
    if (o.order === "close") parts.push(L.closeFirst);
    if (!parts.length) return "";
    const dot = ' <span class="faint">·</span> ';
    return `<div class="pt-fline">${icon("listFilter")}<span>${parts.join(dot)}${dot}<span class="faint">${L.shown(count)}</span></span><span class="t-link" role="button" tabindex="0" data-pt-clear="1">${L.clear}</span></div>`;
  }

  function srcLine(o, L) {
    const dot = '<span class="faint">·</span>';
    const app = window.APP_NAME;
    const t = o.src === "stale" ? L.srcStale(STALE[o.lang], dot) : o.src === "none" ? L.srcNone(app) : L.srcLive(LOGIN, SERVER, dot);
    let tip = "";
    if (o.srcTip) {
      const [a, b] = L.staleTip(STALE[o.lang], app);
      tip = `<div class="t-tip g-tip" role="tooltip">${a}<br><span class="faint">${b}</span></div>`;
    }
    return `<div class="pt-srcwrap"><div class="g-src">${icon("gamepad")}<span>${t}</span></div>${tip}</div>`;
  }

  function findRow(o, L) {
    let inner;
    if (o.interactive) inner = `<input type="text" value="${esc(o.q)}" placeholder="${esc(L.search)}" aria-label="${esc(L.search)}" data-pt-q="1" spellcheck="false" autocomplete="off">`;
    else if (o.q) inner = `<span class="val">${esc(o.q)}</span>${o.searchFocus ? '<span class="o-caret"></span>' : ""}<span class="grow"></span>`;
    else inner = `${o.searchFocus ? '<span class="o-caret" style="margin-left:0"></span>' : ""}<span class="ph">${L.search}</span>`;
    const clear = o.q ? `<button type="button" class="o-btn is-sm" aria-label="${L.clearSearch}" data-pt-qclear="1">${icon("close")}</button>` : "";
    const fb = `<button type="button" class="pt-filter${filterOn(o) ? " is-on" : ""}${o.filterOpen ? " is-open" : ""}" aria-haspopup="menu" aria-expanded="${!!o.filterOpen}" aria-label="${L.filter}" data-pt-filter="1"${o.off ? " disabled" : ""}>${icon("listFilter")}</button>`;
    const box = o.interactive ? "label" : "div";
    return `<div class="pt-find${o.off ? " is-off" : ""}"><${box} class="o-input pt-search${o.searchFocus ? " is-focus" : ""}" role="search">${icon("search")}${inner}${clear}</${box}>${fb}${o.filterOpen ? filterMenu(o, L) : ""}</div>`;
  }

  function tabs(o, L) {
    const all = [["all", L.tabAll], ...FAM.map((f) => [f.id, f[o.lang]])];
    return `<div class="o-tabs${o.off ? " is-off" : ""}" role="tablist">${all
      .map(([id, t]) => `<span class="o-tab${o.tab === id ? " is-on" : ""}" role="tab" tabindex="${o.tab === id ? 0 : -1}" aria-selected="${o.tab === id}" data-pt-tab="${id}">${t}</span>`)
      .join("")}</div>`;
  }

  function noMatch(o, L) {
    const line = o.tab === "all" ? L.noMatchAll : `${L.noMatchIn(famName(o.tab, o.lang))} <span class="t-link" role="button" tabindex="0" data-pt-tab="all">${L.searchAll}</span>`;
    return `<div class="o-empty"><strong>${o.q ? L.noMatchT(o.q) : L.shown(0)}</strong>${o.q ? line : ""}</div>`;
  }

  function win(o) {
    o = { lang: "en", tab: "all", q: "", show: "all", order: "game", src: "live", pinned: DEFAULT_PINS, pets: PETS, ...o };
    const L = S[o.lang];
    const head = `<div class="o-head"><span class="o-title">${icon("paw")}${L.title}</span><span class="grow"></span><button type="button" class="o-btn" aria-label="${L.close}" data-pt-close="1">${icon("close")}</button></div>`;
    const foot = `<div class="o-foot t-foot">${icon("info")}<span>${L.perLevel} <b>5</b> <span class="faint">·</span> <b>25</b> <span class="faint">·</span> <b>75</b></span><span class="grow"></span><span class="faint">${L.capLine}</span></div>`;
    const frame = (inner) => `<div class="ov o td pt" role="dialog" aria-label="${esc(L.title)}">${inner}</div>`;
    if (o.src === "none") {
      const off = { ...o, off: true, filterOpen: false };
      return frame(head + srcLine(o, L) + findRow(off, L) + tabs(off, L) + `<div class="o-empty"><strong>${L.emptyTitle}</strong>${L.emptyText(window.APP_NAME)}</div>` + foot);
    }
    const vis = visible(o);
    const body = vis.count ? list(o, vis, L) : noMatch(o, L);
    return frame(head + srcLine(o, L) + findRow(o, L) + tabs(o, L) + summary(tabPets(o), L) + filterLine(o, L, vis.count) + body + foot);
  }

  // ---- Soul tracker card ------------------------------------------------------
  // items: [{ p, lv, souls, ev: "gain" | "up" | "unlock" | "max", from }] in
  // arrival order (a newcomer is added at the end; nothing moves); at most
  // three. o: { lang, locked, hover (paused), left (time left, 0..1), live,
  //             enter, test }
  function gainOf(name, before, n) {
    const p = byName(name);
    const after = addSouls(before, n);
    const ev = before.lv === 0 && after.lv >= 1 ? "unlock" : after.lv > before.lv ? "up" : before.lv >= CAP ? "max" : "gain";
    return { p, ...after, ev };
  }

  // The bar's width: floor 4% while above zero, none at zero.
  const pctOf = (it) => {
    const v = it.lv >= CAP ? 100 : (it.souls / NEED[it.lv]) * 100;
    return v > 0 ? Math.max(4, v) : 0;
  };
  const barW = (it, o) => (o.live && it.from != null ? Math.max(4, it.from) : pctOf(it));
  // The timer ring's time left: full while live (pets.css runs it over 10 s).
  const lifeOf = (o) => (o.live ? 1 : o.left == null ? 0.62 : o.left);

  function card(items, o = {}) {
    o = { lang: "en", ...o };
    const L = S[o.lang];
    const x = o.locked ? "" : `<span class="pc-x${o.hover ? " is-hot" : ""}" role="button" aria-label="${L.dismiss}">${icon("close")}</span>`;
    const blocks = items
      .map((it) => {
        const chip = it.ev === "up" ? `<span class="pc-chip">${icon("chevronsUp")}${L.reached(it.lv)}</span>` : it.ev === "unlock" ? `<span class="pc-chip">${icon("sparkles")}${L.unlocked}</span>` : "";
        const next = it.lv < CAP ? `${icon("arrowRight")}<span>Lv ${it.lv + 1}</span>` : "";
        return `<div class="rm-ev pc-ev lv-${it.lv}"><div class="pc-top"><span class="rm-name">${esc(nameOf(it.p, o.lang))}</span>${chip}</div><div class="pc-fact"><span class="pc-lv">Lv ${it.lv}</span><span class="pc-fig">${figInner(it)}</span>${next}</div><div class="pc-bar"><i style="width:${barW(it, o).toFixed(1)}%" data-to="${pctOf(it).toFixed(1)}"></i></div></div>`;
      })
      .join("");
    const cls = ["ov rm pc", o.live && "pc-live is-live", o.enter && "is-enter", o.hover && !o.locked && "is-paused", o.locked && "is-locked"].filter(Boolean).join(" ");
    const title = o.test ? L.testCard : L.card;
    return `<div class="${cls}" role="status" aria-live="polite" aria-label="${esc(title)}"><div class="rm-head">${icon("paw")}<span class="rm-when">${title}</span>${Hub.ring(lifeOf(o), o.locked)}${x}</div>${blocks}</div>`;
  }

  // ---- Lists: the scroll thumb, drawn after a list is in the page -----------
  function afterRender(root) {
    root.querySelectorAll(".pt-scrollwrap").forEach((w) => {
      const sc = w.querySelector(".pt-scroll");
      const th = w.querySelector(".o-thumb");
      const upd = () => {
        const over = sc.scrollHeight > sc.clientHeight + 1;
        w.classList.toggle("is-over", over);
        if (!over) return;
        const h = Math.max(24, (sc.clientHeight * sc.clientHeight) / sc.scrollHeight);
        const t = (sc.scrollTop / (sc.scrollHeight - sc.clientHeight)) * (sc.clientHeight - h - 6);
        th.style.setProperty("--h", h + "px");
        th.style.setProperty("--t", t + 3 + "px");
      };
      upd();
      if (w.dataset.bound) return;
      w.dataset.bound = "1";
      const s = Number(w.dataset.scroll || 0);
      if (s) sc.scrollTop = s;
      upd();
      sc.addEventListener("scroll", upd, { passive: true });
    });
  }

  // Draws the thumbs whenever the page's scenes are drawn again.
  function watch(root) {
    afterRender(root);
    new MutationObserver(() => afterRender(root)).observe(root, { childList: true, subtree: true });
  }

  window.Pets = { win, card, gainOf, addSouls, afterRender, watch, icon, idOf, byName, PETS, DEFAULT_PINS, NEED, CAP };
})();
