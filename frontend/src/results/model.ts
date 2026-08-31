// Derived display data for the results surface.
//
// The API response is the source of truth. These helpers only group and label
// it for presentation; keeping that work outside React components makes the
// distinction between aggregate and representative-run data explicit.

import type {
  CombatOutcome,
  FleetComposition,
  RoundComposition,
  SimulationResult,
  SimulationResponse,
} from "@/api/types";
import { entityName } from "@/fleet/catalog";

export interface OutcomeSummary {
  readonly outcome: CombatOutcome;
  readonly label: string;
  readonly shortLabel: string;
  readonly count: number;
  readonly rate: number;
  readonly attackerProfit: number;
  readonly defenderProfit: number;
}

export interface CompositionRow {
  readonly id: string;
  readonly name: string;
  readonly start: number;
  readonly destroyed: number;
  readonly end: number;
}

export interface RoundTrendPoint {
  readonly round: string;
  readonly attacker: number;
  readonly defender: number;
}

const OUTCOMES: readonly {
  readonly outcome: CombatOutcome;
  readonly label: string;
  readonly shortLabel: string;
}[] = [
  { outcome: "AttackersWin", label: "Attacker wins", shortLabel: "Attacker" },
  { outcome: "Draw", label: "Draws", shortLabel: "Draw" },
  { outcome: "DefendersWin", label: "Defender wins", shortLabel: "Defender" },
];

function average(
  results: readonly SimulationResult[],
  select: (result: SimulationResult) => number,
): number {
  if (results.length === 0) return 0;
  return results.reduce((total, result) => total + select(result), 0) / results.length;
}

/** Distribution plus average economics inside each outcome bucket. */
export function outcomeSummaries(response: SimulationResponse): readonly OutcomeSummary[] {
  const { results } = response.results;
  const simulationCount = response.results.simulations;

  return OUTCOMES.map(({ outcome, label, shortLabel }) => {
    const matching = results.filter((result) => result.outcome === outcome);
    return {
      outcome,
      label,
      shortLabel,
      count: matching.length,
      rate: simulationCount === 0 ? 0 : matching.length / simulationCount,
      attackerProfit: average(matching, (result) => result.attacker_profit),
      defenderProfit: average(matching, (result) => result.defender_profit),
    };
  });
}

/** One row per entity mentioned in any of the three round snapshots. */
export function compositionRows(
  start: FleetComposition,
  destroyed: FleetComposition,
  end: FleetComposition,
): readonly CompositionRow[] {
  const ids = new Set([...Object.keys(start), ...Object.keys(destroyed), ...Object.keys(end)]);
  return [...ids]
    .map((id) => ({
      id,
      name: entityName(id),
      start: start[id] ?? 0,
      destroyed: destroyed[id] ?? 0,
      end: end[id] ?? 0,
    }))
    .filter((row) => row.start > 0 || row.destroyed > 0 || row.end > 0)
    .sort((left, right) => Number(left.id) - Number(right.id));
}

export function totalFleet(composition: FleetComposition): number {
  return Object.values(composition).reduce((total, count) => total + count, 0);
}

/** End-of-round fleet totals for the compact trend chart. */
export function roundTrend(rounds: readonly RoundComposition[]): readonly RoundTrendPoint[] {
  return rounds.map((round) => ({
    round: `R${String(round.round_number)}`,
    attacker: totalFleet(round.attacker_by_type_end),
    defender: totalFleet(round.defender_by_type_end),
  }));
}

export function formatInteger(value: number): string {
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 }).format(value);
}

export function formatPercent(rate: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "percent",
    minimumFractionDigits: rate > 0 && rate < 0.01 ? 1 : 0,
    maximumFractionDigits: 1,
  }).format(rate);
}

export function formatSigned(value: number): string {
  const rounded = Math.round(value);
  if (rounded === 0) return "0";
  return `${rounded > 0 ? "+" : "−"}${formatInteger(Math.abs(rounded))}`;
}

export function formatSeconds(value: number): string {
  if (value < 60) return `${String(value)}s`;
  const minutes = Math.floor(value / 60);
  const seconds = value % 60;
  return `${String(minutes)}m ${String(seconds)}s`;
}
