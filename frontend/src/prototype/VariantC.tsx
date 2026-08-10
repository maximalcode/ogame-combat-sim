// PROTOTYPE — throwaway. Variant C: "Cockpit".
//
// Results-first: the main area is permanently the report — verdict, losses,
// economy, per-round — and all input lives in a persistent left sidebar as
// compact accordions (one per slot, one for tech, one for universe). Simulate
// sits at the sidebar's foot and the report re-renders in place. The
// hierarchy says "the answer is the page; the inputs are a control panel" —
// built for iterating on a fleet, not for one-shot lookups.

import {
  DEFENCE_NAMES,
  DEMO_ATTACKER_SLOTS,
  DEMO_DEFENDER_SLOTS,
  DEMO_RESULT,
  SHIP_NAMES,
  fmt,
} from "./data";

function Accordion({
  label,
  open,
  children,
}: Readonly<{
  label: string;
  open?: boolean;
  children?: React.ReactNode;
}>) {
  return (
    <details open={open} className="rounded border border-slate-800 bg-slate-900/40">
      <summary className="cursor-pointer px-3 py-2 text-xs font-semibold text-slate-300">
        {label}
      </summary>
      <div className="border-t border-slate-800 px-3 py-2">{children}</div>
    </details>
  );
}

export function VariantC() {
  const r = DEMO_RESULT;
  return (
    <main className="mx-auto flex max-w-6xl gap-4 px-4 py-6">
      <aside className="w-72 shrink-0 space-y-2">
        {DEMO_ATTACKER_SLOTS.map((s) => (
          <Accordion key={s.id} label={`⚔ ${s.id} · ${s.name}`} open={s.id === "A1"}>
            {Object.entries(s.entities).map(([id, count]) => (
              <p key={id} className="flex justify-between text-xs text-slate-300">
                <span>{SHIP_NAMES[id] ?? DEFENCE_NAMES[id]}</span>
                <span className="font-mono">{fmt(count)}</span>
              </p>
            ))}
          </Accordion>
        ))}
        {DEMO_DEFENDER_SLOTS.map((s) => (
          <Accordion key={s.id} label={`🛡 ${s.id} · ${s.name}`}>
            <p className="text-xs text-slate-500">…</p>
          </Accordion>
        ))}
        <Accordion label="Technology & classes">
          <p className="text-xs text-slate-500">W14 / S12 / A13 · General</p>
        </Accordion>
        <Accordion label="Universe & resources">
          <p className="text-xs text-slate-500">Debris 30/0 · plunder 50%</p>
        </Accordion>
        <button className="w-full rounded-md bg-indigo-600 py-2 text-sm font-semibold text-white">
          Simulate · 1000×
        </button>
      </aside>

      <section className="flex-1 space-y-3">
        <div className="flex items-center gap-4 rounded-lg border border-slate-800 bg-slate-900/40 p-4">
          <div className="h-20 w-20 shrink-0 rounded-full border-8 border-emerald-500/70 border-r-rose-500/70 border-b-slate-600" />
          <div>
            <p className="text-xl font-bold text-emerald-400">
              Attacker wins {Math.round(r.attackerWinRate * 100)}%
            </p>
            <p className="text-xs text-slate-400">
              {Math.round(r.defenderWinRate * 100)}% defender ·{" "}
              {Math.round(r.drawRate * 100)}% draw · {r.averageRounds} avg rounds ·
              1000 simulations
            </p>
          </div>
        </div>

        <div className="grid gap-3 lg:grid-cols-3">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-xs">
            <p className="mb-1 font-semibold text-slate-400">Attacker losses</p>
            {Object.entries(r.attackerLosses).map(([id, n]) => (
              <p key={id} className="flex justify-between text-slate-300">
                <span>{SHIP_NAMES[id]}</span>
                <span className="font-mono">{fmt(n)}</span>
              </p>
            ))}
            <p className="mt-1 border-t border-slate-800 pt-1 font-mono text-emerald-400">
              +{fmt(r.attackerProfit)} profit
            </p>
          </div>
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-xs">
            <p className="mb-1 font-semibold text-slate-400">Defender losses</p>
            {Object.entries(r.defenderLosses).map(([id, n]) => (
              <p key={id} className="flex justify-between text-slate-300">
                <span>{SHIP_NAMES[id] ?? DEFENCE_NAMES[id]}</span>
                <span className="font-mono">{fmt(n)}</span>
              </p>
            ))}
          </div>
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-xs">
            <p className="mb-1 font-semibold text-slate-400">Field after battle</p>
            <p className="flex justify-between text-slate-300">
              <span>Debris</span>
              <span className="font-mono">
                {fmt(r.debris.metal)} / {fmt(r.debris.crystal)}
              </span>
            </p>
            <p className="flex justify-between text-slate-300">
              <span>Loot</span>
              <span className="font-mono">
                {fmt(r.loot.metal)} / {fmt(r.loot.crystal)} / {fmt(r.loot.deuterium)}
              </span>
            </p>
            <p className="flex justify-between text-slate-300">
              <span>Moon chance</span>
              <span className="font-mono">{r.moonChance}%</span>
            </p>
            <p className="flex justify-between text-slate-300">
              <span>Recyclers needed</span>
              <span className="font-mono">{r.recyclersNeeded}</span>
            </p>
          </div>
        </div>

        <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
          <p className="text-xs font-semibold text-slate-400">
            Rounds — fleet strength per round (chart placeholder)
          </p>
          <div className="mt-2 flex h-28 items-end gap-2">
            {[100, 84, 61, 47, 31].map((h, i) => (
              <div key={i} className="flex-1">
                <div
                  className="rounded-t bg-indigo-500/60"
                  style={{ height: `${String(h)}%` }}
                />
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}
