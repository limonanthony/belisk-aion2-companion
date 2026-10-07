/* Fight report mockups (TASK-15.1; choices decided 2026-10-07): invented
 * fights and the pieces every direction shares (window header, key figures,
 * player rows, deaths list, the player view). Mounted through ../approved/kit.js.
 *
 * Game names are data: bosses and monsters from crates/core/data/
 * npc_names_en.json, the dungeon from crates/app/assets/game/data/
 * dungeons.json, servers from servers.json, class and monster skills from
 * skill_names_en.json; skill icons are the game PNGs. Which monster used
 * which skill, every player name and every figure are invented.
 *
 * Icons: Lucide (https://lucide.dev), ISC License; see ../approved/kit.js. */

(function () {
  "use strict";

  // ---- Icons ----------------------------------------------------------------

  const EXTRA = {
    fileText:
      '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
    chevronLeft: '<path d="m15 18-6-6 6-6"/>',
    users:
      '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
    swords:
      '<polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" x2="19" y1="19" y2="13"/><line x1="16" x2="20" y1="16" y2="20"/><line x1="19" x2="21" y1="21" y2="19"/><polyline points="14.5 6.5 18 3 21 3 21 6 17.5 9.5"/><line x1="5" x2="9" y1="14" y2="18"/><line x1="7" x2="4" y1="17" y2="20"/><line x1="3" x2="5" y1="19" y2="21"/>',
    mousePointer:
      '<path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"/>',
    // The boss-record icon candidates (History): Lucide crown; skull is in kit.js.
    crown:
      '<path d="M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.246a1 1 0 0 1-.956.734H5.81a1 1 0 0 1-.957-.734L2.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z"/><path d="M5 21h14"/>',
  };

  function icon(name, cls) {
    if (EXTRA[name]) {
      return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${EXTRA[name]}</svg>`;
    }
    return Kit.icon(name, cls);
  }

  // ---- Formatting -------------------------------------------------------------

  const int = (n) => Math.round(n).toLocaleString("en-US");

  function compact(n) {
    if (n < 10000) return int(n);
    if (n < 1e6) return (n / 1e3).toFixed(1) + "K";
    return (n / 1e6).toFixed(2) + "M";
  }

  const p1 = (n) => (n * 100).toFixed(1) + "%";
  const p0 = (n) => Math.round(n * 100) + "%";

  function time(secs) {
    const m = Math.floor(secs / 60);
    const s = Math.round(secs % 60);
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  function mask(name) {
    const t = name.trim();
    if (t.length <= 1) return t ? "*" : "";
    if (t.length === 2) return t[0] + "*";
    return t[0] + "*".repeat(t.length - 2) + t[t.length - 1];
  }

  // ---- Classes and shades (approved/tokens.md) -------------------------------

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

  const CLASS_NAME = {
    assassin: "Assassin",
    chanter: "Chanter",
    cleric: "Cleric",
    elementalist: "Elementalist",
    gladiator: "Gladiator",
    ranger: "Ranger",
    sorcerer: "Sorcerer",
    templar: "Templar",
  };

  const classIcon = (cls) => `../../../crates/app/assets/game/class/${cls}.png`;

  // Skill icons follow the app's rule (game_data::skill_icon): an eight-digit
  // id shares the icon of its first four digits. 1400 and 1600 have none, so
  // those rows show the quiet square, as the detail window does.
  const NO_ICON = new Set(["1400", "1600"]);
  function skillIcon(code) {
    const base = String(code).slice(0, 4);
    return NO_ICON.has(base) ? null : `../../../crates/app/assets/game/skills/${base}.png`;
  }

  const SHADE_STEPS = { assassin: [1, -1, 2, -2], cleric: [-1, -2, -3], ranger: [-1, -2, -3], chanter: [1, -1, -3, -4], fighter: [1, -1, -3, -4] };

  function shadeRgb(cls, order) {
    const steps = SHADE_STEPS[cls] || [1, -1, -2, -3];
    const n = order % (steps.length + 1);
    const base = CLASS_RGB[cls];
    if (n === 0) return base;
    const [r, g, b] = base.split(",").map((v) => Number(v) / 255);
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const c = max - min;
    let l = (max + min) / 2;
    const s = c === 0 ? 0 : c / (1 - Math.abs(2 * l - 1));
    const h = c === 0 ? 0 : max === r ? ((g - b) / c + 6) % 6 : max === g ? (b - r) / c + 2 : (r - g) / c + 4;
    l = Math.min(1, Math.max(0, l + steps[n - 1] * 0.1));
    const c2 = (1 - Math.abs(2 * l - 1)) * s;
    const x = c2 * (1 - Math.abs((h % 2) - 1));
    const m = l - c2 / 2;
    const rgb = [[c2, x, 0], [x, c2, 0], [0, c2, x], [0, x, c2], [x, 0, c2], [c2, 0, x]][Math.min(5, Math.floor(h))];
    return rgb.map((v) => Math.round((v + m) * 255)).join(", ");
  }

  // ---- Invented skill data ------------------------------------------------------

  // [id, name, share of the player's damage, hits]
  const SKILLS = {
    ranger: [
      [14020000, "Snipe", 0.263, 188],
      [14110000, "Gale Arrow", 0.189, 342],
      [14030000, "Rapid Fire", 0.147, 516],
      [14270000, "Arrow Storm", 0.115, 44],
      [14040000, "Spiral Arrow", 0.092, 96],
      [14080000, "Burst Arrow", 0.068, 71],
      [14090000, "Marking Shot", 0.053, 58],
      [14100000, "Tempest Arrow", 0.041, 39],
      [14060000, "Griffon Arrow", 0.02, 22],
      [14000001, "Basic Attack", 0.012, 211],
    ],
    sorcerer: [
      [15060000, "Hellfire", 0.24, 61],
      [15210000, "Flame Arrow", 0.2, 288],
      [15040000, "Firestorm", 0.16, 94],
      [15250000, "Pyroclasm", 0.13, 27],
      [15120000, "Glacial Smite", 0.11, 66],
      [15030000, "Burst", 0.09, 112],
      [15090000, "Ice Chain", 0.07, 73],
    ],
    gladiator: [
      [11040000, "Wrathful Strike", 0.25, 141],
      [11010000, "Rending Blow", 0.21, 233],
      [11090000, "Blade Dance", 0.17, 96],
      [11100000, "Ruinous Blow", 0.15, 38],
      [11050000, "Crushing Wave", 0.12, 77],
      [11020000, "Keen Strike", 0.1, 164],
    ],
    assassin: [
      [13070000, "Shadowstrike", 0.27, 118],
      [13350000, "Heart Gore", 0.2, 42],
      [13040000, "Swift Slice", 0.16, 301],
      [13060000, "Ambush", 0.13, 37],
      [13210000, "Whirlwind Slice", 0.11, 88],
      [13050000, "Flash Slice", 0.08, 129],
      [13160000, "Apply Poison", 0.05, 402],
    ],
    templar: [
      [12090000, "Punishment", 0.29, 74],
      [12100000, "Shield Smite", 0.23, 118],
      [12240000, "Judgment", 0.19, 41],
      [12010000, "Vicious Strike", 0.17, 247],
      [12060000, "Punishing Strike", 0.12, 63],
    ],
    cleric: [
      [17060000, "Bolt", 0.32, 206],
      [17040000, "Judgment Thunder", 0.23, 48],
      [17010000, "Earth's Retribution", 0.18, 61],
      [17070000, "Chain of Torment", 0.14, 52],
      [17050000, "Divine Punishment", 0.13, 34],
    ],
    chanter: [
      [18050000, "Bursting Blow", 0.26, 102],
      [18010000, "Onslaught", 0.22, 248],
      [18070000, "Crushing Blow", 0.19, 87],
      [18040000, "Incandescent Blow", 0.14, 53],
      [18210000, "Tremor Crush", 0.11, 39],
      [18300000, "Gust Rampage", 0.08, 21],
    ],
    elementalist: [
      [16240000, "Jointstrike: Destructive Attack", 0.24, 36],
      [16001301, "Fire Spirit: Summon Meteor", 0.19, 22],
      [16020000, "Vacuum Explosion", 0.16, 91],
      [16040000, "Combustion", 0.14, 77],
      [16710000, "Spirit Strike", 0.11, 140],
      [16010000, "Cold Shock", 0.09, 118],
      [16210000, "Curse of Despair", 0.07, 64],
    ],
  };

  // [id, name, share of the player's healing, heals]
  const HEALS = {
    cleric: [
      [17100000, "Healing Light", 0.377, 58],
      [17120000, "Radiant Recovery", 0.239, 24],
      [17090000, "Light of Regeneration", 0.176, 212],
      [17320000, "Blessing of Regeneration", 0.121, 96],
      [17270000, "Salvation", 0.087, 3],
    ],
    chanter: [
      [18170000, "Healing Touch", 0.48, 40],
      [18120000, "Recuperation", 0.31, 22],
      [18710000, "Blessing of Life", 0.21, 8],
    ],
    templar: [
      [12230000, "Noble Armor", 0.61, 14],
      [12250000, "Comrade in Arms", 0.39, 9],
    ],
    gladiator: [[11120000, "Blood Absorption", 1, 31]],
    ranger: [[14790000, "Revitalization Contract", 1, 6]],
    sorcerer: [[15160000, "Steel Barrier", 1, 4]],
    elementalist: [[16790000, "Revitalization Contract", 1, 3]],
    assassin: [],
  };

  // Deterministic rates per skill, so every page draws the same figures.
  function rng(seed) {
    let s = seed % 233280;
    return () => (s = (s * 9301 + 49297) % 233280) / 233280;
  }

  function skillRows(cls, damage, seed) {
    return SKILLS[cls].map(([code, name, share, hits], i) => {
      const r = rng(code + seed + i * 17);
      const total = damage * share;
      const rate = { crit: 0.36 + r() * 0.12, back: 0.08 + r() * 0.07, front: 0.05 + r() * 0.04, parry: r() * 0.02, perfect: 0.02 + r() * 0.03, double: 0.04 + r() * 0.04, multi: r() < 0.4 ? 0 : r() * 0.18 };
      const count = {};
      for (const k of Object.keys(rate)) count[k] = Math.round(hits * rate[k]);
      const avg = total / hits;
      const spec = [1, 2, 3, 4, 5].filter(() => r() < 0.4);
      return {
        code,
        name,
        icon: skillIcon(code),
        hits,
        total,
        count,
        multiDmg: count.multi ? total * 0.11 : 0,
        min: avg * (0.32 + r() * 0.1),
        max: avg * (2.3 + r() * 0.7),
        spec,
      };
    });
  }

  function healRows(cls, healing, seed) {
    if (!healing) return [];
    return (HEALS[cls] || []).map(([code, name, share, heals], i) => {
      const r = rng(code + seed + i * 31);
      return { code, name, icon: skillIcon(code), heals, total: healing * share, crits: Math.round(heals * (0.28 + r() * 0.14)) };
    });
  }

  // ---- Damage taken by source ---------------------------------------------------

  // [monster, skill, typical hit]
  const SOURCES = {
    basilus: [
      ["Basilus the False", "Attack", 16900],
      ["Basilus the False", "Sword Aura Blast", 47800],
      ["Basilus the False", "Hand of Destruction", 58200],
      ["Basilus the False", "Ground Explosion", 39400],
      ["Basilus the False", "Extinction", 96000],
      ["Corrupted Daeva Executioner", "Piercing Stab", 28700],
      ["Corrupted Daeva Executioner", "Attack", 7800],
      ["Corrupted Daeva Elite Sorcerer", "Flame Orb", 29600],
    ],
    tassin: [
      ["Tyrant Tassin", "Attack", 21400],
      ["Tyrant Tassin", "Rampage", 82000],
      ["Tyrant Tassin", "Ground Smite", 61500],
      ["Tyrant Tassin", "Tail Slash", 34800],
    ],
    superbia: [
      ["Superbia, Sacred Beast of Life", "Claw Swipe", 24800],
      ["Superbia, Sacred Beast of Life", "Toxic Gas", 12600],
      ["Superbia, Sacred Beast of Life", "Vine Trap", 41300],
      ["Superbia, Sacred Beast of Life", "Attack", 15200],
    ],
  };

  const ROLE_WEIGHTS = {
    basilus: {
      tank: [0.42, 0.22, 0.16, 0.08, 0.05, 0, 0.07, 0],
      melee: [0.06, 0.26, 0.18, 0.19, 0.08, 0.13, 0.05, 0.05],
      ranged: [0, 0.245, 0, 0.43, 0.1, 0, 0.073, 0.152],
    },
    tassin: { tank: [0.55, 0.2, 0.15, 0.1], melee: [0.2, 0.35, 0.3, 0.15], ranged: [0, 0.4, 0.6, 0] },
    superbia: { tank: [0.5, 0.2, 0.1, 0.2], melee: [0.3, 0.3, 0.2, 0.2], ranged: [0.05, 0.5, 0.45, 0] },
  };

  const ROLE = { templar: "tank", gladiator: "melee", assassin: "melee", chanter: "melee", ranger: "ranged", sorcerer: "ranged", cleric: "ranged", elementalist: "ranged" };

  function takenRows(fightKey, cls, taken) {
    if (!taken) return [];
    const weights = ROLE_WEIGHTS[fightKey][ROLE[cls]];
    return SOURCES[fightKey]
      .map(([by, skill, hit], i) => {
        const total = taken * weights[i];
        return { by, skill, total, hits: Math.max(1, Math.round(total / hit)) };
      })
      .filter((s) => s.total > 0)
      .sort((a, b) => b.total - a.total);
  }

  // ---- Buffs (the player view) --------------------------------------------------

  // [id, name, caster, seconds up]
  const BUFFS_ON = [
    [17720000, "Empyrean Lords' Benediction", "Dovrek", 287],
    [12450000, "Battlefield Banner", "Quenna", 241],
    [14710000, "Vigilant Eye", "Mirelle", 198],
    [14720000, "Concentrated Fire", "Mirelle", 126],
    [17430000, "Prayer of Amplification", "Dovrek", 94],
    [14220000, "Bow of Blessing", "Mirelle", 61],
    [17410000, "Light of Protection", "Dovrek", 38],
  ];

  const BUFFS_BOSS = [
    [14090000, "Marking Shot", "Mirelle", 264],
    [11140000, "Intimidating Roar", "Korrinth", 172],
    [12270000, "Debilitating Smash", "Quenna", 149],
    [17080000, "Debilitating Mark", "Dovrek", 88],
  ];

  // ---- Fights ---------------------------------------------------------------------

  // [name, class, server, combat power, damage, healing, damage taken, you]
  const PARTY = [
    ["Vaelith", "sorcerer", "SIE", 41820, 19.11e6, 0.15e6, 1.35e6],
    ["Mirelle", "ranger", "NEZ", 38904, 15.04e6, 0.21e6, 0.962e6, true],
    ["Korrinth", "gladiator", "SIE", 40112, 12.44e6, 0.62e6, 2.91e6],
    ["Saebrin", "assassin", "VAI", 36455, 10.8e6, 0, 1.88e6],
    ["Quenna", "templar", "SIE", 39670, 6.96e6, 0.84e6, 7.42e6],
    ["Dovrek", "cleric", "NEZ", 33880, 4.04e6, 6.12e6, 1.12e6],
  ];

  const FIELD = [
    ["Kaedric", "gladiator", "VAI", 42310, 24.6e6, 0.94e6, 9.8e6],
    ["Vaelith", "sorcerer", "SIE", 41820, 22.9e6, 0.12e6, 2.1e6],
    ["Mirelle", "ranger", "NEZ", 38904, 18.4e6, 0.18e6, 1.4e6, true],
    ["Pellam", "ranger", "VAI", 37120, 15.2e6, 0.09e6, 1.7e6],
    ["Korrinth", "gladiator", "SIE", 40112, 14.7e6, 0.71e6, 4.2e6],
    ["Tobiah", "elementalist", "NEZ", 35020, 13.9e6, 0.11e6, 1.9e6],
    ["Saebrin", "assassin", "VAI", 36455, 12.8e6, 0, 3.1e6],
    ["Brannoc", "templar", "SIE", 37450, 9.6e6, 1.21e6, 11.4e6],
    ["Quenna", "templar", "SIE", 39670, 8.1e6, 0.98e6, 8.7e6],
    ["Ilyrae", "chanter", "VAI", 34210, 6.4e6, 3.42e6, 2.6e6],
    ["Ysolde", "chanter", "SIE", 33940, 5.1e6, 2.88e6, 2.2e6],
    ["Dovrek", "cleric", "NEZ", 33880, 4.6e6, 7.94e6, 1.8e6],
    ["Nalla", "cleric", "NEZ", 30420, 3.2e6, 6.31e6, 2.4e6],
    ["Ferelith", "elementalist", "VAI", 35980, 0.96e6, 0, 0],
  ];

  const LONG = [
    // Longer than real names, so the ellipsis shows: about 23 characters
    // and a server tag fit before it.
    ["Seraphinaveyllithaerinelle", "elementalist", "SIE", 41210, 17.62e6, 0.08e6, 1.21e6],
    ["Mirelle", "ranger", "NEZ", 38904, 14.11e6, 0.19e6, 0.88e6, true],
    ["Vexmorradunelle", "assassin", "VAI", 36980, 12.37e6, 0, 2.02e6],
    ["Thornwallowbrandethor", "templar", "SIE", 40350, 7.45e6, 0.92e6, 6.84e6],
    ["Aurelindoravaine", "chanter", "VAI", 35110, 6.02e6, 3.11e6, 1.64e6],
    ["Kassiopheliandrael", "cleric", "NEZ", 34470, 3.86e6, 5.74e6, 1.33e6],
  ];

  // [seconds into the fight, who, monster, skill, last hit]
  const DEATHS = {
    one: [[167, "Saebrin", "Basilus the False", "Sword Aura Blast", 52310]],
    none: [],
    several: [
      [167, "Saebrin", "Basilus the False", "Sword Aura Blast", 52310],
      [245, "Vaelith", "Corrupted Daeva Executioner", "Piercing Stab", 31840],
      [298, "Saebrin", "Basilus the False", "Hand of Destruction", 61200],
      [301, "Dovrek", "Basilus the False", "Extinction", 98400],
    ],
    field: [
      [133, "Saebrin", "Tyrant Tassin", "Rampage", 88400],
      [287, "Pellam", "Tyrant Tassin", "Ground Smite", 71200],
    ],
    long: [[214, "Seraphinaveyllithaerinelle", "Superbia, Sacred Beast of Life", "Vine Trap", 44100]],
  };

  const FIGHT_DEFS = {
    basilus: { key: "basilus", boss: "Basilus the False", place: "Fallen Guardian's City · Hard", when: "Today 21:42", secs: 312, roster: PARTY, deaths: "one" },
    tassin: { key: "tassin", boss: "Tyrant Tassin", place: "", when: "Today 20:58", secs: 401, roster: FIELD, deaths: "field" },
    superbia: { key: "superbia", boss: "Superbia, Sacred Beast of Life", place: "", when: "Today 22:16", secs: 287, roster: LONG, deaths: "long" },
  };

  /* fight(key, { deaths, outcome: "final" | "ended", old })
   *   old: saved before the meter kept healing, damage taken and deaths. */
  function fight(key, o = {}) {
    const def = FIGHT_DEFS[key];
    const deathList = (DEATHS[o.deaths || def.deaths] || []).map(([at, who, by, skill, hit]) => ({ at, who, by, skill, hit }));
    const seen = {};
    const players = def.roster
      .map(([name, cls, server, cp, dmg, heal, taken, you], i) => ({ name, cls, server, cp, dmg, heal, taken, you: !!you, seed: i * 101 + name.length }))
      .sort((a, b) => b.dmg - a.dmg)
      .map((p, i) => {
        const order = (seen[p.cls] = (seen[p.cls] ?? -1) + 1);
        return {
          ...p,
          rank: i + 1,
          dps: p.dmg / def.secs,
          rgb: shadeRgb(p.cls, order),
          icon: classIcon(p.cls),
          deaths: deathList.filter((d) => d.who === p.name),
        };
      });
    const sum = (k) => players.reduce((s, p) => s + p[k], 0);
    return {
      ...def,
      outcome: o.outcome || "final",
      old: !!o.old,
      deathList: o.old ? [] : deathList,
      players,
      total: sum("dmg"),
      healTotal: sum("heal"),
      takenTotal: sum("taken"),
      bossDebuffsOn: def.boss,
    };
  }

  function player(f, name) {
    const p = f.players.find((x) => x.name === name);
    return {
      ...p,
      skills: skillRows(p.cls, p.dmg, p.seed),
      heals: healRows(p.cls, p.heal, p.seed),
      sources: takenRows(f.key, p.cls, p.taken),
    };
  }

  /* The party's damage taken by monster skill, biggest first. */
  function partySources(f) {
    const m = new Map();
    for (const p of f.players) {
      for (const s of takenRows(f.key, p.cls, p.taken)) {
        const k = s.by + "|" + s.skill;
        const t = m.get(k) || { by: s.by, skill: s.skill, total: 0, hits: 0, players: 0 };
        t.total += s.total;
        t.hits += s.hits;
        t.players += 1;
        m.set(k, t);
      }
    }
    return [...m.values()].sort((a, b) => b.total - a.total);
  }

  // ---- Shared pieces ----------------------------------------------------------------

  const nm = (ctx, s) => esc(ctx.opts.mask ? mask(s) : s);

  const btn = (name, label, extra = "") =>
    `<span class="o-btn${extra}" role="button" tabindex="-1" title="${label}" aria-label="${label}">${icon(name)}</span>`;

  function outcomeChip(f) {
    return f.outcome === "final"
      ? `<span class="o-chip is-good">${icon("check")}Defeated</span>`
      : `<span class="o-chip is-warn">${icon("hourglass")}Fight ended <small>may resume</small></span>`;
  }

  function reportHead(ctx, f) {
    const sub = [f.place, f.when, `${f.players.length} players`].filter(Boolean).join(" · ");
    return `<div class="o-head"><span class="o-title">${icon("fileText")}Fight report</span><span class="grow"></span>${outcomeChip(f)}${btn("close", "Close")}</div>
<div class="rp-boss"><div class="rp-boss-top"><span class="rp-boss-name">${esc(f.boss)}</span><span class="rp-time">${time(f.secs)}</span></div><div class="rp-boss-sub">${esc(sub)}</div></div>`;
  }

  function figures(f) {
    const v = (label, value, none) => `<div><span>${label}</span><b${none ? ' class="is-none"' : ""}>${value}</b></div>`;
    return `<div class="det-figs">${v("Damage", compact(f.total))}${v("Party DPS", compact(f.total / f.secs))}${v("Healing", f.old ? "--" : compact(f.healTotal), f.old)}${v("Damage taken", f.old ? "--" : compact(f.takenTotal), f.old)}${v("Deaths", f.old ? "--" : f.deathList.length, f.old)}</div>`;
  }

  const oldNote = (margin = 6) =>
    `<div class="o-note" style="margin-top:${margin}px">${icon("info")}<span>Healing, damage taken and deaths were not recorded for this fight: it was saved before the meter kept them. Damage, skills and buffs are complete.</span></div>`;

  function meta(ctx, p) {
    return ctx.opts.server ? `<small>${p.server}</small>` : "";
  }

  /* One player row: rank badge, the class-coloured bar, name, cells, and the
   * chevron slot that shows under the pointer or with keyboard focus.
   *   r: { rank, scale (0-1, 0 = no fill), cells (html), hover, focus, dim } */
  function playerRow(ctx, p, r) {
    const cls = `o-row${p.you ? " is-you" : ""}${r.hover ? " is-hover" : ""}${r.focus ? " is-focus" : ""}${r.dim ? " is-dim" : ""}`;
    const fill = r.scale > 0 ? `<span class="o-fill" style="width:${(Math.max(0.06, r.scale) * 100).toFixed(1)}%"></span>` : "";
    return `<div class="${cls}" role="button" tabindex="-1" aria-label="${esc(p.name)}, open details"><span class="o-rk">${r.rank}</span><span class="o-track" style="--rgb:${p.rgb}">${fill}<img class="o-icon" src="${p.icon}" alt="${CLASS_NAME[p.cls]}"><span class="o-name">${nm(ctx, p.name)}${meta(ctx, p)}</span>${r.cells}<span class="rp-go">${icon("chevronRight")}</span></span>${r.after || ""}</div>`;
  }

  /* A list of monster skills (party or player), with the health-colour bar. */
  function sourceRows(list, total, { hover = -1 } = {}) {
    const max = list[0].total;
    return `<div class="o-rows">${list
      .map(
        (k, i) =>
          `<div class="o-row${i === hover ? " is-hover" : ""}"><span class="o-track is-plain"><span class="o-fill is-hp" style="width:${Math.max(6, (k.total / max) * 100).toFixed(1)}%"></span><span class="o-name">${esc(k.skill)}<small>${esc(k.by)}</small></span><span class="rp-cell is-side" style="width:62px">${k.players ? `${k.players} players` : ""}</span><span class="rp-cell is-side" style="width:44px">${int(k.hits)}</span><span class="rp-cell o-strong" style="width:56px">${compact(k.total)}</span></span><span class="o-pc">${p1(k.total / total)}</span></div>`
      )
      .join("")}</div>`;
  }

  function scroll(inner, { rows, maxRows, rowH = 25, thumbTop = 2 }) {
    if (!scrolls(rows, maxRows)) return `<div class="rp-scroll">${inner}</div>`;
    const h = maxRows * rowH - 3;
    const thumb = Math.max(24, Math.round((h * maxRows) / rows));
    return `<div class="rp-scroll has-thumb" style="height:${h + 6}px">${inner}<span class="o-thumb" style="--t:${3 + thumbTop}px;--h:${thumb}px"></span></div>`;
  }

  /* A list taller than maxRows scrolls inside an 8 px thumb gutter; its header
   * takes the gutter too (no-scroll drops it), so the figures stay under it. */
  const scrolls = (rows, maxRows) => Boolean(maxRows) && rows > maxRows;
  const headScroll = (rows, maxRows) => (scrolls(rows, maxRows) ? "" : " no-scroll");

  const deathHead = () =>
    `<div class="o-th" style="padding-left:6px"><span style="width:32px;text-align:right">Time</span><span style="width:128px;padding-left:22px;text-align:left">Player</span><span class="grow">Killed by</span><span>Last hit</span></div>`;

  /* The deaths list: when, who, what killed them, the killing blow. */
  function deathRows(ctx, f, { hover = -1, focus = -1 } = {}) {
    return `<div class="rp-deaths">${f.deathList
      .map((d, i) => {
        const p = f.players.find((x) => x.name === d.who);
        return `<div class="rp-death${p.you ? " is-you" : ""}${i === hover ? " is-hover" : ""}${i === focus ? " is-focus" : ""}" role="button" tabindex="-1" aria-label="${esc(d.who)} died at ${time(d.at)}, killed by ${esc(d.by)}, ${esc(d.skill)}"><span class="at">${time(d.at)}</span><span class="who"><img class="o-icon" src="${p.icon}" alt="${CLASS_NAME[p.cls]}"><b>${nm(ctx, d.who)}</b></span><span class="by"><span class="faint">by</span>${esc(d.by)}<small>${esc(d.skill)}</small></span><span class="blow">${compact(d.hit)}</span></div>`;
      })
      .join("")}</div>`;
  }

  const noDeaths = () => `<div class="rp-quiet is-good">${icon("check")}Nobody died</div>`;

  function reportFoot() {
    return `<div class="o-foot"><span class="rp-hint">${icon("mousePointer")}Click a player for their skills, buffs, healing and damage taken</span></div>`;
  }

  // ---- The player view (report drill-in or the detail window) --------------------

  /* s: { tab: "damage" | "buffs" | "healing" | "taken", where: "report" | "detail",
   *      expanded (skill index), hoverRow, focusTab, hoverBack }
   *   where "detail" is the not-chosen alternative, kept as history. */
  function playerView(ctx, f, name, s = {}) {
    const p = player(f, name);
    const where = s.where || "report";
    const tab = s.tab || "damage";
    const ident = `<span class="det-who"><img class="cls" src="${p.icon}" alt="${CLASS_NAME[p.cls]}"><b>${nm(ctx, p.name)}</b><span class="srv">${p.server}</span></span><span class="det-cp"><img src="../../../crates/app/assets/game/profile_power_icon_pc.png" alt="Combat power">${int(p.cp)}</span>`;
    let head;
    if (where === "report") {
      head = `<div class="o-head" style="padding-left:0"><span class="o-tbtn is-quiet rp-back${s.hoverBack ? " is-hot" : ""}" role="button" tabindex="-1">${icon("chevronLeft")}Report</span>${ident}<span class="grow"></span>${btn("close", "Close")}</div>
<div class="det-sub">${icon("fileText")}${esc(f.boss)}<span class="faint">·</span>${f.when}<span class="faint">·</span>${time(f.secs)}</div>`;
    } else {
      head = `<div class="o-head">${ident}<span class="grow"></span><span class="o-chip is-warn">${icon("history")}History</span>${btn("close", "Close")}</div>
<div class="det-sub">${icon("clock")}${esc(f.boss)}<span class="faint">·</span>${f.when}, saved record</div>`;
    }

    const v = (label, value, none) => `<div><span>${label}</span><b${none ? ' class="is-none"' : ""}>${value}</b></div>`;
    const hits = p.skills.reduce((t, k) => t + k.hits, 0);
    const cnt = (k) => p.skills.reduce((t, x) => t + x.count[k], 0);
    const figs = `<div class="det-figs">${v("Damage", compact(p.dmg))}${v("DPS", compact(p.dps))}${v("Healing", f.old ? "--" : p.heal ? compact(p.heal) : "0", f.old || !p.heal)}${v("Damage taken", f.old ? "--" : p.taken ? compact(p.taken) : "0", f.old || !p.taken)}${v("Deaths", f.old ? "--" : p.deaths.length, f.old || !p.deaths.length)}</div>`;
    const hv = (label, k) => `<div><span>${label}</span><b>${int(cnt(k))}<small>${p1(cnt(k) / hits)}</small></b></div>`;
    const types = `<div class="det-figs is-small">${hv("Critical", "crit")}${hv("Back", "back")}${hv("Front", "front")}${hv("Double", "double")}${hv("Perfect", "perfect")}${hv("Parry", "parry")}${hv("Multi-hit", "multi")}</div>`;
    const deaths = p.deaths.length
      ? `<div class="rp-pdeaths">${p.deaths
          .map((d) => `<div>${icon("skull")}<b>Died at ${time(d.at)}</b><span class="src">${esc(d.by)} <span class="muted">· ${esc(d.skill)}</span></span><span class="grow"></span><small>last hit ${compact(d.hit)}</small></div>`)
          .join("")}</div>`
      : "";
    const old = f.old ? oldNote(4) : "";

    const buffCount = BUFFS_ON.length + BUFFS_BOSS.length;
    const t = (id, label, n, off) =>
      `<span class="o-tab${tab === id ? " is-on" : ""}${s.focusTab === id ? " is-focus" : ""}${s.hoverTab === id ? " is-hover" : ""}${off ? " is-off" : ""}" role="tab" aria-selected="${tab === id}">${label}${n !== null ? `<small>${n}</small>` : ""}</span>`;
    const tabs = `<div class="o-tabs rp-ptabs" role="tablist">${t("damage", "Damage", p.skills.length)}${t("buffs", "Buffs", buffCount)}${t("healing", "Healing", f.old ? null : p.heals.length, f.old)}${t("taken", "Damage taken", f.old ? null : p.sources.length, f.old)}</div>`;

    let body = "";
    if (tab === "damage") body = skillTable(p, s);
    if (tab === "buffs") body = buffList(ctx, f, p, s);
    if (tab === "healing") body = healTable(ctx, f, p, s);
    if (tab === "taken") body = takenTable(ctx, f, p, s);

    return `<div class="ov o rp">${head}${figs}${types}${deaths}${old}${tabs}${body}</div>`;
  }

  const spec = (slots) => `<span class="spec">${[1, 2, 3, 4, 5].map((n) => `<i class="${slots.includes(n) ? "on" : ""}"></i>`).join("")}</span>`;
  const skIcon = (src) => (src ? `<img class="sk-icon" src="${src}" alt="">` : `<span class="sk-icon is-none"></span>`);

  function skillTable(p, s) {
    const max = p.skills[0].total;
    const n = p.skills.length + (s.expanded != null ? 4 : 0);
    const head = `<div class="o-th is-skills sk-head${headScroll(n, 9)}"><span class="grow">Skill<span class="faint">${p.skills.length}</span></span><span class="c-n">Hits</span><span class="c-n">Crits</span><span class="c-p">Crit %</span><span class="c-t is-sort">Damage${icon("arrowDown")}</span><span class="c-chev"></span></div>`;
    const more = (k) => {
      const m = (label, val) => `<div><span>${label}</span><b>${val}</b></div>`;
      const hc = (key) => `${int(k.count[key])}<small>${p1(k.count[key] / k.hits)}</small>`;
      return `<div class="sk-more">${m("Perfect", hc("perfect"))}${m("Double", hc("double"))}${m("Front", hc("front"))}${m("Back", hc("back"))}${m("Parry", hc("parry"))}${m("Multi-hit", hc("multi"))}${m("Multi-hit damage", k.multiDmg ? int(k.multiDmg) : "--")}${m("Spec", spec(k.spec))}${m("Min", int(k.min))}${m("Max", int(k.max))}${m("Average", int(k.total / k.hits))}${m("Total", int(k.total))}</div>`;
    };
    const rows = p.skills
      .map((k, i) => {
        const open = s.expanded === i;
        return `<div class="o-row${s.hoverRow === i ? " is-hover" : ""}${s.focusRow === i ? " is-focus" : ""}"><span class="o-track" style="--rgb:${CLASS_RGB[p.cls]}"><span class="o-fill" style="width:${Math.max(6, (k.total / max) * 100).toFixed(1)}%"></span>${skIcon(k.icon)}<span class="o-name">${esc(k.name)}</span><span class="o-num c-n">${int(k.hits)}</span><span class="o-num c-n">${int(k.count.crit)}</span><span class="o-num c-p">${p1(k.count.crit / k.hits)}</span><span class="o-num c-t o-strong">${compact(k.total)}<small>${p1(k.total / p.dmg)}</small></span><span class="o-num c-chev"${s.hoverRow === i ? ' style="color:var(--o-text)"' : ""}>${icon(open ? "chevronUp" : "chevronDown")}</span></span></div>${open ? more(k) : ""}`;
      })
      .join("");
    return `${head}<div class="sk rp">${scroll(`<div class="o-rows">${rows}</div>`, { rows: n, maxRows: 9, rowH: 27 })}</div>`;
  }

  function buffList(ctx, f, p, s) {
    const rows = (list) =>
      list
        .map(([code, name, caster, secs]) => {
          const by = f.players.find((x) => x.name === caster) || p;
          const share = secs / f.secs;
          return `<div class="bf"><img src="${skillIcon(code)}" alt=""><div class="bf-main"><div class="bf-line"><span class="nm">${esc(name)}</span><span class="by"><img src="${classIcon(by.cls)}" alt="${CLASS_NAME[by.cls]}">${nm(ctx, caster)}</span></div><div class="bf-bar" style="--rgb:${CLASS_RGB[by.cls]}"><span class="o-fill" style="width:${(share * 100).toFixed(1)}%"></span></div></div><span class="bf-up">${time(secs)}</span><span class="bf-cov">${p0(share)}</span></div>`;
        })
        .join("");
    const sorted = (l) => l.slice().sort((a, b) => b[3] - a[3]);
    return `<div class="bf-th"><span class="grow">Buff and caster</span><span class="h-up">Up for</span><span class="h-cov">of ${time(f.secs)}</span></div>
<div class="rp-scroll has-thumb" style="height:${s.tall ? 330 : 286}px"><div class="bf-sec">On ${nm(ctx, p.name)}<span class="o-count">${BUFFS_ON.length}</span></div><div class="bf-list">${rows(sorted(BUFFS_ON))}</div><div class="bf-sec">On ${esc(f.boss)}<span class="o-count">${BUFFS_BOSS.length}</span></div><div class="bf-list">${rows(sorted(BUFFS_BOSS))}</div><span class="o-thumb" style="--t:5px;--h:190px"></span></div>`;
  }

  function healTable(ctx, f, p, s) {
    if (f.old) return `<div class="o-empty"><strong>Not recorded</strong>This fight was saved before the meter kept healing.</div>`;
    if (!p.heals.length) return `<div class="o-empty"><strong>No healing</strong>${nm(ctx, p.name)} cast no heals in this fight.</div>`;
    const max = p.heals[0].total;
    const head = `<div class="o-th is-skills sk-head${headScroll(p.heals.length, 9)}"><span class="grow">Skill<span class="faint">${p.heals.length}</span></span><span class="c-n">Heals</span><span class="c-n">Crits</span><span class="c-p">Crit %</span><span class="c-t is-sort">Healed${icon("arrowDown")}</span><span class="c-chev"></span></div>`;
    const rows = p.heals
      .map(
        (k, i) =>
          `<div class="o-row${s.hoverRow === i ? " is-hover" : ""}"><span class="o-track" style="--rgb:${CLASS_RGB[p.cls]}"><span class="o-fill" style="width:${Math.max(6, (k.total / max) * 100).toFixed(1)}%"></span>${skIcon(k.icon)}<span class="o-name">${esc(k.name)}</span><span class="o-num c-n">${int(k.heals)}</span><span class="o-num c-n">${int(k.crits)}</span><span class="o-num c-p">${p1(k.crits / k.heals)}</span><span class="o-num c-t o-strong">${compact(k.total)}<small>${p1(k.total / p.heal)}</small></span><span class="o-num c-chev"></span></span></div>`
      )
      .join("");
    return `${head}<div class="sk rp">${scroll(`<div class="o-rows">${rows}</div>`, { rows: p.heals.length, maxRows: 9, rowH: 27 })}</div>`;
  }

  function takenTable(ctx, f, p, s) {
    if (f.old) return `<div class="o-empty"><strong>Not recorded</strong>This fight was saved before the meter kept damage taken.</div>`;
    if (!p.sources.length) return `<div class="o-empty"><strong>No damage taken</strong>Nothing hit ${nm(ctx, p.name)} in this fight.</div>`;
    const max = p.sources[0].total;
    const head = `<div class="o-th is-skills sk-head${headScroll(p.sources.length, 9)}" style="padding-left:13px"><span class="grow">Monster skill<span class="faint">${p.sources.length}</span></span><span class="c-n">Hits</span><span class="c-p">Largest</span><span class="c-t is-sort">Damage${icon("arrowDown")}</span><span class="c-chev"></span></div>`;
    const rows = p.sources
      .map(
        (k, i) =>
          `<div class="o-row${s.hoverRow === i ? " is-hover" : ""}"><span class="o-track is-plain"><span class="o-fill is-hp" style="width:${Math.max(6, (k.total / max) * 100).toFixed(1)}%"></span><span class="o-name">${esc(k.skill)}<small>${esc(k.by)}</small></span><span class="o-num c-n">${int(k.hits)}</span><span class="o-num c-p">${compact((k.total / k.hits) * 1.6)}</span><span class="o-num c-t o-strong">${compact(k.total)}<small>${p1(k.total / p.taken)}</small></span><span class="o-num c-chev"></span></span></div>`
      )
      .join("");
    return `${head}<div class="sk rp">${scroll(`<div class="o-rows">${rows}</div>`, { rows: p.sources.length, maxRows: 9, rowH: 27 })}</div>`;
  }

  window.Report = {
    icon,
    fmt: { int, compact, p1, p0, time, esc, mask },
    CLASS_RGB,
    CLASS_NAME,
    classIcon,
    fight,
    player,
    nm,
    btn,
    reportHead,
    figures,
    oldNote,
    playerRow,
    partySources,
    sourceRows,
    scroll,
    deathHead,
    deathRows,
    noDeaths,
    reportFoot,
    playerView,
  };

  /* The report window around a direction's body. */
  window.Report.window = function (ctx, f, body) {
    return `<div class="ov o rp">${reportHead(ctx, f)}${figures(f)}${f.old ? oldNote() : ""}${body}${reportFoot()}</div>`;
  };

  /* A player opened: inside the report, with "‹ Report" (decided 2026-10-07). */
  window.Report.playerState = function (ctx, f, name, s = {}) {
    return playerView(ctx, f, name, { where: "report", ...s });
  };

  /* Long names: server tags shown, so names have to give way. */
  window.Report.withServer = (ctx) => ({ ...ctx, opts: { ...ctx.opts, server: true } });

  /* The invented game backdrop (kit.js) names the approved pages' invented
     boss on its nameplate; name this page's boss instead. */
  const plateName = "Basilus the False";
  const plate = () =>
    document.querySelectorAll(".fx .plate").forEach((el) => {
      if (el.textContent !== plateName) el.textContent = plateName;
    });
  new MutationObserver(plate).observe(document.documentElement, { childList: true, subtree: true });
})();
