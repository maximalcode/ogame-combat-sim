import type { PlanetResources, SimulationResponse } from "@/api/types";
import { formatInteger, formatSeconds, formatSigned } from "@/results/model";

interface EconomicsSummaryProps {
  readonly response: SimulationResponse;
}

interface ResourceLineProps {
  readonly label: string;
  readonly resources: PlanetResources;
}

function ResourceLine({ label, resources }: ResourceLineProps) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wide text-slate-500">{label}</p>
      <dl className="mt-2 grid grid-cols-3 gap-2 text-sm">
        <div>
          <dt className="text-xs text-slate-600">Metal</dt>
          <dd className="font-mono text-slate-200">{formatInteger(resources.metal)}</dd>
        </div>
        <div>
          <dt className="text-xs text-slate-600">Crystal</dt>
          <dd className="font-mono text-slate-200">{formatInteger(resources.crystal)}</dd>
        </div>
        <div>
          <dt className="text-xs text-slate-600">Deuterium</dt>
          <dd className="font-mono text-slate-200">{formatInteger(resources.deuterium)}</dd>
        </div>
      </dl>
    </div>
  );
}

function profitClass(value: number): string {
  if (value > 0) return "text-emerald-300";
  if (value < 0) return "text-rose-300";
  return "text-slate-300";
}

export function EconomicsSummary({ response }: EconomicsSummaryProps) {
  const { economics } = response.report;
  const debris = economics.debris_field;
  const harvest = economics.harvest_info;

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-4">
        <h3 className="font-medium text-slate-100">Debris and loot</h3>
        <div className="mt-4 space-y-4">
          <ResourceLine label="Average debris field" resources={debris} />
          <ResourceLine label="Average loot" resources={economics.plunder} />
        </div>
        <dl className="mt-4 grid grid-cols-3 gap-3 border-t border-slate-800 pt-4">
          <div>
            <dt className="text-xs text-slate-500">Moon chance</dt>
            <dd className="mt-1 font-mono text-slate-100">{economics.moon_chance.toFixed(1)}%</dd>
          </div>
          <div>
            <dt className="text-xs text-slate-500">Recyclers needed</dt>
            <dd className="mt-1 font-mono text-slate-100">
              {formatInteger(harvest?.recyclers_needed ?? 0)}
            </dd>
          </div>
          <div>
            <dt className="text-xs text-slate-500">Harvest time</dt>
            <dd className="mt-1 font-mono text-slate-100">
              {harvest === undefined ? "—" : formatSeconds(harvest.harvest_time_seconds)}
            </dd>
          </div>
        </dl>
      </div>

      <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-4">
        <h3 className="font-medium text-slate-100">Average net result</h3>
        <p className="mt-1 text-xs text-slate-500">
          Debris and loot minus losses, averaged over every simulation.
        </p>
        <div className="mt-4 grid gap-3 sm:grid-cols-2">
          <div className="rounded border border-indigo-900/60 bg-indigo-950/20 p-4">
            <p className="text-xs uppercase tracking-wide text-indigo-300/70">Attacker</p>
            <p className={`mt-2 font-mono text-2xl font-semibold ${profitClass(economics.attacker_profit)}`}>
              {formatSigned(economics.attacker_profit)}
            </p>
          </div>
          <div className="rounded border border-amber-900/60 bg-amber-950/20 p-4">
            <p className="text-xs uppercase tracking-wide text-amber-300/70">Defender</p>
            <p className={`mt-2 font-mono text-2xl font-semibold ${profitClass(economics.defender_profit)}`}>
              {formatSigned(economics.defender_profit)}
            </p>
          </div>
        </div>
        <p className="mt-4 text-xs leading-relaxed text-slate-500">
          The outcome-specific table above is the sharper decision tool: this average can hide an
          expensive losing tail behind profitable wins.
        </p>
      </div>
    </div>
  );
}
