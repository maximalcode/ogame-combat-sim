// Optional player and alliance class selectors for one battle side.
//
// A blank selector means "not supplied" and collapses back to no bonus block.
// The labels say which classes affect combat so the UI does not imply that
// choosing Collector, Discoverer, Trader or Researcher changes the result.

import type {
  AllianceClass,
  PlayerClass,
} from "@/api/types";
import type { ClassBonuses } from "@/combat/input";
import { SIDE_LABELS, type Side } from "@/fleet/types";

interface ClassInputProps {
  readonly side: Side;
  readonly value: ClassBonuses | undefined;
  readonly onChange: (bonuses: ClassBonuses | undefined) => void;
}

interface ClassOption<T> {
  readonly value: T;
  readonly label: string;
}

const PLAYER_CLASSES: readonly ClassOption<PlayerClass>[] = [
  { value: "none", label: "None — no combat effect" },
  { value: "collector", label: "Collector — no combat effect" },
  { value: "general", label: "General — +2 combat levels" },
  { value: "discoverer", label: "Discoverer — no combat effect" },
];

const ALLIANCE_CLASSES: readonly ClassOption<AllianceClass>[] = [
  { value: "none", label: "None — no combat effect" },
  { value: "trader", label: "Trader — no combat effect" },
  { value: "warrior", label: "Warrior — +1 combat level" },
  { value: "researcher", label: "Researcher — no combat effect" },
];

function findOption<T>(
  options: readonly ClassOption<T>[],
  rawValue: string,
): T | undefined {
  return options.find(({ value }) => value === rawValue)?.value;
}

function finish(next: ClassBonuses): ClassBonuses | undefined {
  return next.player_class === undefined && next.alliance_class === undefined
    ? undefined
    : next;
}

export function ClassInput({ side, value, onChange }: ClassInputProps) {
  const changePlayerClass = (rawValue: string): void => {
    if (rawValue === "") {
      const { player_class: _, ...next } = value ?? {};
      onChange(finish(next));
      return;
    }
    const playerClass = findOption(PLAYER_CLASSES, rawValue);
    if (playerClass !== undefined) {
      onChange({ ...value, player_class: playerClass });
    }
  };

  const changeAllianceClass = (rawValue: string): void => {
    if (rawValue === "") {
      const { alliance_class: _, ...next } = value ?? {};
      onChange(finish(next));
      return;
    }
    const allianceClass = findOption(ALLIANCE_CLASSES, rawValue);
    if (allianceClass !== undefined) {
      onChange({ ...value, alliance_class: allianceClass });
    }
  };

  return (
    <fieldset className="mt-4 border-t border-slate-800 pt-3">
      <legend className="px-1 text-xs font-semibold uppercase tracking-wide text-slate-500">
        Classes
      </legend>
      <div className="mt-2 space-y-2">
        <label className="block text-xs text-slate-400">
          Player class
          <select
            aria-label={`${SIDE_LABELS[side]} player class`}
            value={value?.player_class ?? ""}
            onChange={(event) => {
              changePlayerClass(event.target.value);
            }}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-200 focus:border-slate-500 focus:outline-none"
          >
            <option value="">Not supplied</option>
            {PLAYER_CLASSES.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label className="block text-xs text-slate-400">
          Alliance class
          <select
            aria-label={`${SIDE_LABELS[side]} alliance class`}
            value={value?.alliance_class ?? ""}
            onChange={(event) => {
              changeAllianceClass(event.target.value);
            }}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-200 focus:border-slate-500 focus:outline-none"
          >
            <option value="">Not supplied</option>
            {ALLIANCE_CLASSES.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <p className="mt-2 text-xs leading-relaxed text-slate-500">
        General and Warrior add together, up to +3 Weapons, Shielding and Armour levels.
      </p>
    </fieldset>
  );
}
