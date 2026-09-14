import { useEffect, useRef, useState } from "react";
import { postSimulate } from "@/api/client";
import { FleetPanel } from "@/planner/FleetPanel";
import { NumberField } from "@/planner/NumberField";
import { emptyScenario, makeRequest } from "@/planner/model";
import { Results } from "@/planner/Results";
import { averages, type Attempts } from "@/planner/attempts";
import "@/planner/planner.css";
import { ReportImport } from "@/planner/ReportImport";
import { readAgr } from "@/planner/agr";

export function App() {
  const [scenario, setScenario] = useState(emptyScenario);
  const [simulations, setSimulations] = useState(100);
  const [attempts, setAttempts] = useState<Attempts>({ latest: null, previous: null });
  const nextAttempt = useRef(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const running = useRef(false);
  const [importError, setImportError] = useState("");
  const [sources, setSources] = useState<string[]>([]);
  const currentScenario = useRef(scenario);
  currentScenario.current = scenario;
  useEffect(() => {
    const receive = () => {
      try {
        const imported = readAgr(window.location.hash, currentScenario.current);
        if (imported) {
          currentScenario.current = imported.scenario;
          setScenario(imported.scenario);
          setSources(imported.sources);
          setImportError("");
        }
      } catch (cause) {
        setImportError(cause instanceof Error ? cause.message : "AGR-Übergabe fehlgeschlagen.");
      }
    };
    receive();
    window.addEventListener("hashchange", receive);
    return () => {
      window.removeEventListener("hashchange", receive);
    };
  }, []);
  const signature = JSON.stringify({ scenario, simulations });
  const empty = [scenario.attacker, scenario.defender].some(
    (fleet) => !fleet.units.some((unit) => Number(unit.selected) > 0),
  );
  async function simulate() {
    if (running.current || empty) return;
    running.current = true;
    setBusy(true);
    setError("");
    const job = { number: ++nextAttempt.current, request: makeRequest(scenario, simulations), signature };
    try {
      const response = await postSimulate(job.request);
      averages(response);
      setAttempts((previous) => ({ latest: { ...job, response }, previous: previous.latest }));
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
        <span>Angriffsplaner</span>
      </header>
      <main>
        <div className="workspace-title">
          <span className="eyebrow">FLOTTENPLANUNG / DOCK</span>
          <h1>Bereit für den Angriff.</h1>
          <p>Bestand ergänzen. Flotte wählen. Chancen prüfen.</p>
        </div>
        {importError && (
          <p role="alert" className="error">
            {importError}
          </p>
        )}
        {sources.length > 0 && (
          <details className="agr-source">
            <summary>AGR übernommen · Quellen der Übergabe</summary>
            <p>
              Importierte Ausgangswerte; spätere Änderungen sind manuell. Fehlende Zahlen bleiben
              leer. Fehlende Klassen: keine Klasse angenommen.
            </p>
            <ul>
              {sources.map((source, index) => (
                <li key={`${String(index)}-${source}`}>{source}</li>
              ))}
            </ul>
          </details>
        )}
        <ReportImport defender={scenario.defender} onImport={(defender) => {
          setScenario((previous) => ({ ...previous, defender }));
        }} />
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
          <Results attempts={attempts} signature={signature} />
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
          Available fleet: gelieferter Snapshot · Selected attacking fleet: deine Auswahl daraus.
          Kein Live-Inventar.
          <br />
          Inoffizielles Fan-Tool. Eigene schematische Flottenillustration; keine
          Gameforge-Bilddateien.
        </footer>
      </main>
    </div>
  );
}
