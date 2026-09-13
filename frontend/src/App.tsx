import { useRef, useState } from "react";
import { postSimulate } from "@/api/client";
import { FleetPanel } from "@/planner/FleetPanel";
import { NumberField } from "@/planner/NumberField";
import { emptyScenario, makeRequest } from "@/planner/model";
import { Results, type Run } from "@/planner/Results";
import "@/planner/planner.css";

export function App() {
  const [scenario, setScenario] = useState(emptyScenario);
  const [simulations, setSimulations] = useState(100);
  const [run, setRun] = useState<Run | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const running = useRef(false);
  const signature = JSON.stringify({ scenario, simulations });
  const empty = [scenario.attacker, scenario.defender].some(
    (fleet) => !fleet.units.some((unit) => Number(unit.selected) > 0),
  );
  async function simulate() {
    if (running.current || empty) return;
    running.current = true;
    setBusy(true);
    setError("");
    try {
      const response = await postSimulate(makeRequest(scenario, simulations));
      setRun({ response, signature });
    } catch {
      setError(
        "Die Simulation ist fehlgeschlagen. Deine Eingaben und das letzte erfolgreiche Ergebnis bleiben erhalten. Bitte erneut versuchen.",
      );
    } finally {
      running.current = false;
      setBusy(false);
    }
  }
  return (
    <div className="planner">
      <header className="app-header">
        <span className="brand">
          ◈ ORBIT <small>Kampfsimulator</small>
        </span>
        <span>Manueller Angriffsplaner</span>
      </header>
      <main>
        <div className="workspace-title">
          <span className="eyebrow">FLOTTENPLANUNG / DOCK</span>
          <h1>Bereit für den Angriff.</h1>
          <p>Bestand ergänzen. Flotte wählen. Chancen prüfen.</p>
        </div>
        <div className="fleet-pair">
          <FleetPanel
            attacker
            fleet={scenario.attacker}
            onChange={(attacker) => {
              setScenario({ ...scenario, attacker });
            }}
          />
          <FleetPanel
            attacker={false}
            fleet={scenario.defender}
            onChange={(defender) => {
              setScenario({ ...scenario, defender });
            }}
          />
        </div>
        <details className="universe">
          <summary>Universum & Kampfregeln</summary>
          <div className="universe-fields">
            <NumberField
              label="Flotte ins Trümmerfeld %"
              value={scenario.debrisFleet}
              max={100}
              onChange={(debrisFleet) => {
                setScenario({ ...scenario, debrisFleet });
              }}
            />
            <NumberField
              label="Verteidigung ins Trümmerfeld %"
              value={scenario.debrisDefence}
              max={100}
              onChange={(debrisDefence) => {
                setScenario({ ...scenario, debrisDefence });
              }}
            />
            <label>
              <input
                type="checkbox"
                checked={scenario.deuterium}
                onChange={(event) => {
                  setScenario({ ...scenario, deuterium: event.target.checked });
                }}
              />{" "}
              Deuterium im TF
            </label>
            <label>
              <input
                type="checkbox"
                checked={scenario.rapidFire}
                onChange={(event) => {
                  setScenario({ ...scenario, rapidFire: event.target.checked });
                }}
              />{" "}
              Rapidfire
            </label>
          </div>
        </details>
        <p className="assumption-note">
          Leere Zahlenfelder sind rot markiert: Assumed zero (angenommen: 0). Erst das
          Angriffsszenario rechnet mit 0; dies ist keine verifizierte Rekonstruktion.
        </p>
        <div className="launch-deck">
          <Results run={run} dirty={run !== null && run.signature !== signature} />
          <div className="controls">
            <span className="eyebrow">DURCHLÄUFE</span>
            <div className="run-options">
              {[100, 1000].map((count) => (
                <button
                  type="button"
                  key={count}
                  aria-pressed={simulations === count}
                  onClick={() => {
                    setSimulations(count);
                  }}
                >
                  {count.toLocaleString("de-DE")}
                </button>
              ))}
            </div>
            {empty && <small>Auf beiden Seiten werden Einheiten benötigt.</small>}
            <button
              type="button"
              className="simulate"
              disabled={busy || empty}
              onClick={() => {
                void simulate();
              }}
            >
              {busy ? "Wird berechnet…" : "Simulieren"}
            </button>
          </div>
        </div>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <footer>
          Available fleet: manuell gelieferter Snapshot · Selected attacking fleet: deine Auswahl
          daraus. Kein Live-Inventar.
          <br />
          Inoffizielles Fan-Tool. Eigene schematische Flottenillustration; keine
          Gameforge-Bilddateien.
        </footer>
      </main>
    </div>
  );
}
