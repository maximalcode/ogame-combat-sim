// Results region — placeholder.
//
// One of the three layout regions the app shell establishes. It is deliberately
// an empty placeholder: actual results visualisation (recharts, per-round
// breakdown, profit tables) is a sibling issue and slots in here without
// touching the other two regions. See issue #22 for the seam this exists to
// provide.
//
// What it does carry in this issue is the *state* of the typed end-to-end call
// — idle, loading, error, or ok — because surfacing API errors as a visible
// state is an acceptance criterion and the shell has to prove the client
// round-trips. The ok state renders a single status line, not a results
// surface: anything that reads the response body belongs to the sibling issue.
//
// Errors surface here as a visible state, never as an unhandled rejection —
// the API client throws `ApiError`, which the App catches and passes down as
// `state.kind === "error"`.

import type { ApiError } from "@/api/client";

/** The states the results region can be in. Owned by the App, rendered here. */
export type ResultsState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; error: ApiError }
  | { kind: "ok" };

interface ResultsPanelProps {
  readonly state: ResultsState;
}

export function ResultsPanel({ state }: ResultsPanelProps) {
  return (
    <section
      aria-labelledby="results-heading"
      className="rounded-lg border border-slate-800 bg-slate-900/40 p-4"
    >
      <h2
        id="results-heading"
        className="text-sm font-semibold uppercase tracking-wide text-slate-400"
      >
        Results
      </h2>

      <div className="mt-3 flex min-h-[8rem] items-center justify-center rounded border border-dashed border-slate-700 p-4 text-center text-sm">
        {state.kind === "idle" && (
          <p className="text-slate-500">
            Run a simulation to see results here.
            <br />
            <span className="text-slate-600">
              Placeholder — implemented in a sibling issue.
            </span>
          </p>
        )}

        {state.kind === "loading" && (
          <p className="text-slate-400" aria-live="polite">
            Simulating…
          </p>
        )}

        {state.kind === "error" && (
          <div
            role="alert"
            className="rounded border border-red-800 bg-red-950/40 p-3 text-left text-sm text-red-200"
          >
            <p className="font-semibold">Request failed</p>
            <p className="mt-1 break-words">{state.error.message}</p>
            {state.error.status > 0 && (
              <p className="mt-1 text-xs text-red-300/80">
                HTTP {state.error.status} on {state.error.endpoint}
              </p>
            )}
          </div>
        )}

        {state.kind === "ok" && (
          <p className="text-slate-400">
            Simulation completed — response received and typed.
            <br />
            <span className="text-slate-600">
              Results rendering is a sibling issue.
            </span>
          </p>
        )}
      </div>
    </section>
  );
}
