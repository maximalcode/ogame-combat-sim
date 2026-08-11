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
// Technology levels are out of scope for this issue (#23) — they arrive in #24,
// which will replace `DEFAULT_TECHNOLOGY`. Until then every party fights at
// level 0, which the engine accepts.

import type {
  CombatRequest,
  FleetComposition,
  PartyData,
  PartySlot,
  Technology,
} from "@/api/types";

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

/**
 * The technology every party fights at until issue #24 wires real tech input.
 * Level 0 across the board — the engine's own `Default` for `Technology`.
 */
export const DEFAULT_TECHNOLOGY: Technology = {
  weapon: 0,
  shield: 0,
  armour: 0,
};

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
export function nextSlotId(prefix: "A" | "D", slots: readonly FleetSlot[]): string {
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

/** Total ship count across every slot on a side (defences included). */
export function sideTotal(slots: readonly FleetSlot[]): number {
  let total = 0;
  for (const slot of slots) {
    for (const count of Object.values(slot.entities)) {
      total += count;
    }
  }
  return total;
}

/** True when a side has no ships in any slot. */
export function isSideEmpty(slots: readonly FleetSlot[]): boolean {
  return sideTotal(slots) === 0;
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
    for (const [id, count] of Object.entries(slot.entities)) {
      if (count > 0) out[id] = (out[id] ?? 0) + count;
    }
  }
  return out;
}

function partyData(entities: FleetComposition): PartyData {
  return { technology: DEFAULT_TECHNOLOGY, entities: prune(entities) };
}

function toPartySlot(slot: FleetSlot): PartySlot {
  return { id: slot.id, data: partyData(slot.entities) };
}

/**
 * Build the `CombatRequest` for a fleet state.
 *
 * Single slot on each side → the simple party shape (flat `attacker`/`defender`,
 * no slot arrays). More than one slot on either side → the multi-slot shape,
 * with both sides carried as slot arrays and the flat fields mirroring the
 * aggregate for the downscale check.
 */
export function buildCombatRequest(
  fleet: FleetState,
  simulations = 100,
): CombatRequest {
  const multiSlot = fleet.attacker.length > 1 || fleet.defender.length > 1;

  if (multiSlot) {
    return {
      attacker: partyData(aggregate(fleet.attacker)),
      defender: partyData(aggregate(fleet.defender)),
      attacker_slots: fleet.attacker.map(toPartySlot),
      defender_slots: fleet.defender.map(toPartySlot),
      use_rapid_fire: true,
      simulations,
    };
  }

  return {
    attacker: partyData(aggregate(fleet.attacker)),
    defender: partyData(aggregate(fleet.defender)),
    use_rapid_fire: true,
    simulations,
  };
}
