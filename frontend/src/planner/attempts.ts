import type { CombatRequest, ResourceCost, SimulationResponse } from "@/api/types";

export interface Run {
  readonly number: number;
  // makeRequest creates a detached value at launch. Neither it nor the editor
  // snapshot is updated when the player edits the next scenario.
  readonly request: CombatRequest;
  readonly signature: string;
  readonly response: SimulationResponse;
}
export interface Attempts {
  readonly latest: Run | null;
  readonly previous: Run | null;
}
const total = (resources: ResourceCost) =>
  resources.metal + resources.crystal + resources.deuterium;

export function averages(response: SimulationResponse) {
  const { results: battles, simulations, attacker_wins, defender_wins, draws } = response.results;
  const counts = [attacker_wins, defender_wins, draws];
  if (
    battles.length !== simulations ||
    !counts.every((count) => Number.isInteger(count) && count >= 0) ||
    attacker_wins + defender_wins + draws !== simulations
  ) {
    throw new Error("Unvollständige Stichprobe erhalten.");
  }
  // Recover exact resource losses from each battle's economic identity, before
  // averaging. The summary report's integer average ship counts lose fractions.
  const sums = battles.reduce(
    (sum, battle) => ({
      losses: sum.losses + total(battle.debris_field) + total(battle.loot) - battle.attacker_profit,
      profit: sum.profit + battle.attacker_profit,
      metal: sum.metal + battle.debris_field.metal,
      crystal: sum.crystal + battle.debris_field.crystal,
      deuterium: sum.deuterium + battle.debris_field.deuterium,
    }),
    { losses: 0, profit: 0, metal: 0, crystal: 0, deuterium: 0 },
  );
  const count = battles.length;
  if (count === 0) throw new Error("Keine Einzelergebnisse erhalten.");
  const result = {
    losses: sums.losses / count,
    profit: sums.profit / count,
    metal: sums.metal / count,
    crystal: sums.crystal / count,
    deuterium: sums.deuterium / count,
  };
  if (!Object.values(result).every(Number.isFinite)) {
    throw new Error("Ungültige Ressourcenbeträge erhalten.");
  }
  return result;
}
export const format = (value: number) =>
  value.toLocaleString("de-DE", { maximumFractionDigits: 2 });
export const signed = (value: number) =>
  value.toLocaleString("de-DE", { maximumFractionDigits: 2, signDisplay: "exceptZero" });
