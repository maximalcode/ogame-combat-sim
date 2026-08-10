// PROTOTYPE — throwaway. Variant A: "Form flow".
//
// Single column, input-first: setup on top (two mirrored party cards, slots as
// tabs inside each), one Simulate bar, results appended below full-width.
// The hierarchy says "fill in the form, press the button, scroll for the
// answer" — closest to TrashSim and to what OGame players already know.

import {
  DEFENCE_NAMES,
  DEMO_ATTACKER_SLOTS,
  DEMO_DEFENDER_SLOTS,
  DEMO_RESULT,
  DEMO_TECH,
  type DemoSlot,
  SHIP_NAMES,
  fmt,
} from "./data";

function PartyCard({
  title,
  slots,
  defences,
}: Readonly<{
  title: string;
  slots: DemoSlot[];
  defences: boolean;
}>) {
  const active = slots[0] ?? { id: "?", name: "", entities: {} };
  return (
    <section className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-400">
          {title}
        </h2>
        <div className="flex gap-1">
          {slots.map((s, i) => (
            <button
              key={s.id}
              className={`rounded px-2 py-1 text-xs ${i === 0 ? "bg-indigo-600 text-white" : "bg-slate-800 text-slate-400"}`}
            >
              {s.id}
            </button>
          ))}
          <button className="rounded bg-slate-800 px-2 py-1 text-xs text-slate-500">
            + slot
          </button>
        </div>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1">
        {Object.entries(active.entities).map(([id, count]) => (
          <label key={id} className="flex items-center justify-between text-sm">
            <span className="text-slate-300">
              {SHIP_NAMES[id] ?? DEFENCE_NAMES[id]}
            </span>
            <input
              readOnly
              value={count}
              className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-0.5 text-right font-mono text-xs"
            />
          </label>
        ))}
        <button className="col-span-2 mt-1 rounded border border-dashed border-slate-700 py-1 text-xs text-slate-500">
          + add {defences ? "ship / defence" : "ship"} by name…
        </button>
      </div>

      <div className="mt-4 border-t border-slate-800 pt-3">
        <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
          Technology & bonuses
        </p>
        <div className="mt-2 flex flex-wrap items-end gap-3 text-xs">
          {Object.entries(DEMO_TECH).map(([k, v]) => (
            <label key={k} className="flex flex-col gap-1">
              <span className="capitalize text-slate-400">{k}</span>
              <input
                readOnly
                value={v}
                className="w-14 rounded border border-slate-700 bg-slate-950 px-2 py-0.5 text-right font-mono"
              />
            </label>
          ))}
          <label className="flex flex-col gap-1">
            <span className="text-slate-400">Class</span>
            <select className="rounded border border-slate-700 bg-slate-950 px-2 py-1">
              <option>General</option>
            </select>
          </label>
          <button className="rounded border border-slate-700 px-2 py-1 text-slate-400">
            Lifeforms…
          </button>
        </div>
      </div>
    </section>
  );
}

export function VariantA() {
  const r = DEMO_RESULT;
  return (
    <main className="mx-auto max-w-5xl space-y-4 px-4 py-6">
      <div className="grid gap-4 lg:grid-cols-2">
        <PartyCard title="Attacker" slots={DEMO_ATTACKER_SLOTS} defences={false} />
        <PartyCard title="Defender" slots={DEMO_DEFENDER_SLOTS} defences />
      </div>

      <div className="flex items-center gap-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3">
        <button className="rounded-md bg-indigo-600 px-5 py-2 text-sm font-medium text-white">
          Simulate
        </button>
        <label className="flex items-center gap-2 text-xs text-slate-400">
          Simulations
          <input
            readOnly
            value={1000}
            className="w-16 rounded border border-slate-700 bg-slate-950 px-2 py-0.5 text-right font-mono"
          />
        </label>
        <label className="flex items-center gap-2 text-xs text-slate-400">
          Planet resources
          <input
            readOnly
            value="1.2M / 800k / 300k"
            className="w-36 rounded border border-slate-700 bg-slate-950 px-2 py-0.5 text-right font-mono"
          />
        </label>
      </div>

      <section className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-400">
          Results
        </h2>
        <div className="mt-3 grid gap-3 sm:grid-cols-3">
          <div className="rounded bg-emerald-950/40 p-3 text-center">
            <p className="text-2xl font-bold text-emerald-400">
              {Math.round(r.attackerWinRate * 100)}%
            </p>
            <p className="text-xs text-slate-400">Attacker wins</p>
          </div>
          <div className="rounded bg-rose-950/40 p-3 text-center">
            <p className="text-2xl font-bold text-rose-400">
              {Math.round(r.defenderWinRate * 100)}%
            </p>
            <p className="text-xs text-slate-400">Defender wins</p>
          </div>
          <div className="rounded bg-slate-800/60 p-3 text-center">
            <p className="text-2xl font-bold text-slate-300">
              {Math.round(r.drawRate * 100)}%
            </p>
            <p className="text-xs text-slate-400">Draws · {r.averageRounds} avg rounds</p>
          </div>
        </div>
        <div className="mt-3 grid gap-3 lg:grid-cols-2">
          <div className="rounded border border-slate-800 p-3 text-xs">
            <p className="mb-1 font-semibold text-slate-400">Average losses</p>
            {Object.entries(r.attackerLosses).map(([id, n]) => (
              <p key={id} className="flex justify-between text-slate-300">
                <span>{SHIP_NAMES[id]}</span>
                <span className="font-mono">{fmt(n)}</span>
              </p>
            ))}
          </div>
          <div className="rounded border border-slate-800 p-3 text-xs">
            <p className="mb-1 font-semibold text-slate-400">Economy</p>
            <p className="flex justify-between text-slate-300">
              <span>Debris</span>
              <span className="font-mono">
                {fmt(r.debris.metal)} M / {fmt(r.debris.crystal)} C
              </span>
            </p>
            <p className="flex justify-between text-slate-300">
              <span>Attacker profit</span>
              <span className="font-mono text-emerald-400">+{fmt(r.attackerProfit)}</span>
            </p>
            <p className="flex justify-between text-slate-300">
              <span>Moon chance · recyclers</span>
              <span className="font-mono">
                {r.moonChance}% · {r.recyclersNeeded}
              </span>
            </p>
          </div>
        </div>
        <button className="mt-3 text-xs text-indigo-400">
          Show per-round detail ▾
        </button>
      </section>
    </main>
  );
}
