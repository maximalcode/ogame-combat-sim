// Optional universe debris controls.
//
// Merely opening this accordion changes no request state. The complete
// three-field block is created only when the user enables the override; turning
// it off removes `universe_settings` instead of sending serde's defaults.

import { useRef } from "react";

import {
  STANDARD_UNIVERSE_DEBRIS,
  type UniverseDebrisInput,
} from "@/combat/input";

interface UniverseSettingsInputProps {
  readonly value: UniverseDebrisInput | undefined;
  readonly onChange: (settings: UniverseDebrisInput | undefined) => void;
}

type DebrisPercentageKey = "debris_fleet" | "debris_defence";

function percentageFromInput(rawValue: string): number | undefined {
  if (rawValue.trim() === "") return undefined;
  const number = Number(rawValue);
  return Number.isInteger(number) && number >= 0 && number <= 100
    ? number
    : undefined;
}

export function UniverseSettingsInput({
  value,
  onChange,
}: UniverseSettingsInputProps) {
  const remembered = useRef(value ?? STANDARD_UNIVERSE_DEBRIS);
  const enabled = value !== undefined;

  const changePercentage = (
    key: DebrisPercentageKey,
    rawValue: string,
  ): void => {
    const percentage = percentageFromInput(rawValue);
    if (percentage === undefined || value === undefined) return;
    const next = { ...value, [key]: percentage };
    remembered.current = next;
    onChange(next);
  };

  return (
    <details className="rounded border border-slate-800 bg-slate-950/40 p-3 lg:self-start">
      <summary className="cursor-pointer text-sm font-medium text-slate-300 hover:text-slate-100">
        Universe debris
        <span className="ml-2 text-xs font-normal text-slate-600">
          {enabled ? "override on" : "standard fallback"}
        </span>
      </summary>
      <label className="mt-3 flex items-start gap-2 text-xs text-slate-300">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(event) => {
            if (event.target.checked) {
              onChange(remembered.current);
            } else {
              if (value !== undefined) remembered.current = value;
              onChange(undefined);
            }
          }}
          className="mt-0.5 h-4 w-4 rounded border-slate-600 bg-slate-950 text-indigo-600 focus:ring-indigo-500"
        />
        <span>
          Override debris rules
          <span className="mt-1 block leading-relaxed text-slate-500">
            Off sends no universe block, preserving the request&apos;s debris fallback.
          </span>
        </span>
      </label>
      <div className="mt-3 space-y-2">
        <label className="flex items-center justify-between gap-2 text-xs text-slate-400">
          Fleet debris
          <span className="flex items-center gap-1">
            <input
              type="number"
              min="0"
              max="100"
              step="1"
              inputMode="numeric"
              disabled={!enabled}
              value={value?.debris_fleet ?? remembered.current.debris_fleet}
              onChange={(event) => {
                changePercentage("debris_fleet", event.target.value);
              }}
              className="w-16 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-right font-mono text-xs text-slate-100 tabular-nums focus:border-slate-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
            />
            <span className="text-slate-600">%</span>
          </span>
        </label>
        <label className="flex items-center justify-between gap-2 text-xs text-slate-400">
          Defence debris
          <span className="flex items-center gap-1">
            <input
              type="number"
              min="0"
              max="100"
              step="1"
              inputMode="numeric"
              disabled={!enabled}
              value={value?.debris_defence ?? remembered.current.debris_defence}
              onChange={(event) => {
                changePercentage("debris_defence", event.target.value);
              }}
              className="w-16 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-right font-mono text-xs text-slate-100 tabular-nums focus:border-slate-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
            />
            <span className="text-slate-600">%</span>
          </span>
        </label>
        <label className="flex items-start gap-2 text-xs text-slate-400">
          <input
            type="checkbox"
            disabled={!enabled}
            checked={value?.debris_deuterium ?? remembered.current.debris_deuterium}
            onChange={(event) => {
              if (value === undefined) return;
              const next = {
                ...value,
                debris_deuterium: event.target.checked,
              };
              remembered.current = next;
              onChange(next);
            }}
            className="mt-0.5 h-4 w-4 rounded border-slate-600 bg-slate-950 text-indigo-600 focus:ring-indigo-500 disabled:cursor-not-allowed disabled:opacity-40"
          />
          Include deuterium in debris
        </label>
      </div>
    </details>
  );
}
