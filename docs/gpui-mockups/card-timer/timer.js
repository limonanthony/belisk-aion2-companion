// The card timer round, 2026-10-10, not approved yet. The soul tracker card
// (../approved/pets.js, Pets.card) and the reminder card (the approved
// reminders.html markup, hub.css) drawn with each timer under study:
//   line  today's 2 px time-left line under the card (reference)
//   a     a hollow ring left of the × (the user's idea)
//   b     a ring around the × itself
//   c     a ring around the header's icon (paw, bell)
//   d     the card's outline draining
//   e     the seconds left, "8 s", before the ×
//   f     a on the pet card, the reminder card keeps its line
// The approved files are not edited: the pet card is Pets.card()'s HTML
// with its time-left line replaced. Styles: timer.css.
//
// Values, pets and times are invented. Nothing is saved or sent anywhere.
// Icons: Lucide (ISC), through ../approved/kit.js, hub.js and pets.js.
(function () {
  "use strict";

  const icon = Pets.icon;
  const DUR = { pet: 10, rem: 30 };
  const off = (f) => (-(1 - Math.max(0, Math.min(1, f))) * 100).toFixed(2);

  // A hollow ring: `size` px, `stroke` px; the arc is the time left,
  // emptying clockwise from 12 o'clock.
  function ring(size, stroke, frac, cls = "") {
    const c = size / 2;
    const r = (size - stroke) / 2;
    return `<svg class="ct-ring${cls}" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" aria-hidden="true"><circle class="tr" cx="${c}" cy="${c}" r="${r}" stroke-width="${stroke}"/><circle class="arc ct-arc" cx="${c}" cy="${c}" r="${r}" stroke-width="${stroke}" pathLength="100" transform="rotate(-90 ${c} ${c})" style="stroke-dashoffset:${off(frac)}"/></svg>`;
  }

  const outline = (frac) => `<svg class="ct-outline" aria-hidden="true"><rect class="tr"/><rect class="arc ct-arc" pathLength="100" style="stroke-dashoffset:${off(frac)}"/></svg>`;

  const secs = (left) => `${Math.max(0, Math.ceil(left - 0.001))} s`;

  // f: the ring on the pet card, the line on the reminder card.
  const timerOf = (kind, t) => (t === "f" ? (kind === "pet" ? "a" : "line") : t);

  // o: { timer, frac (time left, 0..1), hover (paused), locked, live, size }
  function dress(html, kind, o) {
    const t = timerOf(kind, o.timer);
    const cls = [`ct-t-${t}`, o.hover && !o.locked && "is-paused", o.locked && "is-locked", o.live && "ct-live"].filter(Boolean).join(" ");
    html = html.replace(/^<div class="ov rm/, `<div data-ct="${kind}" class="ov rm ${cls}`);
    // Since the approval (2026-10-10) the approved cards draw ring A
    // themselves (hub.css .rm-timer); this round strips it and draws the
    // option under study, today's line included, as when it was chosen.
    html = html.replace(/<span class="rm-timer[\s\S]*?<\/svg><\/span>/, "");
    html = html.replace(/<div class="rm-left">[\s\S]*?<\/div>/, "");
    if (t === "line") html = html.replace(/<\/div>$/, `<div class="rm-left"><i style="width:${(o.frac * 100).toFixed(1)}%"></i></div></div>`);
    html = html.replace(/<div class="rm-head">([\s\S]*?)<\/div>/, (m, inner) => {
      const xm = inner.match(/<span class="(?:pc|rm)-x[\s\S]*?<\/span>/);
      const x = xm ? xm[0] : "";
      let rest = x ? inner.replace(x, "") : inner;
      let right = x;
      if (t === "a") right = `<span class="ct-slot${x ? "" : " is-solo"}">${ring(o.size || 12, 2, o.frac)}</span>${x}`;
      else if (t === "b") right = x ? x.replace(/^(<span[^>]*>)/, `$1${ring(22, 1.5, o.frac, " is-x")}`) : `<span class="ct-xsolo">${ring(22, 1.5, o.frac, " is-x")}</span>`;
      else if (t === "e") right = `<span class="ct-secs">${secs(o.frac * DUR[kind])}</span>${x}`;
      if (t === "c") rest = rest.replace(/^(<svg[\s\S]*?<\/svg>)/, `<span class="ct-ic">${ring(18, 1.5, o.frac, " is-ic")}$1</span>`);
      return `<div class="rm-head">${rest}${right}</div>`;
    });
    if (t === "d") html = html.replace(/^(<div [^>]*>)/, `$1${outline(o.frac)}`);
    return html;
  }

  function petCard(items, o) {
    const html = Pets.card(items, { lang: o.lang || "en", hover: o.hover, locked: o.locked, left: o.frac });
    return dress(html, "pet", o);
  }

  // The reminder card, as approved (reminders.html): bell, "In 1 min",
  // "·", the time, × when unlocked; each event's name and fact.
  const EV1 = { at: "22:00", ev: [["Shugo Festival", "3 min to join"]] };
  const EV3 = {
    at: "19:00",
    ev: [
      ["Arena of Tactics opens", "The 10 vs 10 queue, until 21:00"],
      ["Watcher Kaira", "Field boss in Chaotic Lower Reshanta"],
      ["Shugo Festival", "The minigames, with 3 min to join"],
    ],
  };

  function remCard(R, o) {
    const x = o.locked ? "" : `<span class="rm-x${o.hover ? " is-hot" : ""}" role="button" aria-label="Dismiss">${icon("close")}</span>`;
    const head = `<div class="rm-head">${icon("bell")}<span class="rm-when">In 1 min</span><span class="rm-dot">·</span><span class="rm-at">${R.at}</span>${x}</div>`;
    const evs = R.ev.map(([n, d]) => `<div class="rm-ev"><div class="rm-name">${n}</div><div class="rm-detail">${d}</div></div>`).join("");
    return dress(`<div class="ov rm" role="status" aria-label="Event reminder">${head}${evs}<div class="rm-left"><i style="width:62%"></i></div></div>`, "rem", o);
  }

  // ---- Pets (the approved invented data) --------------------------------
  const G = {
    gain: Pets.gainOf("Fossa", { lv: 2, souls: 27 }, 1),
    gain2: Pets.gainOf("Fossa", { lv: 2, souls: 27 }, 2),
    up: Pets.gainOf("Hill Coradon", { lv: 1, souls: 24 }, 5),
    slime: Pets.gainOf("Slime", { lv: 1, souls: 12 }, 1),
    unlock: Pets.gainOf("Forest Spirit", { lv: 0, souls: 4 }, 1),
  };
  const ONE = [G.gain];
  const TWO = [G.gain, G.slime];
  const THREE = [G.gain, G.slime, G.up];

  // ---- Page harness ---------------------------------------------------------
  const params = new URLSearchParams(location.search);
  const OPTS = {
    alpha: Number(params.get("alpha") || 150),
    scale: Number(params.get("scale") || 100),
    scene: params.get("scene") || "dark",
    timer: ["line", "a", "b", "c", "d", "e", "f"].includes(params.get("timer")) ? params.get("timer") : "a",
  };

  const TAGS = { rec: ["is-rec", "Recommended"], proposed: ["is-proposed", "Proposed"], agreed: ["is-agreed", "Approved today"] };
  const tagHtml = (t) => (Array.isArray(t) ? t : [t]).map((k) => `<span class="dec ${TAGS[k][0]}">${TAGS[k][1]}</span>`).join("");
  const col = (...p) => `<div class="ct-col">${p.join("")}</div>`;
  const bar = () => Hub.toolbar({}, { v: Hub.view() });

  function fx(seed) {
    let s = seed * 9301 + 49297;
    const rnd = () => (s = (s * 9301 + 49297) % 233280) / 233280;
    let out = "";
    for (let i = 0; i < 5; i++) {
      const crit = rnd() < 0.4;
      out += `<span class="${crit ? "crit" : "hit"}" style="left:${(55 + rnd() * 40).toFixed(1)}%;top:${(rnd() * 90).toFixed(1)}%">${Math.round(2000 + rnd() * (crit ? 50000 : 15000)).toLocaleString("en-US")}</span>`;
    }
    return `<div class="fx" aria-hidden="true">${out}</div>`;
  }

  const NAMES = {
    line: "Today: the 2 px line",
    a: "A. A ring left of the ×",
    b: "B. A ring around the ×",
    c: "C. A ring around the icon",
    d: "D. The outline draining",
    e: "E. Seconds left",
    f: "A on the pet card, the line on the reminder",
  };

  function sections() {
    const P = (items, o = {}) => petCard(items, { timer: "a", frac: 0.62, ...o });
    const Rm = (R, o = {}) => remCard(R, { timer: "a", frac: 0.62, ...o });
    const alt = (t) => col(P(TWO, { timer: t }), Rm(EV1, { timer: t, hover: true, frac: 0.4 }), P(ONE, { timer: t, locked: true, frac: 0.8 }));
    const both = (o = {}) => col(Rm(EV1, o), P(TWO, o));
    return [
      {
        id: "today",
        title: "Today: two bars on the pet card",
        sub: "as built: each pet's 4 px progress bar, then the card's 2 px time-left line 8 px under the last one",
        tileMin: 380,
        stageMin: 230,
        tiles: [
          { tag: "agreed", title: "Pet card, one pet", sub: "the progress bar and the time-left line read as two bars of the same kind", html: P(ONE, { timer: "line" }) },
          { tag: "agreed", title: "Pet card, three pets", sub: "four bars: which one is the time?", html: P(THREE, { timer: "line" }) },
          { tag: "agreed", title: "Reminder card, one event", sub: "no other bar here, so its line reads as time", html: Rm(EV1, { timer: "line" }) },
          { tag: "agreed", title: "Reminder card, three events", sub: "30 s, paused under the pointer", html: Rm(EV3, { timer: "line" }) },
          { tag: "agreed", title: "Both under the toolbar", sub: "the meter closed: the reminder next to the toolbar, the pet card 6 px beyond it", html: col(bar(), Rm(EV1, { timer: "line" }), P(TWO, { timer: "line" })) },
        ],
      },
      {
        id: "ring",
        title: "A. The user's idea: a hollow ring left of the ×",
        sub: "12 px, 2 px stroke, 8 px hole; track gauge-empty, the arc muted; it empties clockwise over 10 s (pet card) and 30 s (reminder card). The time-left line goes, so each card is 10 px shorter",
        tileMin: 380,
        stageMin: 230,
        tiles: [
          { tag: ["proposed", "rec"], title: "Running", sub: "62% left: the arc from the clock hand round to 12 o'clock", html: both() },
          { tag: ["proposed", "rec"], title: "Three pets, three events", sub: "the only bars left are the pets' progress", html: col(Rm(EV3), P(THREE)) },
          { tag: "proposed", title: "Pointer over the card: paused", sub: "the ring stops and its arc turns text, next to the × that lights (the pointer's two signs); a click dismisses", html: both({ hover: true, frac: 0.4 }) },
          { tag: "proposed", title: "Locked overlay", sub: "click-through, no ×: the ring takes the ×'s slot, so its centre does not move; it runs out by itself", html: both({ locked: true, frac: 0.8 }) },
          { tag: "proposed", title: "The last seconds", sub: "12% left (about 1 s on the pet card, 4 s on the reminder): a sliver; no colour change, nothing flashes", html: both({ frac: 0.12 }) },
          { tag: "proposed", title: "A gain, before", sub: "Fossa's card with 3 s left", html: P(ONE, { frac: 0.3 }) },
          { tag: "proposed", title: "A gain, after", sub: "Fossa +1 and Slime +1: Slime is added at the end; the ring is full again at once (no sweep back) and the 10 s start over", html: P([G.gain2, G.slime], { frac: 1 }) },
          { tag: "proposed", title: "Under the toolbar", sub: "both cards with the ring: one timer language", html: col(bar(), Rm(EV1, { frac: 0.75 }), P(TWO, { frac: 0.35 })) },
          { tag: "proposed", title: "Variant: the pet card only", sub: "the ring on the pet card, the reminder card keeps its line (it has no other bar)", html: col(bar(), Rm(EV1, { timer: "line", frac: 0.75 }), P(TWO, { frac: 0.35 })) },
        ],
      },
      {
        id: "ring-game",
        title: "A over the game",
        sub: "fixed scenes and opacities (the selectors at the top do not change this section); opacity is Settings › Overlay's background, 0 to 255",
        tileMin: 380,
        stageMin: 300,
        tiles: [
          { title: "Dark, opacity 150 (the default)", scene: "dark", alpha: 150, html: both() },
          { title: "Bright, opacity 150", scene: "bright", alpha: 150, html: both() },
          { title: "Bright, opacity 60", scene: "bright", alpha: 60, html: both() },
          { title: "Busy, opacity 255", scene: "busy", alpha: 255, html: both() },
          { title: "Dark, opacity 60", scene: "dark", alpha: 60, html: both() },
          { title: "Busy, opacity 60, last seconds", scene: "busy", alpha: 60, html: both({ frac: 0.12 }) },
        ],
      },
      {
        id: "closeup",
        title: "Close-up at 200%",
        sub: "the header at twice the size, to judge the ring's weight; 12 px is proposed, 14 px drawn for comparison",
        tileMin: 680,
        stageMin: 230,
        tiles: [
          { tag: ["proposed", "rec"], title: "A, 12 px", scale: 2, html: P(ONE) },
          { tag: "proposed", title: "A, 14 px", sub: "heavier; close to the × icon's 13 px", scale: 2, html: P(ONE, { size: 14 }) },
          { tag: "proposed", title: "A, paused", scale: 2, html: P(ONE, { hover: true, frac: 0.4 }) },
          { tag: "proposed", title: "B, around the ×", scale: 2, html: P(ONE, { timer: "b" }) },
          { tag: "proposed", title: "C, around the paw", scale: 2, html: P(ONE, { timer: "c" }) },
          { tag: "proposed", title: "E, seconds left", scale: 2, html: P(ONE, { timer: "e" }) },
        ],
      },
      {
        id: "other",
        title: "Other ideas",
        sub: "each drawn running (pet card, two pets), paused (reminder card) and locked (pet card)",
        tileMin: 380,
        stageMin: 360,
        tiles: [
          { tag: "proposed", title: "B. A ring around the ×", sub: "the × button turns round and the 22 px ring runs around it: the timer says what it closes. Locked: the ring alone", html: alt("b") },
          { tag: "proposed", title: "C. A ring around the header's icon", sub: "the paw or the bell inside an 18 px ring; the × untouched", html: alt("c") },
          { tag: "proposed", title: "D. The card's outline draining", sub: "the 1 px border is the track; a 1.5 px arc runs round the card and empties clockwise from the top-left corner", html: alt("d") },
          { tag: "proposed", title: "E. Seconds left", sub: "\"8 s\" before the ×, counting down each second", html: alt("e") },
        ],
      },
      {
        id: "try",
        title: "Try it",
        sub: "both cards with their real timers: 10 s for the pet card, 30 s for the reminder card; the pointer over a card pauses it, a click dismisses it. Pick the timer in the panel",
        tileMin: 900,
        stageMin: 460,
        wide: true,
        tiles: [{ title: "Working cards", sub: "the meter closed: the toolbar, the reminder next to it, the pet card 6 px beyond. Nothing is saved", html: '<div class="ct-try" id="ct-try"></div>' }],
      },
    ];
  }

  // ---- Working tile ----------------------------------------------------------
  const START = { Fossa: { lv: 2, souls: 27 }, Slime: { lv: 1, souls: 12 }, "Hill Coradon": { lv: 1, souls: 24 }, Kalgolem: { lv: 3, souls: 75 }, "Forest Spirit": { lv: 0, souls: 4 } };
  const LIVE = { timer: OPTS.timer, locked: false, souls: {}, pet: null, rem: null };
  const resetSouls = () => (LIVE.souls = JSON.parse(JSON.stringify(START)));
  resetSouls();

  const host = (k) => document.getElementById(`ct-${k}`);
  const cardEl = (k) => host(k) && host(k).querySelector(":scope > .ov");

  function draw(k, enter) {
    const s = LIVE[k];
    const el = host(k);
    if (!el) return;
    if (!s) {
      el.innerHTML = "";
      return;
    }
    const o = { timer: LIVE.timer, frac: s.left / DUR[k], locked: LIVE.locked, live: true };
    el.innerHTML = k === "pet" ? petCard(s.items, o) : remCard(EV1, o);
    if (enter) cardEl(k).classList.add("ct-enter");
  }

  function paint(k) {
    const s = LIVE[k];
    const el = cardEl(k);
    if (!s || !el) return;
    const f = s.left / DUR[k];
    el.querySelectorAll(".ct-arc").forEach((a) => (a.style.strokeDashoffset = off(f)));
    const line = el.querySelector(".rm-left i");
    if (line) line.style.width = (f * 100).toFixed(2) + "%";
    const t = el.querySelector(".ct-secs");
    if (t) t.textContent = secs(s.left);
  }

  function dismiss(k) {
    const s = LIVE[k];
    const el = cardEl(k);
    if (!s || s.gone || !el) return;
    s.gone = true;
    el.classList.add("is-gone");
    setTimeout(() => {
      if (LIVE[k] === s) {
        LIVE[k] = null;
        draw(k);
      }
    }, 160);
  }

  let last = performance.now();
  function frame(now) {
    const dt = Math.min(0.25, (now - last) / 1000);
    last = now;
    for (const k of ["pet", "rem"]) {
      const s = LIVE[k];
      const el = cardEl(k);
      if (!s || s.gone || !el) continue;
      const paused = !LIVE.locked && el.matches(":hover");
      el.classList.toggle("is-paused", paused);
      if (!paused) s.left = Math.max(0, s.left - dt);
      paint(k);
      if (s.left <= 0) dismiss(k);
    }
    requestAnimationFrame(frame);
  }

  function gain(name, n) {
    const p = LIVE.souls[name];
    const before = { ...p };
    const after = Pets.addSouls(p, n);
    Object.assign(p, after);
    const ev = before.lv === 0 && after.lv >= 1 ? "unlock" : after.lv > before.lv ? "up" : before.lv >= Pets.CAP ? "max" : "gain";
    const item = { p: Pets.byName(name), lv: after.lv, souls: after.souls, ev };
    const s = LIVE.pet;
    if (s && !s.gone) {
      const it = s.items.find((x) => x.p.name === name);
      if (it) Object.assign(it, { lv: item.lv, souls: item.souls, ev: ev === "up" || ev === "unlock" ? ev : it.ev });
      else {
        s.items.push(item);
        if (s.items.length > 3) s.items.shift();
      }
      s.left = DUR.pet;
      draw("pet", false);
    } else {
      LIVE.pet = { items: [item], left: DUR.pet };
      draw("pet", true);
    }
  }

  function reminder() {
    const fresh = !LIVE.rem || LIVE.rem.gone;
    LIVE.rem = { left: DUR.rem };
    draw("rem", fresh);
  }

  function tryMount() {
    const root = document.getElementById("ct-try");
    if (!root) return;
    const opt = (v) => `<option value="${v}"${v === LIVE.timer ? " selected" : ""}>${NAMES[v]}</option>`;
    root.innerHTML = `<div class="ct-col">${bar()}<div id="ct-rem"></div><div id="ct-pet"></div></div><div class="mk-sim"><b>Page only: what the game and the clock do</b><label>Timer <select data-ct-timer>${["a", "line", "b", "c", "d", "e", "f"].map(opt).join("")}</select></label><button type="button" data-ct-gain="Fossa" data-n="1">Fossa +1</button><button type="button" data-ct-gain="Slime" data-n="1">Slime +1</button><button type="button" data-ct-gain="Hill Coradon" data-n="5">Hill Coradon +5 (level up)</button><button type="button" data-ct-gain="Kalgolem" data-n="1">Kalgolem +1 (at Lv 3)</button><button type="button" data-ct-rem="1">A reminder (30 s)</button><button type="button" data-ct-lock="1">${LIVE.locked ? "Unlock the overlay" : "Lock the overlay"}</button><button type="button" data-ct-reset="1">Start again</button></div>`;
    draw("rem");
    draw("pet");
  }

  document.addEventListener("click", (e) => {
    if (!e.target.closest("#ct-try")) return;
    const t = (s) => e.target.closest(s);
    let el;
    if ((el = t("[data-ct-gain]"))) gain(el.dataset.ctGain, Number(el.dataset.n));
    else if (t("[data-ct-rem]")) reminder();
    else if (t("[data-ct-lock]")) {
      LIVE.locked = !LIVE.locked;
      el = t("[data-ct-lock]");
      el.textContent = LIVE.locked ? "Unlock the overlay" : "Lock the overlay";
      draw("pet");
      draw("rem");
    } else if (t("[data-ct-reset]")) {
      Object.assign(LIVE, { pet: null, rem: null, locked: false });
      resetSouls();
      tryMount();
    } else if (t("#ct-pet > .ov")) dismiss("pet");
    else if (t("#ct-rem > .ov")) dismiss("rem");
  });

  document.addEventListener("change", (e) => {
    if (!e.target.matches("[data-ct-timer]")) return;
    LIVE.timer = e.target.value;
    draw("pet");
    draw("rem");
  });

  // ---- Mount -------------------------------------------------------------------
  function controlsHtml() {
    const sel = (id, label, cur, opts) => `<label>${label} <select id="${id}">${opts.map(([v, t]) => `<option value="${v}"${v === cur ? " selected" : ""}>${t}</option>`).join("")}</select></label>`;
    return `<div class="controls" role="region" aria-label="Mockup controls"><div class="controls__inner">
<label>Background opacity <input type="range" id="c-alpha" min="0" max="255" value="${OPTS.alpha}"><output id="o-alpha">${OPTS.alpha}</output></label>
<label>Content scale <input type="range" id="c-scale" min="70" max="150" step="5" value="${OPTS.scale}"><output id="o-scale">${OPTS.scale}%</output></label>
${sel("c-scene", "Scene", OPTS.scene, [["dark", "Dark"], ["busy", "Busy"], ["bright", "Bright"]])}
</div></div>`;
  }

  function applyVars() {
    const r = document.documentElement.style;
    r.setProperty("--bg-a", (OPTS.alpha / 255).toFixed(3));
    r.setProperty("--scale", (OPTS.scale / 100).toFixed(2));
  }

  function render() {
    applyVars();
    let html = "";
    let seed = 1;
    let n = 0;
    for (const sec of sections()) {
      n++;
      html += `<section class="scene-group ct-sec" id="${sec.id}"><h2>${n}. ${sec.title}<span>${sec.sub}</span></h2><div class="tiles${sec.wide ? " is-wide" : ""}" style="--tile-min:${sec.tileMin}px">`;
      for (const t of sec.tiles) {
        const vars = [`--stage-min:${sec.stageMin}px`, t.alpha != null && `--bg-a:${(t.alpha / 255).toFixed(3)}`, t.scale && `--scale:${t.scale}`].filter(Boolean).join(";");
        html += `<figure class="tile" style="margin:0"><figcaption class="tile__cap">${t.tag ? tagHtml(t.tag) : ""}<b>${t.title}</b>${t.sub ? `<span class="sub">${t.sub}</span>` : ""}</figcaption><div class="tile__stage scene--${t.scene || OPTS.scene}" style="${vars}">${fx(seed++)}${t.html}</div></figure>`;
      }
      html += `</div></section>`;
    }
    document.querySelector("main.scenes").innerHTML = html;
    tryMount();
  }

  function mount() {
    const main = document.querySelector("main.scenes");
    main.insertAdjacentHTML("beforebegin", controlsHtml());
    const bind = (id, k, read, out) =>
      document.getElementById(id).addEventListener("input", (e) => {
        OPTS[k] = read(e.target);
        if (out) document.getElementById(out[0]).textContent = out[1](OPTS[k]);
        if (k === "scene") render();
        else applyVars();
      });
    bind("c-alpha", "alpha", (e) => Number(e.value), ["o-alpha", (v) => v]);
    bind("c-scale", "scale", (e) => Number(e.value), ["o-scale", (v) => v + "%"]);
    bind("c-scene", "scene", (e) => e.value);
    render();
    requestAnimationFrame(frame);
  }

  window.CardTimer = { petCard, remCard, LIVE, gain, reminder };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", mount);
  else mount();
})();
