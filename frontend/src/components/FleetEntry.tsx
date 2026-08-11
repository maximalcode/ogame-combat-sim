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
} from "@/fleet/types";
import type { FleetComposition } from "@/api/types";

interface FleetEntryProps {
  readonly value: FleetState;
  readonly onChange: (state: FleetState) => void;
}

export function FleetEntry({ value, onChange }: FleetEntryProps) {
  const [activeAttacker, setActiveAttacker] = useState(0);
  const [activeDefender, setActiveDefender] = useState(0);

  const replace = (side: "attacker" | "defender", slots: FleetSlot[]): void => {
    onChange({ ...value, [side]: slots });
  };

  const addSlot = (side: "attacker" | "defender"): void => {
    const current = value[side];
    const slot = emptySlot(nextSlotId(side === "attacker" ? "A" : "D", current));
    replace(side, [...current, slot]);
    if (side === "attacker") setActiveAttacker(current.length);
    else setActiveDefender(current.length);
  };

  const removeSlot = (side: "attacker" | "defender", index: number): void => {
    const current = value[side];
    if (current.length <= 1) return;
    const next = current.filter((_, i) => i !== index);
    replace(side, next);
    if (side === "attacker") {
      setActiveAttacker((prev) => Math.min(prev, next.length - 1));
    } else {
      setActiveDefender((prev) => Math.min(prev, next.length - 1));
    }
  };

  const changeSlot = (
    side: "attacker" | "defender",
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
        <PartyColumn
          side="attacker"
          label="Attacker"
          slots={value.attacker}
          activeIndex={activeAttacker}
          onSelectSlot={setActiveAttacker}
          onAddSlot={() => {
            addSlot("attacker");
          }}
          onRemoveSlot={(index) => {
            removeSlot("attacker", index);
          }}
          onChangeSlot={(index, entities) => {
            changeSlot("attacker", index, entities);
          }}
        />
        <PartyColumn
          side="defender"
          label="Defender"
          slots={value.defender}
          activeIndex={activeDefender}
          onSelectSlot={setActiveDefender}
          onAddSlot={() => {
            addSlot("defender");
          }}
          onRemoveSlot={(index) => {
            removeSlot("defender", index);
          }}
          onChangeSlot={(index, entities) => {
            changeSlot("defender", index, entities);
          }}
        />
      </div>
    </section>
  );
}
