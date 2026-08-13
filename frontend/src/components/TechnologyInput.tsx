// Technology and defender-resource controls. This is deliberately a controlled
// region: App owns the values that become a combat request.

import { useRef } from "react";

import {
  EMPTY_PLANET_RESOURCES,
  type CombatInput,
} from "@/combat/input";
import type { PlanetResources, Technology } from "@/api/types";
import { SIDE_LABELS, type Side } from "@/fleet/types";

interface TechnologyInputProps {
  readonly value: CombatInput;
  readonly onChange: (input: CombatInput) => void;
}

type CombatTechnology = keyof Pick<Technology, "weapon" | "shield" | "armour">;

const COMBAT_TECHNOLOGIES: readonly {
  readonly key: CombatTechnology;
  readonly label: string;
}[] = [
  { key: "weapon", label: "Weapons" },
  { key: "shield", label: "Shielding" },
  { key: "armour", label: "Armour" },
];

const RESOURCE_FIELDS: readonly {
  readonly key: keyof PlanetResources;
  readonly label: string;
}[] = [
  { key: "metal", label: "Metal" },
  { key: "crystal", label: "Crystal" },
  { key: "deuterium", label: "Deuterium" },
];

function integerFromInput(value: string, maximum: number): number | undefined {
  if (value.trim() === "") return undefined;
  const number = Number(value);
  return Number.isInteger(number) && number >= 0 && number <= maximum
    ? number
    : undefined;
}

export function TechnologyInput({ value, onChange }: TechnologyInputProps) {
  const rememberedResources = useRef(
    value.planetResources ?? EMPTY_PLANET_RESOURCES,
  );

  const changeTechnology = (
    side: Side,
    key: CombatTechnology,
    rawValue: string,
  ): void => {
    const level = integerFromInput(rawValue, 255);
    if (level === undefined) return;
    onChange({
      ...value,
      technology: {
        ...value.technology,
        [side]: { ...value.technology[side], [key]: level },
      },
    });
  };

  const changeResources = (key: keyof PlanetResources, rawValue: string): void => {
    const amount = integerFromInput(rawValue, Number.MAX_SAFE_INTEGER);
    if (amount === undefined || value.planetResources === undefined) return;
    const planetResources = { ...value.planetResources, [key]: amount };
    rememberedResources.current = planetResources;
    onChange({
      ...value,
      planetResources,
    });
  };

  const resourcesEnabled = value.planetResources !== undefined;

  return (
    <section
      aria-labelledby="technology-input-heading"
      className="rounded-lg border border-slate-800 bg-slate-900/40 p-4"
    >
      <h2
        id="technology-input-heading"
        className="text-sm font-semibold uppercase tracking-wide text-slate-400"
      >
        Technology &amp; planet resources
      </h2>
      <div className="mt-3 grid gap-5 md:grid-cols-2">
        {(["attacker", "defender"] as const).map((side) => (
          <fieldset key={side} className="space-y-2">
            <legend className="text-sm font-medium text-slate-200">
              {SIDE_LABELS[side]}
            </legend>
            {COMBAT_TECHNOLOGIES.map(({ key, label }) => (
              <label key={key} className="flex items-center justify-between gap-3 text-sm text-slate-400">
                {label}
                <input
                  type="number"
                  min="0"
                  max="255"
                  step="1"
                  inputMode="numeric"
                  aria-label={`${SIDE_LABELS[side]} ${label} level`}
                  value={value.technology[side][key]}
                  onChange={(event) => {
                    changeTechnology(side, key, event.target.value);
                  }}
                  className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-right font-mono text-sm text-slate-100 tabular-nums focus:border-slate-500 focus:outline-none"
                />
              </label>
            ))}
          </fieldset>
        ))}
      </div>

      <fieldset className="mt-5 border-t border-slate-800 pt-4">
        <label className="flex items-start gap-3 text-sm text-slate-300">
          <input
            type="checkbox"
            checked={resourcesEnabled}
            onChange={(event) => {
              if (event.target.checked) {
                onChange({
                  ...value,
                  planetResources: rememberedResources.current,
                });
              } else {
                if (value.planetResources !== undefined) {
                  rememberedResources.current = value.planetResources;
                }
                const { planetResources: _, ...withoutResources } = value;
                onChange(withoutResources);
              }
            }}
            className="mt-0.5 h-4 w-4 rounded border-slate-600 bg-slate-950 text-indigo-600 focus:ring-indigo-500"
          />
          <span>
            Defender planet resources
            <span className="mt-1 block text-xs text-slate-500">
              Leave unchecked when the planet is unknown; checked zeroes mean a known empty planet.
            </span>
          </span>
        </label>
        <div className="mt-3 grid gap-2 sm:grid-cols-3">
          {RESOURCE_FIELDS.map(({ key, label }) => (
            <label key={key} className="flex items-center justify-between gap-2 text-sm text-slate-400">
              {label}
              <input
                type="number"
                min="0"
                step="1"
                inputMode="numeric"
                disabled={!resourcesEnabled}
                value={value.planetResources?.[key] ?? ""}
                onChange={(event) => {
                  changeResources(key, event.target.value);
                }}
                className="w-28 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-right font-mono text-sm text-slate-100 tabular-nums focus:border-slate-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
              />
            </label>
          ))}
        </div>
      </fieldset>
      <p className="mt-4 text-xs text-slate-500">
        Drive technologies are not shown: this simulator does not yet calculate flight time or speed.
      </p>
    </section>
  );
}
