// The pets round (TASK-34.3), 2026-10-10, not approved yet. Round 1 chose
// window layout A (meter rows) and card design A (the reminder card's twin);
// layouts B and C, cards B and C and the dropped items stay drawable with
// ?compare. Round 2 adds the level colours ("lvc": (a) the badge, (b) the
// badge and the fill, "none" round 1's look) and embeds Settings › Pets
// (settings.html). Also drawn: the toolbar's entry, the card's places, and
// a working tile. Styles: pets.css. Built on the approved drawing, which is
// not edited: the toolbar is `Hub.toolbar()` with the pets tool inserted
// after To-dos (as the updates round inserted its slot); the meter and the
// reminder card are `Hub.meter()` and `Hub.reminder()`.
//
// Everything here is invented: the pet list, families, levels and souls,
// times. Seven names come from public sources (Fossa, Kalgolem, Splendent
// Armadon, Hill Coradon, Forest Spirit, Slime, Krall Laborer); the rest are
// built from public creature names and invented words to fill 290 rows, and
// the families they sit in are guesses. The pet catalogue (TASK-34.2)
// supplies the real names (English and French), families and order.
// French pet names are drafts for the few fixed pets only. Nothing is saved
// or sent anywhere.
//
// Icons: Lucide (https://lucide.dev), ISC License; see ../approved/kit.js.
// paw-print, pin, search, list-filter, arrow-right, chevrons-up, sparkles
// are copied from gpui-kit-assets 0.6.6.
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

  // ---- Clock (mocked, as the hub round): Monday 21:48, logged in at 19:02 --
  const LOGIN = "19:02";
  const STALE = { en: "Sun 22:40", fr: "dim. 22:40" };
  const SERVER = "Ossia";

  // ---- The Global rule (agreed): 5 souls for Lv 0 -> 1, 25 for 1 -> 2, 75
  // for 2 -> 3; Lv 3 is the cap; souls beyond a level carry over. A pet at
  // the cap reads 75/75.
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

  // The pets named in the brief, with the values the states need.
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

  function rng(seed) {
    return function () {
      seed = (seed + 0x6d2b79f5) | 0;
      let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

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

  // ---- Strings (English; French drafted to check the room, through i18n) ---
  const LABEL = {
    pets: { en: "Pets", fr: "Familiers" },
    tracker: { en: "Pet tracker", fr: "Suivi des familiers" },
    souls: { en: "Pet souls", fr: "Âmes de familiers" },
  };

  const S = {
    en: {
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
      specialInAll: "Special pets in All",
      closeFirst: "closest first",
      noSpecial: "without Special pets",
      shown: (n) => `${n} shown`,
      clear: "Clear",
      emptyTitle: "No pets yet",
      emptyText: (app) => `The game sends your pets once, when you log in. Log in with a character while ${app} runs, and every pet shows here with its level and souls.`,
      noMatchT: (q) => `No pet matches “${esc(q)}”`,
      noMatchIn: (tab) => `Nothing in ${tab}.`,
      searchAll: "Search all families",
      noMatchAll: "No pet in any family has that in its name.",
      th: { pet: "Pet", lv: "Lv", souls: "Souls", left: "Left" },
      secSum: (o, n, m) => `<b>${o}</b>/${n} owned · <b>${m}</b> max`,
      card: "Pet souls",
      testCard: "Test card",
      reached: (lv) => `Lv ${lv} reached`,
      unlocked: "Unlocked",
      dismiss: "Dismiss",
      typedTip: "Typed by you. Your next login replaces it.",
      stopTyping: "Stop typing",
      pinnedPets: "Pinned pets",
      allPetsMenu: "All pets",
      pinCountTip: (nm, p, n) => [`${nm} · Lv ${p.lv} · ${p.souls}/${needOf(p)}`, `${n} pinned. Click for your pinned pets.`],
    },
    fr: {
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
      specialInAll: "Familiers spéciaux dans Tous",
      closeFirst: "les plus proches d'abord",
      noSpecial: "sans les spéciaux",
      shown: (n) => `${n} affichés`,
      clear: "Effacer",
      emptyTitle: "Aucun familier pour l'instant",
      emptyText: (app) => `Le jeu envoie vos familiers une fois, à la connexion. Connectez-vous avec un personnage pendant que ${app} tourne : chaque familier s'affiche ici avec son niveau et ses âmes.`,
      noMatchT: (q) => `Aucun familier ne correspond à « ${esc(q)} »`,
      noMatchIn: (tab) => `Rien dans ${tab}.`,
      searchAll: "Chercher dans toutes les familles",
      noMatchAll: "Aucun familier ne porte ce nom, toutes familles confondues.",
      th: { pet: "Familier", lv: "Lv", souls: "Âmes", left: "Reste" },
      secSum: (o, n, m) => `<b>${o}</b>/${n} obtenus · <b>${m}</b> max`,
      card: "Âmes de familier",
      testCard: "Carte de test",
      reached: (lv) => `Lv ${lv} atteint`,
      unlocked: "Débloqué",
      dismiss: "Fermer",
      typedTip: "Saisi par vous. Votre prochaine connexion le remplace.",
      stopTyping: "Arrêter la saisie",
      pinnedPets: "Familiers épinglés",
      allPetsMenu: "Tous les familiers",
      pinCountTip: (nm, p, n) => [`${nm} · Lv ${p.lv} · ${p.souls}/${needOf(p)}`, `${n} épinglés. Cliquer pour vos familiers épinglés.`],
    },
  };

  // ---- Window ------------------------------------------------------------
  // o: { layout "a" | "b" | "c", lang, label, tab, q, searchFocus,
  //      interactive (a real input), show "all" | "unowned" | "notmax",
  //      order "game" | "close", special "mark" | "hide", filterOpen,
  //      pinned [ids], pets, src "live" | "stale" | "none", srcTip,
  //      fallback (typed-in counts before any read), typed {id: {lv,
  //      souls}}, edit id, editVal, hover id, pinHover id, tip id, focus id,
  //      open [family ids unfolded in C's All tab], summary, scroll px }
  const figInner = (p) => `${p.souls}<small>/${needOf(p)}</small>`;

  function pinBtn(p, o, L) {
    const on = o.pinned.includes(p.id);
    const c = ["o-btn is-sm pt-pin", on && "is-on", o.pinHover === p.id && "is-hover", o.focus === p.id && "is-focus"].filter(Boolean).join(" ");
    const label = `${on ? L.unpin : L.pin} ${nameOf(p, o.lang)}`;
    return `<button type="button" class="${c}" data-pt-pin="${p.id}" aria-pressed="${on}" aria-label="${esc(label)}">${icon("pin")}</button>`;
  }

  const tipFor = (p, o, L) => (o.tip === p.id ? `<div class="t-tip" role="tooltip">${o.pinned.includes(p.id) ? L.unpinTip : L.pinTip}</div>` : "");

  // A pet as the row shows it: typed values over the game's, or unknown in
  // the typed-in fallback before any read.
  function shown(p0, o) {
    const t = o.typed && o.typed[p0.id];
    if (t) return { ...p0, ...t, typed: true };
    if (o.fallback) return { ...p0, unknown: true };
    return p0;
  }

  const tagOf = (p, o, L, cls = "") => (p.fam === "special" && o.special === "mark" ? `<small${cls ? ` class="${cls}"` : ""}>${L.noStats}</small>` : "");

  function editHtml(p, o, L) {
    return `<span class="pt-edit"><span class="pt-lvsel" role="combobox" aria-label="Level">Lv ${p.lv}${icon("chevronDown")}</span><span class="t-field"><span class="val">${esc(o.editVal)}</span><span class="o-caret"></span><span class="t-cap">/${needOf(p)}</span><button type="button" class="o-btn is-sm t-clear" aria-label="${L.stopTyping}">${icon("close")}</button></span></span>`;
  }

  // Layout A: the meter's row.
  function rowA(p0, o, L) {
    const p = shown(p0, o);
    const max = !p.unknown && p.lv >= CAP;
    const none = p.unknown || (p.lv === 0 && p.souls === 0);
    const cls = ["pt-row", !p.unknown && `lv-${p.lv}`, max && "is-max", none && "is-none", o.hover === p.id && "is-hover", o.isNew && "is-new"].filter(Boolean).join(" ");
    const pct = p.unknown ? 0 : progress(p);
    const fill = pct > 0 ? `<span class="pt-fill" style="width:${Math.max(4, pct * 100).toFixed(1)}%"></span>` : "";
    let right;
    if (o.edit === p.id) right = editHtml(p, o, L);
    else {
      const fig = p.unknown ? `<span class="pt-fig is-blank">&mdash;</span>` : `<span class="pt-fig">${figInner(p)}</span>`;
      const to = p.unknown ? `<span class="pt-to"></span>` : max ? `<span class="pt-to is-max">${L.max}</span>` : `<span class="pt-to">${icon("arrowRight")}Lv ${p.lv + 1}</span>`;
      right = (p.typed ? `<span class="pt-typed" title="${esc(L.typedTip)}">${icon("pencil")}</span>` : "") + fig + to;
    }
    const lv = p.unknown ? "&mdash;" : `Lv ${p.lv}`;
    return `<div class="${cls}"><span class="pt-lv">${lv}</span><span class="pt-track">${fill}<span class="pt-name">${esc(nameOf(p, o.lang))}${tagOf(p, o, L)}</span>${right}</span>${o.nopin ? "" : pinBtn(p, o, L)}${tipFor(p, o, L)}</div>`;
  }

  // Layout B: a table row.
  function rowB(p0, o, L) {
    const p = shown(p0, o);
    const max = !p.unknown && p.lv >= CAP;
    const none = p.unknown || (p.lv === 0 && p.souls === 0);
    const cls = ["pt-trow", max && "is-max", none && "is-none", o.hover === p.id && "is-hover"].filter(Boolean).join(" ");
    const gut = p.typed ? icon("pencil") : "";
    let cols;
    if (o.edit === p.id) cols = editHtml(p, o, L);
    else {
      const lv = p.unknown ? "&mdash;" : p.lv;
      const souls = p.unknown ? "&mdash;" : figInner(p);
      const left = p.unknown || max ? "&mdash;" : leftOf(p);
      cols = `<span class="pt-c c-lv">${lv}</span><span class="pt-c c-souls">${souls}</span><span class="pt-c c-left${p.unknown || max ? " is-none" : ""}">${left}</span>`;
    }
    return `<div class="${cls}"><span class="pt-c c-gut">${gut}</span><span class="pt-name">${esc(nameOf(p, o.lang))}${tagOf(p, o, L)}</span>${cols}${pinBtn(p, o, L)}${tipFor(p, o, L)}</div>`;
  }

  // Layout C: a to-do row with level pips.
  function rowC(p0, o, L) {
    const p = shown(p0, o);
    const max = !p.unknown && p.lv >= CAP;
    const none = p.unknown || (p.lv === 0 && p.souls === 0);
    const cls = ["t-row pt-crow", max && "is-max", none && "is-none", o.hover === p.id && "is-hover"].filter(Boolean).join(" ");
    let right;
    if (o.edit === p.id) right = editHtml(p, o, L);
    else {
      const pips = `<span class="pt-pips" role="img" aria-label="Lv ${p.unknown ? "?" : p.lv}">${[1, 2, 3].map((i) => `<i${!p.unknown && p.lv >= i ? ' class="on"' : ""}></i>`).join("")}</span>`;
      right = `${pips}<span class="pt-cfig">${p.unknown ? "&mdash;" : figInner(p)}</span>`;
    }
    return `<div class="${cls}"><span class="t-gut">${p.typed ? icon("pencil") : ""}</span><span class="t-name">${esc(nameOf(p, o.lang))}${tagOf(p, o, L, "pt-tag")}</span>${right}${pinBtn(p, o, L)}${tipFor(p, o, L)}</div>`;
  }

  function tabPets(o) {
    return o.pets.filter((p) => (o.tab === "all" ? !(o.special === "hide" && p.fam === "special") : p.fam === o.tab));
  }

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

  const scrollWrap = (inner, o) => `<div class="pt-scrollwrap" data-scroll="${o.scroll || 0}"><div class="pt-scroll" role="list">${inner}</div><span class="o-thumb" aria-hidden="true"></span></div>`;

  const label = (ic, text, n) => `<div class="t-sec is-static">${icon(ic)}<span>${text}</span><span class="n">${n}</span></div>`;

  function listA(o, vis, L) {
    const rows = (arr) => `<div class="pt-rows">${arr.map((p) => rowA(p, o, L)).join("")}</div>`;
    let h;
    if (vis.pins.length) {
      const restLabel = o.tab === "all" ? L.allPets : famName(o.tab, o.lang);
      h = label("pin", L.pinned, vis.pins.length) + rows(vis.pins) + (vis.rest.length ? label("paw", restLabel, vis.rest.length) + rows(vis.rest) : "");
    } else h = rows(vis.rest);
    return scrollWrap(h, o);
  }

  function listB(o, vis, L) {
    const active = (k) => (o.order === "close" ? k === "left" : k === "pet");
    const head = (k, text) => (active(k) ? `<span class="is-sort" data-pt-order="${k === "left" ? "close" : "game"}">${text}${icon("arrowDown")}</span>` : `<span class="can-sort" data-pt-order="${k === "left" ? "close" : "game"}">${text}</span>`);
    const th = `<div class="o-th pt-th"><span class="pt-c c-gut"></span><span class="c-pet">${head("pet", L.th.pet)}<small>${vis.count}</small></span><span class="pt-c c-lv">${L.th.lv}</span><span class="pt-c c-souls">${L.th.souls}</span><span class="pt-c c-left">${head("left", L.th.left)}</span><span class="pt-c c-pin"></span></div>`;
    const rows = (arr) => arr.map((p) => rowB(p, o, L)).join("");
    const inner = `<div class="pt-trows">${rows(vis.pins)}${vis.pins.length && vis.rest.length ? '<div class="pt-gap"></div>' : ""}${rows(vis.rest)}</div>`;
    return th + scrollWrap(inner, o);
  }

  function famSummary(list) {
    return { n: list.length, owned: list.filter((p) => p.lv >= 1).length, max: list.filter((p) => p.lv >= CAP).length };
  }

  function listC(o, vis, L) {
    let h = "";
    if (vis.pins.length) h += label("pin", L.pinned, vis.pins.length) + `<div class="t-list">${vis.pins.map((p) => rowC(p, o, L)).join("")}</div>`;
    const fams = o.tab === "all" ? FAM.filter((f) => !(o.special === "hide" && f.id === "special")) : FAM.filter((f) => f.id === o.tab);
    for (const f of fams) {
      const rows = vis.rest.filter((p) => p.fam === f.id);
      if ((o.q || o.show !== "all") && !rows.length) continue;
      const s = famSummary(o.pets.filter((p) => p.fam === f.id));
      const fold = o.tab === "all" && !o.q;
      const open = !fold || (o.open || []).includes(f.id);
      const chev = fold ? icon(open ? "chevronDown" : "chevronRight") : icon("paw");
      const attrs = fold ? ` role="button" tabindex="0" aria-expanded="${open}" data-pt-fold="${f.id}"` : "";
      const sum = o.fallback ? "" : `<span class="at">${L.secSum(s.owned, s.n, s.max)}</span>`;
      h += `<div class="t-sec${fold ? "" : " is-static"}"${attrs}>${chev}<span>${f[o.lang]}</span><span class="n">${rows.length}</span>${sum}</div>`;
      if (open) h += `<div class="t-list">${rows.map((p) => rowC(p, o, L)).join("")}</div>`;
    }
    return scrollWrap(h, o);
  }

  function summaryHtml(list, L) {
    const s = famSummary(list);
    const lv = list.reduce((a, p) => a + p.lv, 0);
    const all = list.length * CAP;
    return `<div class="pt-sum"><span>${L.owned(s.owned, s.n)}</span><span class="faint">·</span><span>${L.maxed(s.max)}</span><span class="grow"></span><span>${L.levels} <b>${lv}</b><small>/${all}</small></span></div><div class="pt-prog" role="progressbar" aria-label="${L.levels}" aria-valuemin="0" aria-valuemax="${all}" aria-valuenow="${lv}"><i style="width:${((lv / all) * 100).toFixed(1)}%"></i></div>`;
  }

  function filterMenu(o, L) {
    const item = (key, val, text, cur, role = "menuitemradio") =>
      `<div class="o-menu-item${cur ? " is-cur" : ""}" role="${role}" aria-checked="${cur}" data-pt-${key}="${val}"><span class="lead">${cur ? icon("check") : ""}</span><span>${text}</span></div>`;
    let h = `<div class="hb-drop-label">${L.show}</div>`;
    h += item("show", "all", L.showAll, o.show === "all") + item("show", "unowned", L.showUnowned, o.show === "unowned") + item("show", "notmax", L.showNotMax, o.show === "notmax");
    // Layout B orders from its table header.
    if (o.layout !== "b") h += `<div class="o-menu-sep"></div><div class="hb-drop-label">${L.order}</div>` + item("order", "game", L.orderGame, o.order === "game") + item("order", "close", L.orderClose, o.order === "close");
    h += `<div class="o-menu-sep"></div>` + item("special", "toggle", L.specialInAll, o.special !== "hide", "menuitemcheckbox");
    return `<div class="o-menu pt-menu" role="menu" aria-label="${L.filter}">${h}</div>`;
  }

  const filterOn = (o) => o.show !== "all" || o.order !== "game" || o.special === "hide";

  function filterLine(o, L, count) {
    const parts = [];
    if (o.show === "unowned") parts.push(L.showUnowned);
    if (o.show === "notmax") parts.push(L.showNotMax);
    if (o.order === "close") parts.push(L.closeFirst);
    if (o.special === "hide" && o.tab === "all") parts.push(L.noSpecial);
    if (!parts.length) return "";
    const dot = ' <span class="faint">·</span> ';
    return `<div class="pt-fline">${icon("listFilter")}<span>${parts.join(dot)}${dot}<span class="faint">${L.shown(count)}</span></span><span class="t-link" role="button" tabindex="0" data-pt-clear="1">${L.clear}</span></div>`;
  }

  function srcLine(o, L) {
    const dot = '<span class="faint">·</span>';
    const app = window.APP_NAME;
    let t;
    if (o.src === "stale") t = L.srcStale(STALE[o.lang], dot);
    else if (o.src === "none") t = L.srcNone(app);
    else t = L.srcLive(LOGIN, SERVER, dot);
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

  function tabsHtml(o, L) {
    const tabs = [["all", L.tabAll], ...FAM.map((f) => [f.id, f[o.lang]])];
    return `<div class="o-tabs${o.off ? " is-off" : ""}" role="tablist">${tabs
      .map(([id, t]) => `<span class="o-tab${o.tab === id ? " is-on" : ""}" role="tab" tabindex="${o.tab === id ? 0 : -1}" aria-selected="${o.tab === id}" data-pt-tab="${id}">${t}</span>`)
      .join("")}</div>`;
  }

  function noMatch(o, L) {
    const tab = o.tab === "all" ? "" : famName(o.tab, o.lang);
    const line = o.q
      ? o.tab === "all"
        ? L.noMatchAll
        : `${L.noMatchIn(tab)} <span class="t-link" role="button" tabindex="0" data-pt-tab="all">${L.searchAll}</span>`
      : "";
    return `<div class="o-empty"><strong>${o.q ? L.noMatchT(o.q) : L.shown(0)}</strong>${line}</div>`;
  }

  function win(o) {
    o = {
      layout: "a",
      lvc: OPTS.lvc,
      lang: "en",
      label: "pets",
      tab: "all",
      q: "",
      show: "all",
      order: "game",
      special: "mark",
      src: "live",
      summary: true,
      pinned: DEFAULT_PINS,
      pets: PETS,
      ...o,
    };
    const L = S[o.lang];
    const title = LABEL[o.label][o.lang];
    const head = `<div class="o-head"><span class="o-title">${icon("paw")}${title}</span><span class="grow"></span><button type="button" class="o-btn" aria-label="${L.close}" data-pt-close="1">${icon("close")}</button></div>`;
    const foot = `<div class="o-foot t-foot">${icon("info")}<span>${L.perLevel} <b>5</b> <span class="faint">·</span> <b>25</b> <span class="faint">·</span> <b>75</b></span><span class="grow"></span><span class="faint">${L.capLine}</span></div>`;
    const frame = (inner) => `<div class="ov o td pt is-${o.layout}${lvcCls(o.lvc)}" role="dialog" aria-label="${esc(title)}">${inner}</div>`;
    if (o.src === "none" && !o.fallback) {
      const off = { ...o, off: true, filterOpen: false };
      return frame(head + srcLine(o, L) + findRow(off, L) + tabsHtml(off, L) + `<div class="o-empty"><strong>${L.emptyTitle}</strong>${L.emptyText(window.APP_NAME)}</div>` + foot);
    }
    const vis = visible(o);
    const sum = o.layout !== "c" && o.summary && !o.fallback ? summaryHtml(tabPets(o), L) : "";
    let body;
    if (!vis.count) body = noMatch(o, L);
    else if (o.layout === "b") body = listB(o, vis, L);
    else if (o.layout === "c") body = listC(o, vis, L);
    else body = listA(o, vis, L);
    return frame(head + srcLine(o, L) + findRow(o, L) + tabsHtml(o, L) + sum + filterLine(o, L, vis.count) + body + foot);
  }

  // ---- Soul tracker card --------------------------------------------------
  // items: [{ p, lv, souls, ev: "gain" | "up" | "unlock" | "max", from }]
  // in arrival order (a new pet is added at the end; nothing moves); at
  // most three. o: { lang, locked, hover, left (0..1), live, enter }
  function gainOf(name, before, n) {
    const p = byName(name);
    const after = addSouls(before, n);
    const ev = before.lv === 0 && after.lv >= 1 ? "unlock" : after.lv > before.lv ? "up" : before.lv >= CAP ? "max" : "gain";
    return { p, ...after, ev };
  }

  const isNew = (it) => it.ev === "up" || it.ev === "unlock";
  // The bar's width: the window's floor, 4% while above zero, none at zero.
  const pctOf = (it) => {
    const v = it.lv >= CAP ? 100 : (it.souls / NEED[it.lv]) * 100;
    return v > 0 ? Math.max(4, v) : 0;
  };
  const barW = (it, o) => (o.live && it.from != null ? Math.max(4, it.from) : pctOf(it));
  const lifeW = (o) => (o.live ? 100 : (o.left == null ? 0.62 : o.left) * 100);

  // Level colours (round 2): "a" tints the level badge by level, "b" also
  // tints the progress fill; "none" is round 1's look.
  const lvcCls = (v) => (v === "a" || v === "b" ? ` has-lvc lvc-${v}` : "");

  function cardA(items, o) {
    const L = S[o.lang];
    const x = o.locked ? "" : `<span class="pc-x${o.hover ? " is-hot" : ""}" role="button" aria-label="${L.dismiss}">${icon("close")}</span>`;
    const blocks = items
      .map((it) => {
        const chip = it.ev === "up" ? `<span class="pc-chip">${icon("chevronsUp")}${L.reached(it.lv)}</span>` : it.ev === "unlock" ? `<span class="pc-chip">${icon("sparkles")}${L.unlocked}</span>` : "";
        const next = it.lv < CAP ? `${icon("arrowRight")}<span>Lv ${it.lv + 1}</span>` : "";
        return `<div class="rm-ev pc-ev lv-${it.lv}"><div class="pc-top"><span class="rm-name">${esc(nameOf(it.p, o.lang))}</span>${chip}</div><div class="pc-fact"><span class="pc-lv">Lv ${it.lv}</span><span class="faint pc-dot">·</span><span class="pc-fig">${figInner(it)}</span>${next}</div><div class="pc-bar"><i style="width:${barW(it, o).toFixed(1)}%" data-to="${pctOf(it).toFixed(1)}"></i></div></div>`;
      })
      .join("");
    const cls = ["ov rm pc", o.live && "pc-live", o.enter && "is-enter"].filter(Boolean).join(" ") + lvcCls(o.lvc);
    return `<div class="${cls}" role="status" aria-live="polite" aria-label="${esc(o.test ? L.testCard : L.card)}"><div class="rm-head">${icon("paw")}<span class="rm-when">${o.test ? L.testCard : L.card}</span>${x}</div>${blocks}<div class="rm-left"><i style="width:${lifeW(o)}%"></i></div></div>`;
  }

  function cardB(items, o) {
    const L = S[o.lang];
    const rows = items
      .map((it) => {
        let html = rowA({ ...it.p, lv: it.lv, souls: it.souls }, { lang: o.lang, pinned: [], nopin: true, isNew: isNew(it), special: "plain" }, L);
        if (o.live && it.from != null) html = html.replace(/(class="pt-fill" style="width:)[\d.]+%"/, `$1${Math.max(4, it.from).toFixed(1)}%" data-to="${Math.max(4, pctOf(it)).toFixed(1)}"`);
        return html;
      })
      .join("");
    const cls = ["ov o pc-strip", o.live && "pc-live", o.enter && "is-enter"].filter(Boolean).join(" ");
    return `<div class="${cls}" role="status" aria-live="polite" aria-label="${esc(L.card)}"><div class="pt-rows">${rows}</div><div class="rm-left"><i style="width:${lifeW(o)}%"></i></div></div>`;
  }

  function cardC(items, o) {
    const L = S[o.lang];
    const pills = items
      .map((it) => {
        const ic = it.ev === "unlock" ? "sparkles" : it.ev === "up" ? "chevronsUp" : "paw";
        const to = it.lv < CAP ? `<span class="to">${icon("arrowRight")}Lv ${it.lv + 1}</span>` : "";
        return `<div class="ov o pc-pill${isNew(it) ? " is-new" : ""}">${icon(ic)}<span class="nm">${esc(nameOf(it.p, o.lang))}</span><span class="dot">·</span><span class="lv">Lv ${it.lv}</span><span class="fig">${figInner(it)}</span>${to}</div>`;
      })
      .join("");
    const cls = ["pc-stack", o.live && "pc-live", o.enter && "is-enter"].filter(Boolean).join(" ");
    return `<div class="${cls}" role="status" aria-live="polite" aria-label="${esc(L.card)}">${pills}${o.live ? '<span class="pc-life"><i></i></span>' : ""}</div>`;
  }

  function card(design, items, o = {}) {
    o = { lang: "en", lvc: OPTS.lvc, ...o };
    if (design === "b") return cardB(items, o);
    if (design === "c") return cardC(items, o);
    return cardA(items, o);
  }

  // ---- Toolbar: the approved drawing, the pets tool inserted after To-dos -
  const CTX = {
    opts: { mask: false, cp: true },
    fmt: {
      int: (n) => Math.round(n).toLocaleString("en-US"),
      compact: (n) => (n < 10000 ? Math.round(n).toLocaleString("en-US") : n < 1e6 ? (n / 1e3).toFixed(1) + "K" : (n / 1e6).toFixed(2) + "M"),
      pct: (n) => (n * 100 >= 99.95 ? "100%" : (n * 100).toFixed(1) + "%"),
      time: (s) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`,
      esc,
      mask: (s) => s,
    },
    CLASS_RGB: { gladiator: "251, 146, 60", ranger: "163, 230, 53", sorcerer: "232, 121, 249", chanter: "52, 211, 153", templar: "56, 189, 248" },
    classIcon: (c) => `../../../crates/app/assets/game/class/${c}.png`,
    sys: { pingMs: 42, cpu: 18, memGb: 11.2 },
  };

  function pinCount(o, L) {
    const pins = (o.pinned || DEFAULT_PINS).map((id) => PETS.find((p) => p.id === id));
    const first = pins[0];
    const nm = nameOf(first, o.lang);
    const [a, b] = L.pinCountTip(nm, first, pins.length);
    const tip = o.tip === "pinned" ? `<span class="hb-tip is-on" role="tooltip">${esc(a)}<span>${b}</span></span>` : "";
    const drop = o.pinOpen
      ? `<div class="o-menu hb-drop" role="menu"><div class="hb-drop-label">${icon("pin")}${L.pinnedPets}</div>${pins
          .map((p) => `<div class="o-menu-item" role="menuitem"><span class="nm">${esc(nameOf(p, o.lang))}</span><span class="pt-dlv">Lv ${p.lv}</span><span class="fig pt-dfig">${figInner(p)}</span></div>`)
          .join("")}<div class="o-menu-sep"></div><div class="o-menu-item" role="menuitem"><span class="lead">${icon("paw")}</span><span class="nm">${L.allPetsMenu}</span>${icon("chevronRight")}</div></div>`
      : "";
    const c = ["hb-count pt-pc", o.hover === "pinned" && "is-hover", o.pinOpen && "is-open"].filter(Boolean).join(" ");
    return `<span style="position:relative;display:inline-flex"><button type="button" class="${c}" aria-haspopup="menu" aria-expanded="${!!o.pinOpen}" aria-label="${esc(`${L.pinnedPets}: ${a}`)}">${icon("paw")}<span class="hb-fig">${figInner(first)}</span>${tip}</button>${drop}</span>`;
  }

  function bar(o = {}) {
    const lang = o.lang || "en";
    const L = S[lang];
    let html = Hub.toolbar(CTX, { v: Hub.view(), lang, lit: o.lit, hover: o.hover, tip: o.tip, focus: o.focus, live: o.live });
    const lit = o.lit && o.lit.pets;
    const title = LABEL[o.label || "pets"][lang];
    const c = ["hb-btn", lit && "is-lit", o.hover === "pets" && "is-hover", o.focus === "pets" && "is-focus"].filter(Boolean).join(" ");
    const tool = `<button type="button" class="${c}" data-hb="pets" data-pt-tool="1" aria-label="${esc(title)}" aria-pressed="${!!lit}">${icon("paw")}<span class="hb-tip${o.tip === "pets" ? " is-on" : ""}" role="tooltip">${esc(title)}</span></button>`;
    // Since the approval the toolbar draws a pets tool itself; this round's
    // own tool (with the name selector) takes its place.
    html = html.replace(/<button[^>]*data-hb="pets"[^>]*>[\s\S]*?<\/button>/, "");
    html = html.replace(/(<button[^>]*data-hb="todos"[^>]*>[\s\S]*?<\/button>)/, `$1${tool}`);
    if (o.pinCount) {
      const at = html.lastIndexOf('<span class="hb-vr"');
      html = html.slice(0, at) + pinCount({ ...o, lang }, L) + html.slice(at);
    }
    return html;
  }

  // ---- Page harness ---------------------------------------------------------
  // The approved pages' controls (opacity, scale, scene) plus this round's
  // selectors; tiles grouped by section so options sit side by side.
  const params = new URLSearchParams(location.search);
  const OPTS = {
    alpha: Number(params.get("alpha") || 150),
    scale: Number(params.get("scale") || 100),
    scene: params.get("scene") || "dark",
    lang: params.get("lang") || "en",
    win: params.get("win") || "a",
    card: params.get("card") || "a",
    label: params.get("label") || "pets",
    lvc: ["a", "b", "none"].includes(params.get("lvc")) ? params.get("lvc") : "a",
  };

  const TAGS = { rec: ["is-rec", "Recommended"], proposed: ["is-proposed", "Proposed"], agreed: ["is-agreed", "Agreed"], open: ["is-open", "Open"] };
  const tagHtml = (t) => (Array.isArray(t) ? t : [t]).map((k) => `<span class="dec ${TAGS[k][0]}">${TAGS[k][1]}</span>`).join("");

  const col = (...parts) => `<div class="hb-col">${parts.join("")}</div>`;
  const meter = () => Hub.meter(CTX, { board: "session" });

  function fx(seed) {
    let s = seed * 9301 + 49297;
    const rnd = () => (s = (s * 9301 + 49297) % 233280) / 233280;
    const kinds = [
      ["hit", () => Math.round(2000 + rnd() * 18000).toLocaleString("en-US")],
      ["crit", () => Math.round(20000 + rnd() * 40000).toLocaleString("en-US")],
      ["heal", () => "+" + Math.round(800 + rnd() * 4000).toLocaleString("en-US")],
    ];
    let out = "";
    for (let i = 0; i < 6; i++) {
      const [cls, text] = kinds[Math.floor(rnd() * kinds.length)];
      out += `<span class="${cls}" style="left:${(rnd() * 88).toFixed(1)}%;top:${(rnd() * 90).toFixed(1)}%">${text()}</span>`;
    }
    out += `<span class="plate" style="left:${(55 + rnd() * 25).toFixed(1)}%;top:${(6 + rnd() * 10).toFixed(1)}%">Hill Coradon</span>`;
    return `<div class="fx" aria-hidden="true">${out}</div>`;
  }

  // Round 1's comparison (window B and C, card B and C, the dropped items)
  // is kept reachable with ?compare.
  const COMPARE = params.has("compare");
  const LVC_NAME = { a: "(a) the badge only", b: "(b) the badge and the fill", none: "round 1, no level colours" };

  function sections() {
    const lang = OPTS.lang;
    const W = (o) => win({ layout: OPTS.win, lang, label: OPTS.label, ...o });
    const C = (items, o = {}) => card(OPTS.card, items, { lang, ...o });
    const fossa = idOf("Fossa");
    const armadon = idOf("Splendent Armadon");
    const coradon = idOf("Hill Coradon");
    const layoutName = { a: "A, meter rows", b: "B, a table", c: "C, sections by family" }[OPTS.win];
    const cardName = { a: "A, the reminder card's twin", b: "B, the window's rows", c: "C, a pill" }[OPTS.card];
    const lvcName = LVC_NAME[OPTS.lvc];
    const G = {
      gain: gainOf("Fossa", { lv: 2, souls: 27 }, 1),
      up: gainOf("Hill Coradon", { lv: 1, souls: 24 }, 5),
      unlock: gainOf("Forest Spirit", { lv: 0, souls: 4 }, 1),
      max: gainOf("Kalgolem", { lv: 3, souls: 75 }, 1),
      cap: gainOf("Splendent Armadon", { lv: 2, souls: 70 }, 5),
      slime: gainOf("Slime", { lv: 1, souls: 12 }, 1),
      market: gainOf("Slime", { lv: 1, souls: 12 }, 5),
    };
    // The same windows for (a) and (b), side by side in one tile.
    const pair = (o) => `<div class="hb-row pt-pair">${win({ layout: "a", lang, label: OPTS.label, tab: "fera", lvc: "a", ...o })}${win({ layout: "a", lang, label: OPTS.label, tab: "fera", lvc: "b", ...o })}</div>`;
    const cards = (lvc, o = {}) => `<div class="hb-col">${[[G.unlock], [G.gain], [G.max], [G.up, G.slime]].map((it) => card("a", it, { lang, lvc, ...o })).join("")}</div>`;
    const cardPair = (o = {}) => `<div class="hb-row pt-pair">${cards("a", o)}${cards("b", o)}</div>`;
    return [
      {
        id: "colours",
        title: "Level colours in the window",
        sub: "round 2: Lv 0 grey, Lv 1 to Lv 3 greens from the good token, strongest at Lv 3; \"Lv N\" stays. Fera, 92 pets, 25 of them at Lv 3. Pick (a) or (b)",
        tileMin: 400,
        stageMin: 690,
        tiles: [
          { tag: ["proposed", "rec"], title: "(a) The badge only", sub: "the level badge takes the level's green; the bar stays the neutral cream, so the fill still reads as progress only", html: win({ layout: "a", lang, label: OPTS.label, tab: "fera", lvc: "a" }) },
          { tag: "proposed", title: "(b) The badge and the fill", sub: "the progress fill takes the same green too; every Lv 3 row is a full green bar", html: win({ layout: "a", lang, label: OPTS.label, tab: "fera", lvc: "b" }) },
          { tag: "agreed", title: "Round 1, for comparison", sub: "as approved in round 1: neutral badges, Lv 3 in good", html: win({ layout: "a", lang, label: OPTS.label, tab: "fera", lvc: "none" }) },
        ],
      },
      {
        id: "colours-game",
        title: "Level colours over the game",
        sub: "(a) left, (b) right, at fixed scenes and opacities (the selectors at the top do not change this section). Opacity is Settings › Overlay's background, 0 to 255",
        tileMin: 800,
        stageMin: 690,
        tiles: [
          { title: "Dark scene, opacity 150 (the default)", sub: "", scene: "dark", alpha: 150, html: pair() },
          { title: "Bright scene, opacity 150", sub: "the worst case for a translucent card", scene: "bright", alpha: 150, html: pair() },
          { title: "Bright scene, opacity 60", sub: "a nearly clear card over snow", scene: "bright", alpha: 60, html: pair() },
          { title: "Busy scene, opacity 255", sub: "an opaque card", scene: "busy", alpha: 255, html: pair() },
          { title: "Dark scene, opacity 60", sub: "", scene: "dark", alpha: 60, html: pair({ scroll: 300 }) },
          { title: "Busy scene, opacity 150, scrolled", sub: "the middle of Fera", scene: "busy", alpha: 150, html: pair({ scroll: 600 }) },
        ],
      },
      {
        id: "colours-card",
        title: "Level colours in the card",
        sub: "card A: the fact line's \"Lv N\" becomes the window's level badge; (b) also tints the card's 4 px bar. A first unlock (Lv 1), a gain (Lv 2), a pet at Lv 3, a level reached beside another pet. (a) left, (b) right",
        tileMin: 760,
        stageMin: 470,
        tiles: [
          { title: "Dark scene, opacity 150", sub: "", scene: "dark", alpha: 150, html: cardPair() },
          { title: "Bright scene, opacity 150", sub: "", scene: "bright", alpha: 150, html: cardPair() },
          { title: "Bright scene, opacity 60", sub: "", scene: "bright", alpha: 60, html: cardPair() },
          { tag: "agreed", title: "Round 1, for comparison", sub: "", scene: "dark", alpha: 150, html: cards("none") },
        ],
      },
      {
        id: "window-layouts",
        r1: true,
        title: "Round 1: the pets window, three layouts",
        sub: "A was chosen; kept for reference",
        tileMin: 400,
        stageMin: 690,
        tiles: [
          { tag: "agreed", title: "A. Meter rows (chosen)", sub: "Each pet on the meter's bar: a level badge, the name, souls toward the next level as the bar's fill and as \"27/75 → Lv 3\". Lv 3 reads \"75/75 Max\". Pinned on top under their own label. Family summary line under the tabs", html: win({ layout: "a", lang, label: OPTS.label }) },
          { title: "B. A table", sub: "The fight report's death rows: columns Lv, Souls, Left (souls to the next level). Click Left to sort closest first, Pet for game order", html: win({ layout: "b", lang, label: OPTS.label }) },
          { title: "C. Sections by family", sub: "The to-do window's sections and rows. In All, every family folds to one line with its summary; the level as three pips", html: win({ layout: "c", lang, label: OPTS.label }) },
        ],
      },
      {
        id: "window-states",
        title: `The window's states, in layout ${layoutName}, level colours ${lvcName}`,
        sub: "the selectors at the top switch the level colours (and, for reference, round 1's layouts)",
        tileMin: 400,
        stageMin: 690,
        tiles: [
          { tag: "agreed", title: "Fera, a long family", sub: "92 pets in game order; the list scrolls past 452 px with the 3 px thumb (scroll it). Never obtained: Lv 0, 0/5, quiet", html: W({ tab: "fera" }) },
          { tag: "agreed", title: "Scrolled down", sub: "pets never obtained among the owned ones (Thornback Boar, others): Lv 0 · 0/5, no bar, the name in muted", html: W({ tab: "fera", scroll: 820 }) },
          { tag: "agreed", title: "Pointer on a row, then on its pin", sub: "the pin shows only while the pointer is in the row (enter and leave, never on moves inside), as the to-do window's ×; tooltip \"Pin to the top\" under it. Rows never move on pointer over", html: W({ tab: "fera", hover: armadon, pinHover: armadon, tip: armadon, scroll: 0 }) },
          { tag: "agreed", title: "Pinned in a family tab", sub: "Hill Coradon pinned too: a tab shows its own family's pinned pets on top (Kalgolem, a Varian, stays in All and Varian). Pins are kept per server, with the collection", html: W({ tab: "fera", pinned: [fossa, idOf("Kalgolem"), coradon] }) },
          { tag: "agreed", title: "Searching", sub: "typing \"co\" in All: the window takes the keyboard only while the field has focus (the PvP name field's rule). Matches anywhere in the name, pinned first; Esc clears, a second Esc gives the keyboard back to the game", html: W({ q: "co", searchFocus: true }) },
          { tag: "agreed", title: "No match in this tab", sub: "\"kalg\" in Fera: the empty state names the tab and offers all families", html: W({ tab: "fera", q: "kalg", searchFocus: true }) },
          { tag: "agreed", title: "Show and order (menu open)", sub: "the filter button beside the search: Show all / not owned yet / not at Lv 3; Order: game order / closest to the next level; Special pets in All", html: W({ tab: "fera", filterOpen: true }) },
          { tag: "agreed", title: "A filter in force", sub: "not at Lv 3, closest first: the button is lit with the dot, a line says what is kept and how many, Clear puts everything back. Closest = fewest souls left to the next level", html: W({ tab: "fera", show: "notmax", order: "close" }) },
          { tag: "agreed", title: "Special pets, marked", sub: "they give no stats: a faint \"no stats\" after the name, in All and in their tab", html: W({ tab: "special" }) },
          { r1: true, title: "Special pets left out of All (not chosen)", sub: "the menu's tick unticked: All and its summary leave them out; their tab still lists them", html: W({ special: "hide", scroll: 0 }) },
          { tag: "agreed", title: "As of an earlier login", sub: "Belisk was not running at this login: the window shows the last saved collection plus the gains it has seen since; the source line says when it was read; pointer on it", html: W({ src: "stale", srcTip: true }) },
          { tag: "agreed", title: "Nothing read yet", sub: "the game sends the collection once, at login. Before that: the empty state, the search and tabs dimmed", html: W({ src: "none" }) },
          { r1: true, title: "Typed-in fallback (dropped)", sub: "before any read every pet shows \"—\"; a click on a figure types a level and souls", html: W({ src: "none", fallback: true, tab: "fera", typed: { [fossa]: { lv: 2, souls: 26 } }, edit: coradon, editVal: "24" }) },
          { tag: "agreed", title: "Keyboard focus on a pin", sub: "Tab: search, filter, tabs (Left and Right), then the rows' pins (Up and Down); Space pins. The ring 1 px outside the button. The window never takes the keyboard otherwise", html: W({ tab: "fera", focus: armadon }) },
          { tag: "agreed", title: "In French", sub: "Familiers; tabs Tous … Spécial; Lv kept as in game. Pet names come from the catalogue (drafts here for a few)", html: win({ layout: OPTS.win, lang: "fr", label: OPTS.label, q: "", tab: "fera" }) },
        ],
      },
      {
        id: "card-designs",
        r1: true,
        title: "Round 1: the soul tracker card, three designs",
        sub: "A was chosen; kept for reference",
        tileMin: 340,
        stageMin: 170,
        tiles: [
          { tag: "agreed", title: "A. The reminder card's twin (chosen)", sub: "Fossa +1: Lv 2 · 28/75 → Lv 3", html: card("a", [G.gain], { lang }) },
          { title: "B. The window's rows", sub: "no header: layout A's row on the meter's card", html: card("b", [G.gain], { lang }) },
          { title: "C. A pill", sub: "the capsule's shape; no time-left line", html: card("c", [G.gain], { lang }) },
        ],
      },
      {
        id: "card-states",
        title: `The card's states, in design ${cardName}, level colours ${lvcName}`,
        sub: "the selectors at the top switch the level colours",
        tileMin: 340,
        stageMin: 170,
        tiles: [
          { tag: "agreed", title: "An ordinary gain", sub: "Lv 2 27/75 +1 → 28/75: \"Lv 2 · 28/75 → Lv 3\"", html: C([G.gain]) },
          { tag: "agreed", title: "A level reached, carry-over", sub: "Lv 1 24/25 +5 → Lv 2 4/75 (the 4 extra souls carry over)", html: C([G.up]) },
          { tag: "agreed", title: "A first unlock", sub: "Forest Spirit Lv 0 4/5 +1 → Lv 1 0/25: \"Unlocked\"", html: C([G.unlock]) },
          { tag: "agreed", title: "Already at Lv 3", sub: "Kalgolem farmed at the cap: \"Lv 3 · 75/75\", the bar full, no next level, no crystal figure (Settings › Pets can turn these off)", html: C([G.max]) },
          { tag: "agreed", title: "Reaching Lv 3", sub: "Splendent Armadon Lv 2 70/75 +5: \"Lv 3 reached\", then 75/75", html: C([G.cap]) },
          { tag: "agreed", title: "Two pets close together", sub: "one card, in arrival order (a newcomer is added at the end, nothing moves), at most three; any gain on the card restarts its 10 s", html: C([G.gain, G.slime]) },
          { tag: "agreed", title: "A black-market purchase", sub: "the same game message (+5): drawn as a gain, Slime 12/25 → 17/25 (Settings › Pets can turn these off)", html: C([G.market]) },
          { tag: "proposed", title: "Test card", sub: "Settings › Pets › Show a test card: the first pinned pet's real progress, nothing added; header \"Test card\"", html: C([gainOf("Fossa", { lv: 2, souls: 27 }, 0)], { test: true, left: 0.9 }) },
          { tag: "agreed", title: "Pointer over the card", sub: "as the reminder card: the time-left line stops, × lights; a click anywhere dismisses", html: C([G.gain], { hover: true, left: 0.4 }) },
          { tag: "agreed", title: "Locked overlay", sub: "click-through like the meter: no ×, nothing reacts, it runs out by itself", html: C([G.gain], { locked: true, left: 0.8 }) },
          { tag: "agreed", title: "In French", sub: "Âmes de familier; Lv kept", html: card(OPTS.card, [G.up, G.slime], { lang: "fr" }) },
        ],
      },
      {
        id: "card-place",
        title: "Where the card shows",
        sub: "the reminder card's places; it never takes focus (no-activate)",
        tileMin: 560,
        stageMin: 330,
        tiles: [
          { tag: "agreed", title: "Meter open", sub: "6 px above the meter, left edges aligned, the meter's width; below it when there is no room above", html: col(C([G.gain], { left: 0.7 }), meter()) },
          { tag: "agreed", title: "Meter closed", sub: "6 px under the toolbar, left edges aligned, 310 px (the reminder card's place while the meter is closed)", html: col(bar({ lang, label: OPTS.label }), C([G.gain], { left: 0.7 })) },
          { tag: "agreed", title: "With a reminder, meter closed", sub: "the reminder keeps its place next to the toolbar; the pet card stacks 6 px beyond it", html: col(bar({ lang, label: OPTS.label }), Hub.reminder(), C([G.gain], { left: 0.7 })) },
          { tag: "agreed", title: "With a reminder, meter open", sub: "the reminder keeps its place above the meter; the pet card stacks 6 px above it", html: col(C([G.gain], { left: 0.7 }), Hub.reminder(), meter()) },
          { tag: "agreed", title: "Toolbar low on the screen", sub: "above the toolbar, as the reminder card", html: `<div class="pt-low">${col(C([G.gain], { left: 0.7 }), bar({ lang, label: OPTS.label }))}</div>` },
        ],
      },
      {
        id: "toolbar",
        title: "Opening the window from the toolbar",
        sub: "the approved toolbar, the pets tool (Lucide paw-print) after To-dos",
        tileMin: 640,
        stageMin: 200,
        tiles: [
          { tag: "agreed", title: "The pets tool, pointer on it", sub: "after To-dos, the other tracker that works without the meter; tooltip with the window's name after 500 ms", html: bar({ lang, label: OPTS.label, hover: "pets", tip: "pets" }) },
          { tag: "agreed", title: "Window open", sub: "the tool lit (hot and the dot); the window opens under the toolbar the first time, then where you left it; a second click closes it", html: col(bar({ lang, label: OPTS.label, lit: { pets: true } }), win({ layout: OPTS.win, lang, label: OPTS.label, tab: "fera" })) },
          { r1: true, title: "A pinned-pets count (dropped)", sub: "after the resource counts: the first pinned pet's souls", html: bar({ lang, label: OPTS.label, pinCount: true, hover: "pinned", tip: "pinned" }) },
          { r1: true, title: "Its dropdown (dropped)", sub: "every pinned pet with its level and souls, then \"All pets\"", html: `<div style="min-height:150px">${bar({ lang, label: OPTS.label, pinCount: true, pinOpen: true })}</div>` },
        ],
      },
      {
        id: "settings",
        title: "Settings › Pets",
        sub: "between To-dos and General; the approved Settings widgets. Each state is settings.html, embedded; the language follows the selector",
        tileMin: 1060,
        stageMin: 0,
        wide: true,
        desk: true,
        tiles: [
          { tag: ["agreed", "proposed"], title: "Defaults", sub: "contents agreed; wording proposed. The card on; Only pinned pets off; Lv 3 pets and black-market buys on; the collection read at this login", html: frame("pets") },
          { tag: "proposed", title: "Soul tracker off", sub: "the rows under it dim and keep their values, as Reminders", html: frame("pets:off") },
          { tag: "proposed", title: "Test card shown", sub: "a good badge for 4 s beside the button", html: frame("pets:test") },
          { tag: "proposed", title: "Collection not read yet", sub: "before any login while the app runs", html: frame("pets:none") },
        ],
      },
      {
        id: "try",
        title: "Try it",
        sub: "a working toolbar, window and card: the paw opens the window; tabs, search, pins and the menu work; the buttons play the game's soul messages; nothing is saved",
        tileMin: 900,
        stageMin: 720,
        wide: true,
        tiles: [{ tag: "agreed", title: "Working mockup", sub: `window layout ${OPTS.win.toUpperCase()}, card design ${OPTS.card.toUpperCase()}, level colours ${lvcName}; the card disappears 10 s after its last gain, the pointer over it pauses`, html: '<div class="pt-try hb-col" id="pt-try"></div>' }],
      },
    ]
      .filter((s) => COMPARE || !s.r1)
      .map((s) => ({ ...s, tiles: s.tiles.filter((t) => COMPARE || !t.r1) }));
  }

  const frame = (state) => `<iframe class="pt-frame" src="settings.html?embed&amp;lang=${OPTS.lang}&amp;states=${state}" title="Settings, Pets: ${state}"></iframe>`;

  // ---- Lists: scroll thumb -------------------------------------------------
  function afterRender(root) {
    root.querySelectorAll(".pt-scrollwrap").forEach((w) => {
      const sc = w.querySelector(".pt-scroll");
      const th = w.querySelector(".o-thumb");
      const upd = () => {
        const over = sc.scrollHeight > sc.clientHeight + 1;
        w.classList.toggle("is-over", over);
        const head = w.previousElementSibling;
        if (head && head.classList.contains("pt-th")) head.style.paddingRight = over ? "11px" : "";
        if (!over) return;
        const h = Math.max(24, (sc.clientHeight * sc.clientHeight) / sc.scrollHeight);
        const t = (sc.scrollTop / (sc.scrollHeight - sc.clientHeight)) * (sc.clientHeight - h - 6);
        th.style.setProperty("--h", h + "px");
        th.style.setProperty("--t", t + 3 + "px");
      };
      upd();
      const s = Number(w.dataset.scroll || 0);
      if (s) sc.scrollTop = s;
      upd();
      sc.addEventListener("scroll", upd, { passive: true });
    });
  }

  // ---- Working tile ----------------------------------------------------------
  const clonePets = () => PETS.map((p) => ({ ...p }));
  const TRY = { open: true, tab: "all", q: "", show: "all", order: "game", special: "mark", pinned: DEFAULT_PINS.slice(), filterOpen: false, fold: [], pets: clonePets(), items: [], up: false, fading: false, gen: 0, scroll: 0 };

  function tryReset() {
    Object.assign(TRY, { open: true, tab: "all", q: "", show: "all", order: "game", special: "mark", pinned: DEFAULT_PINS.slice(), filterOpen: false, fold: [], pets: clonePets(), items: [], up: false, fading: false, scroll: 0 });
    TRY.gen++;
  }

  function tryMount() {
    const root = document.getElementById("pt-try");
    if (!root) return;
    root.innerHTML = `<div id="pt-try-bar"></div><div class="hb-row"><div class="pt-try-card" id="pt-try-card"></div><div id="pt-try-win"></div></div><div class="mk-sim"><b>Page only: what the game sends (soul pickups)</b><button type="button" data-pt-gain="Fossa" data-n="1">Fossa +1</button><button type="button" data-pt-gain="Hill Coradon" data-n="2">Hill Coradon +2</button><button type="button" data-pt-gain="Forest Spirit" data-n="1">Forest Spirit +1</button><button type="button" data-pt-gain="Kalgolem" data-n="1">Kalgolem +1 (at Lv 3)</button><button type="button" data-pt-gain="Slime" data-n="5">Black market: Slime +5</button><button type="button" data-pt-reset="1">Start again</button></div>`;
    tryBar();
    tryWin();
    tryCard(false);
  }

  function tryBar() {
    const el = document.getElementById("pt-try-bar");
    if (el) el.innerHTML = bar({ lang: OPTS.lang, label: OPTS.label, lit: { pets: TRY.open }, live: true });
  }

  function tryWin() {
    const el = document.getElementById("pt-try-win");
    if (!el) return;
    const sc = el.querySelector(".pt-scroll");
    const scroll = TRY.resetScroll ? 0 : sc ? sc.scrollTop : TRY.scroll;
    TRY.resetScroll = false;
    const act = document.activeElement;
    const focused = act && act.matches && act.matches("[data-pt-q]");
    const caret = focused ? act.selectionStart : null;
    el.innerHTML = TRY.open
      ? win({ layout: OPTS.win, lang: OPTS.lang, label: OPTS.label, interactive: true, tab: TRY.tab, q: TRY.q, show: TRY.show, order: TRY.order, special: TRY.special, pinned: TRY.pinned, pets: TRY.pets, open: TRY.fold, filterOpen: TRY.filterOpen, scroll })
      : "";
    afterRender(el);
    if (focused) {
      const i = el.querySelector("[data-pt-q]");
      if (i) {
        i.focus();
        i.setSelectionRange(caret, caret);
      }
    }
  }

  function tryCard(enter) {
    const el = document.getElementById("pt-try-card");
    if (!el) return;
    const gen = ++TRY.gen;
    if (!TRY.items.length) {
      el.innerHTML = "";
      return;
    }
    el.innerHTML = card(OPTS.card, TRY.items, { lang: OPTS.lang, live: true, enter });
    for (const it of TRY.items) it.from = null;
    // Bars grow from their last width over 240 ms; a level reached starts
    // its new bar without travelling backwards.
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        el.querySelectorAll("[data-to]").forEach((b) => (b.style.width = b.dataset.to + "%"));
      })
    );
    const life = el.querySelector(".rm-left i, .pc-life i");
    life.addEventListener("animationend", () => {
      if (gen !== TRY.gen) return;
      el.firstElementChild.classList.add("is-gone");
      TRY.fading = true;
      setTimeout(() => {
        if (gen !== TRY.gen) return;
        TRY.items = [];
        TRY.up = false;
        TRY.fading = false;
        el.innerHTML = "";
      }, 160);
    });
  }

  function tryGain(name, n) {
    const p = TRY.pets.find((x) => x.name === name);
    const before = { lv: p.lv, souls: p.souls };
    const after = addSouls(p, n);
    const ev = before.lv === 0 && after.lv >= 1 ? "unlock" : after.lv > before.lv ? "up" : before.lv >= CAP ? "max" : "gain";
    const fromPct = before.lv >= CAP ? 100 : (before.souls / NEED[before.lv]) * 100;
    p.lv = after.lv;
    p.souls = after.souls;
    const it = TRY.items.find((x) => x.p.id === p.id);
    if (it) {
      it.from = ev === "gain" ? fromPct : null;
      it.lv = p.lv;
      it.souls = p.souls;
      if (ev === "up" || ev === "unlock") it.ev = ev;
    } else {
      TRY.items.push({ p, lv: p.lv, souls: p.souls, ev, from: ev === "gain" ? fromPct : null });
      if (TRY.items.length > 3) TRY.items.shift();
    }
    const enter = !TRY.up || TRY.fading;
    TRY.up = true;
    TRY.fading = false;
    tryCard(enter);
    tryWin();
  }

  function tryDismiss() {
    const el = document.getElementById("pt-try-card");
    if (!el || !el.firstElementChild) return;
    const gen = ++TRY.gen;
    el.firstElementChild.classList.add("is-gone");
    setTimeout(() => {
      if (gen !== TRY.gen) return;
      TRY.items = [];
      TRY.up = false;
      el.innerHTML = "";
    }, 160);
  }

  function onTryClick(e) {
    const root = document.getElementById("pt-try");
    if (!root || !root.contains(e.target)) return;
    const t = (sel) => e.target.closest(sel);
    let el;
    if ((el = t("[data-pt-gain]"))) return tryGain(el.dataset.ptGain, Number(el.dataset.n));
    if (t("[data-pt-reset]")) {
      tryReset();
      tryBar();
      tryWin();
      tryCard(false);
      return;
    }
    if (t("#pt-try-card")) return tryDismiss();
    if (t("[data-pt-tool]")) {
      TRY.open = !TRY.open;
      TRY.filterOpen = false;
      tryBar();
      tryWin();
      return;
    }
    if (!t("#pt-try-win")) return;
    if (t("[data-pt-close]")) {
      TRY.open = false;
      tryBar();
      tryWin();
      return;
    }
    if ((el = t("[data-pt-show]"))) {
      TRY.show = el.dataset.ptShow;
      TRY.filterOpen = false;
      TRY.resetScroll = true;
    } else if ((el = t("[data-pt-order]"))) {
      TRY.order = el.dataset.ptOrder;
      TRY.filterOpen = false;
      TRY.resetScroll = true;
    } else if (t("[data-pt-special]")) {
      TRY.special = TRY.special === "hide" ? "mark" : "hide";
      TRY.filterOpen = false;
    } else if (t("[data-pt-filter]")) TRY.filterOpen = !TRY.filterOpen;
    else if (t("[data-pt-clear]")) Object.assign(TRY, { show: "all", order: "game", special: "mark", resetScroll: true });
    else if ((el = t("[data-pt-tab]"))) Object.assign(TRY, { tab: el.dataset.ptTab, filterOpen: false, resetScroll: true });
    else if ((el = t("[data-pt-pin]"))) {
      const id = Number(el.dataset.ptPin);
      TRY.pinned = TRY.pinned.includes(id) ? TRY.pinned.filter((x) => x !== id) : TRY.pinned.concat(id);
    } else if ((el = t("[data-pt-fold]"))) {
      const f = el.dataset.ptFold;
      TRY.fold = TRY.fold.includes(f) ? TRY.fold.filter((x) => x !== f) : TRY.fold.concat(f);
    } else if (t("[data-pt-qclear]")) {
      TRY.q = "";
      TRY.resetScroll = true;
      tryWin();
      const i = document.querySelector("#pt-try-win [data-pt-q]");
      if (i) i.focus();
      return;
    } else if (TRY.filterOpen && !t(".pt-menu")) TRY.filterOpen = false;
    else return;
    tryWin();
  }

  function onTryInput(e) {
    if (!e.target.matches || !e.target.matches("#pt-try-win [data-pt-q]")) return;
    TRY.q = e.target.value;
    TRY.resetScroll = true;
    tryWin();
  }

  function onTryKey(e) {
    if (!e.target.matches || !e.target.matches("#pt-try-win [data-pt-q]") || e.key !== "Escape") return;
    if (TRY.q) {
      TRY.q = "";
      TRY.resetScroll = true;
      tryWin();
    } else e.target.blur();
  }

  // ---- Mount ------------------------------------------------------------------
  function controlsHtml() {
    const sel = (id, labelText, cur, opts) => `<label>${labelText} <select id="${id}">${opts.map(([v, t]) => `<option value="${v}"${v === cur ? " selected" : ""}>${t}</option>`).join("")}</select></label>`;
    return `<div class="controls" role="region" aria-label="Mockup controls"><div class="controls__inner">
<label>Background opacity <input type="range" id="c-alpha" min="0" max="255" value="${OPTS.alpha}"><output id="o-alpha">${OPTS.alpha}</output></label>
<label>Content scale <input type="range" id="c-scale" min="70" max="150" step="5" value="${OPTS.scale}"><output id="o-scale">${OPTS.scale}%</output></label>
${sel("c-scene", "Scene", OPTS.scene, [["dark", "Dark"], ["busy", "Busy"], ["bright", "Bright"]])}
<span class="controls__sep" aria-hidden="true"></span>
${sel("c-lang", "Language", OPTS.lang, [["en", "English"], ["fr", "Français"]])}
${sel("c-label", "Window name", OPTS.label, [["pets", "Pets (recommended)"], ["tracker", "Pet tracker"], ["souls", "Pet souls"]])}
${sel("c-lvc", "Level colours", OPTS.lvc, [["a", "(a) badge only (recommended)"], ["b", "(b) badge and fill"], ["none", "Round 1, none"]])}
${sel("c-win", "Window layout", OPTS.win, [["a", "A, meter rows (chosen)"], ["b", "B, a table (round 1)"], ["c", "C, sections by family (round 1)"]])}
${sel("c-card", "Card design", OPTS.card, [["a", "A, reminder twin (chosen)"], ["b", "B, the window's rows (round 1)"], ["c", "C, a pill (round 1)"]])}
</div></div>`;
  }

  function applyVars() {
    const r = document.documentElement.style;
    r.setProperty("--bg-a", (OPTS.alpha / 255).toFixed(3));
    r.setProperty("--scale", (OPTS.scale / 100).toFixed(2));
  }

  function render() {
    applyVars();
    const main = document.querySelector("main.scenes");
    let html = "";
    let seed = 1;
    let n = 0;
    for (const sec of sections()) {
      n++;
      html += `<section class="scene-group pt-sec" id="${sec.id}"><h2>${n}. ${sec.title}<span>${sec.sub}</span></h2><div class="tiles${sec.wide ? " is-wide" : ""}" style="--tile-min:${sec.tileMin}px">`;
      for (const t of sec.tiles) {
        const scene = t.scene || OPTS.scene;
        const alpha = t.alpha != null ? `;--bg-a:${(t.alpha / 255).toFixed(3)}` : "";
        const stage = sec.desk ? `<div class="tile__stage pt-desk">${t.html}</div>` : `<div class="tile__stage scene--${scene}" style="--stage-min:${sec.stageMin}px${alpha}">${fx(seed++)}${t.html}</div>`;
        html += `<figure class="tile" style="margin:0"><figcaption class="tile__cap">${t.tag ? tagHtml(t.tag) : ""}<b>${t.title}</b>${t.sub ? `<span class="sub">${t.sub}</span>` : ""}</figcaption>${stage}</figure>`;
      }
      html += `</div></section>`;
    }
    main.innerHTML = html;
    afterRender(main);
    tryMount();
  }

  function mount() {
    const main = document.querySelector("main.scenes");
    main.insertAdjacentHTML("beforebegin", controlsHtml());
    const keep = (k, v) => {
      OPTS[k] = v;
      params.set(k, v);
      history.replaceState(null, "", "?" + params.toString() + location.hash);
    };
    const bind = (id, k, read, out) => {
      document.getElementById(id).addEventListener("input", (e) => {
        keep(k, read(e.target));
        if (out) document.getElementById(out[0]).textContent = out[1](OPTS[k]);
        if (k === "alpha" || k === "scale") applyVars();
        else render();
      });
    };
    bind("c-alpha", "alpha", (e) => Number(e.value), ["o-alpha", (v) => v]);
    bind("c-scale", "scale", (e) => Number(e.value), ["o-scale", (v) => v + "%"]);
    bind("c-scene", "scene", (e) => e.value);
    bind("c-lang", "lang", (e) => e.value);
    bind("c-label", "label", (e) => e.value);
    bind("c-win", "win", (e) => e.value);
    bind("c-card", "card", (e) => e.value);
    bind("c-lvc", "lvc", (e) => e.value);
    document.addEventListener("click", onTryClick);
    document.addEventListener("input", onTryInput);
    document.addEventListener("keydown", onTryKey);
    render();
  }

  window.Pets = { win, card, bar, addSouls, PETS };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", mount);
  else mount();
})();
