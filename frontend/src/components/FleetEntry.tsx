// Fleet entry region — attacker and defender columns with multi-slot ACS.
//
// One of the three layout regions the app shell establishes. This is the
// fleet-entry surface from issue #23: two mirrored columns (attacker amber,
// defender ice-blue) each carrying their own slot tabs (A1/A2/+ , D1/D2/+),
// and each slot its own composition editor.
//
// The region is a controlled component: the fleet state lives in the App (the
// single owner of the request) and arrives as `value`/`onChange`. The only
// state kept here is which slot tab is active on each side — that is ephemeral
// UI state and does not belong in the request, so it does not travel up.
//
// It owns no technology or results logic. The technology-input region (#24)
// and the results region (#25) are siblings and stay untouched.

import { useState } from "react";
import { PartyColumn } from "@/components/fleet/PartyColumn";
import {
  emptySlot,
  nextSlotId,
  type FleetSlot,
  type FleetState,
  type Side,
} from "@/fleet/types";
import type { FleetComposition } from "@/api/types";

const SIDE_LABELS: Record<Side, string> = {
  attacker: "Attacker",
  defender: "Defender",
};

interface FleetEntryProps {
  readonly value: FleetState;
  readonly onChange: (state: FleetState) => void;
}

export function FleetEntry({ value, onChange }: FleetEntryProps) {
  const [active, setActive] = useState<Record<Side, number>>({
    attacker: 0,
    defender: 0,
  });

  const setActiveSlot = (side: Side, index: number): void => {
    setActive((prev) => ({ ...prev, [side]: index }));
  };

  const replace = (side: Side, slots: FleetSlot[]): void => {
    onChange({ ...value, [side]: slots });
  };

  const addSlot = (side: Side): void => {
    const current = value[side];
    replace(side, [...current, emptySlot(nextSlotId(side, current))]);
    setActiveSlot(side, current.length);
  };

  const removeSlot = (side: Side, index: number): void => {
    const current = value[side];
    if (current.length <= 1) return;
    const next = current.filter((_, i) => i !== index);
    replace(side, next);
    setActive((prev) => ({
      ...prev,
      [side]: Math.min(prev[side], next.length - 1),
    }));
  };

  const changeSlot = (
    side: Side,
    index: number,
    entities: FleetComposition,
  ): void => {
    const current = value[side];
    const next = current.map((slot, i) =>
      i === index ? { ...slot, entities } : slot,
    );
    replace(side, next);
  };

  return (
    <section aria-labelledby="fleet-entry-heading" className="space-y-3">
      <h2
        id="fleet-entry-heading"
        className="text-sm font-semibold uppercase tracking-wide text-slate-400"
      >
        Fleet entry
      </h2>

      <div className="grid gap-3 md:grid-cols-2">
        {(["attacker", "defender"] as const).map((side) => (
          <PartyColumn
            key={side}
            side={side}
            label={SIDE_LABELS[side]}
            slots={value[side]}
            activeIndex={active[side]}
            onSelectSlot={(index) => {
              setActiveSlot(side, index);
            }}
            onAddSlot={() => {
              addSlot(side);
            }}
            onRemoveSlot={(index) => {
              removeSlot(side, index);
            }}
            onChangeSlot={(index, entities) => {
              changeSlot(side, index, entities);
            }}
          />
        ))}
      </div>
    </section>
  );
}
