// Per-ship lifeform percentages for one side, behind the approved
// "Lifeforms…" disclosure so the dense board stays scannable.

import type { LifeformBonus, LifeformBonuses } from "@/api/types";
import { DEFENCES, SHIPS } from "@/fleet/catalog";
import { SIDE_LABELS, type Side } from "@/fleet/types";

interface LifeformInputProps {
  readonly side: Side;
  readonly value: LifeformBonuses | undefined;
  readonly onChange: (bonuses: LifeformBonuses | undefined) => void;
}

function isActive(bonus: LifeformBonus | undefined): boolean {
  return (
    bonus !== undefined &&
    Object.values(bonus).some((number) => number !== undefined && number !== 0)
  );
}

function percentageFromInput(rawValue: string): number | undefined {
  if (rawValue.trim() === "") return undefined;
  const number = Number(rawValue);
  return Number.isFinite(number) && number >= 0 ? number : undefined;
}

export function LifeformInput({ side, value, onChange }: LifeformInputProps) {
  const activeCount = Object.values(value ?? {}).filter(isActive).length;
  const entities = side === "attacker" ? SHIPS : [...SHIPS, ...DEFENCES];

  const changeBonus = (entityId: string, rawValue: string): void => {
    const percentage = percentageFromInput(rawValue);
    const withoutEntity = Object.fromEntries(
      Object.entries(value ?? {}).filter(([id]) => id !== entityId),
    );
    if (percentage === undefined || percentage === 0) {
      onChange(
        Object.keys(withoutEntity).length === 0 ? undefined : withoutEntity,
      );
      return;
    }
    const bonus: LifeformBonus = {
      weapon: percentage,
      shield: percentage,
      armour: percentage,
    };
    const next: LifeformBonuses = { ...withoutEntity, [entityId]: bonus };
    onChange(next);
  };

  return (
    <details className="mt-4 border-t border-slate-800 pt-3">
      <summary className="cursor-pointer text-xs font-semibold uppercase tracking-wide text-slate-400 hover:text-slate-200">
        Lifeforms… {activeCount > 0 && `(${String(activeCount)})`}
      </summary>
      <p className="mt-2 text-xs leading-relaxed text-slate-500">
        Enter each ship or defence&apos;s resolved bonus percentage. The same value applies to weapon, shield and armour.
      </p>
      <div className="mt-3 max-h-72 space-y-1.5 overflow-y-auto pr-1">
        {entities.map((entity) => (
          <label
            key={entity.id}
            className="flex items-center justify-between gap-2 text-xs text-slate-400"
          >
            <span>{entity.name}</span>
            <span className="flex items-center gap-1">
              <input
                type="number"
                min="0"
                step="0.01"
                inputMode="decimal"
                aria-label={`${SIDE_LABELS[side]} ${entity.name} lifeform bonus percentage`}
                value={value?.[entity.id]?.weapon ?? ""}
                onChange={(event) => {
                  changeBonus(entity.id, event.target.value);
                }}
                className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-right font-mono text-xs text-slate-100 tabular-nums focus:border-slate-500 focus:outline-none"
              />
              <span className="text-slate-600">%</span>
            </span>
          </label>
        ))}
      </div>
      <p className="mt-3 text-xs leading-relaxed text-amber-300/70">
        Lifeforms affect the simulation, but the report does not repeat these per-ship bonuses.
      </p>
    </details>
  );
}
