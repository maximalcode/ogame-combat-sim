import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import type { SimulationResponse } from "@/api/types";
import {
  formatInteger,
  formatPercent,
  formatSigned,
  outcomeSummaries,
} from "@/results/model";

interface OutcomeDistributionProps {
  readonly response: SimulationResponse;
}

const OUTCOME_COLORS: Readonly<Record<string, string>> = {
  AttackersWin: "#6366f1",
  Draw: "#64748b",
  DefendersWin: "#f59e0b",
};

function profitClass(value: number): string {
  if (value > 0) return "text-emerald-300";
  if (value < 0) return "text-rose-300";
  return "text-slate-300";
}

export function OutcomeDistribution({ response }: OutcomeDistributionProps) {
  const outcomes = outcomeSummaries(response);
  const chartData = outcomes.map((outcome) => ({
    name: outcome.shortLabel,
    rate: outcome.rate * 100,
    fill: OUTCOME_COLORS[outcome.outcome],
  }));

  return (
    <div className="grid min-w-0 gap-4 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
      <div className="min-w-0 rounded-lg border border-slate-800 bg-slate-950/50 p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="font-medium text-slate-100">Outcome distribution</h3>
            <p className="mt-1 text-xs text-slate-500">
              {formatInteger(response.results.simulations)} simulations, not a single verdict
            </p>
          </div>
          <span className="rounded-full border border-slate-700 px-2 py-1 text-xs text-slate-400">
            avg {response.results.average_rounds.toFixed(1)} rounds
          </span>
        </div>

        <div className="mt-4 h-52" aria-label="Outcome rate chart">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData} margin={{ top: 8, right: 8, bottom: 0, left: -12 }}>
              <CartesianGrid stroke="#1e293b" vertical={false} />
              <XAxis dataKey="name" tick={{ fill: "#94a3b8", fontSize: 12 }} tickLine={false} />
              <YAxis
                domain={[0, 100]}
                tick={{ fill: "#64748b", fontSize: 11 }}
                tickFormatter={(value: number) => `${String(value)}%`}
                tickLine={false}
              />
              <Tooltip
                cursor={{ fill: "#1e293b", opacity: 0.45 }}
                contentStyle={{
                  background: "#020617",
                  border: "1px solid #334155",
                  borderRadius: "0.5rem",
                  color: "#e2e8f0",
                }}
                formatter={(value: number) => [`${value.toFixed(1)}%`, "Rate"]}
              />
              <Bar dataKey="rate" radius={[5, 5, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>

        <div className="mt-2 grid grid-cols-3 gap-2 text-center">
          {outcomes.map((outcome) => (
            <div key={outcome.outcome}>
              <p className="text-lg font-semibold text-slate-100">{formatPercent(outcome.rate)}</p>
              <p className="text-xs text-slate-500">
                {outcome.label} · {formatInteger(outcome.count)}
              </p>
            </div>
          ))}
        </div>
      </div>

      <div className="min-w-0 rounded-lg border border-slate-800 bg-slate-950/50 p-4">
        <h3 className="font-medium text-slate-100">What each outcome costs</h3>
        <p className="mt-1 text-xs text-slate-500">
          Average net profit inside each bucket; debris and plunder are already included.
        </p>
        <div className="mt-4 overflow-x-auto">
          <table className="w-full min-w-[28rem] text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-slate-500">
              <tr className="border-b border-slate-800">
                <th className="pb-2 font-medium">Outcome</th>
                <th className="pb-2 text-right font-medium">Runs</th>
                <th className="pb-2 text-right font-medium">Attacker net</th>
                <th className="pb-2 text-right font-medium">Defender net</th>
              </tr>
            </thead>
            <tbody>
              {outcomes.map((outcome) => (
                <tr key={outcome.outcome} className="border-b border-slate-800/70 last:border-0">
                  <td className="py-3 text-slate-200">
                    <span
                      className="mr-2 inline-block h-2 w-2 rounded-full"
                      style={{ backgroundColor: OUTCOME_COLORS[outcome.outcome] }}
                    />
                    {outcome.label}
                  </td>
                  <td className="py-3 text-right text-slate-400">
                    {formatInteger(outcome.count)}
                  </td>
                  <td className={`py-3 text-right font-mono ${profitClass(outcome.attackerProfit)}`}>
                    {outcome.count === 0 ? "—" : formatSigned(outcome.attackerProfit)}
                  </td>
                  <td className={`py-3 text-right font-mono ${profitClass(outcome.defenderProfit)}`}>
                    {outcome.count === 0 ? "—" : formatSigned(outcome.defenderProfit)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="mt-3 text-xs text-slate-600">
          A positive win rate can still be a bad trade. Read the distribution and the cost together.
        </p>
      </div>
    </div>
  );
}
