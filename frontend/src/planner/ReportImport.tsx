import { useEffect, useRef, useState } from "react";
import { importReport, reportFleet, takeReportKey, type ReportCandidate } from "./report";
import type { Fleet } from "./model";

export function ReportImport({
  defender,
  onImport,
}: {
  readonly defender: Fleet;
  readonly onImport: (fleet: Fleet) => void;
}) {
  const [key, setKey] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [candidate, setCandidate] = useState<ReportCandidate | null>(null);
  const pending = useRef<AbortController>();
  useEffect(() => {
    const receive = () => {
      const supplied = takeReportKey();
      if (supplied !== undefined) {
        pending.current?.abort();
        setBusy(false);
        setKey(supplied);
      }
    };
    receive();
    window.addEventListener("hashchange", receive);
    return () => {
      window.removeEventListener("hashchange", receive);
      pending.current?.abort();
    };
  }, []);
  useEffect(() => {
    pending.current?.abort();
    setBusy(false);
  }, [defender]);
  async function load() {
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    setBusy(true);
    setError("");
    try {
      const result = await importReport(key.trim(), controller.signal);
      const fleet = reportFleet(result);
      if (controller.signal.aborted) return;
      setCandidate(result);
      setKey("");
      onImport(fleet);
    } catch (cause) {
      if (!controller.signal.aborted)
        setError(
          cause instanceof Error ? cause.message : "Import fehlgeschlagen. Bitte erneut versuchen.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  return (
    <section className="report-import" aria-label="Gegnerbericht importieren">
      <h2>Gegnerbericht laden</h2>
      <p>
        Mit „Bericht übertragen & laden“ sendest du diesen Schlüssel über unseren Server an den
        Community-Proxy ogapi.faw-kes.de. Der Proxy kann Berichte zwischenspeichern. Diese Anwendung
        speichert Schlüssel und Rohberichte nicht dauerhaft. Nur der Gegner wird ersetzt.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void load();
        }}
      >
        <label>
          Berichtsschlüssel (sr- oder cr-)
          <input
            type="password"
            autoComplete="off"
            maxLength={80}
            value={key}
            onChange={(event) => {
              pending.current?.abort();
              setBusy(false);
              setKey(event.target.value);
            }}
          />
        </label>
        <button type="submit" disabled={!key.trim() || busy}>
          Bericht übertragen & laden
        </button>
        {busy && <span role="status">Bericht wird geladen…</span>}
      </form>
      {error && (
        <p role="alert" className="error">
          {error} Deine Planung und Ergebnisse bleiben erhalten.
        </p>
      )}
      {candidate && (
        <details className="report-source">
          <summary>Gegner übernommen · Herkunft & Annahmen prüfen</summary>
          <p>
            Quelle: {candidate.provenance.source} · {candidate.provenance.community} / Universum{" "}
            {candidate.provenance.universe}
            {" · Zeitpunkt: "}
            {candidate.provenance.event_timestamp === null
              ? "unbekannt"
              : new Date(candidate.provenance.event_timestamp * 1000).toLocaleString("de-DE")}
            {" · Spielversion: "}
            {candidate.provenance.game_version ?? "unbekannt"}
          </p>
          <p>
            Import candidate: ursprüngliche, bereinigte Berichtsevidenz. Spätere Änderungen sind
            manuell. Verborgene Flotten und Verteidigungen bleiben unbekannt; ergänze sie bei
            Bedarf. Im Angriffsszenario werden nicht ergänzte Einheiten als nicht beteiligt
            angenommen. Unklare Kampfforschung und Lebensformwerte bleiben leer (Assumed zero).
            Bekannte Spieler- und Allianzklassen werden übernommen. Fehlende Klassen gelten als
            „keine“; bitte prüfen. Zusammengesetzte Booster werden nicht als Lebensformprozente
            verwendet.
          </p>
          <p>
            Universum: Die bisherigen Werte unter „Universum & Kampfregeln“ bleiben in Verwendung;
            sie wurden nicht aus diesem Bericht ermittelt. Leere TF-Anteile rechnen als 0. Bitte mit
            dem genannten Universum abgleichen. Dies ist kein Verified battle input.
          </p>
          <details>
            <summary>Bereinigte Importdaten (unverändert)</summary>
            <pre>{JSON.stringify(candidate, null, 2)}</pre>
          </details>
        </details>
      )}
    </section>
  );
}
