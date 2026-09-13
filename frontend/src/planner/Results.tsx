import type { SimulationResponse } from "@/api/types";
export interface Run {
  response: SimulationResponse;
  signature: string;
}
const format = (value: number) => value.toLocaleString("de-DE", { maximumFractionDigits: 0 });
export function Results({ run, dirty }: { readonly run: Run | null; readonly dirty: boolean }) {
  const results = run?.response.results;
  const economics = run?.response.report.economics;
  // The summary report truncates average fleet counts. Recover each run's
  // exact loss value from the engine identity: profit = debris + loot - losses.
  const total = (resources: { metal: number; crystal: number; deuterium: number }) =>
    resources.metal + resources.crystal + resources.deuterium;
  const loss = results
    ? results.results.reduce(
        (sum, result) =>
          sum + total(result.debris_field) + total(result.loot) - result.attacker_profit,
        0,
      ) / results.simulations
    : 0;
  return (
    <section className="results" aria-label="Ergebnis" aria-live="polite">
      <header>
        <span className="eyebrow">DEIN ERGEBNIS</span>
        <span>
          {dirty ? "Eingaben geändert · neu simulieren" : "Nur Simulieren startet eine Berechnung"}
        </span>
      </header>
      <div className="metrics">
        <div>
          <span>Siegchance</span>
          <strong>
            {results ? `${format((results.attacker_wins / results.simulations) * 100)} %` : "—"}
          </strong>
          <small>
            {results ? `${format(results.simulations)} Durchläufe` : "Bereit, wenn du es bist"}
          </small>
        </div>
        <div>
          <span>Deine Verluste Ø</span>
          <strong>{economics ? format(loss) : "—"}</strong>
          <small>Metall + Kristall + Deuterium</small>
        </div>
        <div>
          <span>Teilgewinn Ø</span>
          <strong>{economics ? format(economics.attacker_profit) : "—"}</strong>
          <small>Bei vollständigem TF-Abbau</small>
        </div>
      </div>
      {results && (
        <p>
          {format(results.attacker_wins)} Siege · {format(results.draws)} Unentschieden ·{" "}
          {format(results.defender_wins)} Niederlagen
        </p>
      )}
      {economics && (
        <p>
          Trümmerfeld: {format(economics.debris_field.metal)} Metall ·{" "}
          {format(economics.debris_field.crystal)} Kristall ·{" "}
          {format(economics.debris_field.deuterium)} Deuterium
        </p>
      )}
      <p className="scope-note">
        Teilgewinn = gesamtes Trümmerfeld − eigene Verluste (Metall, Kristall und Deuterium).
        Annahme: Du sammelst 100 % des TF ein. Ohne Beute, Treibstoff und Wiederaufbau; kein
        vollständiger Nettogewinn.
      </p>
    </section>
  );
}
