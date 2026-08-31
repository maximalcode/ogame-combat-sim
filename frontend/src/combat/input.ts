// The pieces of a combat request that are not fleet composition.
//
// Keeping these values in their own model lets the request builder consume
// them without making the fleet editor own technology, modifiers or planet
// state. Optional values stay optional all the way to the wire.

import type {
  LifeformBonuses,
  PlanetResources,
  PlayerBonuses,
  Technology,
  UniverseSettings,
} from "@/api/types";
import type { Side } from "@/fleet/types";

/** The class fields the simulator reads; the inert Engineer flag is excluded. */
export type ClassBonuses = Readonly<
  Pick<PlayerBonuses, "player_class" | "alliance_class">
>;

/** The universe fields combat reads, all required once the override is enabled. */
export type UniverseDebrisInput = Required<
  Pick<
    UniverseSettings,
    "debris_fleet" | "debris_defence" | "debris_deuterium"
  >
>;

/** Combat technology, optional modifiers, and optional defender resources. */
export interface CombatInput {
  readonly technology: Readonly<Record<Side, Technology>>;
  /** A missing side means its class block is not supplied. */
  readonly classBonuses: Readonly<Partial<Record<Side, ClassBonuses>>>;
  /** A missing side means its parties carry no lifeform block. */
  readonly lifeform: Readonly<Partial<Record<Side, LifeformBonuses>>>;
  /** Undefined preserves the request's top-level debris fallback. */
  readonly universeSettings?: UniverseDebrisInput;
  /** Undefined means the planet is unknown, not empty. */
  readonly planetResources?: PlanetResources;
}

/** The API's own default technology: level zero in every combat discipline. */
const DEFAULT_TECHNOLOGY: Technology = {
  weapon: 0,
  shield: 0,
  armour: 0,
};

/** Matches the request defaults: zero combat tech and no planet supplied. */
export const DEFAULT_COMBAT_INPUT: CombatInput = {
  technology: {
    attacker: DEFAULT_TECHNOLOGY,
    defender: DEFAULT_TECHNOLOGY,
  },
  classBonuses: {},
  lifeform: {},
};

/** The complete standard debris block used only after the override is enabled. */
export const STANDARD_UNIVERSE_DEBRIS: UniverseDebrisInput = {
  debris_fleet: 30,
  debris_defence: 0,
  debris_deuterium: false,
};

/** A known planet can be empty; this is distinct from an omitted planet. */
export const EMPTY_PLANET_RESOURCES: PlanetResources = {
  metal: 0,
  crystal: 0,
  deuterium: 0,
};
