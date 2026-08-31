// Results region: aggregate distribution and economics plus an opt-in,
// representative per-round battle.
//
// The normal response is an average over many simulations. Round composition
// is intentionally fetched separately only when its tab opens, because the
// engine has to retain per-round, per-ship snapshots to produce it.

import type { ApiError } from "@/api/client";
import type { CombatRequest, SimulationResponse } from "@/api/types";
import { EconomicsSummary } from "@/components/results/EconomicsSummary";
import { LossesTable } from "@/components/results/LossesTable";
import { OutcomeDistribution } from "@/components/results/OutcomeDistribution";
import {
  RoundCompositionView,
  type RoundResultsState,
} from "@/components/results/RoundCompositionView";

export type ResultsState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; error: ApiError }
  | { kind: "ok"; response: SimulationResponse; request: CombatRequest };

export type ResultsView = "summary" | "rounds";

interface ResultsPanelProps {
  readonly state: ResultsState;
  readonly view: ResultsView;
  readonly roundState: RoundResultsState;
  readonly onSelectSummary: () => void;
  readonly onSelectRounds: () => void;
  readonly onRetryRounds: () => void;
}

function RequestError({ error }: { readonly error: ApiError }) {
  return (
    <div
      role="alert"
      className="rounded border border-red-800 bg-red-950/40 p-4 text-left text-sm text-red-200"
    >
      <p className="font-semibold">Request failed</p>
      <p className="mt-1 break-words">{error.message}</p>
      {error.status > 0 && (
        <p className="mt-1 text-xs text-red-300/80">
          HTTP {String(error.status)} on {error.endpoint}
        </p>
      )}
    </div>
  );
}

function LoadingState() {
  return (
    <div className="flex min-h-52 items-center justify-center rounded border border-dashed border-slate-700 p-5 text-center">
      <div>
        <div className="mx-auto h-7 w-7 animate-spin rounded-full border-2 border-slate-700 border-t-indigo-400" />
        <p className="mt-3 text-sm text-slate-300" aria-live="polite">
          Simulating the outcome distribution…
        </p>
        <p className="mt-1 text-xs text-slate-600">Previous results are cleared while this fleet runs.</p>
      </div>
    </div>
  );
}

function SummaryView({ response }: { readonly response: SimulationResponse }) {
  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-indigo-900/60 bg-indigo-950/20 px-4 py-3">
        <p className="text-sm text-indigo-100">
          <strong>Distribution, not verdict.</strong>{" "}
          Rates describe how often each outcome occurred; the profit-by-outcome table shows what
          those wins, draws, and losses cost.
        </p>
      </div>
      <OutcomeDistribution response={response} />
      <LossesTable response={response} />
      <EconomicsSummary response={response} />
    </div>
  );
}

export function ResultsPanel({
  state,
  view,
  roundState,
  onSelectSummary,
  onSelectRounds,
  onRetryRounds,
}: ResultsPanelProps) {
  return (
    <section
      aria-labelledby="results-heading"
      className="min-w-0 rounded-lg border border-slate-800 bg-slate-900/40 p-4"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2
          id="results-heading"
          className="text-sm font-semibold uppercase tracking-wide text-slate-400"
        >
          Results
        </h2>

        {state.kind === "ok" && (
          <div className="flex rounded-md border border-slate-700 bg-slate-950 p-0.5" role="tablist">
            <button
              type="button"
              role="tab"
              aria-selected={view === "summary"}
              onClick={onSelectSummary}
              className={`rounded px-3 py-1.5 text-xs font-medium transition ${
                view === "summary"
                  ? "bg-slate-700 text-slate-100"
                  : "text-slate-500 hover:text-slate-300"
              }`}
            >
              Aggregate
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={view === "rounds"}
              onClick={onSelectRounds}
              className={`rounded px-3 py-1.5 text-xs font-medium transition ${
                view === "rounds"
                  ? "bg-slate-700 text-slate-100"
                  : "text-slate-500 hover:text-slate-300"
              }`}
            >
              Round detail
            </button>
          </div>
        )}
      </div>

      <div className="mt-3">
        {state.kind === "idle" && (
          <div className="flex min-h-40 items-center justify-center rounded border border-dashed border-slate-700 p-5 text-center text-sm text-slate-500">
            Run a simulation to see outcome rates, costs, losses, debris, loot, and profit.
          </div>
        )}
        {state.kind === "loading" && <LoadingState />}
        {state.kind === "error" && <RequestError error={state.error} />}
        {state.kind === "ok" && view === "summary" && (
          <SummaryView response={state.response} />
        )}
        {state.kind === "ok" && view === "rounds" && (
          <RoundCompositionView state={roundState} onRetry={onRetryRounds} />
        )}
      </div>
    </section>
  );
}
