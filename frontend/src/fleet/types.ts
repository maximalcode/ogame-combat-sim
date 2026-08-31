// Fleet-entry state and the request builder that turns it into a
// `CombatRequest`.
//
// The state model is one rung above the wire shape: a side is a list of slots,
// each a bag of entity counts. The builder collapses that into the request the
// engine expects, and the collapse has two rules that are not obvious from the
// types alone — both verified against `combat-core/src/simulator.rs`:
//
//   1. Slot mode is entered only when BOTH sides carry a `*_slots` array. Send
//      slots for one side and a flat party for the other and the engine ignores
//      the slots entirely (the `if let (Some, Some)` at the slot branch). So
//      when either side has more than one slot, the builder emits slots for
//      both — a one-slot side becomes a single-element array.
//
//   2. The flat `attacker`/`defender` fields are required even in slot mode,
//      and the engine reads them for the downscale check. The multi-slot tests
//      mirror the slots into the flat fields for that reason
//      (`combat-core/tests/class_bonuses.rs`); the builder does the same by
//      aggregating every slot on a side into the flat party.
//
// Each side's technology and optional lifeform percentages are supplied by the
// technology-input region. Slot parties inherit their side's modifiers because
// that is the granularity this UI collects; the builder keeps the same party
// shape in flat and slot mode.

import type {
  CombatRequest,
  FleetComposition,
  LifeformBonus,
  LifeformBonuses,
  PartyData,
  PartySlot,
} from "@/api/types";
import type { ClassBonuses, CombatInput } from "@/combat/input";

/** The two parties to a battle, as every component and helper names them. */
export type Side = "attacker" | "defender";

/** User-facing names for the two battle sides. */
export const SIDE_LABELS: Readonly<Record<Side, string>> = {
  attacker: "Attacker",
  defender: "Defender",
};

/** One fleet slot: an id (A1, D2, …) and its entity counts. */
export interface FleetSlot {
  readonly id: string;
  readonly entities: FleetComposition;
}

/** The full fleet-entry state: attacker and defender slots, independent. */
export interface FleetState {
  readonly attacker: readonly FleetSlot[];
  readonly defender: readonly FleetSlot[];
}

/** A slot with no ships in it. */
export function emptySlot(id: string): FleetSlot {
  return { id, entities: {} };
}

/**
 * The next slot id for a side (`A3` when A1 and A2 exist, `A1` when none do).
 *
 * Uses the highest existing numeric suffix rather than `length + 1` so removing
 * a middle slot and re-adding does not collide with a later id.
 */
export function nextSlotId(side: Side, slots: readonly FleetSlot[]): string {
  const prefix = side === "attacker" ? "A" : "D";
  let max = 0;
  for (const slot of slots) {
    const match = /^([A-Z])(\d+)$/.exec(slot.id);
    if (match?.[1] === prefix) {
      const n = Number.parseInt(match[2] ?? "0", 10);
      if (n > max) max = n;
    }
  }
  return `${prefix}${String(max + 1)}`;
}

/** True when a side has no ships in any slot. */
export function isSideEmpty(slots: readonly FleetSlot[]): boolean {
  return slots.every((slot) =>
    Object.values(slot.entities).every((count) => count === 0),
  );
}

/** Drop zero-count entries — the engine treats them as no ships, so do not send them. */
function prune(entities: FleetComposition): FleetComposition {
  const out: Record<string, number> = {};
  for (const [id, count] of Object.entries(entities)) {
    if (count > 0) out[id] = count;
  }
  return out;
}

/** Sum every slot on a side into one composition, dropping zero counts. */
function aggregate(slots: readonly FleetSlot[]): FleetComposition {
  const out: Record<string, number> = {};
  for (const slot of slots) {
    for (const [id, count] of Object.entries(prune(slot.entities))) {
      out[id] = (out[id] ?? 0) + count;
    }
  }
  return out;
}

function hasLifeformStats(bonus: LifeformBonus): boolean {
  return Object.values(bonus).some((value) => value !== undefined && value !== 0);
}

function pruneLifeform(
  lifeform: LifeformBonuses | undefined,
): LifeformBonuses | undefined {
  if (lifeform === undefined) return undefined;
  const entries = Object.entries(lifeform).filter(([, bonus]) =>
    hasLifeformStats(bonus),
  );
  return entries.length === 0 ? undefined : Object.fromEntries(entries);
}

function partyData(
  entities: FleetComposition,
  technology: PartyData["technology"],
  lifeform: LifeformBonuses | undefined,
): PartyData {
  const suppliedLifeform = pruneLifeform(lifeform);
  return {
    technology,
    entities: prune(entities),
    ...(suppliedLifeform === undefined ? {} : { lifeform: suppliedLifeform }),
  };
}

function toPartySlot(
  slot: FleetSlot,
  technology: PartyData["technology"],
  lifeform: LifeformBonuses | undefined,
): PartySlot {
  return { id: slot.id, data: partyData(slot.entities, technology, lifeform) };
}

function withPlanetResources(input: CombatInput): Pick<CombatRequest, "planet_resources"> {
  return input.planetResources === undefined
    ? {}
    : { planet_resources: input.planetResources };
}

function hasClassSelection(bonuses: ClassBonuses | undefined): boolean {
  return (
    bonuses?.player_class !== undefined ||
    bonuses?.alliance_class !== undefined
  );
}

function withClassBonuses(
  input: CombatInput,
): Pick<CombatRequest, "attacker_bonuses" | "defender_bonuses"> {
  const attacker = input.classBonuses.attacker;
  const defender = input.classBonuses.defender;
  return {
    ...(hasClassSelection(attacker) ? { attacker_bonuses: attacker } : {}),
    ...(hasClassSelection(defender) ? { defender_bonuses: defender } : {}),
  };
}

function withUniverseSettings(
  input: CombatInput,
): Pick<CombatRequest, "universe_settings"> {
  return input.universeSettings === undefined
    ? {}
    : { universe_settings: input.universeSettings };
}

// Matches the request's own default, so the UI and an empty JSON body run the
// same number of battles.
const SIMULATIONS = 100;

/**
 * Build the `CombatRequest` for a fleet state.
 *
 * A single slot on each side → the simple party shape (flat
 * `attacker`/`defender`, no slot arrays). More than one slot on either side →
 * the multi-slot shape, with both sides carried as slot arrays and the flat
 * fields mirroring the aggregate for the downscale check.
 */
export function buildCombatRequest(fleet: FleetState, input: CombatInput): CombatRequest {
  const multiSlot = fleet.attacker.length > 1 || fleet.defender.length > 1;

  return {
    attacker: partyData(
      aggregate(fleet.attacker),
      input.technology.attacker,
      input.lifeform.attacker,
    ),
    defender: partyData(
      aggregate(fleet.defender),
      input.technology.defender,
      input.lifeform.defender,
    ),
    ...(multiSlot
      ? {
          attacker_slots: fleet.attacker.map((slot) =>
            toPartySlot(
              slot,
              input.technology.attacker,
              input.lifeform.attacker,
            ),
          ),
          defender_slots: fleet.defender.map((slot) =>
            toPartySlot(
              slot,
              input.technology.defender,
              input.lifeform.defender,
            ),
          ),
        }
      : {}),
    ...withPlanetResources(input),
    ...withClassBonuses(input),
    ...withUniverseSettings(input),
    use_rapid_fire: true,
    simulations: SIMULATIONS,
  };
}
