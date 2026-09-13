import { apiUrl } from "@/config";
import { availableTypes } from "./catalog";
import { blankNumbers, emptyScenario, type Fleet } from "./model";

interface Participant {
  character_class_id: number | null;
  alliance_class_id: number | null;
  entities: Record<string, number> | null;
  ships: Record<string, number> | null;
  defenses: Record<string, number> | null;
  technology: {
    basis: string;
    weapon: number | null;
    shield: number | null;
    armour: number | null;
  };
}
export interface ReportCandidate {
  report_kind: "combat" | "espionage";
  provenance: {
    source: string;
    community: string;
    universe: number;
    event_timestamp: number | null;
    game_version: string | null;
  };
  defenders: Participant[];
}
/** Consume a deliberately supplied key without leaving it in the address or history entry. */
export function takeReportKey(): string | undefined {
  const url = new URL(window.location.href);
  const fragment = new URLSearchParams(url.hash.slice(1));
  const key = url.searchParams.get("SR_KEY") ?? fragment.get("SR_KEY");
  if (key === null) return undefined;
  url.searchParams.delete("SR_KEY");
  fragment.delete("SR_KEY");
  if (url.hash.includes("SR_KEY=")) url.hash = fragment.toString();
  window.history.replaceState(null, "", url);
  return key;
}
export async function importReport(key: string, signal: AbortSignal): Promise<ReportCandidate> {
  const response = await fetch(apiUrl("/api/reports/import"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ key, consent: true }),
    signal,
    cache: "no-store",
    referrerPolicy: "no-referrer",
  }).catch(() => {
    throw new Error("Verbindung zum Importdienst fehlgeschlagen. Bitte später versuchen.");
  });
  if (!response.ok) {
    const messages: Record<number, string> = {
      400: "Ungültiger Schlüssel. Vollständigen sr- oder cr-Schlüssel eingeben, keine URL.",
      422: "Berichtstyp, Inhalt oder Teilnehmerzuordnung wird nicht unterstützt.",
      429: "Import- oder Proxy-Quote erreicht. Mindestens eine Minute warten und später erneut versuchen.",
      502: "Bericht abgelaufen, nicht verfügbar oder Proxy nicht erreichbar. Schlüssel prüfen oder später versuchen.",
    };
    throw new Error(messages[response.status] ?? "Reportimport derzeit nicht verfügbar.");
  }
  const value: unknown = await response.json().catch(() => undefined);
  if (!isCandidate(value)) throw new Error("Ungültige Importantwort. Planung bleibt erhalten.");
  return value;
}
/** Map sanitized evidence only; unknown and ambiguous modifiers remain blank. */
export function reportFleet(candidate: ReportCandidate): Fleet {
  const participant = candidate.defenders[0];
  if (!participant || candidate.defenders.length !== 1)
    throw new Error("Teilnehmerzuordnung nicht unterstützt.");
  const fleet = emptyScenario().defender;
  // Candidate class IDs follow the report completion contract, not AGR's enum order.
  fleet.playerClass =
    (["none", "collector", "general", "discoverer"] as const)[
      participant.character_class_id ?? 0
    ] ?? "none";
  fleet.allianceClass =
    (["none", "trader", "warrior", "researcher"] as const)[participant.alliance_class_id ?? 0] ??
    "none";
  const composition = participant.entities ?? { ...participant.ships, ...participant.defenses };
  for (const [id, count] of Object.entries(composition)) {
    if (!availableTypes(false).some((unit) => unit.id === id))
      throw new Error("Bericht enthält nicht unterstützte Einheiten.");
    fleet.units.push({
      id,
      selected: String(count),
      available: String(count),
      lifeform: blankNumbers(),
    });
  }
  if (participant.technology.basis === "researched") {
    for (const stat of ["weapon", "shield", "armour"] as const) {
      const level = participant.technology[stat];
      fleet.technology[stat] = level === null ? "" : String(level);
    }
  }
  return fleet;
}

function isCandidate(value: unknown): value is ReportCandidate {
  if (typeof value !== "object" || value === null) return false;
  const data = value as Partial<ReportCandidate>;
  const source = data.provenance;
  if (
    !source ||
    typeof source.source !== "string" ||
    typeof source.community !== "string" ||
    !Number.isInteger(source.universe) ||
    !(
      source.event_timestamp === null ||
      (typeof source.event_timestamp === "number" && Number.isFinite(source.event_timestamp))
    ) ||
    !(source.game_version === null || typeof source.game_version === "string") ||
    !["combat", "espionage"].includes(data.report_kind ?? "") ||
    !Array.isArray(data.defenders) ||
    data.defenders.length !== 1
  )
    return false;
  return data.defenders.every((entry: unknown) => {
    if (typeof entry !== "object" || entry === null) return false;
    const participant = entry as Partial<Participant>;
    if (typeof participant.technology?.basis !== "string") return false;
    if (
      ![participant.character_class_id, participant.alliance_class_id].every(
        (value) =>
          value === null ||
          (typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 3),
      )
    )
      return false;
    const compositions = [participant.entities, participant.ships, participant.defenses];
    return (
      compositions.every(
        (entry) =>
          entry === null ||
          (typeof entry === "object" &&
            !Array.isArray(entry) &&
            Object.values(entry).every(
              (count) => Number.isInteger(count) && count >= 0 && count <= 4294967295,
            )),
      ) &&
      [
        participant.technology.weapon,
        participant.technology.shield,
        participant.technology.armour,
      ].every((level) => level === null || (Number.isInteger(level) && level >= 0 && level <= 255))
    );
  });
}
