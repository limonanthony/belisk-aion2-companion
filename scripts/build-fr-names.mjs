// Build the French name catalogues next to the English ones:
//   crates/core/data/npc_names_fr.json              (mob code -> French name)
//   crates/app/assets/game/data/skill_names_fr.json (skill id  -> French name)
//
//   node scripts/build-fr-names.mjs <questlog-cache.json> [extra-names-en-fr.json]
//
// Only ids already in npc_names_en.json / skill_names_en.json get a French
// name, and only when one is found; the app falls back to English for the rest.
// Per id, the first layer that answers wins:
//   1. QuestLog.gg by id (https://questlog.gg/aion-2/api/trpc, language "fr").
//   2. The optional community name list by id.
//   3. QuestLog's French for another of our ids with the same English name.
//   4. The community list's French for the same English name.
// "(DNT)" is stripped from English names before names are compared; layers 3 and
// 4 apply only when every candidate agrees on one French name.
//
// The QuestLog cache ({ "npc": { id: fr | null }, "skill": { ... } }) is read,
// topped up from the API for ids it lacks, and written back. The community list
// ({ "skills": { id: { en, fr } }, "mobs": { ... } }) is optional and stays
// outside the repository; without it only the QuestLog layers apply.
//
// Output is the same shape as the English files: sorted numeric ids, one entry
// per line, CRLF. French is kept exactly as received.
import fs from "node:fs";

const [cachePath, extraNamesPath] = process.argv.slice(2);
if (!cachePath) {
  console.error("usage: node scripts/build-fr-names.mjs <questlog-cache.json> [extra-names-en-fr.json]");
  process.exit(1);
}

const API = "https://questlog.gg/aion-2/api/trpc/";
const IDS_PER_REQUEST = 25;
const REQUESTS_IN_FLIGHT = 4;
const ATTEMPTS = 5;

const kinds = [
  {
    key: "npc",
    proc: "database.getNpc",
    extraKey: "mobs",
    english: new URL("../crates/core/data/npc_names_en.json", import.meta.url),
    french: new URL("../crates/core/data/npc_names_fr.json", import.meta.url),
  },
  {
    key: "skill",
    proc: "database.getSkill",
    extraKey: "skills",
    english: new URL("../crates/app/assets/game/data/skill_names_en.json", import.meta.url),
    french: new URL("../crates/app/assets/game/data/skill_names_fr.json", import.meta.url),
  },
];

const cache = fs.existsSync(cachePath) ? JSON.parse(fs.readFileSync(cachePath, "utf8")) : {};
const extraNames = extraNamesPath ? JSON.parse(fs.readFileSync(extraNamesPath, "utf8")) : {};

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function fetchBatch(proc, ids) {
  const input = Object.fromEntries(ids.map((id, index) => [String(index), { id, language: "fr" }]));
  const url = `${API}${ids.map(() => proc).join(",")}?batch=1&input=${encodeURIComponent(JSON.stringify(input))}`;
  for (let attempt = 1; ; attempt++) {
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const body = await response.json();
      if (!Array.isArray(body) || body.length !== ids.length) throw new Error("unexpected response shape");
      return body.map((entry) => {
        const name = entry?.result?.data?.name;
        return typeof name === "string" && name.trim() ? name : null;
      });
    } catch (error) {
      if (attempt === ATTEMPTS) throw new Error(`${proc} for ${ids[0]}..${ids.at(-1)} failed after ${ATTEMPTS} attempts: ${error.message}`);
      await sleep(500 * 2 ** attempt);
    }
  }
}

async function fillCache(kind, ids) {
  const known = (cache[kind.key] ??= {});
  const missing = ids.filter((id) => !(id in known));
  const batches = [];
  for (let at = 0; at < missing.length; at += IDS_PER_REQUEST) batches.push(missing.slice(at, at + IDS_PER_REQUEST));
  let next = 0;
  const worker = async () => {
    while (next < batches.length) {
      const batch = batches[next++];
      const names = await fetchBatch(kind.proc, batch);
      batch.forEach((id, index) => (known[id] = names[index]));
    }
  };
  await Promise.all(Array.from({ length: Math.min(REQUESTS_IN_FLIGHT, batches.length) }, worker));
  if (missing.length) {
    fs.writeFileSync(cachePath, JSON.stringify(cache));
    console.log(`fetched ${missing.length} ${kind.key} ids from QuestLog`);
  }
}

const matchKey = (english) => english.replace(/\(DNT\)/gi, "").replace(/\s+/g, " ").trim().toLowerCase();

// name key -> French name, or null when candidates disagree
function frenchByName(pairs) {
  const byName = new Map();
  for (const [english, french] of pairs) {
    const key = matchKey(english);
    if (!key) continue;
    if (!byName.has(key)) byName.set(key, french);
    else if (byName.get(key) !== french) byName.set(key, null);
  }
  return byName;
}

// Both sources hold "???" where the game has no French text yet.
const usable = (name) => (typeof name === "string" && /\p{L}/u.test(name) ? name : null);

function build(kind) {
  const english = JSON.parse(fs.readFileSync(kind.english, "utf8"));
  const ids = Object.keys(english).filter((id) => /^\d+$/.test(id));
  const questlog = cache[kind.key];
  const reference = extraNames[kind.extraKey] ?? {};

  const questlogByName = frenchByName(
    ids.filter((id) => usable(questlog[id])).map((id) => [english[id], questlog[id]]),
  );
  const extraByName = frenchByName(
    Object.values(reference)
      .filter((entry) => usable(entry?.en) && usable(entry?.fr))
      .map((entry) => [entry.en, entry.fr]),
  );

  const counts = { questlogId: 0, extraId: 0, questlogName: 0, extraName: 0, none: 0 };
  const names = [];
  for (const id of ids) {
    let name;
    if ((name = usable(questlog[id]))) counts.questlogId++;
    else if ((name = usable(reference[id]?.fr))) counts.extraId++;
    else if ((name = questlogByName.get(matchKey(english[id])) ?? null)) counts.questlogName++;
    else if ((name = extraByName.get(matchKey(english[id])) ?? null)) counts.extraName++;
    else {
      counts.none++;
      continue;
    }
    names.push([id, name]);
  }

  names.sort((a, b) => Number(a[0]) - Number(b[0]));
  const body = names.map(([id, name]) => `  ${JSON.stringify(id)}: ${JSON.stringify(name)}`);
  fs.writeFileSync(kind.french, `{\n${body.join(",\n")}\n}\n`.replace(/\n/g, "\r\n"));
  console.log(
    `${kind.key}: ${ids.length} ids -> QuestLog id ${counts.questlogId}, extra list id ${counts.extraId}, ` +
      `QuestLog name ${counts.questlogName}, extra list name ${counts.extraName}, none ${counts.none}; ` +
      `wrote ${names.length} to ${kind.french.pathname}`,
  );
}

for (const kind of kinds) {
  const english = JSON.parse(fs.readFileSync(kind.english, "utf8"));
  await fillCache(kind, Object.keys(english).filter((id) => /^\d+$/.test(id)));
}
for (const kind of kinds) build(kind);
