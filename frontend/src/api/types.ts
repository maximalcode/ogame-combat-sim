// Typed models for the combat-api HTTP surface.
//
// These mirror `combat-types` field-for-field: the request is exactly what
// `POST /api/simulate` accepts and the response is the `{ results, report }`
// shape `combat-api` serializes. Keeping the model in one place is what lets
// call sites stay typed rather than reaching into untyped JSON.
//
// Naming follows the Rust enums' serde representation:
//   - `CombatOutcome` / `BattleType` have no `rename_all`, so they serialize
//     as the PascalCase variant names written in Rust (`AttackersWin`,
//     `FleetVsFleet`, ...).
//   - `PlayerClass` / `AllianceClass` use `rename_all = "lowercase"`.
//
// `FleetComposition` is `HashMap<u16, u32>` on the Rust side. serde_json
// serializes integer map keys as strings, so the wire shape is a string-keyed
// object — `Record<string, number>`. Entity ids are 202-219, 401-408, 502-503.

/**
 * Fleet composition: entity type id -> count. Keys are stringified ids
 * (202-219 ships, 401-408 defences, 502-503 missiles).
 */
export type FleetComposition = Record<string, number>;

// --- Request model -------------------------------------------------------

/** Combat technology levels for a side. Drive techs are optional. */
export interface Technology {
  weapon: number;
  shield: number;
  armour: number;
  combustion?: number;
  impulse?: number;
  hyperspace?: number;
  hyperspace_tech?: number;
}

/** Per-ship-type lifeform bonus, as percentages of the base stat (50 = +50%). */
export interface LifeformBonus {
  weapon?: number;
  shield?: number;
  armour?: number;
  cargo?: number;
  speed?: number;
}

/** One side's lifeform bonuses, keyed by stringified entity id. */
export type LifeformBonuses = Record<string, LifeformBonus>;

/** Player class (lowercase on the wire). */
export type PlayerClass = "none" | "collector" | "general" | "discoverer";

/** Alliance class (lowercase on the wire). */
export type AllianceClass = "none" | "trader" | "warrior" | "researcher";

/** Class and officer bonuses for a side. */
export interface PlayerBonuses {
  player_class?: PlayerClass;
  alliance_class?: AllianceClass;
  has_engineer?: boolean;
}

/** A combat party: technology, fleet, and optional lifeform research. */
export interface PartyData {
  technology: Technology;
  entities: FleetComposition;
  /** Omitted on the wire when empty (skip_serializing_if). */
  lifeform?: LifeformBonuses;
}

/** A named multi-slot party (ACS). */
export interface PartySlot {
  id: string;
  name?: string;
  data: PartyData;
}

/**
 * Universe settings affecting debris and (eventually) flight calculations.
 *
 * Every field serde-defaults on the Rust side (`#[serde(default = "...")]`),
 * so a request may send any subset — a block that sets only `galaxies` still
 * wins over the top-level `debris_percentage` (see CLAUDE.md's "Debris rules
 * come from two places and one wins"). The fields are optional here to mirror
 * that: the client can express a partial block the server accepts.
 */
export interface UniverseSettings {
  galaxies?: number;
  systems?: number;
  donut_galaxy?: boolean;
  donut_systems?: boolean;
  fleet_speed?: number;
  debris_fleet?: number;
  debris_defence?: number;
  debris_deuterium?: boolean;
  deuterium_save_factor?: number;
}

/** Planet resources for loot calculation. */
export interface PlanetResources {
  metal: number;
  crystal: number;
  deuterium: number;
}

/** The body `POST /api/simulate` accepts. */
export interface CombatRequest {
  attacker: PartyData;
  defender: PartyData;
  attacker_slots?: PartySlot[];
  defender_slots?: PartySlot[];
  planet_resources?: PlanetResources;
  /** Fallback debris % when `universe_settings` is absent. Defaults to 30. */
  debris_percentage?: number;
  use_rapid_fire: boolean;
  simulations: number;
  enable_downscaling?: boolean | null;
  enable_round_compositions?: boolean | null;
  universe_settings?: UniverseSettings | null;
  attacker_bonuses?: PlayerBonuses | null;
  defender_bonuses?: PlayerBonuses | null;
  /** Plunder percentage (50, 75, 100). Defaults to 50. */
  plunder_percentage?: number;
}

// --- Response model ------------------------------------------------------

/** Battle outcome. PascalCase on the wire (no `rename_all` in Rust). */
export type CombatOutcome = "AttackersWin" | "DefendersWin" | "Draw";

