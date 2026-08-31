// App shell: the layout skeleton and the only place that owns simulation
// state.
//
// Three regions — fleet entry, technology input, results — each live in their
// own component file and own no shared state. This component holds the fleet
// state (issue #23) and both result requests. The normal request stays lean;
// one representative battle asks for per-round compositions only when that
// view opens.

import { useCallback, useState } from "react";
import { FleetEntry } from "@/components/FleetEntry";
import { TechnologyInput } from "@/components/TechnologyInput";
import {
  ResultsPanel,
  type ResultsState,
  type ResultsView,
} from "@/components/ResultsPanel";
import type { RoundResultsState } from "@/components/results/RoundCompositionView";
import { ApiError, postSimulate } from "@/api/client";
import type { CombatRequest } from "@/api/types";
import {
  buildCombatRequest,
  emptySlot,
  isSideEmpty,
  type FleetState,
} from "@/fleet/types";
import { API_BASE_URL, isSameOrigin } from "@/config";
import { DEFAULT_COMBAT_INPUT } from "@/combat/input";

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

function asApiError(error: unknown): ApiError {
  const message = error instanceof Error ? error.message : String(error);
  return error instanceof ApiError ? error : new ApiError(message, 0, "(client)");
}

export function App() {
  const [fleet, setFleet] = useState<FleetState>(INITIAL_FLEET);
  const [combatInput, setCombatInput] = useState(DEFAULT_COMBAT_INPUT);
  const [results, setResults] = useState<ResultsState>({ kind: "idle" });
  const [resultsView, setResultsView] = useState<ResultsView>("summary");
  const [roundResults, setRoundResults] = useState<RoundResultsState>({ kind: "idle" });

  const attackerEmpty = isSideEmpty(fleet.attacker);
  const defenderEmpty = isSideEmpty(fleet.defender);
  const emptySide = attackerEmpty || defenderEmpty;

  const runSimulation = useCallback(async () => {
    if (emptySide) return;
    const request = buildCombatRequest(fleet, combatInput);
    setResults({ kind: "loading" });
    setResultsView("summary");
    setRoundResults({ kind: "idle" });
    try {
      const response = await postSimulate(request);
      setResults({ kind: "ok", response, request });
    } catch (error) {
      setResults({ kind: "error", error: asApiError(error) });
    }
  }, [combatInput, emptySide, fleet]);

  const runRoundSimulation = useCallback(async (request: CombatRequest) => {
    setRoundResults({ kind: "loading" });
    try {
      const response = await postSimulate({
        ...request,
        simulations: 1,
        enable_round_compositions: true,
      });
      setRoundResults({ kind: "ok", response });
    } catch (error) {
      setRoundResults({ kind: "error", error: asApiError(error) });
    }
  }, []);

  const selectRoundView = useCallback(() => {
    setResultsView("rounds");
    if (
      results.kind === "ok" &&
      roundResults.kind !== "loading" &&
      roundResults.kind !== "ok"
    ) {
      void runRoundSimulation(results.request);
    }
  }, [results, roundResults.kind, runRoundSimulation]);

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
        <TechnologyInput value={combatInput} onChange={setCombatInput} />

        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => {
                void runSimulation();
              }}
              disabled={
                results.kind === "loading" || roundResults.kind === "loading" || emptySide
              }
              className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {results.kind === "loading" ? "Simulating…" : "Simulate"}
            </button>
            <p className="text-xs text-slate-500">
              Runs 100 battles for a distribution. Round compositions are requested separately, only when opened.
            </p>
          </div>
          {emptySide && (
            <p className="text-xs text-amber-400" role="status">
              {emptyMessage} — add at least one ship before simulating.
            </p>
          )}
        </div>

        <ResultsPanel
          state={results}
          view={resultsView}
          roundState={roundResults}
          onSelectSummary={() => {
            setResultsView("summary");
          }}
          onSelectRounds={selectRoundView}
          onRetryRounds={selectRoundView}
        />
      </main>
    </div>
  );
}
