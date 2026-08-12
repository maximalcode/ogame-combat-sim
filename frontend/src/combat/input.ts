// The pieces of a combat request that are not fleet composition.
//
// Keeping this beside the request builder gives the UI one model for values
// that must travel together, without making the fleet editor own technology
// or planet state.

import type { PlanetResources, Technology } from "@/api/types";

/** Combat technology selected for each side, plus optional defender resources. */
export interface CombatInput {
  readonly attackerTechnology: Technology;
  readonly defenderTechnology: Technology;
  /** Undefined means the planet is unknown, not empty. */
  readonly planetResources?: PlanetResources;
}

/** The API's own default technology: level zero in every combat discipline. */
export const DEFAULT_TECHNOLOGY: Technology = {
  weapon: 0,
  shield: 0,
  armour: 0,
};

/** Matches the request defaults: zero combat tech and no planet supplied. */
export const DEFAULT_COMBAT_INPUT: CombatInput = {
  attackerTechnology: DEFAULT_TECHNOLOGY,
  defenderTechnology: DEFAULT_TECHNOLOGY,
};

/** A known planet can be empty; this is distinct from an omitted planet. */
export const EMPTY_PLANET_RESOURCES: PlanetResources = {
  metal: 0,
  crystal: 0,
  deuterium: 0,
};
