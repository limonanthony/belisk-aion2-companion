// The pet card layout round, 2026-10-10, not approved yet. The approved
// soul tracker card (../approved/pets.js, Pets.card) drawn as today and
// with the two readings of the request:
//   A  the pet block starts at the paw icon's left edge; 8 px above the bar
//   B  one pet: its name in the header in place of "Pet souls"; several
//      pets: "Pet souls" and the pets as in A
// The approved files are not edited: A and B are classes (layout.css) on
// Pets.card()'s HTML, and B's single-pet header is rearranged here.
// Pets and values are the approved invented ones. Nothing is saved.
(function () {
  "use strict";

  const g = Pets.gainOf;
  const G = {
    sparkie: { p: { name: "Sparkie", fam: "fera" }, lv: 2, souls: 34, ev: "gain" },
    fossa: g("Fossa", { lv: 2, souls: 27 }, 1),
    slime: g("Slime", { lv: 1, souls: 12 }, 1),
    up: g("Hill Coradon", { lv: 1, souls: 24 }, 5),
    max: g("Kalgolem", { lv: 3, souls: 75 }, 1),
    long: { p: { name: "Splendent Armadon", fam: "fera" }, lv: 2, souls: 70, ev: "gain" },
  };

  function el(html) {
    const t = document.createElement("template");
    t.innerHTML = html.trim();
    return t.content.firstElementChild;
  }

  function petCard(items, lay, o = {}) {
    const card = el(Pets.card(items, o));
    if (lay === "a" || lay === "b") card.classList.add(`lay-${lay}`);
    if (lay === "b" && items.length === 1) {
      card.classList.add("is-solo-pet");
      const ev = card.querySelector(".pc-ev");
      const title = card.querySelector(".rm-head .rm-when");
      title.textContent = ev.querySelector(".rm-name").textContent;
      const chip = ev.querySelector(".pc-chip");
      if (chip) title.after(chip);
      ev.querySelector(".pc-top").remove();
    }
    return card.outerHTML;
  }

  // The reminder card as Hub.reminder() draws it, with any events.
  function remCard(events, lay, o = {}) {
    const x = o.locked ? "" : `<span class="rm-x" role="button" aria-label="Dismiss">${Hub.icon("close")}</span>`;
    const evs = events.map(([n, d]) => `<div class="rm-ev"><div class="rm-name">${n}</div><div class="rm-detail">${d}</div></div>`).join("");
    return `<div class="ov rm${lay === "a" ? " lay-a" : ""}" role="status"><div class="rm-head">${Hub.icon("bell")}<span class="rm-when">In 1 min</span><span class="rm-dot">·</span><span class="rm-at">${o.at || "22:00"}</span>${Hub.ring(0.62, o.locked)}${x}</div>${evs}</div>`;
  }

  const col = (lab, sub, html) => `<div class="pl-col"><span class="pl-lab">${lab}${sub ? `<small>${sub}</small>` : ""}</span><div class="pl-stack">${html}</div></div>`;
  const trio = (items, o) => `<div class="pl-row">${col("Today", "approved", petCard(items, "", o))}${col("A", "at the icon's edge", petCard(items, "a", o))}${col("B", items.length === 1 ? "name in the header" : "header kept: as A", petCard(items, "b", o))}</div>`;

  const REM1 = [["Shugo Festival", "3 min to join"]];
  const REM2 = [
    ["Rift portals", "Altgard · Verteron · Morheim · Eltnen"],
    ["Shugo Festival", "3 min to join"],
  ];

  function render(state) {
    switch (state) {
      case "one":
        return trio([G.sparkie]);
      case "two":
        return trio([G.fossa, G.slime]);
      case "three":
        return trio([G.fossa, G.slime, G.up]);
      case "up":
        return trio([G.up]);
      case "max":
        return trio([G.max]);
      case "long":
        return trio([G.long]);
      case "locked":
        return trio([G.sparkie], { locked: true, left: 0.8 });
      case "both":
        return `<div class="pl-row">${col("Today", "stacked as under the toolbar", remCard(REM1, "") + petCard([G.sparkie], ""))}${col("A", "pet card only", remCard(REM1, "") + petCard([G.sparkie], "a"))}${col("A", "reminder follows", remCard(REM1, "a") + petCard([G.sparkie], "a"))}</div>`;
      case "reminder":
        return `<div class="pl-row">${col("Reminder today", "", remCard(REM2, ""))}${col("Reminder with A", "events at the bell's edge", remCard(REM2, "a"))}</div>`;
    }
    return "";
  }

  Kit.mount({
    states: [
      { id: "one", title: "One pet", sub: "your screenshot's card: Sparkie, Lv 2 34/75 → Lv 3. Today 92 px, A 96, B 78" },
      { id: "two", title: "Two pets", sub: "B keeps \"Pet souls\" and draws the pets as A. Today 141 px, A and B 149" },
      { id: "three", title: "Three pets", sub: "today 190 px, A and B 202" },
      { id: "up", title: "A level reached", sub: "Hill Coradon Lv 1 24/25 +5 → Lv 2 4/75; in B the chip moves into the header, before the ring" },
      { id: "max", title: "Already at Lv 3", sub: "Kalgolem: Lv 3 · 75/75, the bar full, no next level" },
      { id: "long", title: "A long name", sub: "B's header gives the name less room (the ring and × share the row); it is cut off with an ellipsis only when needed" },
      { id: "locked", title: "Locked overlay", sub: "no ×, the ring in its slot" },
      { id: "both", title: "Stacked with the reminder card", sub: "as under the toolbar, 6 px apart: with only the pet card changed, the two bodies start at different edges; with the reminder following, they match" },
      { id: "reminder", title: "The reminder card, two events", sub: "drawn once with A's change (no bar, so only the indent) so you can decide whether it follows" },
    ],
    controls: ["alpha", "scale"],
    tileMin: 1040,
    stageMin: 160,
    render,
  });
})();
