import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import type { ApiError } from "@/api/client";
import type { FleetComposition, SimulationResponse } from "@/api/types";
import {
  compositionRows,
  formatInteger,
  roundTrend,
  totalFleet,
} from "@/results/model";

export type RoundResultsState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; error: ApiError }
  | { kind: "ok"; response: SimulationResponse };

interface RoundCompositionViewProps {
  readonly state: RoundResultsState;
  readonly onRetry: () => void;
}

interface SideRoundTableProps {
  readonly label: string;
  readonly accent: string;
  readonly start: FleetComposition;
  readonly destroyed: FleetComposition;
  readonly end: FleetComposition;
}

function SideRoundTable({ label, accent, start, destroyed, end }: SideRoundTableProps) {
  const rows = compositionRows(start, destroyed, end);
  return (
    <div className="min-w-0">
      <div className="flex items-center justify-between gap-2">
        <h5 className={`text-sm font-medium ${accent}`}>{label}</h5>
        <span className="text-xs text-slate-500">
          {formatInteger(totalFleet(start))} → {formatInteger(totalFleet(end))}
        </span>
      </div>
      <div className="mt-2 overflow-x-auto">
        <table className="w-full min-w-[19rem] text-xs">
          <thead className="text-left uppercase tracking-wide text-slate-600">
            <tr className="border-b border-slate-800">
              <th className="pb-1.5 font-medium">Type</th>
              <th className="pb-1.5 text-right font-medium">Start</th>
              <th className="pb-1.5 text-right font-medium">Lost</th>
              <th className="pb-1.5 text-right font-medium">End</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.id} className="border-b border-slate-800/60 last:border-0">
                <td className="py-1.5 text-slate-300">{row.name}</td>
                <td className="py-1.5 text-right font-mono text-slate-400">
                  {formatInteger(row.start)}
                </td>
                <td className="py-1.5 text-right font-mono text-rose-300">
                  {formatInteger(row.destroyed)}
                </td>
                <td className="py-1.5 text-right font-mono text-slate-200">
                  {formatInteger(row.end)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function outcomeLabel(outcome: string): string {
  if (outcome === "AttackersWin") return "Attacker win";
  if (outcome === "DefendersWin") return "Defender win";
  return "Draw";
}

function RoundResults({ response }: { readonly response: SimulationResponse }) {
  const result = response.results.results[0];
  const rounds = result?.round_compositions ?? [];

  if (result === undefined || rounds.length === 0) {
    return (
      <p className="rounded border border-dashed border-slate-700 p-5 text-center text-sm text-slate-500">
        The representative simulation returned no round-composition snapshots.
      </p>
    );
  }

  const trend = [...roundTrend(rounds)];

  return (
    <div className="space-y-4">
      <div className="min-w-0 rounded-lg border border-slate-800 bg-slate-950/50 p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h3 className="font-medium text-slate-100">Representative battle</h3>
            <p className="mt-1 text-xs text-slate-500">
              One fresh simulation with composition tracking enabled; not the aggregate above.
            </p>
          </div>
          <span className="rounded-full border border-slate-700 px-2.5 py-1 text-xs text-slate-300">
            {outcomeLabel(result.outcome)} · {String(result.rounds)}{" "}
            {result.rounds === 1 ? "round" : "rounds"}
          </span>
        </div>
        <div className="mt-4 h-48" aria-label="Fleet totals by round">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={trend} margin={{ top: 8, right: 12, bottom: 0, left: -8 }}>
              <CartesianGrid stroke="#1e293b" vertical={false} />
              <XAxis dataKey="round" tick={{ fill: "#64748b", fontSize: 11 }} tickLine={false} />
              <YAxis tick={{ fill: "#64748b", fontSize: 11 }} tickLine={false} />
              <Tooltip
                contentStyle={{
                  background: "#020617",
                  border: "1px solid #334155",
                  borderRadius: "0.5rem",
                  color: "#e2e8f0",
                }}
                formatter={(value: number) => formatInteger(value)}
              />
              <Line
                type="monotone"
                dataKey="attacker"
                name="Attacker"
                stroke="#818cf8"
                strokeWidth={2}
                dot={{ fill: "#818cf8", r: 3 }}
              />
              <Line
                type="monotone"
                dataKey="defender"
                name="Defender"
                stroke="#fbbf24"
                strokeWidth={2}
                dot={{ fill: "#fbbf24", r: 3 }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {rounds.map((round) => (
        <article key={round.round_number} className="rounded-lg border border-slate-800 bg-slate-950/50 p-4">
          <h4 className="text-xs font-semibold uppercase tracking-wide text-slate-400">
            Round {String(round.round_number)}
          </h4>
          <div className="mt-3 grid gap-5 lg:grid-cols-2">
            <SideRoundTable
              label="Attacker"
              accent="text-indigo-300"
              start={round.attacker_by_type_start}
              destroyed={round.attacker_by_type_destroyed}
              end={round.attacker_by_type_end}
            />
            <SideRoundTable
              label="Defender"
              accent="text-amber-300"
              start={round.defender_by_type_start}
              destroyed={round.defender_by_type_destroyed}
              end={round.defender_by_type_end}
            />
          </div>
        </article>
      ))}
    </div>
  );
}

export function RoundCompositionView({ state, onRetry }: RoundCompositionViewProps) {
  if (state.kind === "idle" || state.kind === "loading") {
    return (
      <div className="flex min-h-52 items-center justify-center rounded-lg border border-dashed border-slate-700 p-5 text-center">
        <div>
          <div className="mx-auto h-6 w-6 animate-spin rounded-full border-2 border-slate-700 border-t-indigo-400" />
          <p className="mt-3 text-sm text-slate-300" aria-live="polite">
            Running one battle with round composition tracking…
          </p>
          <p className="mt-1 text-xs text-slate-600">The aggregate simulation did not pay this extra cost.</p>
        </div>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div role="alert" className="rounded-lg border border-red-800 bg-red-950/30 p-4 text-sm text-red-200">
        <p className="font-semibold">Round-detail request failed</p>
        <p className="mt-1 break-words">{state.error.message}</p>
        <button
          type="button"
          onClick={onRetry}
          className="mt-3 rounded border border-red-700 px-3 py-1.5 text-xs font-medium text-red-100 hover:bg-red-900/50"
        >
          Try again
        </button>
      </div>
    );
  }

  return <RoundResults response={state.response} />;
}
