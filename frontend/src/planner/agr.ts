import { availableTypes } from "./catalog";
import {
  blankNumbers,
  emptyScenario,
  MAX_COUNT,
  type Fleet,
  type Numbers,
  type Scenario,
} from "./model";

type JsonObject = Record<string, unknown>;
const invalid = () =>
  new Error("AGR-Übergabe ungültig oder nicht unterstützt. Deine Planung bleibt erhalten.");
function object(value: unknown): JsonObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw invalid();
  return value as JsonObject;
}
function number(value: unknown, max: number, integer = true): number {
  if (typeof value !== "number" && typeof value !== "string") throw invalid();
  if (value === "" || (typeof value === "string" && value.trim() !== value)) throw invalid();
  const result = Number(value);
  if (
    !Number.isFinite(result) ||
    result < 0 ||
    result > max ||
    (integer && !Number.isInteger(result))
  )
    throw invalid();
  return result;
}
function field(value: unknown, key: string): unknown {
  const entry = object(value);
  if (Object.keys(entry).some((name) => name !== key)) throw invalid();
  return entry[key];
}
function optionalObject(value: unknown): JsonObject {
  return value === undefined ? {} : object(value);
}
function party(value: unknown): JsonObject | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || value.length > 1)
    throw new Error(
      "AGR: Mehrere Teilnehmer werden noch nicht unterstützt. Deine Planung bleibt erhalten.",
    );
  return value.length === 0 ? undefined : object(value[0]);
}
const playerClasses = ["none", "collector", "general", "discoverer"] as const;
const allianceClasses = ["none", "warrior", "trader", "researcher"] as const;
function classes(
  data: JsonObject,
  fleet: Fleet,
): { lf: JsonObject; currentAgr: boolean; granted: number } {
  const allowed = [
    "research",
    "ships",
    "defence",
    "resources",
    "planet",
    "speed",
    "class",
    "allianceClass",
    "lifeformBonuses",
  ];
  if (Object.keys(data).some((key) => !allowed.includes(key))) throw invalid();
  const lf = optionalObject(data["lifeformBonuses"]);
  const currentAgr =
    data["lifeformBonuses"] !== undefined &&
    data["class"] !== undefined &&
    data["allianceClass"] !== undefined;
  if (
    (data["class"] !== undefined ||
      data["allianceClass"] !== undefined ||
      data["lifeformBonuses"] !== undefined) &&
    !currentAgr
  ) {
    throw new Error(
      "AGR: Klassen und Forschungsstufen sind nicht eindeutig zugeordnet. Deine Planung bleibt erhalten.",
    );
  }
  if (
    Object.keys(lf).some(
      (key) => !["BaseStatsBooster", "CharacterClassBooster", "ShipFuelConsumption"].includes(key),
    )
  )
    throw invalid();
  const player = data["class"] === undefined ? 0 : number(data["class"], 3);
  const alliance = data["allianceClass"] === undefined ? 0 : number(data["allianceClass"], 3);
  fleet.playerClass = playerClasses[player] ?? "none";
  fleet.allianceClass = allianceClasses[alliance] ?? "none";
  const classBoost = optionalObject(lf["CharacterClassBooster"]);
  const boost = classBoost["2"] === undefined ? undefined : number(classBoost["2"], 10000, false);
  if (player === 2 && (boost === undefined || Math.floor(2 * (1 + boost)) !== 2)) throw invalid();
  const granted = (player === 2 ? 2 : 0) + (alliance === 1 ? 1 : 0);
  return { lf, currentAgr, granted };
}
function researchLevels(
  data: JsonObject,
  fleet: Fleet,
  granted: number,
  currentAgr: boolean,
  sources: string[],
) {
  const research = optionalObject(data["research"]);
  const researchSource = currentAgr ? "AGR (Klassenanteil herausgerechnet)" : "AGR";
  for (const [id, stat] of [
    ["109", "weapon"],
    ["110", "shield"],
    ["111", "armour"],
  ] as const) {
    if (research[id] === undefined) continue;
    const level = number(field(research[id], "level"), 255);
    if (level < granted) throw invalid();
    fleet.technology[stat] = String(level - granted);
    sources.push(`Forschung ${id}: ${researchSource}`);
  }
  fleet.importedTechnology = {};
  for (const [id, key] of [
    ["114", "hyperspace_tech"],
    ["115", "combustion"],
    ["117", "impulse"],
    ["118", "hyperspace"],
  ] as const) {
    if (research[id] === undefined) continue;
    const level = number(field(research[id], "level"), 255);
    fleet.importedTechnology[key] = level;
    sources.push(`Forschung ${id}: AGR (${String(level)}, ohne Flugberechnung)`);
  }
}
function lifeforms(lf: JsonObject, fleet: Fleet, sources: string[]) {
  const stats = optionalObject(lf["BaseStatsBooster"]);
  for (const unit of fleet.units) {
    if (stats[unit.id] === undefined) {
      if (lf["BaseStatsBooster"] !== undefined)
        unit.lifeform = { weapon: "0", shield: "0", armour: "0" };
      continue;
    }
    const entry = object(stats[unit.id]);
    if (
      Object.keys(entry).some(
        (key) => !["weapon", "shield", "armor", "cargo", "speed"].includes(key),
      )
    )
      throw invalid();
    const values: Numbers = blankNumbers();
    for (const [key, stat] of [
      ["weapon", "weapon"],
      ["shield", "shield"],
      ["armor", "armour"],
    ] as const) {
      if (entry[key] === undefined) throw invalid();
      values[stat] = String(number(entry[key], 10000, false) * 100);
    }
    unit.lifeform = values;
  }
  if (lf["BaseStatsBooster"] !== undefined)
    sources.push("Lebensformwerte: AGR BaseStatsBooster (Anteile in Prozent umgerechnet)");
}
function modifiers(data: JsonObject, fleet: Fleet, sources: string[]) {
  const { lf, currentAgr, granted } = classes(data, fleet);
  if (currentAgr && lf["BaseStatsBooster"] === undefined) throw invalid();
  researchLevels(data, fleet, granted, currentAgr, sources);
  if (currentAgr) sources.push("Spieler- und Allianzklasse: AGR");
  lifeforms(lf, fleet, sources);
}
function readFleet(data: JsonObject, attacker: boolean, sources: string[]): Fleet {
  const fleet = emptyScenario().attacker;
  const ships = optionalObject(data["ships"]);
  const defence = optionalObject(data["defence"]);
  if (attacker && Object.keys(defence).length > 0) throw invalid();
  for (const [id, entry] of [...Object.entries(ships), ...Object.entries(defence)]) {
    // Missiles and stationary own units cannot participate in an attacking fleet.
    if (["502", "503"].includes(id) || (attacker && ["212", "217"].includes(id))) continue;
    if (
      !availableTypes(attacker).some((unit) => unit.id === id) ||
      fleet.units.some((unit) => unit.id === id)
    )
      throw invalid();
    const count = String(number(field(entry, "count"), MAX_COUNT));
    fleet.units.push({ id, available: count, selected: count, lifeform: blankNumbers() });
  }
  sources.push(`${attacker ? "Eigener Bestand" : "Gegnerische Aufstellung"}: AGR prefill`);
  const modifierSources: string[] = [];
  modifiers(data, fleet, modifierSources);
  sources.push(
    ...modifierSources.map((source) => `${attacker ? "Angreifer" : "Verteidiger"} – ${source}`),
  );
  return fleet;
}
function universeMetadata(data: JsonObject, scenario: Scenario, sources: string[]) {
  scenario.importedUniverse = {};
  for (const [source, target, max, integer] of [
    ["galaxies", "galaxies", 255, true],
    ["systems", "systems", 65535, true],
    ["speed_fleet", "fleet_speed", 255, true],
    ["global_deuterium_save_factor", "deuterium_save_factor", 1, false],
  ] as const) {
    if (data[source] === undefined) continue;
    const value = number(data[source], max, integer);
    scenario.importedUniverse[target] = value;
    sources.push(`${source}: AGR (${String(value)}, ohne Flugberechnung)`);
  }
  for (const [source, target] of [
    ["donut_galaxy", "donut_galaxy"],
    ["donut_system", "donut_systems"],
  ] as const) {
    if (data[source] === undefined) continue;
    scenario.importedUniverse[target] = number(data[source], 1) === 1;
    sources.push(
      `${source}: AGR (${String(scenario.importedUniverse[target])}, ohne Flugberechnung)`,
    );
  }
}
function settings(data: JsonObject, scenario: Scenario, sources: string[]) {
  const allowed = [
    "speed_fleet", "speed_fleet_holding", "galaxies", "systems", "rapid_fire",
    "def_to_tF", "debris_factor", "repair_factor", "donut_galaxy", "donut_system",
    "global_deuterium_save_factor", "deuterium_in_debris", "server", "plunder", "simulations",
  ];
  if (Object.keys(data).some((key) => !allowed.includes(key))) throw invalid();
  universeMetadata(data, scenario, sources);
  if (data["debris_factor"] !== undefined) {
    scenario.debrisFleet = String(number(data["debris_factor"], 1, false) * 100);
    sources.push("Flotten-TF: AGR debris_factor");
  }
  if (data["def_to_tF"] !== undefined) {
    const enabled = number(data["def_to_tF"], 1);
    scenario.debrisDefence = enabled ? "" : "0";
    sources.push(
      enabled
        ? "Verteidigungs-TF aktiviert, Anteil fehlt: bitte ergänzen"
        : "Verteidigungs-TF: AGR (inaktiv)",
    );
  }
  if (data["rapid_fire"] !== undefined) {
    scenario.rapidFire = number(data["rapid_fire"], 1) === 1;
    sources.push("Rapidfire: AGR");
  } else sources.push("Rapidfire fehlt: manuelle Vorgabe (aktiv)");
  if (data["deuterium_in_debris"] !== undefined) {
    scenario.deuterium = number(data["deuterium_in_debris"], 1) === 1;
    sources.push("Deuterium im TF: AGR");
  } else sources.push("Deuterium im TF fehlt: manuelle Vorgabe (inaktiv)");
}
export interface AgrImport {
  scenario: Scenario;
  sources: string[];
}
/** No raw payload, identity, coordinates or capability is retained. */
export function readAgr(hash: string, previous: Scenario): AgrImport | undefined {
  if (!hash.startsWith("#prefill")) return undefined;
  if (!hash.startsWith("#prefill=")) throw invalid();
  if (hash.length > 131072)
    throw new Error("AGR-Übergabe zu groß (maximal 128 KiB). Deine Planung bleibt erhalten.");
  let data: JsonObject;
  try {
    const encoded = decodeURIComponent(hash.slice(9));
    const bytes = Uint8Array.from(atob(encoded), (char) => char.charCodeAt(0));
    data = object(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)));
  } catch {
    throw invalid();
  }
  if (Object.keys(data).some((key) => !["0", "1", "settings"].includes(key))) throw invalid();
  const own = party(data["0"]);
  const enemy = party(data["1"]);
  if (own?.["ships"] === undefined)
    throw new Error("AGR: Eigene Flotte fehlt im prefill. Deine Planung bleibt erhalten.");
  const sources: string[] = [];
  const scenario = emptyScenario();
  scenario.attacker = readFleet(own, true, sources);
  scenario.defender = enemy ? readFleet(enemy, false, sources) : previous.defender;
  if (!enemy) sources.push("Gegner nicht übergeben: bisherige Aufstellung bleibt erhalten");
  settings(optionalObject(data["settings"]), scenario, sources);
  return { scenario, sources };
}
