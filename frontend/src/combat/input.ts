// The pieces of a combat request that are not fleet composition.
//
// Keeping these values in their own model lets the request builder consume
// them without making the fleet editor own technology or planet state.

import type { PlanetResources, Technology } from "@/api/types";
import type { Side } from "@/fleet/types";

/** Combat technology selected for each side, plus optional defender resources. */
export interface CombatInput {
  readonly technology: Readonly<Record<Side, Technology>>;
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
};

/** A known planet can be empty; this is distinct from an omitted planet. */
export const EMPTY_PLANET_RESOURCES: PlanetResources = {
  metal: 0,
  crystal: 0,
  deuterium: 0,
};
