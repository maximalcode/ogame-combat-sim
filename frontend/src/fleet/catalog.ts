// Entity catalog for fleet entry.
//
// The authority is `combat-types/src/names.rs` (`ENTITY_INFO`): the ids, the
// English names and the ship/defence split are copied from it. Two tests there
// assert the name and stat tables cover exactly the same ids, so this catalog
// is a display-side projection of a table the engine already keeps in step —
// do not invent ids here.
//
// Missiles (502 Anti-Ballistic, 503 Interplanetary) are deliberately excluded:
// they trigger the `MissileAttack` battle type, which is a separate combat mode
// from the fleet-vs-fleet/defence battle this UI composes. Issue #23 scopes
// entry to "every ship and defence", and missiles are neither. They get a UI
// when the engine's missile path gets a UI, not before.

/** A selectable entity with its OGame id and display name. */
export interface EntityEntry {
  /** Stringified entity id, matching `FleetComposition` keys on the wire. */
  readonly id: string;
  readonly name: string;
}

/** Ships — both sides may field them. ids 202-219, in game order. */
export const SHIPS: readonly EntityEntry[] = [
  { id: "202", name: "Small Cargo" },
  { id: "203", name: "Large Cargo" },
  { id: "204", name: "Light Fighter" },
  { id: "205", name: "Heavy Fighter" },
  { id: "206", name: "Cruiser" },
  { id: "207", name: "Battleship" },
  { id: "208", name: "Colony Ship" },
  { id: "209", name: "Recycler" },
  { id: "210", name: "Espionage Probe" },
  { id: "211", name: "Bomber" },
  { id: "212", name: "Solar Satellite" },
  { id: "213", name: "Destroyer" },
  { id: "214", name: "Deathstar" },
  { id: "215", name: "Battlecruiser" },
  { id: "217", name: "Crawler" },
  { id: "218", name: "Reaper" },
  { id: "219", name: "Pathfinder" },
] as const;

/** Defences — the defender only. ids 401-408, in game order. */
export const DEFENCES: readonly EntityEntry[] = [
  { id: "401", name: "Rocket Launcher" },
  { id: "402", name: "Light Laser" },
  { id: "403", name: "Heavy Laser" },
  { id: "404", name: "Gauss Cannon" },
  { id: "405", name: "Ion Cannon" },
  { id: "406", name: "Plasma Turret" },
  { id: "407", name: "Small Shield Dome" },
  { id: "408", name: "Large Shield Dome" },
] as const;

const BY_ID: ReadonlyMap<string, string> = new Map<string, string>(
  [...SHIPS, ...DEFENCES].map((entry) => [entry.id, entry.name]),
);

/**
 * The display name for an entity id, or the id itself if the catalog does not
 * know it. The fallback keeps a stray id visible rather than rendering a blank
 * row; the catalog covers every ship and defence, so in practice it never fires.
 */
export function entityName(id: string): string {
  return BY_ID.get(id) ?? id;
}
