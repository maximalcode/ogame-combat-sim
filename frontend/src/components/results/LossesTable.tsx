import type { FleetComposition, SimulationResponse } from "@/api/types";
import { entityName } from "@/fleet/catalog";
import { formatInteger } from "@/results/model";

interface LossesTableProps {
  readonly response: SimulationResponse;
}

interface LossRow {
  readonly id: string;
  readonly name: string;
  readonly attacker: number;
  readonly defender: number;
}

function lossRows(attacker: FleetComposition, defender: FleetComposition): readonly LossRow[] {
  const ids = new Set([...Object.keys(attacker), ...Object.keys(defender)]);
  return [...ids]
    .map((id) => ({
      id,
      name: entityName(id),
      attacker: attacker[id] ?? 0,
      defender: defender[id] ?? 0,
    }))
    .filter((row) => row.attacker > 0 || row.defender > 0)
    .sort((left, right) => Number(left.id) - Number(right.id));
}

export function LossesTable({ response }: LossesTableProps) {
  const { report } = response;
  const rows = lossRows(report.attacker_losses.ships, report.defender_losses.ships);

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-4">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <h3 className="font-medium text-slate-100">Average fleet losses</h3>
          <p className="mt-1 text-xs text-slate-500">Per ship type across the full simulation set</p>
        </div>
        <div className="flex gap-4 text-xs text-slate-500">
          <span>
            Attacker cost{" "}
            <strong className="font-mono font-medium text-rose-300">
              {formatInteger(
                report.economics.attacker_losses_cost.metal +
                  report.economics.attacker_losses_cost.crystal +
                  report.economics.attacker_losses_cost.deuterium,
              )}
            </strong>
          </span>
          <span>
            Defender cost{" "}
            <strong className="font-mono font-medium text-amber-300">
              {formatInteger(
                report.economics.defender_losses_cost.metal +
                  report.economics.defender_losses_cost.crystal +
                  report.economics.defender_losses_cost.deuterium,
              )}
            </strong>
          </span>
        </div>
      </div>

      {rows.length === 0 ? (
        <p className="mt-4 rounded border border-dashed border-slate-800 p-4 text-center text-sm text-slate-500">
          No ships were lost in the average result.
        </p>
      ) : (
        <div className="mt-4 overflow-x-auto">
          <table className="w-full min-w-[26rem] text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-slate-500">
              <tr className="border-b border-slate-800">
                <th className="pb-2 font-medium">Ship or defence</th>
                <th className="pb-2 text-right font-medium">Attacker lost</th>
                <th className="pb-2 text-right font-medium">Defender lost</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.id} className="border-b border-slate-800/70 last:border-0">
                  <td className="py-2.5 text-slate-200">
                    {row.name} <span className="font-mono text-xs text-slate-600">#{row.id}</span>
                  </td>
                  <td className="py-2.5 text-right font-mono text-indigo-200">
                    {formatInteger(row.attacker)}
                  </td>
                  <td className="py-2.5 text-right font-mono text-amber-200">
                    {formatInteger(row.defender)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
