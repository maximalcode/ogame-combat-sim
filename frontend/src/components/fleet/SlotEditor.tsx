// One slot's composition editor.
//
// Renders the entity types the user has added to a slot as rows (name, count,
// remove) plus an "add" picker listing the entities still available for this
// side. Counts are entered by name rather than by memorised id — the picker is
// the only way in — and the count field rejects negatives and non-integers at
// entry: it is a digit-only text field, so a minus sign or decimal point can
// never reach state, and anything above u32::MAX is rejected too — the count
// is a u32 on the Rust side, so a larger value would clear the digit filter
// only to fail serde at the API. That satisfies the issue's "reject at entry
// rather than at the API" requirement without a separate validation pass.

import { DEFENCES, SHIPS, entityName } from "@/fleet/catalog";
import type { FleetComposition } from "@/api/types";

interface SlotEditorProps {
  /** Which side — defences are offered to the defender only. */
  readonly side: "attacker" | "defender";
  readonly entities: FleetComposition;
  readonly onChange: (entities: FleetComposition) => void;
}

/** Digits only, including the empty string. Anything else is rejected at entry. */
const DIGITS_ONLY = /^\d*$/;

// u32::MAX — `FleetComposition` counts are u32 in combat-types, so this is the
// largest count the API will deserialize. Digit strings too long for exact
// float representation parse to something even larger, so the comparison
// still rejects them.
const MAX_COUNT = 4_294_967_295;

export function SlotEditor({ side, entities, onChange }: SlotEditorProps) {
  const available = side === "defender" ? [...SHIPS, ...DEFENCES] : SHIPS;
  const rows = Object.entries(entities);
  const usedIds = new Set(rows.map(([id]) => id));
  const addable = available.filter((entry) => !usedIds.has(entry.id));

  const setCount = (id: string, count: number): void => {
    onChange({ ...entities, [id]: count });
  };

  const removeEntity = (id: string): void => {
    const next: Record<string, number> = {};
    for (const [key, count] of Object.entries(entities)) {
      if (key !== id) next[key] = count;
    }
    onChange(next);
  };

  const addEntity = (id: string): void => {
    if (id === "") return;
    onChange({ ...entities, [id]: 0 });
  };

  const onCountChange = (id: string, raw: string): void => {
    if (!DIGITS_ONLY.test(raw)) return;
    const count = raw === "" ? 0 : Number.parseInt(raw, 10);
    if (count > MAX_COUNT) return;
    setCount(id, count);
  };

  return (
    <div className="space-y-1.5">
      {rows.length === 0 && (
        <p className="px-1 py-2 text-xs text-slate-500">
          No ships in this slot yet — add one below.
        </p>
      )}

      {rows.map(([id, count]) => (
        <div key={id} className="flex items-center gap-2">
          <span className="flex-1 truncate text-sm text-slate-200">
            {entityName(id)}
          </span>
          <input
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            value={count === 0 ? "" : String(count)}
            onChange={(event) => {
              onCountChange(id, event.target.value);
            }}
            placeholder="0"
            aria-label={`${entityName(id)} count`}
            className="w-24 rounded border border-slate-700 bg-slate-900 px-2 py-1 text-right font-mono text-sm text-slate-100 tabular-nums focus:border-slate-500 focus:outline-none"
          />
          <button
            type="button"
            onClick={() => {
              removeEntity(id);
            }}
            aria-label={`Remove ${entityName(id)}`}
            className="rounded px-1.5 py-0.5 text-slate-500 transition hover:bg-slate-800 hover:text-slate-300"
          >
            ×
          </button>
        </div>
      ))}

      {addable.length > 0 ? (
        <div className="pt-1">
          <select
            value=""
            onChange={(event) => {
              addEntity(event.target.value);
            }}
            aria-label="Add entity"
            className="w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm text-slate-300 focus:border-slate-500 focus:outline-none"
          >
            <option value="" disabled>
              Add {side === "defender" ? "ship or defence…" : "ship…"}
            </option>
            {addable.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.name}
              </option>
            ))}
          </select>
        </div>
      ) : (
        <p className="px-1 pt-1 text-xs text-slate-600">
          Every entity is already in this slot.
        </p>
      )}
    </div>
  );
}