/** Battle type classification. PascalCase on the wire. */
export type BattleType =
  | "FleetVsFleet"
  | "FleetVsDefense"
  | "Mixed"
  | "MissileAttack"
  | "MoonDestruction";

/** Debris field from destroyed ships. */
export interface DebrisField {
  metal: number;
  crystal: number;
  deuterium: number;
}

/** The debris rules a battle was actually resolved under. */
export interface DebrisSettings {
  fleet_percentage: number;
  defence_percentage: number;
  deuterium: boolean;
}

/** Resource cost breakdown. */
export interface ResourceCost {
  metal: number;
  crystal: number;
  deuterium: number;
}

/** Fleet state at a point in time. */
export interface FleetSnapshot {
  ships: FleetComposition;
  total_value: ResourceCost;
}

/** A combat participant (attacker or defender). */
export interface Participant {
  name: string;
  player_id?: number;
  coordinates?: string;
  technology: Technology;
  alliance?: string;
}

/** Recycler harvest information. */
export interface HarvestInfo {
  recyclers_needed: number;
  harvest_time_seconds: number;
}

/** Moon destruction chance (when Death Stars are involved). */
export interface MoonDestructionInfo {
  destruction_chance: number;
  rip_chance: number;
  death_stars: number;
  moon_size: number;
}

/** Economic summary of a battle. */
export interface EconomicSummary {
  debris_field: DebrisField;
  moon_chance: number;
  plunder: PlanetResources;
  attacker_losses_cost: ResourceCost;
  defender_losses_cost: ResourceCost;
  attacker_profit: number;
  defender_profit: number;
  harvest_info?: HarvestInfo;
}

/** Round-by-round combat details. */
export interface RoundDetails {
  round_number: number;
  attackers_start: number;
  defenders_start: number;
  attackers_destroyed: number;
  defenders_destroyed: number;
  attackers_end: number;
  defenders_end: number;
  attacker_damage?: number;
  defender_damage?: number;
  attacker_shots?: number;
  defender_shots?: number;
  attacker_shield_damage?: number;
  defender_shield_damage?: number;
}

/** Per-round, per-ship-type composition snapshot. */
export interface RoundComposition {
  round_number: number;
  attacker_by_type_start: FleetComposition;
  defender_by_type_start: FleetComposition;
  attacker_by_type_destroyed: FleetComposition;
  defender_by_type_destroyed: FleetComposition;
  attacker_by_type_end: FleetComposition;
  defender_by_type_end: FleetComposition;
}

/** Per-slot result. */
export interface SlotResult {
  slot_id: string;
  initial: FleetComposition;
  losses: FleetComposition;
  remaining: FleetComposition;
}

/** Result of a single simulation. */
export interface SimulationResult {
  outcome: CombatOutcome;
  rounds: number;
  attacker_losses: FleetComposition;
  defender_losses: FleetComposition;
  attacker_remaining: FleetComposition;
  defender_remaining: FleetComposition;
  debris_field: DebrisField;
  loot: PlanetResources;
  attacker_profit: number;
  defender_profit: number;
  round_details?: RoundDetails[];
  round_compositions?: RoundComposition[];
  round_compositions_by_slot?: Record<string, RoundComposition[]>;
  attacker_slots?: SlotResult[];
  defender_slots?: SlotResult[];
}

/** Aggregated results across many simulations. */
export interface CombatResults {
  simulations: number;
  attacker_wins: number;
  defender_wins: number;
  draws: number;
  results: SimulationResult[];
  duration_ms: number;
  average_rounds: number;
  debris_settings: DebrisSettings;
}

/** The full combat report. */
export interface CombatReport {
  battle_id: string;
  timestamp: number;
  battle_type: BattleType;
  attacker: Participant;
  defender: Participant;
  outcome: CombatOutcome;
  rounds: number;
  attacker_fleet_start: FleetSnapshot;
  defender_fleet_start: FleetSnapshot;
  attacker_losses: FleetSnapshot;
  defender_losses: FleetSnapshot;
  attacker_fleet_end: FleetSnapshot;
  defender_fleet_end: FleetSnapshot;
  economics: EconomicSummary;
  round_details?: RoundDetails[];
  moon_destruction?: MoonDestructionInfo;
  simulation_count: number;
  duration_ms: number;
}

/** The `POST /api/simulate` response: two halves, `results` and `report`. */
export interface SimulationResponse {
  results: CombatResults;
  report: CombatReport;
}
