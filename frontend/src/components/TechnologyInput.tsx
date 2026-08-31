// Technology, class, lifeform and universe controls. This is deliberately a
// controlled region: App owns every value that becomes a combat request.

import { useRef } from "react";

import { ClassInput } from "@/components/ClassInput";
import { LifeformInput } from "@/components/LifeformInput";
import { UniverseSettingsInput } from "@/components/UniverseSettingsInput";
import {
  EMPTY_PLANET_RESOURCES,
  type ClassBonuses,
  type CombatInput,
} from "@/combat/input";
import type {
  LifeformBonuses,
  PlanetResources,
  Technology,
} from "@/api/types";
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

function replaceOptionalSide<T>(
  values: Readonly<Partial<Record<Side, T>>>,
  side: Side,
  next: T | undefined,
): Partial<Record<Side, T>> {
  const otherSide: Side = side === "attacker" ? "defender" : "attacker";
  const other = values[otherSide];
  return {
    ...(other === undefined ? {} : { [otherSide]: other }),
    ...(next === undefined ? {} : { [side]: next }),
  };
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
        Technology &amp; bonuses
      </h2>
      <div className="mt-3 grid gap-4 md:grid-cols-2 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_15rem]">
        {(["attacker", "defender"] as const).map((side) => (
          <div
            key={side}
            className={`rounded border bg-slate-950/30 p-3 ${
              side === "attacker"
                ? "border-attacker/30"
                : "border-defender/30"
            }`}
          >
            <fieldset className="space-y-2">
              <legend
                className={`text-sm font-medium ${
                  side === "attacker" ? "text-attacker" : "text-defender"
                }`}
              >
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

            <ClassInput
              side={side}
              value={value.classBonuses[side]}
              onChange={(bonuses: ClassBonuses | undefined) => {
                onChange({
                  ...value,
                  classBonuses: replaceOptionalSide(
                    value.classBonuses,
                    side,
                    bonuses,
                  ),
                });
              }}
            />

            <LifeformInput
              side={side}
              value={value.lifeform[side]}
              onChange={(lifeform: LifeformBonuses | undefined) => {
                onChange({
                  ...value,
                  lifeform: replaceOptionalSide(
                    value.lifeform,
                    side,
                    lifeform,
                  ),
                });
              }}
            />

            {side === "defender" && (
              <fieldset className="mt-4 border-t border-slate-800 pt-3">
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
                    Planet resources
                    <span className="mt-1 block text-xs text-slate-500">
                      Leave unchecked when unknown; checked zeroes mean a known empty planet.
                    </span>
                  </span>
                </label>
                <div className="mt-3 space-y-2">
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
            )}
          </div>
        ))}

        <UniverseSettingsInput
          value={value.universeSettings}
          onChange={(settings) => {
            if (settings === undefined) {
              const { universeSettings: _, ...withoutUniverseSettings } = value;
              onChange(withoutUniverseSettings);
            } else {
              onChange({ ...value, universeSettings: settings });
            }
          }}
        />
      </div>
      <p className="mt-4 text-xs text-slate-500">
        Drive technologies are not shown: this simulator does not yet calculate flight time or speed.
      </p>
    </section>
  );
}
