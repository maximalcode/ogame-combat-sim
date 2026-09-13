import type {
  AllianceClass,
  CombatRequest,
  PlayerClass,
  UniverseSettings,
  Technology,
} from "@/api/types";

export const STATS = [
  ["weapon", "Waffen"],
  ["shield", "Schilde"],
  ["armour", "Panzerung"],
] as const;
export type Stat = (typeof STATS)[number][0];
export type Numbers = Record<Stat, string>;
export interface Unit {
  id: string;
  available: string;
  selected: string;
  lifeform: Numbers;
}
export interface Fleet {
  importedTechnology?: Partial<Technology>;
  units: Unit[];
  technology: Numbers;
  playerClass: PlayerClass;
  allianceClass: AllianceClass;
}
export interface Scenario {
  importedUniverse?: UniverseSettings;
  attacker: Fleet;
  defender: Fleet;
  debrisFleet: string;
  debrisDefence: string;
  deuterium: boolean;
  rapidFire: boolean;
}
export const blankNumbers = (): Numbers => ({ weapon: "", shield: "", armour: "" });
const emptyFleet = (): Fleet => ({
  units: [],
  technology: blankNumbers(),
  playerClass: "none",
  allianceClass: "none",
});
export const emptyScenario = (): Scenario => ({
  attacker: emptyFleet(),
  defender: emptyFleet(),
  debrisFleet: "",
  debrisDefence: "",
  deuterium: false,
  rapidFire: true,
});
export const MAX_COUNT = 4294967295;

/** Preserve blanks in the editor; only the scenario's wire boundary assumes zero. */
export function numericInput(raw: string, max: number, integer = true): string {
  if (raw === "") return "";
  const value = Number(raw);
  if (!Number.isFinite(value)) return "";
  return String(Math.max(0, Math.min(max, integer ? Math.floor(value) : value)));
}
export function selectQuantity(unit: Unit, raw: string, maximum = Number(unit.available)): Unit {
  return { ...unit, selected: numericInput(raw, maximum) };
}
export function scaleSelection(fleet: Fleet, factor: number): Fleet {
  return {
    ...fleet,
    units: fleet.units.map((unit) =>
      unit.selected === ""
        ? unit
        : selectQuantity(unit, String(Math.floor(Number(unit.selected) * factor))),
    ),
  };
}
export function restoreSelection(fleet: Fleet): Fleet {
  return { ...fleet, units: fleet.units.map((unit) => ({ ...unit, selected: unit.available })) };
}
const numbers = (values: Numbers) => ({
  weapon: Number(values.weapon),
  shield: Number(values.shield),
  armour: Number(values.armour),
});
export function makeRequest(scenario: Scenario, simulations: number): CombatRequest {
  const party = (fleet: Fleet) => ({
    technology: { ...fleet.importedTechnology, ...numbers(fleet.technology) },
    entities: Object.fromEntries(fleet.units.map((unit) => [unit.id, Number(unit.selected)])),
    lifeform: Object.fromEntries(fleet.units.map((unit) => [unit.id, numbers(unit.lifeform)])),
  });
  const bonuses = (fleet: Fleet) => ({
    player_class: fleet.playerClass,
    alliance_class: fleet.allianceClass,
  });
  return {
    attacker: party(scenario.attacker),
    defender: party(scenario.defender),
    attacker_bonuses: bonuses(scenario.attacker),
    defender_bonuses: bonuses(scenario.defender),
    simulations,
    use_rapid_fire: scenario.rapidFire,
    universe_settings: {
      ...scenario.importedUniverse,
      debris_fleet: Number(scenario.debrisFleet),
      debris_defence: Number(scenario.debrisDefence),
      debris_deuterium: scenario.deuterium,
    },
  };
}
