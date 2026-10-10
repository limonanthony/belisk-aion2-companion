// Build the pet catalogue, crates/app/assets/game/data/pets.json.
//
//   node scripts/build-pet-data.mjs <questlog-pets-cache.json>
//
// Source: QuestLog.gg's public API (https://questlog.gg/aion-2/api/trpc), the
// same one scripts/build-fr-names.mjs reads. Its pets use the game's own pet
// ids (the ids of the 00 90 / 0D 90 pet messages), the Global client's names
// and French. Nothing comes from the game client or its files.
// Licence: no licence or terms were found for the API or its data (the site is
// rendered in the browser; its terms page was not readable without one). Only
// names, the family and the mob codes of the soul drops are bundled, fetched
// at build time like the French names. Open point for the project owner.
//
//   database.getPets         every pet: id, English / French name, mainCategory
//                            (the family: intellect = Cogni, feral = Fera,
//                            nature = Natura, trans = Varian, special = Special)
//   database.getItems        the "vehiclesoul" items: "Soul: <pet name> (Bound)"
//   database.getItem         one soul item's itemIsDroppedByNpcs, the mobs
//                            that drop it
//
// A mob gives the pet whose "Soul: <pet name>" it drops; pet names are unique.
// The script stops when a mob would give two pets, when a soul names no pet or
// when a name holds Chinese characters or no letters.
//
// The cache ({ "pets": { id: { en, fr, family } }, "souls": { itemId:
// { petId, mobs } } }) is read, topped up from the API for souls it lacks and
// written back. The pet list is always refetched: it is six pages per language.
//
// Output: { "<pet id>": { "en", "fr", "family", "mobs": [codes] } }, sorted
// numeric ids, one entry per line, CRLF.
import fs from "node:fs";

const [cachePath] = process.argv.slice(2);
if (!cachePath) {
  console.error("usage: node scripts/build-pet-data.mjs <questlog-pets-cache.json>");
  process.exit(1);
}

const API = "https://questlog.gg/aion-2/api/trpc/";
const REQUESTS_IN_FLIGHT = 4;
const ATTEMPTS = 5;
const OUTPUT = new URL("../crates/app/assets/game/data/pets.json", import.meta.url);

const FAMILIES = { intellect: "Cogni", feral: "Fera", nature: "Natura", trans: "Varian", special: "Special" };

const cache = fs.existsSync(cachePath) ? JSON.parse(fs.readFileSync(cachePath, "utf8")) : {};
cache.souls ??= {};

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function call(proc, input) {
  const url = `${API}${proc}?batch=1&input=${encodeURIComponent(JSON.stringify({ 0: input }))}`;
  for (let attempt = 1; ; attempt++) {
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = (await response.json())?.[0]?.result?.data;
      if (!data) throw new Error("unexpected response shape");
      return data;
    } catch (error) {
      if (attempt === ATTEMPTS) throw new Error(`${proc} ${JSON.stringify(input)} failed after ${ATTEMPTS} attempts: ${error.message}`);
      await sleep(500 * 2 ** attempt);
    }
  }
}

async function allPages(proc, input) {
  const rows = [];
  let pageCount = 1;
  for (let page = 1; page <= pageCount; page++) {
    const data = await call(proc, { ...input, page });
    pageCount = data.pageCount;
    rows.push(...data.pageData);
  }
  return rows;
}

async function fetchPets() {
  const english = await allPages("database.getPets", { language: "en" });
  const french = new Map((await allPages("database.getPets", { language: "fr" })).map((pet) => [pet.id, pet.name]));
  const pets = {};
  for (const pet of english) {
    const family = FAMILIES[pet.mainCategory];
    if (!family) throw new Error(`pet ${pet.id} ${pet.name}: unknown family "${pet.mainCategory}"`);
    pets[pet.id] = { en: pet.name, fr: french.get(pet.id) ?? null, family };
  }
  return pets;
}

async function fillSouls(pets) {
  const petByName = new Map(Object.entries(pets).map(([id, pet]) => [pet.en, id]));
  const listed = await allPages("database.getItems", { language: "en", mainCategory: "usable", subCategory: "vehiclesoul" });
  const souls = listed.filter((item) => item.name.startsWith("Soul:"));
  const missing = souls.filter((item) => !(item.id in cache.souls));
  let next = 0;
  const worker = async () => {
    while (next < missing.length) {
      const item = missing[next++];
      const petName = item.name.replace(/^Soul:\s*/, "").replace(/\s*\(Bound\)$/, "");
      const petId = petByName.get(petName);
      if (!petId) throw new Error(`soul item ${item.id} "${item.name}" names no pet`);
      const detail = await call("database.getItem", { id: item.id, language: "en" });
      cache.souls[item.id] = { petId, mobs: (detail.itemIsDroppedByNpcs ?? []).map((npc) => Number(npc.id)) };
    }
  };
  await Promise.all(Array.from({ length: Math.min(REQUESTS_IN_FLIGHT, missing.length) }, worker));
  if (missing.length) console.log(`fetched ${missing.length} soul items from QuestLog`);
  return new Set(souls.map((item) => item.id));
}

const hasHan = (text) => /\p{Script=Han}/u.test(text);
const isName = (text) => typeof text === "string" && /\p{L}/u.test(text) && !hasHan(text);

cache.pets = await fetchPets();
const currentSouls = await fillSouls(cache.pets);
fs.writeFileSync(cachePath, JSON.stringify(cache));

const mobsByPet = {};
const petOfMob = new Map();
for (const itemId of currentSouls) {
  const { petId, mobs } = cache.souls[itemId];
  for (const mob of mobs) {
    if (petOfMob.has(mob) && petOfMob.get(mob) !== petId) {
      throw new Error(`mob ${mob} would give pets ${petOfMob.get(mob)} and ${petId}`);
    }
    petOfMob.set(mob, petId);
    (mobsByPet[petId] ??= new Set()).add(mob);
  }
}

const ids = Object.keys(cache.pets).sort((a, b) => Number(a) - Number(b));
const lines = ids.map((id) => {
  const pet = cache.pets[id];
  if (!isName(pet.en)) throw new Error(`pet ${id}: English name "${pet.en}" is not a name`);
  if (!isName(pet.fr)) throw new Error(`pet ${id}: French name "${pet.fr}" is not a name`);
  const mobs = [...(mobsByPet[id] ?? [])].sort((a, b) => a - b);
  return `  ${JSON.stringify(id)}: ${JSON.stringify({ en: pet.en, fr: pet.fr, family: pet.family, mobs })}`;
});
fs.writeFileSync(OUTPUT, `{\r\n${lines.join(",\r\n")}\r\n}\r\n`);

const withMobs = ids.filter((id) => mobsByPet[id]).length;
console.log(`${ids.length} pets, ${withMobs} with mobs, ${petOfMob.size} mobs -> ${OUTPUT.pathname}`);
