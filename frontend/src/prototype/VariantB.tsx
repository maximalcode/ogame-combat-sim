// PROTOTYPE — throwaway. Variant B: "Versus board".
//
// Two mirrored full-height columns facing each other across a centre rail,
// slots stacked vertically as cards inside each column. The centre rail owns
// the Simulate action and the headline verdict; detailed results replace the
// board in a second step with a back link. The hierarchy says "this is a
// matchup" — the two sides are the page, everything else hangs off them.

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

function SlotCard({ slot }: Readonly<{ slot: DemoSlot }>) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
      <div className="flex items-center justify-between">
        <p className="text-xs font-semibold text-slate-300">
          {slot.id} · {slot.name}
        </p>
        <button className="text-xs text-slate-600">remove</button>
      </div>
      <div className="mt-2 space-y-1">
        {Object.entries(slot.entities).map(([id, count]) => (
          <p key={id} className="flex justify-between text-xs text-slate-300">
            <span>{SHIP_NAMES[id] ?? DEFENCE_NAMES[id]}</span>
            <span className="font-mono">{fmt(count)}</span>
          </p>
        ))}
      </div>
      <button className="mt-2 w-full rounded border border-dashed border-slate-700 py-1 text-xs text-slate-500">
        + add by name…
      </button>
    </div>
  );
}

function SideColumn({
  title,
  accent,
  slots,
}: Readonly<{
  title: string;
  accent: string;
  slots: DemoSlot[];
}>) {
  return (
    <div className="flex-1 space-y-3">
      <div className={`rounded-lg border-t-2 ${accent} bg-slate-900/60 p-3`}>
        <h2 className="text-sm font-bold uppercase tracking-wide">{title}</h2>
        <div className="mt-2 flex flex-wrap gap-2 text-xs text-slate-400">
          <span className="rounded bg-slate-800 px-2 py-0.5 font-mono">
            W{DEMO_TECH.weapon} / S{DEMO_TECH.shield} / A{DEMO_TECH.armour}
          </span>
          <span className="rounded bg-slate-800 px-2 py-0.5">General</span>
          <span className="rounded bg-slate-800 px-2 py-0.5">Lifeforms ✓</span>
          <button className="text-indigo-400">edit</button>
        </div>
      </div>
      {slots.map((s) => (
        <SlotCard key={s.id} slot={s} />
      ))}
      <button className="w-full rounded-lg border border-dashed border-slate-700 py-2 text-xs text-slate-500">
        + add ACS slot
      </button>
    </div>
  );
}

export function VariantB() {
  const r = DEMO_RESULT;
  return (
    <main className="mx-auto flex max-w-6xl gap-4 px-4 py-6">
      <SideColumn
        title="Attacker"
        accent="border-emerald-500"
        slots={DEMO_ATTACKER_SLOTS}
      />

      <div className="flex w-48 shrink-0 flex-col items-center gap-3 pt-8">
        <span className="text-2xl font-black text-slate-600">VS</span>
        <button className="w-full rounded-md bg-indigo-600 py-3 text-sm font-semibold text-white">
          Simulate
        </button>
        <label className="w-full text-center text-xs text-slate-500">
          <input
            readOnly
            value={1000}
            className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-center font-mono"
          />
          simulations
        </label>
        <div className="w-full rounded-lg border border-slate-800 bg-slate-900/60 p-3 text-center">
          <p className="text-3xl font-bold text-emerald-400">
            {Math.round(r.attackerWinRate * 100)}%
          </p>
          <p className="text-xs text-slate-400">attacker wins</p>
          <p className="mt-1 text-xs text-slate-500">
            {r.averageRounds} avg rounds
          </p>
          <button className="mt-2 w-full rounded border border-indigo-500 py-1 text-xs text-indigo-400">
            Full report →
          </button>
        </div>
        <p className="text-center text-[10px] leading-tight text-slate-600">
          Full report replaces the board; back link returns to setup unchanged.
        </p>
      </div>

      <SideColumn
        title="Defender"
        accent="border-rose-500"
        slots={DEMO_DEFENDER_SLOTS}
      />
    </main>
  );
}
