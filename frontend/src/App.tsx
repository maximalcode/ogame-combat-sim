// App shell: the layout skeleton and the only place that owns simulation
// state.
//
// Three regions — fleet entry, technology input, results — each live in their
// own component file and own no shared state. This component holds the single
// `ResultsState` and the request the placeholders will eventually populate;
// for now the request is a built-in demo so the shell can prove the typed
// client round-trips against a running API. The sibling issues replace the
// placeholders with real inputs and feed a real `CombatRequest` in.

import { useCallback, useState } from "react";
import { FleetEntry } from "@/components/FleetEntry";
import { TechnologyInput } from "@/components/TechnologyInput";
import {
  ResultsPanel,
  type ResultsState,
} from "@/components/ResultsPanel";
import { ApiError, postSimulate } from "@/api/client";
import type { CombatRequest } from "@/api/types";
import { API_BASE_URL, isSameOrigin } from "@/config";

/**
 * A minimal valid request, so the shell can prove the typed client round-trips.
 * Real input arrives in the sibling issues; this is not a results surface.
 */
const DEMO_REQUEST: CombatRequest = {
  attacker: {
    technology: { weapon: 10, shield: 10, armour: 10 },
    entities: { "206": 100 }, // 100 Cruisers
  },
  defender: {
    technology: { weapon: 8, shield: 8, armour: 8 },
    entities: { "204": 1000 }, // 1000 Light Fighters
  },
  use_rapid_fire: true,
  simulations: 100,
};

/**
 * The client throws `ApiError` for every failure mode it knows about; anything
 * else reaching the catch is a programmer error, which we still surface as a
 * visible state rather than letting it reject unhandled.
 */
function asApiError(error: unknown): ApiError {
  if (error instanceof ApiError) return error;
  const message = error instanceof Error ? error.message : String(error);
  return new ApiError(message, 0, "(client)");
}

export function App() {
  const [results, setResults] = useState<ResultsState>({ kind: "idle" });

  const runSimulation = useCallback(async () => {
    setResults({ kind: "loading" });
    try {
      await postSimulate(DEMO_REQUEST);
      // The response is typed end to end inside the client; the shell only
      // needs to know the call succeeded. Rendering the body is the sibling
      // issue's job.
      setResults({ kind: "ok" });
    } catch (error) {
      setResults({ kind: "error", error: asApiError(error) });
    }
  }, []);

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
        <div className="grid gap-4 lg:grid-cols-2">
          <FleetEntry />
          <TechnologyInput />
        </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => {
              void runSimulation();
            }}
            disabled={results.kind === "loading"}
            className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {results.kind === "loading" ? "Simulating…" : "Simulate"}
          </button>
          <p className="text-xs text-slate-500">
            Runs the demo request (100 Cruisers vs 1000 Light Fighters) against
            the configured API, proving the typed client round-trips. Real
            inputs and results rendering arrive in the sibling issues.
          </p>
        </div>

        <ResultsPanel state={results} />
      </main>
    </div>
  );
}
