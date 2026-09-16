import { validateAndAverage, format, signed, type Attempts, type Run } from "@/planner/attempts";
import type { AllianceClass, PlayerClass } from "@/api/types";
import { unitName } from "@/planner/catalog";

const classNames: Record<PlayerClass | AllianceClass, string> = {
  none: "Keine", collector: "Kollektor", general: "General", discoverer: "Entdecker",
  trader: "Händler", warrior: "Krieger", researcher: "Forscher",
};

function Inputs({ run }: { readonly run: Run }) {
  return (
    <details className="attempt-inputs">
      <summary>Eingaben von Versuch {run.number}</summary>
      <p>Gespeicherter Stand beim Start · {format(run.request.simulations)} Durchläufe angefordert.</p>
      {(["attacker", "defender"] as const).map((side) => (
        <p key={side}>
          {side === "attacker" ? "Deine Auswahl" : "Gegner"}: {Object.entries(run.request[side].entities)
            .map(([id, count]) => `${unitName(id)}: ${format(count)}`).join(" · ")}
          <br />
          Waffen / Schilde / Panzerung: {run.request[side].technology.weapon} /{" "}
          {run.request[side].technology.shield} / {run.request[side].technology.armour}
          <br />
          Spielerklasse: {classNames[run.request[`${side}_bonuses`]?.player_class ?? "none"]} ·{" "}
          Allianzklasse: {classNames[run.request[`${side}_bonuses`]?.alliance_class ?? "none"]}
          <br />
          Lebensformboni (Waffen / Schilde / Panzerung in %): {Object.entries(run.request[side].lifeform ?? {})
            .map(([id, bonus]) => `${unitName(id)}: ${format(bonus.weapon ?? 0)} / ${format(bonus.shield ?? 0)} / ${format(bonus.armour ?? 0)}`).join(" · ")}
        </p>
      ))}
      <p>
        Flotte ins TF: {format(run.request.universe_settings?.debris_fleet ?? 0)} % ·{" "}
        Verteidigung ins TF: {format(run.request.universe_settings?.debris_defence ?? 0)} % ·{" "}
        Deuterium im TF: {run.request.universe_settings?.debris_deuterium ? "Ja" : "Nein"} ·{" "}
        Rapidfire: {run.request.use_rapid_fire ? "Ja" : "Nein"}
      </p>
      <p>Leere Zahlen wurden für diesen Versuch als 0 angenommen.</p>
    </details>
  );
}

function Comparison({ latest, previous }: { readonly latest: Run; readonly previous: Run }) {
  return (
    <div className="attempt-comparison">
      <table aria-label="Versuchsvergleich">
        <caption>Letzte zwei erfolgreiche Versuche · Ressourcen: Metall + Kristall + Deuterium</caption>
        <thead><tr><th>Versuch</th><th>Deine Verluste Ø</th><th>Teilgewinn Ø</th></tr></thead>
        <tbody>
          {[latest, previous].map((run, index) => {
            const average = validateAndAverage(run.response);
            return (
              <tr key={run.number}>
                <th scope="row">{index === 0 ? "Letzter" : "Vorheriger"} · Versuch {run.number}</th>
                <td>{format(average.losses)}</td>
                <td className={average.profit < 0 ? "negative" : "positive"}>{signed(average.profit)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <Inputs run={previous} />
    </div>
  );
}

export function Results({ attempts, signature }: { readonly attempts: Attempts; readonly signature: string }) {
  const { latest: run, previous } = attempts;
  const results = run?.response.results;
  const average = run ? validateAndAverage(run.response) : null;
  const dirty = run !== null && run.signature !== signature;
  const currentLabel = run ? "Passend zu den aktuellen Eingaben" : "Nur Simulieren startet eine Berechnung";
  return (
    <section className="results" aria-label="Ergebnis" aria-live="polite">
      <header>
        <span className="eyebrow">{run ? `VERSUCH ${String(run.number)}` : "DEIN ERGEBNIS"}</span>
        <span className={dirty ? "stale" : undefined}>
          {dirty ? "Veraltet · Eingaben geändert · neu simulieren" : currentLabel}
        </span>
      </header>
      <div className="metrics">
        <div>
          <span>Siegchance</span>
          <strong>{results ? `${format((results.attacker_wins / results.simulations) * 100)} %` : "—"}</strong>
          <small>{results ? `${format(results.simulations)} Durchläufe` : "Bereit, wenn du es bist"}</small>
        </div>
        <div>
          <span>Deine Verluste Ø</span>
          <strong>{average ? format(average.losses) : "—"}</strong>
          <small>Metall + Kristall + Deuterium</small>
        </div>
        <div>
          <span>Teilgewinn Ø</span>
          <strong className={average && average.profit < 0 ? "negative" : "positive"}>{average ? signed(average.profit) : "—"}</strong>
          <small>Bei vollständigem TF-Abbau</small>
        </div>
      </div>
      {results && <p>
        Häufigkeit in dieser Stichprobe, keine garantierte Siegchance: {format(results.attacker_wins)} Siege ·{" "}
        {format(results.draws)} Unentschieden · {format(results.defender_wins)} Niederlagen
      </p>}
      {average && <p>
        Trümmerfeld Ø: {format(average.metal)} Metall · {format(average.crystal)} Kristall ·{" "}
        {format(average.deuterium)} Deuterium
      </p>}
      <p className="scope-note">
        Teilgewinn = gesamtes Trümmerfeld − eigene Verluste (Metall, Kristall und Deuterium).
        Annahme: Du sammelst 100 % des TF ein. Ohne Beute, Treibstoff und Wiederaufbau; kein
        vollständiger Nettogewinn. Positive Beträge: Überschuss; negative Beträge: Verlust.
      </p>
      {run && <Inputs run={run} />}
      {run && previous && <Comparison latest={run} previous={previous} />}
    </section>
  );
}
