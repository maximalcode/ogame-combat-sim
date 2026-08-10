// App shell: the layout skeleton and the only place that owns simulation
// state.
//
// Three regions — fleet entry, technology input, results — each live in their
// own component file and own no shared state. This component holds the fleet
// state (issue #23) and the `ResultsState` the request produces; the request is
// built from the fleet state, so the Simulate button runs what the user
// composed rather than a hardcoded demo. Technology levels are still a
// placeholder (issue #24) — `buildCombatRequest` fights every party at level 0
// until that lands — and results rendering is still a placeholder (issue #25).

import { useCallback, useState } from "react";
import { FleetEntry } from "@/components/FleetEntry";
import { TechnologyInput } from "@/components/TechnologyInput";
import {
  ResultsPanel,
  type ResultsState,
} from "@/components/ResultsPanel";
import { ApiError, postSimulate } from "@/api/client";
import {
  buildCombatRequest,
  emptySlot,
  isSideEmpty,
  type FleetState,
} from "@/fleet/types";
import { API_BASE_URL, isSameOrigin } from "@/config";

/**
 * The fleet the shell opens with — the demo matchup the shell already shipped
 * (100 Cruisers vs 1000 Light Fighters), now expressed as one slot per side.
 * One slot means the simple party shape, so this still round-trips the same
 * request the shell proved; adding a slot on either side switches to the
 * multi-slot shape without the App knowing or caring.
 */
const INITIAL_FLEET: FleetState = {
  attacker: [{ ...emptySlot("A1"), entities: { "206": 100 } }],
  defender: [{ ...emptySlot("D1"), entities: { "204": 1000 } }],
};

export function App() {
  const [fleet, setFleet] = useState<FleetState>(INITIAL_FLEET);
  const [results, setResults] = useState<ResultsState>({ kind: "idle" });

  const attackerEmpty = isSideEmpty(fleet.attacker);
  const defenderEmpty = isSideEmpty(fleet.defender);
  const emptySide = attackerEmpty || defenderEmpty;

  const runSimulation = useCallback(async () => {
    if (emptySide) return;
    setResults({ kind: "loading" });
    try {
      await postSimulate(buildCombatRequest(fleet));
      // The response is typed end to end inside the client; the shell only
      // needs to know the call succeeded. Rendering the body is the sibling
      // issue's job.
      setResults({ kind: "ok" });
    } catch (error) {
      // The client throws ApiError for every failure mode; anything else is a
      // programmer error, which we still surface rather than letting reject.
      const message =
        error instanceof Error ? error.message : String(error);
      const apiError =
        error instanceof ApiError ? error : new ApiError(message, 0, "(client)");
      setResults({ kind: "error", error: apiError });
    }
  }, [emptySide, fleet]);

  let emptyMessage = "The defender fleet is empty";
  if (attackerEmpty && defenderEmpty) {
    emptyMessage = "Both fleets are empty";
  } else if (attackerEmpty) {
    emptyMessage = "The attacker fleet is empty";
  }

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <header className="border-b border-slate-800 bg-slate-900/60">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-4 py-3">
          <h1 className="text-lg font-semibold">OGame Combat Simulator</h1>
          <span className="font-mono text-xs text-slate-500">
            API: {isSameOrigin ? "same-origin" : API_BASE_URL}
          </span>
        </div>
      </header>

      <main className="mx-auto max-w-5xl space-y-4 px-4 py-6">
        <FleetEntry value={fleet} onChange={setFleet} />
        <TechnologyInput />

        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => {
                void runSimulation();
              }}
              disabled={results.kind === "loading" || emptySide}
              className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {results.kind === "loading" ? "Simulating…" : "Simulate"}
            </button>
            <p className="text-xs text-slate-500">
              Runs the composed fleet against the configured API. Technology
              levels and results rendering arrive in the sibling issues.
            </p>
          </div>
          {emptySide && (
            <p className="text-xs text-amber-400" role="status">
              {emptyMessage} — add at least one ship before simulating.
            </p>
          )}
        </div>

        <ResultsPanel state={results} />
      </main>
    </div>
  );
}
