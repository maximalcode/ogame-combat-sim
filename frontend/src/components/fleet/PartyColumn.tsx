// One side of the fleet-entry board — attacker or defender.
//
// Renders the slot tab strip (A1/A2/+ or D1/D2/+) and the active slot's
// composition editor. The side's accent hue (attacker amber, defender ice-blue)
// runs through the header, the active tab and the column border, so a side is
// recognisable without reading its label — that is the layout decision's
// "carried through headers and tables".
//
// Owns no state: the slot list, the active index and every mutation arrive as
// props from the FleetEntry region, which is the single state owner. This keeps
// the two columns symmetric and prevents the attacker column from knowing
// anything about the defender's slots.

import { SlotEditor } from "@/components/fleet/SlotEditor";
import type { FleetComposition } from "@/api/types";
import type { FleetSlot } from "@/fleet/types";

export type Side = "attacker" | "defender";

interface PartyColumnProps {
  readonly side: Side;
  readonly label: string;
  readonly slots: readonly FleetSlot[];
  readonly activeIndex: number;
  readonly onSelectSlot: (index: number) => void;
  readonly onAddSlot: () => void;
  readonly onRemoveSlot: (index: number) => void;
  readonly onChangeSlot: (index: number, entities: FleetComposition) => void;
}

interface AccentClasses {
  readonly header: string;
  readonly border: string;
  readonly tabActive: string;
  readonly tabInactive: string;
  readonly addTab: string;
}

const ACCENT: Record<Side, AccentClasses> = {
  attacker: {
    header: "text-attacker",
    border: "border-attacker/40",
    tabActive: "bg-attacker text-slate-950 border-attacker",
    tabInactive: "border-attacker/30 text-attacker/80 hover:text-attacker",
    addTab: "border-attacker/30 text-attacker/70 hover:text-attacker hover:border-attacker",
  },
  defender: {
    header: "text-defender",
    border: "border-defender/40",
    tabActive: "bg-defender text-slate-950 border-defender",
    tabInactive: "border-defender/30 text-defender/80 hover:text-defender",
    addTab: "border-defender/30 text-defender/70 hover:text-defender hover:border-defender",
  },
};

export function PartyColumn({
  side,
  label,
  slots,
  activeIndex,
  onSelectSlot,
  onAddSlot,
  onRemoveSlot,
  onChangeSlot,
}: PartyColumnProps) {
  const accent = ACCENT[side];
  const safeIndex = Math.min(activeIndex, slots.length - 1);
  const activeSlot = slots[safeIndex];

  return (
    <section
      aria-labelledby={`${side}-heading`}
      className={`flex flex-col rounded-lg border ${accent.border} bg-slate-900/40 p-3`}
    >
      <h2
        id={`${side}-heading`}
        className={`text-sm font-semibold uppercase tracking-wide ${accent.header}`}
      >
        {label}
      </h2>

      <div className="mt-2 flex flex-wrap items-center gap-1.5">
        {slots.map((slot, index) => (
          <div key={slot.id} className="flex items-center">
            <button
              type="button"
              onClick={() => {
                onSelectSlot(index);
              }}
              className={`rounded-l border px-2.5 py-1 text-xs font-semibold transition ${
                index === safeIndex ? accent.tabActive : accent.tabInactive
              }`}
            >
              {slot.id}
            </button>
            {slots.length > 1 && (
              <button
                type="button"
                onClick={() => {
                  onRemoveSlot(index);
                }}
                aria-label={`Remove slot ${slot.id}`}
                className={`rounded-r border-y border-r px-1.5 py-1 text-xs transition ${
                  index === safeIndex ? accent.tabActive : accent.tabInactive
                }`}
              >
                ×
              </button>
            )}
          </div>
        ))}
        <button
          type="button"
          onClick={onAddSlot}
          aria-label="Add slot"
          className={`rounded border border-dashed px-2.5 py-1 text-xs font-semibold transition ${accent.addTab}`}
        >
          +
        </button>
      </div>

      <div className="mt-3 min-h-[6rem]">
        {activeSlot && (
          <SlotEditor
            side={side}
            entities={activeSlot.entities}
            onChange={(entities) => {
              onChangeSlot(safeIndex, entities);
            }}
          />
        )}
      </div>
    </section>
  );
}
